//! Single-line and multi-line engineering: status of the devices in both
//! representations and the transformation between them.

use crate::builtin::common::{page_index, unlocked, PAGE};
use crate::params::Args;
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::index::Index;
use openecraft_core::model::Project;
use openecraft_core::singleline::{self, Generated, Status};
use openecraft_i18n::tr_const;
use serde_json::{json, Value};

pub struct SingleLine;

impl Plugin for SingleLine {
    fn info(&self) -> Info {
        info("single_line", tr_const!("Single-line diagrams"), tr_const!("Single-line pages and their multi-line counterparts"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "list_single_line",
        help: tr_const!("Lists the devices of the single-line diagram and the power devices of the schematic with their status: planned, engineered, not in the single-line diagram or not matching"),
        params: &[],
        run: list_single_line,
    },
    Spec {
        name: "single_line_to_schematic",
        help: tr_const!("Generates the multi-line schematic (power symbols, conductors, contactor coils) for the single-line symbols of a page on a new page after it"),
        params: &[PAGE],
        run: to_schematic,
    },
    Spec {
        name: "schematic_to_single_line",
        help: tr_const!("Generates a single-line diagram of the schematic's power devices that have none yet on a new overview page after a page"),
        params: &[PAGE],
        run: to_single_line,
    },
    Spec {
        name: "sync_single_line",
        help: tr_const!("Copies function texts between the single-line diagram and the schematic where one side is empty"),
        params: &[],
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

fn list_single_line(prj: &mut Project, _: &Args) -> Result<Value, String> {
    let idx = Index::build(prj);
    let side = |x: &Option<(String, (usize, u64))>| x.as_ref().map(|(key, (page, id))| json!({ "symbol": key, "page": prj.pages[*page].name, "id": id }));
    let devices: Vec<Value> = singleline::devices(prj, &idx)
        .iter()
        .map(|d| json!({ "tag": d.tag, "single_line": side(&d.single), "schematic": side(&d.multi), "status": status_code(d.status) }))
        .collect();
    Ok(Value::Array(devices))
}

fn generated(prj: &Project, g: Generated) -> Result<Value, String> {
    let page = &prj.pages[g.page];
    Ok(json!({ "page": page.name, "symbols": g.symbols, "wires": g.wires, "skipped": g.skipped }))
}

fn to_schematic(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let page = page_index(prj, a.req("page"))?;
    let g = singleline::to_multiline(prj, page)?;
    generated(prj, g)
}

fn to_single_line(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let page = unlocked(prj, a.req("page"))?;
    let g = singleline::to_singleline(prj, page)?;
    generated(prj, g)
}

fn sync_single_line(prj: &mut Project, _: &Args) -> Result<Value, String> {
    let copied = singleline::synchronize(prj);
    Ok(json!({ "texts_copied": copied }))
}
