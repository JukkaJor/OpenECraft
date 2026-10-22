//! Engineering calculations: enclosure climate and electrical sizing.

use super::{optional, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::climate::{self, Cooling, Installation, Material};
use openecraft_core::netlist::Netlist;
use openecraft_core::reports::Analysis;
use openecraft_core::sizing;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "enclosure_climate",
        help: tr_const!("Climate of the enclosures (IEC/TR 60890): power loss, inside temperature, fan or cooling unit and heating needed; optionally changes the settings of one"),
        params: &[
            optional("enclosure", Kind::Str, tr_const!("Enclosure (location, e.g. +CAB1) whose settings change")),
            optional("installation", Kind::Str, tr_const!("free_standing, wall_mounted, end_free_standing, end_wall_mounted, middle_free_standing, middle_wall_mounted or middle_wall_mounted_covered")),
            optional("material", Kind::Str, tr_const!("sheet_steel, stainless_steel, aluminium or plastic")),
            optional("ambient_max", Kind::Num, tr_const!("Highest ambient temperature in °C")),
            optional("ambient_min", Kind::Num, tr_const!("Lowest ambient temperature in °C")),
            optional("inside_max", Kind::Num, tr_const!("Highest inside temperature in °C")),
            optional("inside_min", Kind::Num, tr_const!("Lowest inside temperature in °C")),
            optional("extra_loss", Kind::Num, tr_const!("Additional power loss in W")),
        ],
        needs_project: true,
        run: enclosure_climate,
    },
    Tool {
        name: "electrical_sizing",
        help: tr_const!("Wires too thin for their protective device and the voltage drop of the cables (IEC 60204-1)"),
        params: &[],
        needs_project: true,
        run: electrical_sizing,
    },
];

const INSTALLATIONS: [(&str, Installation); 7] = [
    ("free_standing", Installation::FreeStanding),
    ("wall_mounted", Installation::WallMounted),
    ("end_free_standing", Installation::EndFreeStanding),
    ("end_wall_mounted", Installation::EndWallMounted),
    ("middle_free_standing", Installation::MiddleFreeStanding),
    ("middle_wall_mounted", Installation::MiddleWallMounted),
    ("middle_wall_mounted_covered", Installation::MiddleWallMountedCovered),
];

const MATERIALS: [(&str, Material); 4] =
    [("sheet_steel", Material::SheetSteel), ("stainless_steel", Material::StainlessSteel), ("aluminium", Material::Aluminium), ("plastic", Material::Plastic)];

fn enclosure_climate(s: &mut Session, a: &Args) -> Result<Value, String> {
    if let Some(name) = a.str("enclosure") {
        let prj = s.project_mut()?;
        let e = prj.layout.enclosures.iter_mut().find(|e| e.name == name).ok_or_else(|| tr_fmt!("No enclosure {}", name))?;
        let c = &mut e.climate;
        if let Some(v) = a.str("installation") {
            c.installation = INSTALLATIONS.iter().find(|x| x.0 == v).map(|x| x.1).ok_or_else(|| tr_fmt!("Unknown installation: {}", v))?;
        }
        if let Some(v) = a.str("material") {
            c.material = MATERIALS.iter().find(|x| x.0 == v).map(|x| x.1).ok_or_else(|| tr_fmt!("Unknown material: {}", v))?;
        }
        for (name, field) in [
            ("ambient_max", &mut c.ambient_max),
            ("ambient_min", &mut c.ambient_min),
            ("inside_max", &mut c.inside_max),
            ("inside_min", &mut c.inside_min),
            ("extra_loss", &mut c.extra_loss),
        ] {
            if let Some(v) = a.num(name) {
                *field = v;
            }
        }
    }
    let prj = s.project()?;
    let out: Vec<Value> = prj
        .layout
        .enclosures
        .iter()
        .map(|e| {
            let r = climate::calculate(prj, e);
            let c = &e.climate;
            let (fan, unit) = match r.cooling {
                Cooling::None => (None, None),
                Cooling::Fan(v) => (Some(v), None),
                Cooling::CoolingUnit(q) => (None, Some(q)),
            };
            json!({
                "enclosure": e.name,
                "installation": INSTALLATIONS.iter().find(|x| x.1 == c.installation).map(|x| x.0),
                "material": MATERIALS.iter().find(|x| x.1 == c.material).map(|x| x.0),
                "ambient": [c.ambient_min, c.ambient_max],
                "inside_allowed": [c.inside_min, c.inside_max],
                "surface_m2": r.surface,
                "power_loss_w": r.loss,
                "inside_without_cooling": r.inside,
                "fan_m3_per_h": fan,
                "cooling_unit_w": unit,
                "heating_w": r.heating,
                "devices_without_power_loss": r.without_data,
            })
        })
        .collect();
    Ok(Value::Array(out))
}

fn electrical_sizing(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let (nl, open) = (Netlist::build(prj, true), Netlist::build(prj, false));
    let an = Analysis::build(prj);
    let overloads: Vec<Value> = sizing::overloads(prj, &nl, &open)
        .into_iter()
        .map(|o| json!({ "device": o.device, "rated_a": o.rated, "cross_section": o.cross_section, "capacity_a": o.ampacity, "page": prj.pages[o.target.0].name, "wire": o.target.1 }))
        .collect();
    let cables: Vec<Value> = sizing::cable_drops(prj, &an.connections)
        .into_iter()
        .map(|d| {
            json!({
                "cable": d.cable,
                "length_m": d.length,
                "cross_section": d.cross_section,
                "load": d.load,
                "current_a": d.current,
                "voltage_v": d.voltage,
                "three_phase": d.three_phase,
                "voltage_drop_v": d.drop.map(|x| x.0),
                "voltage_drop_percent": d.drop.map(|x| x.1),
                "capacity_a": d.ampacity,
            })
        })
        .collect();
    Ok(json!({ "too_thin": overloads, "cables": cables, "voltage_drop_limit_percent": sizing::VOLTAGE_DROP_LIMIT }))
}

#[cfg(test)]
mod tests;
