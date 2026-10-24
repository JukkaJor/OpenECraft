//! Electrical calculations and verification: supply and installation conditions,
//! short circuit and selectivity, loads, EMC, safety functions and rule sets.

use super::{optional, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::electrical::{Earthing, Insulation, Material};
use openecraft_core::analysis::Analysis;
use openecraft_core::rules::{self, RuleSet};
use openecraft_core::safety::{self, SafetyFunction};
use openecraft_core::{emc, loads, shortcircuit};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "electrical_settings",
        help: tr_const!("Supply (voltage, earthing system, short-circuit currents) and installation conditions of the conductors; changes the given values"),
        params: &[
            optional("voltage", Kind::Num, tr_const!("Line-to-line voltage in V")),
            optional("frequency", Kind::Num, tr_const!("Frequency in Hz")),
            optional("earthing", Kind::Str, tr_const!("tn_s, tn_c, tn_c_s, tt or it")),
            optional("ik_max", Kind::Num, tr_const!("Highest three-phase short-circuit current at the supply in kA")),
            optional("ik_min", Kind::Num, tr_const!("Lowest line-to-earth fault current at the supply in kA (0: from ik_max)")),
            optional("rated_current", Kind::Num, tr_const!("Rated current of the supply in A (0: not limited)")),
            optional("ambient", Kind::Num, tr_const!("Ambient temperature of the conductors in °C")),
            optional("grouped", Kind::Int, tr_const!("Loaded circuits bunched together")),
            optional("material", Kind::Str, tr_const!("copper or aluminium")),
            optional("insulation", Kind::Str, tr_const!("pvc or xlpe")),
            optional("diversity", Kind::Num, tr_const!("Diversity factor of the loads (0: IEC 61439-1, table 101)")),
        ],
        needs_project: true,
        run: electrical_settings,
    },
    Tool {
        name: "short_circuit",
        help: tr_const!("Short-circuit currents, breaking capacities, disconnection times, short-circuit withstand of the cables and selectivity of the protective devices"),
        params: &[],
        needs_project: true,
        run: short_circuit,
    },
    Tool {
        name: "load_calculation",
        help: tr_const!("Loads of the mains, load of each protective device, demand with the diversity factor, phase balance and power supply budgets"),
        params: &[],
        needs_project: true,
        run: load_calculation,
    },
    Tool {
        name: "emc_check",
        help: tr_const!("EMC classes of the cables (drive output, analogue, power, signal, shield) and the EMC problems"),
        params: &[],
        needs_project: true,
        run: emc_check,
    },
    Tool {
        name: "safety_functions",
        help: tr_const!("Safety functions (ISO 13849-1) with the performance level reached; adds, replaces or removes one"),
        params: &[
            optional("set", Kind::Object, tr_const!("Safety function to add or replace (by name): {name, description, required: A…E, subsystems: [{name, devices, category: B|One|Two|Three|Four, dc, operations, mttfd, ccf}]}")),
            optional("remove", Kind::Str, tr_const!("Name of the safety function to remove")),
        ],
        needs_project: true,
        run: safety_functions,
    },
    Tool {
        name: "rule_sets",
        help: tr_const!("Rule libraries of standards and customer rule sets with the limits in effect; enables, disables, adds or removes them"),
        params: &[
            optional("enable", Kind::Str, tr_const!("Library id or customer rule set name to verify against")),
            optional("disable", Kind::Str, tr_const!("Library id or customer rule set name to no longer verify against")),
            optional("add", Kind::Object, tr_const!("Customer rule set to add or replace (by name), as exported")),
            optional("remove", Kind::Str, tr_const!("Name of the customer rule set to remove")),
        ],
        needs_project: true,
        run: rule_sets,
    },
    Tool {
        name: "verify_project",
        help: tr_const!("Verifies the project against the enabled rule libraries and customer rule sets"),
        params: &[],
        needs_project: true,
        run: verify_project,
    },
];

const EARTHING: [(&str, Earthing); 5] = [("tn_s", Earthing::TnS), ("tn_c", Earthing::TnC), ("tn_c_s", Earthing::TnCS), ("tt", Earthing::Tt), ("it", Earthing::It)];

fn pick<T: Copy>(table: &[(&str, T)], value: &str) -> Option<T> {
    table.iter().find(|x| x.0 == value).map(|x| x.1)
}

