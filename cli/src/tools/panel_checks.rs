//! Panel layout checks: clearances, service space, heights above the floor,
//! collision classes, the thermal simulation and manufacturability, and their rules.

use super::common::enclosure_index;
use super::output::severity_name;
use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::model::Project;
use openecraft_core::panel::{self, thermal, ClassRule, Enclosure, PanelRules};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};

const ENCLOSURE: &str = tr_const!("Enclosure (location, e.g. +CAB1)");
const CLASS: &str = tr_const!("Collision class: power, drive, control, signal, terminal, rail, duct, busbar, cutout, accessory, an own class or * for any");

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "check_panel",
        help: tr_const!("Checks the panel layout: collisions, clearances (mounting, ventilation, EMC), service space, heights above the floor, collision classes, temperatures and manufacturability"),
        params: &[optional("enclosure", Kind::Str, ENCLOSURE)],
        needs_project: true,
        run: check_panel,
    },
    Tool {
        name: "panel_thermal",
        help: tr_const!("Thermal simulation of the mounting plate: air temperatures, hot spot, heat sources and the temperature at every device"),
        params: &[param("enclosure", Kind::Str, ENCLOSURE)],
        needs_project: true,
        run: panel_thermal,
    },
    Tool {
        name: "panel_rules",
        help: tr_const!("Shows or changes the rules of the panel layout checks; values in mm (temperature in °C)"),
        params: &[
            optional("device_gap", Kind::Num, tr_const!("Gap between devices side by side")),
            optional("duct_gap", Kind::Num, tr_const!("Distance of devices to cable ducts")),
            optional("edge_distance", Kind::Num, tr_const!("Distance of devices to the plate edge")),
            optional("service_space", Kind::Num, tr_const!("Free space in front of every device on the plate")),
            optional("operating_min", Kind::Num, tr_const!("Lowest device above the floor")),
            optional("operating_max", Kind::Num, tr_const!("Highest device above the floor (0: not checked)")),
            optional("actuator_min", Kind::Num, tr_const!("Lowest actuator on the door above the floor")),
            optional("actuator_max", Kind::Num, tr_const!("Highest actuator on the door above the floor (0: not checked)")),
            optional("terminal_min", Kind::Num, tr_const!("Lowest terminal above the floor")),
            optional("device_temperature", Kind::Num, tr_const!("Highest air temperature at devices without data (0: not checked)")),
            optional("stratification", Kind::Num, tr_const!("Temperature rise at the roof against the middle (1 … 2)")),
            optional("hole_web", Kind::Num, tr_const!("Smallest web between holes")),
            optional("hole_edge", Kind::Num, tr_const!("Smallest distance of holes from the sheet edge")),
            optional("cutout_web", Kind::Num, tr_const!("Smallest web at cutouts")),
            optional("min_profile", Kind::Num, tr_const!("Shortest rail, duct or busbar")),
            optional("stock_length", Kind::Num, tr_const!("Stock length of rails, ducts and busbars")),
            optional("reset", Kind::Bool, tr_const!("Back to the default rules first")),
        ],
        needs_project: true,
        run: panel_rules,
    },
    Tool {
        name: "set_collision_rule",
        help: tr_const!("Sets the distance between two collision classes or lets them overlap; replaces the rule of the same classes"),
        params: &[
            param("a", Kind::Str, CLASS),
            param("b", Kind::Str, CLASS),
            optional("distance", Kind::Num, tr_const!("Smallest distance in mm")),
            optional("ignore", Kind::Bool, tr_const!("The classes may overlap (no collision)")),
            optional("remove", Kind::Bool, tr_const!("Removes the rule")),
        ],
        needs_project: true,
        run: set_collision_rule,
    },
    Tool {
        name: "set_item_class",
        help: tr_const!("Sets the collision class of a panel item (empty: automatic)"),
        params: &[param("enclosure", Kind::Str, ENCLOSURE), param("id", Kind::Int, tr_const!("Item id")), param("class", Kind::Str, CLASS)],
        needs_project: true,
        run: set_item_class,
    },
    Tool {
        name: "set_enclosure_elevation",
        help: tr_const!("Sets the height of the enclosure's bottom above the floor (heights of devices, terminals and actuators are checked)"),
        params: &[param("enclosure", Kind::Str, ENCLOSURE), param("elevation", Kind::Num, tr_const!("Height in mm without plinth; negative: unknown (wall mounted)"))],
        needs_project: true,
        run: set_enclosure_elevation,
    },
];

fn rules_json(r: &PanelRules) -> Value {
    json!({
        "device_gap": r.device_gap,
        "duct_gap": r.duct_gap,
        "edge_distance": r.edge_distance,
        "service_space": r.service_space,
        "operating": [r.operating_min, r.operating_max],
        "actuator": [r.actuator_min, r.actuator_max],
        "terminal_min": r.terminal_min,
        "device_temperature": r.device_temperature,
        "stratification": r.stratification,
        "hole_web": r.hole_web,
        "hole_edge": r.hole_edge,
        "cutout_web": r.cutout_web,
        "min_profile": r.min_profile,
        "stock_length": r.stock_length,
        "classes": r.classes.iter().map(|c| json!({ "a": c.a, "b": c.b, "distance": c.distance, "ignore": c.ignore })).collect::<Vec<_>>(),
    })
}

