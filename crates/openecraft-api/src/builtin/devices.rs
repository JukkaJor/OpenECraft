//! Device commands: list, rename, assign parts.

use crate::builtin::common::page_index;
use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::index::Index;
use openecraft_core::model::Project;
use openecraft_core::numbering::{self, DeviceNumbering};
use openecraft_core::ops;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct Devices;

impl Plugin for Devices {
    fn info(&self) -> Info {
        info("devices", tr_const!("Devices"), tr_const!("Lists and renames devices"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "list_devices",
        help: tr_const!("Lists the devices with cross references and assigned parts"),
        params: &[],
        run: list_devices,
    },
    Spec {
        name: "rename_device",
        help: tr_const!("Renames a device with all its functions"),
        params: &[
            param("old", Kind::Str, tr_const!("Current device tag")),
            param("new", Kind::Str, tr_const!("New device tag")),
        ],
        run: rename_device,
    },
    Spec {
        name: "renumber_devices",
        help: tr_const!("Numbers the device tags per structure and identifying letter in schematic order (page, column, top to bottom)"),
        params: &[
            optional("format", Kind::Str, tr_const!("Format of the device name: {letter}, {n} (counter), {page}, {column}, {row}; default {letter}{n}")),
            optional("start", Kind::Int, tr_const!("First counter value (default 1)")),
            optional("step", Kind::Int, tr_const!("Counting step (default 1)")),
            optional("pages", Kind::Str, tr_const!("Only devices on these pages (names separated by commas)")),
            optional("letters", Kind::Str, tr_const!("Only these identifying letters (separated by commas, e.g. K,Q)")),
            optional("terminals", Kind::Bool, tr_const!("Also renumber terminal strips, plugs and PLC cards")),
            optional("dry_run", Kind::Bool, tr_const!("Only list the new tags, change nothing")),
        ],
        run: renumber_devices,
    },
];

/// Page indices of a comma separated list of page names.
pub(crate) fn page_list(prj: &Project, list: Option<&str>) -> Result<Vec<usize>, String> {
    list.unwrap_or_default().split(',').map(str::trim).filter(|s| !s.is_empty()).map(|name| page_index(prj, name)).collect()
}

fn renumber_devices(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let defaults = DeviceNumbering::default();
    let opts = DeviceNumbering {
        format: a.str("format").filter(|f| !f.trim().is_empty()).map(str::to_string).unwrap_or(defaults.format),
        start: a.int("start").map_or(defaults.start, |v| v.max(0) as u32),
        step: a.int("step").map_or(defaults.step, |v| v.max(1) as u32),
        pages: page_list(prj, a.str("pages"))?,
        letters: a.str("letters").unwrap_or_default().split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect(),
        skip_terminals: !a.flag("terminals"),
    };
    let renamed = if a.flag("dry_run") { numbering::plan_devices(prj, &opts) } else { numbering::renumber_devices(prj, &opts) };
    Ok(json!({ "renamed": renamed.iter().map(|(old, new)| json!({ "old": old, "new": new })).collect::<Vec<_>>() }))
}

fn list_devices(prj: &mut Project, _: &Args) -> Result<Value, String> {
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

fn rename_device(prj: &mut Project, a: &Args) -> Result<Value, String> {
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
