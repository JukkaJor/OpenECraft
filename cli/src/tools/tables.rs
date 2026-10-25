//! Table tools: devices, connections and terminals as rows to read and change in bulk,
//! and the CSV round trip.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::netlist::Netlist;
use openecraft_core::tables::{self};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Map, Value};
use openecraft_api::builtin::tables::{KIND, kind};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "table_get",
        help: tr_const!("Shows a table (devices, connections, terminals, cables, plugs, PLC, pages or parts) with its rows; the key of a row is used to change it"),
        params: &[KIND, optional("filter", Kind::Str, tr_const!("Only rows containing this text")), optional("path", Kind::Str, tr_const!("Write the table as CSV to this file"))],
        needs_project: true,
        run: table_get,
    },
    Tool {
        name: "table_import",
        help: tr_const!("Applies a CSV exported with table_get (and changed in a spreadsheet): the changed cells are set"),
        params: &[KIND, param("path", Kind::Str, tr_const!("CSV file"))],
        needs_project: true,
        run: table_import,
    },
];

fn table_get(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let kind = kind(a)?;
    let rows: Vec<tables::Row> =
        tables::rows(prj, &Netlist::build(prj, true), &Netlist::build(prj, false), kind).into_iter().filter(|r| tables::matches(r, a.str("filter").unwrap_or(""))).collect();
    if let Some(path) = a.str("path") {
        let csv = tables::to_csv(kind, &rows);
        std::fs::write(path, &csv).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
        return Ok(json!({ "path": path, "rows": rows.len() }));
    }
    let columns = kind.columns();
    let rows: Vec<Value> = rows
        .iter()
        .map(|r| {
            let mut m = Map::new();
            m.insert("key".into(), json!(r.key));
            for (c, cell) in columns.iter().zip(&r.cells) {
                m.insert(c.name.into(), json!(cell));
            }
            if r.locked {
                m.insert("locked".into(), json!(true));
            }
            Value::Object(m)
        })
        .collect();
    Ok(json!({ "columns": columns.iter().map(|c| json!({ "name": c.name, "editable": c.editable })).collect::<Vec<_>>(), "rows": rows }))
}

fn table_import(s: &mut Session, a: &Args) -> Result<Value, String> {
    let kind = kind(a)?;
    let path = a.req("path");
    let text = std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let prj = s.project_mut()?;
    let (nl, open) = (Netlist::build(prj, true), Netlist::build(prj, false));
    let (n, problems) = tables::apply_csv(prj, &nl, &open, kind, &text)?;
    Ok(json!({ "changed": n, "problems": problems }))
}

#[cfg(test)]
mod tests;