fn electrical_settings(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let el = &mut prj.electrical;
    for (name, field) in [
        ("voltage", &mut el.supply.voltage),
        ("frequency", &mut el.supply.frequency),
        ("ik_max", &mut el.supply.ik_max),
        ("ik_min", &mut el.supply.ik_min),
        ("rated_current", &mut el.supply.rated_current),
        ("ambient", &mut el.conditions.ambient),
        ("diversity", &mut el.diversity),
    ] {
        if let Some(v) = a.num(name) {
            *field = v;
        }
    }
    if let Some(v) = a.str("earthing") {
        el.supply.earthing = pick(&EARTHING, v).ok_or_else(|| tr_fmt!("Unknown earthing system: {}", v))?;
    }
    if let Some(v) = a.str("material") {
        el.conditions.material = pick(&[("copper", Material::Copper), ("aluminium", Material::Aluminium)], v).ok_or_else(|| tr_fmt!("Unknown material: {}", v))?;
    }
    if let Some(v) = a.str("insulation") {
        el.conditions.insulation = pick(&[("pvc", Insulation::Pvc), ("xlpe", Insulation::Xlpe)], v).ok_or_else(|| tr_fmt!("Unknown insulation: {}", v))?;
    }
    if let Some(v) = a.int("grouped") {
        el.conditions.grouped = v.max(1) as u32;
    }
    let el = &s.project()?.electrical;
    Ok(json!({
        "voltage": el.supply.voltage,
        "frequency": el.supply.frequency,
        "earthing": EARTHING.iter().find(|x| x.1 == el.supply.earthing).map(|x| x.0),
        "ik_max_ka": el.supply.ik_max,
        "ik_min_ka": el.supply.ik_min,
        "rated_current_a": el.supply.rated_current,
        "ambient": el.conditions.ambient,
        "grouped": el.conditions.grouped,
        "material": if el.conditions.material == Material::Copper { "copper" } else { "aluminium" },
        "insulation": if el.conditions.insulation == Insulation::Pvc { "pvc" } else { "xlpe" },
        "capacity_factor": el.conditions.factor(),
        "diversity": el.diversity,
    }))
}

fn finite(v: f32) -> Value {
    if v.is_finite() {
        json!(v)
    } else {
        Value::Null
    }
}

fn short_circuit(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let an = Analysis::build(prj);
    let faults: Vec<Value> = shortcircuit::faults(prj, &an.netlist, &an.connections)
        .iter()
        .map(|f| {
            json!({
                "device": f.device,
                "characteristic": f.protection.characteristic.title(),
                "rated_a": f.protection.rated,
                "breaking_capacity_ka": f.protection.breaking,
                "ik_max_a": f.ik_max,
                "cable": f.cable,
                "ik_max_end_a": f.ik_max_end,
                "ik_min_end_a": f.ik_min_end,
                "disconnection_s": finite(f.time),
                "permitted_s": f.limit,
                "trip_current_a": finite(f.trip_current),
                "residual_ma": f.residual,
                "energy_a2s": f.energy.map(|e| e.0),
                "withstand_a2s": f.energy.map(|e| e.1),
                "breaks": f.breaks(),
                "disconnects": f.disconnects(),
                "withstands": f.withstands(),
            })
        })
        .collect();
    let selectivity: Vec<Value> = shortcircuit::selectivity(prj, &an.netlist, &an.connections)
        .iter()
        .map(|x| json!({ "upstream": x.upstream, "downstream": x.downstream, "selective_up_to_a": x.limit, "ik_max_a": x.ik, "total": x.is_total() }))
        .collect();
    Ok(json!({ "devices": faults, "selectivity": selectivity }))
}

fn load_calculation(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let c = loads::calculate(prj, &Analysis::build(prj).netlist);
    let loads: Vec<Value> = c
        .loads
        .iter()
        .map(|l| json!({ "device": l.device, "kind": l.kind.title(), "feeder": l.feeder, "phases": l.phases, "power_kw": l.power, "apparent_kva": l.apparent, "current_a": l.current }))
        .collect();
    let feeders: Vec<Value> = c.feeders.iter().map(|f| json!({ "device": f.device, "rated_a": f.rated, "loads": f.loads, "current_a": f.current })).collect();
    let budgets: Vec<Value> = c
        .budgets
        .iter()
        .map(|b| json!({ "device": b.device, "voltage_v": b.voltage, "rated_a": b.rated, "current_a": b.current, "loads": b.loads.iter().map(|(t, i)| json!({ "device": t, "current_a": i })).collect::<Vec<_>>(), "without_data": b.without_data }))
        .collect();
    Ok(json!({
        "loads": loads,
        "feeders": feeders,
        "power_supplies": budgets,
        "installed_kw": c.installed.0,
        "installed_kva": c.installed.1,
        "diversity": c.diversity,
        "demand_kva": c.demand,
        "demand_a": c.demand_current,
        "phases_a": c.phases,
        "unbalance_percent": c.unbalance,
    }))
}

fn emc_check(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let an = Analysis::build(prj);
    let cables: Vec<Value> = emc::classes(prj, &an.netlist, &an.connections)
        .iter()
        .map(|c| json!({ "cable": c.cable, "drive": c.drive, "analog": c.analog, "power": c.power, "signal": c.signal, "shielded": c.shielded, "shield_both_ends": c.both_ends }))
        .collect();
    let problems: Vec<String> = emc::problems(prj, &an.netlist, &an.connections).into_iter().map(|i| i.message).collect();
    Ok(json!({ "cables": cables, "problems": problems }))
}

