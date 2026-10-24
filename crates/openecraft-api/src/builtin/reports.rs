//! Report block and scheme commands: report pages generated per strip, plug, cable or
//! device, named report forms (filter, sort order, grouping) and the output of report
//! pages into other projects.

use crate::params::{param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::model::Project;
use openecraft_core::report_blocks::{self, ReportBlock};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct ReportBlocks;

impl Plugin for ReportBlocks {
    fn info(&self) -> Info {
        info("report_blocks", tr_const!("Report blocks"), tr_const!("Report blocks and report schemes"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "list_report_blocks",
        help: tr_const!("Lists the report blocks with their report, scheme, groups and generated pages"),
        params: &[],
        run: list_report_blocks,
    },
    Spec {
        name: "remove_report_block",
        help: tr_const!("Removes a report block; the pages it generated stay"),
        params: &[param("name", Kind::Str, tr_const!("Name of the block"))],
        run: remove_report_block,
    },
    Spec {
        name: "list_report_schemes",
        help: tr_const!("Lists the report schemes (named report forms) with their settings"),
        params: &[],
        run: list_report_schemes,
    },
];

pub fn block_json(prj: &openecraft_core::model::Project, b: &ReportBlock) -> Value {
    let pages: Vec<String> = report_blocks::report_pages(prj, Some(&b.name)).into_iter().map(|i| prj.pages[i].name.clone()).collect();
    json!({
        "name": b.name,
        "kind": b.kind.as_str(),
        "scheme": b.scheme,
        "per_group": b.per_group,
        "groups": b.groups,
        "function": b.function,
        "location": b.location,
        "pages": pages,
    })
}

fn list_report_blocks(prj: &mut Project, _: &Args) -> Result<Value, String> {
    Ok(Value::Array(prj.report_blocks.iter().map(|b| block_json(prj, b)).collect()))
}

fn remove_report_block(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let name = a.req("name");
    let i = prj.report_blocks.iter().position(|b| b.name == name).ok_or_else(|| tr_fmt!("No report block named {}", name))?;
    prj.report_blocks.remove(i);
    Ok(json!({ "removed": name }))
}

fn list_report_schemes(prj: &mut Project, _: &Args) -> Result<Value, String> {
    Ok(Value::Array(prj.forms.schemes.iter().map(|s| json!({ "name": s.name, "kind": s.kind.as_str(), "form": s.form })).collect()))
}
