//! Result tools: project check, reports, wire numbering and export.

use super::common::*;
use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
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
            param("kind", Kind::Str, tr_const!("table_of_contents, device_list, bill_of_materials, terminal_diagram, plug_diagram, connection_list, plc_assignment, potential_list, panel_wiring, cable_overview, cable_diagram, cable_sizing, revision_overview, harness_data, duct_fill, climate, marking, hose_list or pct_loops")),
            optional("format", Kind::Str, tr_const!("json (default) or csv")),
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
        help: tr_const!("Exports the project: svg (one page), html, pdf, dxf, wire_list (CSV), plc_table (CSV), kbl (harness XML), marking (CSV), gltf or step (3D panel layout), dictionary (CSV of the project text translations)"),
        params: &[
            param("format", Kind::Str, tr_const!("svg, html, pdf, dxf, wire_list, plc_table, kbl, marking, gltf, step or dictionary")),
            optional("language", Kind::Str, tr_const!("svg, html, pdf, dxf: language of the project texts and reports (a project language, default the source language)")),
            optional("path", Kind::Str, tr_const!("Output file; without it the content is returned")),
            optional("page", Kind::Str, tr_const!("Page (svg: required, dxf: default all pages)")),
        ],
        needs_project: true,
        run: export,
    },
];

fn severity_name(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn check(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let an = Analysis::build(prj);
    let issues: Vec<Value> = an
        .issues
        .iter()
        .map(|i| {
            json!({
                "severity": severity_name(i.severity),
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
    let kind = report_kind(a.req("kind"))?;
    let an = Analysis::build(prj);
    let table = an.reports.get(&kind).cloned().unwrap_or_default();
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
    let content = translation::with_language(prj, &lang, |prj| export_content(prj, a, page))?;
    match a.str("path") {
        Some(path) => {
            std::fs::write(path, &content).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
            Ok(json!({ "path": path, "bytes": content.len() }))
        }
        None => Ok(Value::String(content)),
    }
}

fn export_content(prj: &openecraft_core::model::Project, a: &Args, page: Option<usize>) -> Result<String, String> {
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
        "kbl" => harness::kbl(prj, &Analysis::build(prj)),
        "marking" => openecraft_core::marking::csv(&openecraft_core::marking::markers(prj, &Analysis::build(prj).connections)),
        "gltf" => openecraft_core::export3d::gltf(prj),
        "step" => openecraft_core::export3d::step(prj, a.str("path").map(|p| p.rsplit(['/', '\\']).next().unwrap_or(p)).unwrap_or("panel.stp")),
        "dictionary" => translation::to_csv(prj),
        other => return Err(tr_fmt!("Unknown format: {}", other)),
    })
}

#[cfg(test)]
mod tests;