fn safety_functions(s: &mut Session, a: &Args) -> Result<Value, String> {
    if let Some(v) = a.value("set") {
        let f: SafetyFunction = serde_json::from_value(v.clone()).map_err(|e| tr_fmt!("Not a valid safety function: {}", e))?;
        let list = &mut s.project_mut()?.electrical.safety;
        match list.iter_mut().find(|x| x.name == f.name) {
            Some(x) => *x = f,
            None => list.push(f),
        }
    }
    if let Some(name) = a.str("remove") {
        let list = &mut s.project_mut()?.electrical.safety;
        let n = list.len();
        list.retain(|x| x.name != name);
        if list.len() == n {
            return Err(tr_fmt!("No safety function {}", name));
        }
    }
    let prj = s.project()?;
    let out: Vec<Value> = prj
        .electrical
        .safety
        .iter()
        .map(|f| {
            let e = safety::evaluate(prj, f);
            let subs: Vec<Value> = e.subsystems.iter().map(|r| json!({ "name": r.name, "mttfd_years": r.mttfd, "pl": r.pl.map(|p| p.title()), "problems": r.problems })).collect();
            json!({
                "name": f.name,
                "required": f.required.title(),
                "achieved": e.achieved.map(|p| p.title()),
                "sil": e.achieved.and_then(|p| p.sil()),
                "met": e.is_met(),
                "subsystems": subs,
                "definition": f,
            })
        })
        .collect();
    Ok(Value::Array(out))
}

fn rule_sets(s: &mut Session, a: &Args) -> Result<Value, String> {
    let library = rules::library();
    let prj = s.project_mut()?;
    let el = &mut prj.electrical;
    for (name, on) in [(a.str("enable"), true), (a.str("disable"), false)] {
        let Some(name) = name else { continue };
        if library.iter().any(|r| r.id == name) {
            el.standards.retain(|x| x != name);
            if on {
                el.standards.push(name.into());
            }
        } else if let Some(r) = el.rule_sets.iter_mut().find(|r| r.name == name) {
            r.active = on;
        } else {
            return Err(tr_fmt!("No rule library or rule set {}", name));
        }
    }
    if let Some(v) = a.value("add") {
        let r: RuleSet = serde_json::from_value(v.clone()).map_err(|e| tr_fmt!("Not a valid rule set file: {}", e))?;
        match el.rule_sets.iter_mut().find(|x| x.name == r.name) {
            Some(x) => *x = r,
            None => el.rule_sets.push(r),
        }
    }
    if let Some(name) = a.str("remove") {
        let n = el.rule_sets.len();
        el.rule_sets.retain(|x| x.name != name);
        if el.rule_sets.len() == n {
            return Err(tr_fmt!("No rule library or rule set {}", name));
        }
    }
    let prj = s.project()?;
    let l = rules::limits(prj);
    let libs: Vec<Value> = library
        .iter()
        .map(|r| json!({ "id": r.id, "name": r.name, "description": r.description, "source": r.source, "enabled": prj.electrical.standards.contains(&r.id), "checks": r.checks.len() }))
        .collect();
    let custom: Vec<Value> = prj.electrical.rule_sets.iter().map(|r| json!({ "name": r.name, "source": r.source, "active": r.active, "checks": r.checks.len() })).collect();
    Ok(json!({
        "libraries": libs,
        "rule_sets": custom,
        "limits": {
            "voltage_drop_percent": l.voltage_drop,
            "disconnect_final_s": l.disconnect_final,
            "disconnect_distribution_s": l.disconnect_distribution,
            "min_cross_section_power_mm2": l.min_cross_section_power,
            "min_cross_section_control_mm2": l.min_cross_section_control,
            "load_reserve_percent": l.load_reserve,
            "max_unbalance_percent": l.max_unbalance,
            "total_selectivity": l.total_selectivity,
            "shield_drive_cables": l.shield_drive_cables,
            "shield_analog_cables": l.shield_analog_cables,
            "surge_protection": l.surge_protection,
        },
    }))
}

fn verify_project(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let an = Analysis::build(prj);
    let results = rules::verify(prj, &an.connections);
    let failed = results.iter().filter(|r| !r.failed.is_empty()).count();
    let checks: Vec<Value> = results
        .iter()
        .map(|r| {
            json!({
                "rule_set": r.set,
                "rule": r.check.message,
                "level": format!("{:?}", r.check.level).to_lowercase(),
                "checked": r.checked,
                "violations": r.failed.iter().map(|i| &i.label).collect::<Vec<_>>(),
            })
        })
        .collect();
    let messages: Vec<String> = rules::problems(prj, &an.netlist, &an.connections).into_iter().map(|i| i.message).collect();
    Ok(json!({ "checks": checks, "failed_checks": failed, "problems": messages }))
}

#[cfg(test)]
mod tests;
