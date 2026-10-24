//! Result tools: project check, reports, wire numbering and export.

use super::common::*;
use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_api::params::{self, text};
use openecraft_core::reports::{self, Analysis, Severity};
use openecraft_core::{translation, wirenum};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Map, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "check",
        help: tr_const!("Checks the project (errors, warnings, hints)"),
        params: &[],
        needs_project: true,
        run: check,
    },
    Tool {
        name: "list_reports",
        help: tr_const!("Lists the reports (built-in and of plugins): name for report and title"),
        params: &[],
        needs_project: false,
        run: list_reports,
    },
    Tool {
        name: "report",
        help: tr_const!("Generates a report as table"),
        params: &[
            param("kind", Kind::Str, tr_const!("Report name as shown by list_reports")),
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
        name: "list_exports",
        help: tr_const!("Lists the exports (built-in and of plugins): name for export, title, file extension and parameters"),
        params: &[],
        needs_project: false,
        run: list_exports,
    },
    Tool {
        name: "export",
        help: tr_const!("Exports the project in a format of list_exports; binary formats (dwg, jt) need an output file"),
        params: &[
            param("format", Kind::Str, tr_const!("Export name as shown by list_exports")),
            optional("language", Kind::Str, tr_const!("Language of the project texts and reports (a project language, default the source language)")),
            optional("path", Kind::Str, tr_const!("Output file; without it the content is returned")),
            optional("page", Kind::Str, tr_const!("Page (svg: required, dxf, dwg: default all pages)")),
        ],
        needs_project: true,
        run: export,
    },
];

fn check(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let mut an = Analysis::build(prj);
    an.issues = s.plugins.check(prj, &an);
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

fn list_reports(s: &mut Session, _: &Args) -> Result<Value, String> {
    Ok(Value::Array(s.plugins.reports().map(|r| json!({ "name": r.name, "title": text(&r.title) })).collect()))
}

fn report(s: &mut Session, a: &Args) -> Result<Value, String> {
    let id = report_id(s, a.req("kind"))?;
    let prj = s.project()?;
    let mut an = Analysis::build(prj);
    an.set_reports(s.plugins.reports().cloned());
    let title = s.plugins.report(id.as_str()).map(|r| text(&r.title).to_string()).unwrap_or_else(|| id.title());
    let mut table = an.report(prj, id).map(|t| (*t).clone()).unwrap_or_default();
    if let Some(name) = a.str("scheme") {
        let scheme = prj.forms.schemes.iter().find(|s| s.kind == id && s.name == name).ok_or_else(|| tr_fmt!("Unknown report scheme: {}", name))?;
        let t = openecraft_core::report_blocks::scheme_table(&table, Some(&scheme.form));
        let cols = openecraft_core::forms::report_columns(Some(&scheme.form), &t.headers);
        table = reports::Table {
            headers: cols.iter().map(|&c| t.headers[c].clone()).collect(),
            widths: cols.iter().map(|&c| t.widths[c]).collect(),
            rows: t.rows.iter().map(|r| cols.iter().map(|&c| r.get(c).cloned().unwrap_or_default()).collect()).collect(),
            targets: t.targets,
        };
    }
    match a.str("format").unwrap_or("json") {
        "csv" => Ok(Value::String(table.to_csv())),
        "json" => Ok(json!({ "report": id.as_str(), "title": title, "headers": table.header_texts(), "rows": table.rows })),
        other => Err(tr_fmt!("Unknown format: {}", other)),
    }
}

fn renumber_wires(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let an = Analysis::build(prj);
    let n = wirenum::renumber(prj, &an.open, &an.netlist, a.flag("overwrite_fixed"));
    Ok(json!({ "renumbered": n }))
}

fn list_exports(s: &mut Session, _: &Args) -> Result<Value, String> {
    let list: Vec<Value> = s
        .plugins
        .exporters()
        .map(|e| json!({ "name": e.name, "title": text(&e.title), "extension": e.extension, "binary": e.binary, "params": e.params.iter().map(|p| p.name.as_ref()).collect::<Vec<_>>() }))
        .collect();
    Ok(Value::Array(list))
}

/// Runs an exporter of the registry: `page` and the file name of `path` are passed
/// on if the exporter takes them; the texts are generated in `language`.
fn export(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let name = a.req("format");
    let e = s.plugins.exporter(name).cloned().ok_or_else(|| tr_fmt!("Unknown format: {}", name))?;
    let lang = match a.str("language") {
        Some(l) => {
            let code = translation::normalize_code(l).filter(|c| prj.languages.all().contains(c));
            code.ok_or_else(|| tr_fmt!("{} is not a language of the project", l))?
        }
        None => prj.languages.source.clone(),
    };
    let takes = |n: &str| e.params.iter().any(|p| p.name == n);
    let mut args = Map::new();
    if let Some(page) = a.str("page").filter(|_| takes("page")) {
        args.insert("page".into(), page.into());
    }
    if let Some(path) = a.str("path").filter(|_| takes("file")) {
        args.insert("file".into(), path.rsplit(['/', '\\']).next().unwrap_or(path).into());
    }
    params::check(&e.params, &args)?;
    let out = translation::with_language(prj, &lang, |prj| {
        let mut an = Analysis::build(prj);
        an.set_reports(s.plugins.reports().cloned());
        (e.run)(prj, &an, &Args(&args))
    })?;
    match a.str("path") {
        Some(path) => {
            let content = out.into_bytes();
            std::fs::write(path, &content).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
            Ok(json!({ "path": path, "bytes": content.len() }))
        }
        None => out.text().map(Value::String).ok_or_else(|| tr("This format needs an output file (path)").to_string()),
    }
}

#[cfg(test)]
mod tests;
