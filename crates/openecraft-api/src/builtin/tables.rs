//! Table commands: devices, connections and terminals as rows to read and change in bulk,
//! and the CSV round trip.

use crate::params::{optional, param, Args, Kind, Param};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::model::Project;
use openecraft_core::netlist::Netlist;
use openecraft_core::tables::{self, TableKind};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct Tables;

impl Plugin for Tables {
    fn info(&self) -> Info {
        info("tables", tr_const!("Tables"), tr_const!("Changes the device, connection and terminal tables"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "table_set",
        help: tr_const!("Changes a cell of a table: device tag, function text, description, part; wire number, colour, cross-section, cable, core; strip, terminal"),
        params: &[KIND, param("key", Kind::Str, tr_const!("Row key (from table_get)")), param("column", Kind::Str, tr_const!("Column name, e.g. Description")), param("value", Kind::Str, tr_const!("New value"))],
        run: table_set,
    },
    Spec {
        name: "table_replace",
        help: tr_const!("Finds and replaces text in a column of all (filtered) rows; without find the column is filled with the replacement"),
        params: &[
            KIND,
            param("column", Kind::Str, tr_const!("Column name, e.g. Description")),
            optional("find", Kind::Str, tr_const!("Text to find; empty: fill")),
            param("replace", Kind::Str, tr_const!("Replacement")),
            optional("filter", Kind::Str, tr_const!("Only rows containing this text")),
        ],
        run: table_replace,
    },
];

pub const KIND: Param = param("table", Kind::Str, tr_const!("devices, connections or terminals"));

pub fn kind(a: &Args) -> Result<TableKind, String> {
    TableKind::from_key(a.req("table")).ok_or_else(|| tr_fmt!("Unknown table: {}", a.req("table")))
}

fn column(kind: TableKind, name: &str) -> Result<usize, String> {
    kind.column(name).ok_or_else(|| tr_fmt!("Unknown column: {}", name))
}

fn table_set(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let kind = kind(a)?;
    let col = column(kind, a.req("column"))?;
    let open = Netlist::build(prj, false);
    let changed = tables::set_cell(prj, &open, kind, a.req("key"), col, a.req("value"))?;
    Ok(json!({ "changed": changed }))
}

fn table_replace(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let kind = kind(a)?;
    let col = column(kind, a.req("column"))?;
    let (nl, open) = (Netlist::build(prj, true), Netlist::build(prj, false));
    let rows: Vec<tables::Row> = tables::rows(prj, &nl, &open, kind).into_iter().filter(|r| tables::matches(r, a.str("filter").unwrap_or(""))).collect();
    let (n, problems) = tables::replace(prj, &open, kind, &rows, col, a.str("find").unwrap_or(""), a.req("replace"));
    Ok(json!({ "changed": n, "problems": problems }))
}
