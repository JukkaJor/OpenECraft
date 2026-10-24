//! Result tools: project check, reports, wire numbering and export.

use super::common::*;
use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_api::params::text;
use openecraft_core::reports::{self, Analysis, Severity};
use openecraft_core::{dxf, harness, pdf, plc, svg, translation, wirenum};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "check",
        help: tr_const!("Checks the project (errors, warnings, hints)"),
        params: &[],
        needs_project: true,
        run: check,
    },
    Tool {
        name: "report",
        help: tr_const!("Generates a report as table"),
        params: &[
            param("kind", Kind::Str, tr_const!("table_of_contents, device_list, bill_of_materials, terminal_diagram, terminal_lineup, terminal_strip_overview, terminal_connections, plug_diagram, pin_connections, connection_list, device_connections, plc_assignment, potential_list, panel_wiring, cable_overview, cable_diagram, cable_connections, cable_sizing, revision_overview, harness_data, duct_fill, panel_legend, climate, marking, hose_list, pct_loops, fluid_netlist, hose_configuration, tubing_list, fluid_device_groups or electro_fluid, or a report of a plugin")),
            optional("format", Kind::Str, tr_const!("json (default) or csv")),
            optional("scheme", Kind::Str, tr_const!("Report scheme: its filter, sort order and columns")),
        ],
        needs_project: true,
        run: report,
    },
    Tool {
        name: "renumber_wires",
        help: tr_const!("Numbers the wires by the numbering scheme of the project"),
        params: &[optional("overwrite_fixed", Kind::Bool, tr_const!("Also renumber wires with a fixed number"))],
        needs_project: true,
        run: renumber_wires,
    },
    Tool {
        name: "export",
        help: tr_const!("Exports the project: svg (one page), html, pdf, dxf, wire_list (CSV), plc_table (CSV), plc_variables (IEC 61131-3 global variables), aml (AutomationML PLC hardware), kbl (harness XML), marking (CSV), gltf, step, iges, ifc or jt (3D panel layout; jt only to a file), dictionary (CSV of the project text translations)"),
        params: &[
            param("format", Kind::Str, tr_const!("svg, html, pdf, dxf, wire_list, plc_table, plc_variables, aml, kbl, marking, gltf, step, iges, ifc, jt or dictionary, or an export of a plugin")),
            optional("language", Kind::Str, tr_const!("svg, html, pdf, dxf: language of the project texts and reports (a project language, default the source language)")),
            optional("path", Kind::Str, tr_const!("Output file; without it the content is returned")),
            optional("page", Kind::Str, tr_const!("Page (svg: required, dxf: default all pages)")),
        ],
        needs_project: true,
        run: export,
    },
];

fn check(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let mut an = Analysis::build(prj);
    an.issues.extend(s.plugins.check(prj, &an));
    an.issues.sort_by_key(|i| i.severity);
    let issues: Vec<Value> = an
        .issues
        .iter()
        .map(|i| {
            json!({
                "severity": i.severity.key(),
                "message": i.message,
                "page": i.target.map(|(pg, _)| prj.pages[pg].name.clone()),
                "id": i.target.map(|(_, id)| id).filter(|id| *id != 0),
            })
        })
        .collect();
    let count = |sev| an.issues.iter().filter(|i| i.severity == sev).count();
    Ok(json!({ "errors": count(Severity::Error), "warnings": count(Severity::Warning), "issues": issues }))
}