fn findings_json(prj: &Project, e: &Enclosure) -> Vec<Value> {
    panel::validate(prj, e)
        .into_iter()
        .map(|f| json!({ "enclosure": e.name, "check": f.kind.key(), "severity": severity_name(f.severity), "items": f.items, "message": f.message }))
        .collect()
}

fn check_panel(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let list: Vec<&Enclosure> = match a.str("enclosure") {
        Some(name) => vec![&prj.layout.enclosures[enclosure_index(prj, name)?]],
        None => prj.layout.enclosures.iter().collect(),
    };
    let findings: Vec<Value> = list.into_iter().flat_map(|e| findings_json(prj, e)).collect();
    let warnings = findings.iter().filter(|f| f["severity"] == "warning").count();
    Ok(json!({ "warnings": warnings, "findings": findings }))
}

fn panel_thermal(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let e = &prj.layout.enclosures[enclosure_index(prj, a.req("enclosure"))?];
    let map = thermal::heat_map(prj, e);
    let tag = |id: u64| panel::item_name(e, id);
    let round = |v: f32| (v * 10.0).round() / 10.0;
    Ok(json!({
        "mean_c": round(map.mean),
        "bottom_c": round(map.bottom),
        "top_c": round(map.top),
        "min_c": round(map.min),
        "max_c": round(map.max),
        "hottest": [round(map.hottest.x), round(map.hottest.y)],
        "sources": map.sources.iter().map(|s| json!({ "device": tag(s.id), "power_loss_w": s.loss, "rise_k": round(s.rise) })).collect::<Vec<_>>(),
        "devices": thermal::device_temperatures(prj, e, &map)
            .into_iter()
            .map(|t| json!({ "device": t.tag, "air_c": round(t.temperature), "limit_c": t.limit, "too_warm": t.limit.is_some_and(|l| t.temperature > l) }))
            .collect::<Vec<_>>(),
    }))
}

fn panel_rules(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let r = &mut prj.layout.checks;
    if a.value("reset").and_then(Value::as_bool) == Some(true) {
        *r = PanelRules::default();
    }
    for (name, field) in [
        ("device_gap", &mut r.device_gap),
        ("duct_gap", &mut r.duct_gap),
        ("edge_distance", &mut r.edge_distance),
        ("service_space", &mut r.service_space),
        ("operating_min", &mut r.operating_min),
        ("operating_max", &mut r.operating_max),
        ("actuator_min", &mut r.actuator_min),
        ("actuator_max", &mut r.actuator_max),
        ("terminal_min", &mut r.terminal_min),
        ("device_temperature", &mut r.device_temperature),
        ("hole_web", &mut r.hole_web),
        ("hole_edge", &mut r.hole_edge),
        ("cutout_web", &mut r.cutout_web),
        ("min_profile", &mut r.min_profile),
        ("stock_length", &mut r.stock_length),
    ] {
        if let Some(v) = a.num(name) {
            *field = v.max(0.0);
        }
    }
    if let Some(v) = a.num("stratification") {
        r.stratification = v.clamp(1.0, 2.0);
    }
    Ok(rules_json(r))
}

fn set_collision_rule(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let (ca, cb) = (a.req("a").trim().to_ascii_lowercase(), a.req("b").trim().to_ascii_lowercase());
    if ca.is_empty() || cb.is_empty() {
        return Err(tr("Both classes are needed").to_string());
    }
    let classes = &mut prj.layout.checks.classes;
    let same = |r: &ClassRule| (r.a == ca && r.b == cb) || (r.a == cb && r.b == ca);
    let at = classes.iter().position(same);
    if a.value("remove").and_then(Value::as_bool) == Some(true) {
        let i = at.ok_or_else(|| tr_fmt!("No rule for {} and {}", ca, cb))?;
        classes.remove(i);
    } else {
        let rule = ClassRule { a: ca, b: cb, distance: a.num("distance").unwrap_or(0.0).max(0.0), ignore: a.value("ignore").and_then(Value::as_bool).unwrap_or(false) };
        match at {
            Some(i) => classes[i] = rule,
            None => classes.push(rule),
        }
    }
    Ok(rules_json(&prj.layout.checks))
}

fn set_item_class(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let ei = enclosure_index(prj, a.req("enclosure"))?;
    let id = a.int("id").unwrap_or(0) as u64;
    let it = prj.layout.enclosures[ei].items.iter_mut().find(|i| i.id == id).ok_or_else(|| tr_fmt!("No item {}", id))?;
    it.class = a.req("class").trim().to_ascii_lowercase();
    let it = prj.layout.enclosures[ei].items.iter().find(|i| i.id == id).unwrap();
    let class = panel::item_class(prj, &panel::device_kinds(prj), it);
    Ok(json!({ "id": id, "class": class, "automatic": it.class.is_empty() }))
}

fn set_enclosure_elevation(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let ei = enclosure_index(prj, a.req("enclosure"))?;
    let v = a.num("elevation").unwrap_or(-1.0);
    let e = &mut prj.layout.enclosures[ei];
    e.elevation = (v >= 0.0).then_some(v);
    Ok(json!({ "enclosure": e.name, "elevation": e.elevation, "floor_height": panel::floor_height(e) }))
}

#[cfg(test)]
mod tests;
