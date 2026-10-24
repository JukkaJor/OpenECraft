//! Device commands: list, rename, assign parts.

use crate::params::{param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::index::Index;
use openecraft_core::model::Project;
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
];

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
