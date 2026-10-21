//! Device tools: list, rename, assign parts.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::index::Index;
use openecraft_core::ops;
use openecraft_core::parts::{self, PartsDb};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "list_devices",
        help: tr_const!("Lists the devices with cross references and assigned parts"),
        params: &[],
        needs_project: true,
        run: list_devices,
    },
    Tool {
        name: "rename_device",
        help: tr_const!("Renames a device with all its functions"),
        params: &[
            param("old", Kind::Str, tr_const!("Current device tag")),
            param("new", Kind::Str, tr_const!("New device tag")),
        ],
        needs_project: true,
        run: rename_device,
    },
    Tool {
        name: "assign_part",
        help: tr_const!("Assigns a part to a device (from the project, the built-in catalog or the parts server)"),
        params: &[
            param("tag", Kind::Str, tr_const!("Device tag")),
            param("number", Kind::Str, tr_const!("Part number")),
            optional("count", Kind::Int, tr_const!("Quantity (default 1)")),
            optional("replace", Kind::Bool, tr_const!("Remove the parts assigned so far")),
        ],
        needs_project: true,
        run: assign_part,
    },
];

fn list_devices(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let idx = Index::build(prj);
    let refs = |locs: &[openecraft_core::index::Loc]| locs.iter().map(|l| l.reference(prj)).collect::<Vec<_>>();
    let list = idx.sorted_tags().into_iter().map(|tag| {
        let e = &idx.devices[tag];
        let data = prj.devices.get(tag);
        let parts: Vec<Value> = data.map(|d| d.parts.iter().map(|r| json!({ "number": r.number, "count": r.count })).collect()).unwrap_or_default();
        json!({
            "tag": tag,
            "description": data.map(|d| d.description.as_str()).unwrap_or_default(),
            "parts": parts,
            "main": refs(&e.mains),
            "auxiliary": refs(&e.auxs),
            "other": refs(&e.others),
            "overview": refs(&e.overviews),
        })
    });
    Ok(Value::Array(list.collect()))
}

fn rename_device(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let (old, new) = (a.req("old"), a.req("new").trim());
    let tags = prj.all_tags();
    if !tags.iter().any(|t| t == old) {
        return Err(tr_fmt!("No device {}", old));
    }
    if new.is_empty() || tags.iter().any(|t| t == new) {
        return Err(tr_fmt!("Device {} already exists", new));
    }
    ops::rename_device(prj, old, new);
    Ok(json!({ "old": old, "new": new }))
}

fn assign_part(s: &mut Session, a: &Args) -> Result<Value, String> {
    let (tag, number) = (a.req("tag"), a.req("number"));
    let prj = s.project()?;
    if !prj.all_tags().iter().any(|t| t == tag) {
        return Err(tr_fmt!("No device {}", tag));
    }
    let mut db = PartsDb::default();
    let known = parts::lookup(prj, Some(&db), number).cloned();
    let part = match known {
        Some(p) => p,
        None => s.server.get(number).map_err(|e| tr_fmt!("Unknown part {} (parts server: {})", number, e))?.ok_or_else(|| tr_fmt!("Unknown part: {}", number))?,
    };
    db.upsert(part);
    let prj = s.project_mut()?;
    if a.flag("replace") {
        prj.devices.entry(tag.to_string()).or_default().parts.clear();
    }
    parts::add_to_device(prj, &db, tag, number, a.int("count").unwrap_or(1).max(1) as u32);
    let assigned: Vec<Value> = prj.devices[tag].parts.iter().map(|r| json!({ "number": r.number, "count": r.count })).collect();
    let designated = parts::apply_designations(prj, Some(&db), tag);
    Ok(json!({ "tag": tag, "parts": assigned, "designations_applied": designated }))
}

#[cfg(test)]
mod tests;