fn report(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    if let Some(r) = s.plugins.report(a.req("kind")) {
        let table = (r.run)(prj, &Analysis::build(prj))?;
        return match a.str("format").unwrap_or("json") {
            "csv" => Ok(Value::String(table.to_csv())),
            "json" => Ok(json!({ "report": r.name, "title": text(&r.title), "headers": table.header_texts(), "rows": table.rows })),
            other => Err(tr_fmt!("Unknown format: {}", other)),
        };
    }
    let kind = report_kind(a.req("kind"))?;
    let an = Analysis::build(prj);
    let mut table = an.reports.get(&kind).cloned().unwrap_or_default();
    if let Some(name) = a.str("scheme") {
        let scheme = prj.forms.schemes.iter().find(|s| s.kind == kind && s.name == name).ok_or_else(|| tr_fmt!("Unknown report scheme: {}", name))?;
        let t = openecraft_core::report_blocks::scheme_table(&table, Some(&scheme.form));
        let cols = openecraft_core::forms::report_columns(Some(&scheme.form), &t.headers);
        table = reports::Table {
            headers: cols.iter().map(|&c| t.headers[c]).collect(),
            widths: cols.iter().map(|&c| t.widths[c]).collect(),
            rows: t.rows.iter().map(|r| cols.iter().map(|&c| r.get(c).cloned().unwrap_or_default()).collect()).collect(),
            targets: t.targets,
        };
    }
    match a.str("format").unwrap_or("json") {
        "csv" => Ok(Value::String(table.to_csv())),
        "json" => Ok(json!({ "report": snake(kind), "title": kind.title(), "headers": table.headers, "rows": table.rows })),
        other => Err(tr_fmt!("Unknown format: {}", other)),
    }
}

fn renumber_wires(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let an = Analysis::build(prj);
    let n = wirenum::renumber(prj, &an.open, &an.netlist, a.flag("overwrite_fixed"));
    Ok(json!({ "renumbered": n }))
}

fn export(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let page = a.str("page").map(|n| page_index(prj, n)).transpose()?;
    let lang = match a.str("language") {
        Some(l) => {
            let code = translation::normalize_code(l).filter(|c| prj.languages.all().contains(c));
            code.ok_or_else(|| tr_fmt!("{} is not a language of the project", l))?
        }
        None => prj.languages.source.clone(),
    };
    let content = match s.plugins.exporter(a.req("format")) {
        Some(e) => translation::with_language(prj, &lang, |prj| (e.run)(prj))?.into_bytes(),
        None => translation::with_language(prj, &lang, |prj| export_content(prj, a, page))?,
    };
    match a.str("path") {
        Some(path) => {
            std::fs::write(path, &content).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
            Ok(json!({ "path": path, "bytes": content.len() }))
        }
        None => String::from_utf8(content).map(Value::String).map_err(|_| tr("This format needs an output file (path)").to_string()),
    }
}

fn export_content(prj: &openecraft_core::model::Project, a: &Args, page: Option<usize>) -> Result<Vec<u8>, String> {
    let file_name = |default: &'static str| a.str("path").map(|p| p.rsplit(['/', '\\']).next().unwrap_or(p)).unwrap_or(default).to_string();
    Ok(match a.req("format") {
        "svg" => svg::page_svg(prj, page.ok_or_else(|| tr("This format needs a page").to_string())?),
        "html" => svg::print_document(prj),
        "pdf" => pdf::document(prj),
        "dxf" => {
            let pages: Vec<usize> = page.map(|p| vec![p]).unwrap_or_else(|| (0..prj.pages.len()).collect());
            dxf::pages_dxf(prj, &pages)
        }
        "wire_list" => reports::production_wire_list(prj, &Analysis::build(prj)).to_csv(),
        "plc_table" => plc::symbol_table(prj, &Analysis::build(prj).netlist),
        "plc_variables" => plc::global_variables(prj, &Analysis::build(prj).netlist),
        "aml" => openecraft_core::aml::export(prj, &Analysis::build(prj).netlist),
        "kbl" => harness::kbl(prj, &Analysis::build(prj)),
        "marking" => openecraft_core::marking::csv(&openecraft_core::marking::markers(prj, &Analysis::build(prj).connections)),
        "gltf" => openecraft_core::export3d::gltf(prj),
        "step" => openecraft_core::export3d::step(prj, &file_name("panel.stp")),
        "iges" => openecraft_core::export3d::iges(prj, &file_name("panel.igs")),
        "ifc" => openecraft_core::export3d::ifc(prj, &file_name("panel.ifc")),
        "jt" => return Ok(openecraft_core::export3d::jt(prj)),
        "dictionary" => translation::to_csv(prj),
        other => return Err(tr_fmt!("Unknown format: {}", other)),
    }
    .into_bytes())
}

#[cfg(test)]
mod tests;
