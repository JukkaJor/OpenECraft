//! Single-line and multi-line engineering: status of the devices in both
//! representations and the transformation between them.

use super::common::{page_index, unlocked, PAGE};
use super::{Args, Tool};
use crate::session::Session;
use openecraft_core::index::Index;
use openecraft_core::singleline::{self, Generated, Status};
use openecraft_i18n::tr_const;
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "list_single_line",
        help: tr_const!("Lists the devices of the single-line diagram and the power devices of the schematic with their status: planned, engineered, not in the single-line diagram or not matching"),
        params: &[],
        needs_project: true,
        run: list_single_line,
    },
    Tool {
        name: "single_line_to_schematic",
        help: tr_const!("Generates the multi-line schematic (power symbols, conductors, contactor coils) for the single-line symbols of a page on a new page after it"),
        params: &[PAGE],
        needs_project: true,
        run: to_schematic,
    },
    Tool {
        name: "schematic_to_single_line",
        help: tr_const!("Generates a single-line diagram of the schematic's power devices that have none yet on a new overview page after a page"),
        params: &[PAGE],
        needs_project: true,
        run: to_single_line,
    },
    Tool {
        name: "sync_single_line",
        help: tr_const!("Copies function texts between the single-line diagram and the schematic where one side is empty"),
        params: &[],
        needs_project: true,
        run: sync_single_line,
    },
];

fn status_code(s: Status) -> &'static str {
    match s {
        Status::Planned => "planned",
        Status::Engineered => "engineered",
        Status::NotInSingleLine => "not_in_single_line",
        Status::Mismatch => "mismatch",
    }
}

fn list_single_line(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let idx = Index::build(prj);
    let side = |x: &Option<(String, (usize, u64))>| x.as_ref().map(|(key, (page, id))| json!({ "symbol": key, "page": prj.pages[*page].name, "id": id }));
    let devices: Vec<Value> = singleline::devices(prj, &idx)
        .iter()
        .map(|d| json!({ "tag": d.tag, "single_line": side(&d.single), "schematic": side(&d.multi), "status": status_code(d.status) }))
        .collect();
    Ok(Value::Array(devices))
}

fn generated(s: &Session, g: Generated) -> Result<Value, String> {
    let page = &s.project()?.pages[g.page];
    Ok(json!({ "page": page.name, "symbols": g.symbols, "wires": g.wires, "skipped": g.skipped }))
}

fn to_schematic(s: &mut Session, a: &Args) -> Result<Value, String> {
    let page = page_index(s.project()?, a.req("page"))?;
    let g = singleline::to_multiline(s.project_mut()?, page)?;
    generated(s, g)
}

fn to_single_line(s: &mut Session, a: &Args) -> Result<Value, String> {
    let page = unlocked(s.project()?, a.req("page"))?;
    let g = singleline::to_singleline(s.project_mut()?, page)?;
    generated(s, g)
}

fn sync_single_line(s: &mut Session, _: &Args) -> Result<Value, String> {
    let copied = singleline::synchronize(s.project_mut()?);
    Ok(json!({ "texts_copied": copied }))
}

#[cfg(test)]
mod tests;
