//! Potential tools: potential definitions (wire colour and cross-section per
//! potential), the default of nets without a defined potential and presets.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::potentials::{PotentialDef, PotentialKind, Preset};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "list_potentials",
        help: tr_const!("Lists the potential definitions and the wire default of nets without a defined potential"),
        params: &[],
        needs_project: true,
        run: list_potentials,
    },
    Tool {
        name: "set_potential",
        help: tr_const!("Creates or changes a potential definition; its wires take the colour and cross-section unless set on the wire"),
        params: &[
            param("name", Kind::Str, tr_const!("Potential name, e.g. L1 or +24V")),
            optional("rename", Kind::Str, tr_const!("New name")),
            optional("kind", Kind::Str, tr_const!("l, n, pe, dc_plus, dc_minus, signal or other")),
            optional("voltage", Kind::Str, tr_const!("Voltage, e.g. 400 V AC")),
            optional("color", Kind::Str, tr_const!("Wire colour (IEC 60757), e.g. BK")),
            optional("cross_section", Kind::Str, tr_const!("Cross section, e.g. 1.5 mm²")),
            optional("description", Kind::Str, tr_const!("Description")),
        ],
        needs_project: true,
        run: set_potential,
    },
    Tool {
        name: "delete_potential",
        help: tr_const!("Deletes a potential definition"),
        params: &[param("name", Kind::Str, tr_const!("Potential name"))],
        needs_project: true,
        run: delete_potential,
    },
    Tool {
        name: "set_wire_defaults",
        help: tr_const!("Sets the wire colour and cross-section of nets without a defined potential (empty = none)"),
        params: &[
            optional("color", Kind::Str, tr_const!("Wire colour, e.g. BU")),
            optional("cross_section", Kind::Str, tr_const!("Cross section, e.g. 0.75 mm²")),
        ],
        needs_project: true,
        run: set_wire_defaults,
    },
    Tool {
        name: "set_wire_colors",
        help: tr_const!("Draws the wires in their colour on the schematic pages and in PDF / SVG export, or in black"),
        params: &[param("enabled", Kind::Bool, tr_const!("Draw wires in their colour"))],
        needs_project: true,
        run: set_wire_colors,
    },
    Tool {
        name: "load_potential_preset",
        help: tr_const!("Loads the potential definitions of a standard (replaces those of the same name)"),
        params: &[param("preset", Kind::Str, tr_const!("iec60204 (machinery, phases BK) or iec60445 (phases BN, BK, GY)"))],
        needs_project: true,
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

fn list(s: &Session) -> Result<Value, String> {
    let prj = s.project()?;
    Ok(json!({
        "potentials": prj.potentials.iter().map(def_json).collect::<Vec<_>>(),
        "wire_defaults": { "color": prj.wire_defaults.color, "cross_section": prj.wire_defaults.cross_section },
        "wire_colors": prj.wire_colors,
    }))
}

fn list_potentials(s: &mut Session, _: &Args) -> Result<Value, String> {
    list(s)
}

fn set_potential(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let name = a.req("name").trim().to_string();
    if name.is_empty() {
        return Err(tr_fmt!("Missing parameter: {}", "name"));
    }
    let kind = match a.str("kind") {
        Some(k) => Some(KINDS.iter().find(|x| x.0 == k.trim().to_ascii_lowercase()).map(|x| x.1).ok_or_else(|| tr_fmt!("Unknown potential kind: {}", k))?),
        None => None,
    };
    if let Some(new) = a.str("rename").map(str::trim).filter(|n| *n != name) {
        if new.is_empty() || prj.potential(new).is_some() {
            return Err(tr_fmt!("Potential {} already exists", new));
        }
    }
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
    for (field, value) in [("voltage", &mut d.voltage), ("color", &mut d.color), ("cross_section", &mut d.cross_section), ("description", &mut d.description)] {
        if let Some(v) = a.str(field) {
            *value = v.trim().into();
        }
    }
    if let Some(new) = a.str("rename") {
        d.name = new.trim().into();
    }
    Ok(def_json(d))
}

fn delete_potential(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let name = a.req("name");
    let i = prj.potentials.iter().position(|p| p.name == name).ok_or_else(|| tr_fmt!("No potential definition {}", name))?;
    prj.potentials.remove(i);
    list(s)
}

fn set_wire_defaults(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    if let Some(v) = a.str("color") {
        prj.wire_defaults.color = v.trim().into();
    }
    if let Some(v) = a.str("cross_section") {
        prj.wire_defaults.cross_section = v.trim().into();
    }
    list(s)
}

fn set_wire_colors(s: &mut Session, a: &Args) -> Result<Value, String> {
    s.project_mut()?.wire_colors = a.flag("enabled");
    list(s)
}

fn load_preset(s: &mut Session, a: &Args) -> Result<Value, String> {
    let preset = match a.req("preset").trim().to_ascii_lowercase().replace(['-', ' '], "").as_str() {
        "iec60204" | "iec602041" => Preset::Iec60204,
        "iec60445" => Preset::Iec60445,
        p => return Err(tr_fmt!("Unknown preset: {}", p)),
    };
    s.project_mut()?.apply_preset(preset);
    list(s)
}

#[cfg(test)]
mod tests;
