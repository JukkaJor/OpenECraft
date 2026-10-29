//! 19-inch rack commands: racks with their rack units, and devices and 19-inch parts
//! mounted on them ([`openecraft_core::panel::rack`]).

use crate::builtin::common::{enclosure_index, int_in};
use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::model::Project;
use openecraft_core::panel::{self, rack};
use openecraft_core::parts::mechanics;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct Racks;

impl Plugin for Racks {
    fn info(&self) -> Info {
        info("racks", tr_const!("19-inch racks"), tr_const!("19-inch racks with their rack units and the devices and 19-inch parts mounted on them"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "add_rack",
        help: tr_const!("Adds a 19-inch rack: a frame with two pairs of 19-inch mounting rails, as many rack units high as fit (42 U in 2000 mm)"),
        params: &[
            param("name", Kind::Str, RACK),
            optional("width", Kind::Num, tr_const!("Width in mm")),
            optional("height", Kind::Num, tr_const!("Height in mm")),
            optional("depth", Kind::Num, tr_const!("Depth in mm")),
            optional("room", Kind::Str, tr_const!("Room the rack stands in (at its rear)")),
        ],
        run: add_rack,
    },
    Spec {
        name: "mount_in_rack",
        help: tr_const!("Mounts a device or a 19-inch part on the rails of a rack, at a rack unit (counted from the bottom) or in the highest free units"),
        params: &[
            param("rack", Kind::Str, RACK),
            param("device", Kind::Str, tr_const!("Device tag, or a part number for a part without device tag")),
            optional("unit", Kind::Int, tr_const!("Lowest rack unit the part takes, from 1 at the bottom")),
        ],
        run: mount_in_rack,
    },
    Spec {
        name: "list_racks",
        help: tr_const!("Lists the 19-inch racks with their rack units, the parts on them from the top and the free units"),
        params: &[optional("rack", Kind::Str, RACK)],
        run: list_racks,
    },
];

const RACK: &str = tr_const!("Rack (location, e.g. +RACK1)");

/// Default size of a new rack (42 U), mm.
const RACK_SIZE: (f32, f32, f32) = (600.0, 2000.0, 800.0);
/// Frame posts beside the 19-inch panels and the smallest depth of a rack, mm.
const FRAME: f32 = 40.0;
const MIN_DEPTH: f32 = 300.0;

fn rack_index(prj: &Project, name: &str) -> Result<usize, String> {
    let i = enclosure_index(prj, name)?;
    if !panel::is_rack(&prj.layout.enclosures[i]) {
        return Err(tr_fmt!("{} is no 19-inch rack", name));
    }
    Ok(i)
}

fn rack_json(prj: &Project, ri: usize) -> Value {
    let e = &prj.layout.enclosures[ri];
    let parts: Vec<Value> = rack::occupied(e)
        .into_iter()
        .filter_map(|(first, n, id)| {
            let it = e.item(id)?;
            Some(json!({ "id": id, "unit": first, "units": n, "tag": it.tag, "part": it.part, "depth": it.d }))
        })
        .collect();
    let used: u32 = parts.iter().filter_map(|p| p["units"].as_u64()).sum::<u64>() as u32;
    json!({
        "name": e.name,
        "size": { "width": e.width, "height": e.height, "depth": e.depth },
        "units": panel::rack_units(e),
        "free": panel::rack_units(e).saturating_sub(used),
        "parts": parts,
        "problems": panel::validate(prj, e).into_iter().map(|f| f.message).collect::<Vec<_>>(),
    })
}

fn add_rack(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let name = a.req("name").trim().to_string();
    if prj.layout.enclosures.iter().any(|e| e.name == name) {
        return Err(tr_fmt!("Enclosure {} exists already", name));
    }
    let size = (a.num("width").unwrap_or(RACK_SIZE.0), a.num("height").unwrap_or(RACK_SIZE.1), a.num("depth").unwrap_or(RACK_SIZE.2));
    let least = (panel::RACK_WIDTH + 2.0 * FRAME, 2.0 * mechanics::RACK_MARGIN + panel::RACK_UNIT, MIN_DEPTH);
    if !(size.0 >= least.0 && size.1 >= least.1 && size.2 >= least.2 && size.0.is_finite() && size.1.is_finite() && size.2.is_finite()) {
        let mm = |v: f32| format!("{:.0}", v.ceil());
        return Err(tr_fmt!("A 19-inch rack is at least {} mm wide, {} mm high and {} mm deep", mm(least.0), mm(least.1), mm(least.2)));
    }
    let room = a.str("room").map(|r| enclosure_index(prj, r)).transpose()?;
    let mut e = panel::new_enclosure(prj, "", &name);
    panel::apply_system(&mut e, panel::RACK_SYSTEM, Some(size));
    prj.layout.enclosures.push(e);
    let ri = prj.layout.enclosures.len() - 1;
    if let Some(r) = room {
        panel::set_room(prj, ri, Some(r), None)?;
    }
    Ok(rack_json(prj, ri))
}

fn mount_in_rack(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let ri = rack_index(prj, a.req("rack"))?;
    let unit = int_in(a, "unit", 1, 100)?.map(|u| u as u32);
    let id = panel::mount_in_rack(prj, ri, a.req("device"), unit)?;
    let e = &prj.layout.enclosures[ri];
    let (first, n) = e.item(id).and_then(|it| rack::slot(e, it)).unwrap_or_default();
    Ok(json!({ "id": id, "unit": first, "units": n, "rack": rack_json(prj, ri) }))
}

fn list_racks(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let list = match a.str("rack") {
        Some(name) => vec![rack_index(prj, name)?],
        None => (0..prj.layout.enclosures.len()).filter(|&i| panel::is_rack(&prj.layout.enclosures[i])).collect(),
    };
    Ok(Value::Array(list.into_iter().map(|ri| rack_json(prj, ri)).collect()))
}

#[cfg(test)]
mod tests;
