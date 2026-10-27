//! Potential commands: potential definitions (wire colour and cross-section per
//! potential), the default of nets without a defined potential and presets.

use crate::builtin::common::{cross_section, wire_color};
use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::model::Project;
use openecraft_core::potentials::{PotentialDef, PotentialKind, Preset};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct Potentials;

impl Plugin for Potentials {
    fn info(&self) -> Info {
        info("potentials", tr_const!("Potentials"), tr_const!("Potential definitions, wire defaults and wire colours"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "list_potentials",
        help: tr_const!("Lists the potential definitions and the wire default of nets without a defined potential"),
        params: &[],
        run: list_potentials,
    },
    Spec {
        name: "set_potential",
        help: tr_const!("Creates or changes a potential definition; its wires take the colour and cross-section unless set on the wire"),
        params: &[
            param("name", Kind::Str, tr_const!("Potential name, e.g. L1 or +24V")),
            optional("new_name", Kind::Str, tr_const!("New name")),
            optional("kind", Kind::Str, tr_const!("l, n, pe, dc_plus, dc_minus, signal or other")),
            optional("voltage", Kind::Str, tr_const!("Voltage, e.g. 400 V AC")),
            optional("color", Kind::Str, tr_const!("Wire colour (IEC 60757), e.g. BK")),
            optional("cross_section", Kind::Str, tr_const!("Cross section, e.g. 1.5 mm²")),
            optional("description", Kind::Str, tr_const!("Description")),
        ],
        run: set_potential,
    },
    Spec {
        name: "delete_potential",
        help: tr_const!("Deletes a potential definition"),
        params: &[param("name", Kind::Str, tr_const!("Potential name"))],
        run: delete_potential,
    },
    Spec {
        name: "set_wire_defaults",
        help: tr_const!("Sets the wire colour and cross-section of nets without a defined potential (empty = none)"),
        params: &[
            optional("color", Kind::Str, tr_const!("Wire colour, e.g. BU")),
            optional("cross_section", Kind::Str, tr_const!("Cross section, e.g. 0.75 mm²")),
        ],
        run: set_wire_defaults,
    },
    Spec {
        name: "set_wire_colors",
        help: tr_const!("Draws the wires in their colour on the schematic pages and in PDF / SVG export, or in black"),
        params: &[param("enabled", Kind::Bool, tr_const!("Draw wires in their colour"))],
        run: set_wire_colors,
    },
    Spec {
        name: "load_potential_preset",
        help: tr_const!("Loads the potential definitions of a standard (replaces those of the same name)"),
        params: &[param("preset", Kind::Str, tr_const!("iec60204 (machinery, phases BK) or iec60445 (phases BN, BK, GY)"))],
        run: load_preset,
    },
];

const KINDS: [(&str, PotentialKind); 7] = [
    ("l", PotentialKind::L),
    ("n", PotentialKind::N),
    ("pe", PotentialKind::Pe),
    ("dc_plus", PotentialKind::DcPlus),
    ("dc_minus", PotentialKind::DcMinus),
    ("signal", PotentialKind::Signal),
    ("other", PotentialKind::Other),
];

fn kind_name(k: PotentialKind) -> &'static str {
    KINDS.iter().find(|x| x.1 == k).map(|x| x.0).unwrap_or("other")
}

fn def_json(d: &PotentialDef) -> Value {
    json!({
        "name": d.name,
        "kind": kind_name(d.kind),
        "voltage": d.voltage,
        "color": d.color,
        "cross_section": d.cross_section,
        "description": d.description,
    })
}

fn list(prj: &Project) -> Result<Value, String> {
    Ok(json!({
        "potentials": prj.potentials.iter().map(def_json).collect::<Vec<_>>(),
        "wire_defaults": { "color": prj.wire_defaults.color, "cross_section": prj.wire_defaults.cross_section },
        "wire_colors": prj.wire_colors,
    }))
}

fn list_potentials(prj: &mut Project, _: &Args) -> Result<Value, String> {
    list(prj)
}

fn set_potential(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let name = a.req("name").trim().to_string();
    if name.is_empty() {
        return Err(tr_fmt!("Missing parameter: {}", "name"));
    }
    let kind = match a.str("kind") {
        Some(k) => Some(KINDS.iter().find(|x| x.0 == k.trim().to_ascii_lowercase()).map(|x| x.1).ok_or_else(|| tr_fmt!("Unknown potential kind: {}", k))?),
        None => None,
    };
    if let Some(new) = a.str("new_name").map(str::trim).filter(|n| *n != name) {
        if new.is_empty() || prj.potential(new).is_some() {
            return Err(tr_fmt!("Potential {} already exists", new));
        }
    }
    let color = a.str("color").map(wire_color).transpose()?;
    let cross = a.str("cross_section").map(cross_section).transpose()?;
    let i = match prj.potentials.iter().position(|p| p.name == name) {
        Some(i) => i,
        None => {
            prj.potentials.push(PotentialDef { name: name.clone(), ..Default::default() });
            prj.potentials.len() - 1
        }
    };
    let d = &mut prj.potentials[i];
    if let Some(k) = kind {
        d.kind = k;
    }
    for (field, value) in [("voltage", &mut d.voltage), ("description", &mut d.description)] {
        if let Some(v) = a.str(field) {
            *value = v.trim().into();
        }
    }
    if let Some(c) = color {
        d.color = c;
    }
    if let Some(c) = cross {
        d.cross_section = c;
    }
    if let Some(new) = a.str("new_name") {
        d.name = new.trim().into();
    }
    Ok(def_json(d))
}

fn delete_potential(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let name = a.req("name");
    let i = prj.potentials.iter().position(|p| p.name == name).ok_or_else(|| tr_fmt!("No potential definition {}", name))?;
    prj.potentials.remove(i);
    list(prj)
}

fn set_wire_defaults(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let color = a.str("color").map(wire_color).transpose()?;
    let cross = a.str("cross_section").map(cross_section).transpose()?;
    if let Some(v) = color {
        prj.wire_defaults.color = v;
    }
    if let Some(v) = cross {
        prj.wire_defaults.cross_section = v;
    }
    list(prj)
}

fn set_wire_colors(prj: &mut Project, a: &Args) -> Result<Value, String> {
    prj.wire_colors = a.flag("enabled");
    list(prj)
}

fn load_preset(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let preset = match a.req("preset").trim().to_ascii_lowercase().replace(['-', ' '], "").as_str() {
        "iec60204" | "iec602041" => Preset::Iec60204,
        "iec60445" => Preset::Iec60445,
        p => return Err(tr_fmt!("Unknown preset: {}", p)),
    };
    prj.apply_preset(preset);
    list(prj)
}

#[cfg(test)]
mod tests;
