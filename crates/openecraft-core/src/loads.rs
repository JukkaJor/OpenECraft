//! Load calculation: the loads of the mains (motors, drives, heaters, power supplies,
//! transformers) with their active and apparent power and current, the load of each
//! protective device, the demand of the whole supply with the rated diversity factor
//! (IEC 61439-1, table 101), the phase balance and the load budget of the power
//! supplies (control voltage). Data from the parts: `Power`, `Current`, tech data
//! `Power factor`, `Efficiency`, `Coil power`, `Current consumption`.

use crate::electrical::{circuits, mains, value_with_unit};
use crate::model::*;
use crate::netlist::Netlist;
use crate::parts::{kind_of_symbol, FnKind, Part};
use crate::potentials::PotentialKind;
use crate::issue::{Issue, Severity};
use crate::sizing::{device_parts, is_protective, parse_current, parse_voltage, passes_current};
use crate::symbols::Func;
use std::collections::{BTreeMap, BTreeSet};

/// Functions that draw power from the mains.
const MAINS_LOADS: [FnKind; 8] =
    [FnKind::Motor3, FnKind::Motor1, FnKind::Heater, FnKind::FrequencyConverter, FnKind::SoftStarter, FnKind::PowerSupply, FnKind::Transformer, FnKind::SocketOutlet];

/// A load of the mains.
#[derive(Clone, Debug, PartialEq)]
pub struct Load {
    pub device: String,
    pub target: (usize, u64),
    pub kind: FnKind,
    /// Active power drawn, kW.
    pub power: f32,
    /// Apparent power, kVA.
    pub apparent: f32,
    pub current: f32,
    pub three_phase: bool,
    /// Phases it is connected to.
    pub phases: BTreeSet<String>,
    /// Protective device feeding it.
    pub feeder: String,
}

/// The load of a protective device.
#[derive(Clone, Debug, PartialEq)]
pub struct Feeder {
    pub device: String,
    pub target: (usize, u64),
    pub rated: f32,
    /// Loads behind it, also behind the devices downstream.
    pub loads: usize,
    /// Current with the diversity factor, A.
    pub current: f32,
}

/// The load budget of a power supply.
#[derive(Clone, Debug, PartialEq)]
pub struct Budget {
    pub device: String,
    pub target: (usize, u64),
    /// Output voltage and current.
    pub voltage: f32,
    pub rated: f32,
    pub current: f32,
    /// Devices fed by it with their current.
    pub loads: Vec<(String, f32)>,
    /// Devices fed by it without consumption data.
    pub without_data: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadCalc {
    pub loads: Vec<Load>,
    pub feeders: Vec<Feeder>,
    pub budgets: Vec<Budget>,
    /// Installed active and apparent power, kW / kVA.
    pub installed: (f32, f32),
    pub diversity: f32,
    /// Apparent power and current with the diversity factor.
    pub demand: f32,
    pub demand_current: f32,
    /// Current per phase, A.
    pub phases: BTreeMap<String, f32>,
    /// Unbalance of the phase currents: largest deviation from the mean, %.
    pub unbalance: f32,
}

/// Rated diversity factor of IEC 61439-1, table 101, by the number of circuits.
pub fn rated_diversity(circuits: usize) -> f32 {
    match circuits {
        0 | 1 => 1.0,
        2 | 3 => 0.9,
        4 | 5 => 0.8,
        6..=9 => 0.7,
        _ => 0.6,
    }
}

/// A number of the tech data (percent values as fraction).
fn tech_fraction(p: &Part, names: &[&str]) -> Option<f32> {
    let v = names.iter().find_map(|n| p.tech_value(n))?;
    let start = v.find(|c: char| c.is_ascii_digit())?;
    let num: f32 = v[start..].chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect::<String>().replace(',', ".").parse().ok()?;
    Some(if v.contains('%') || num > 1.0 { num / 100.0 } else { num })
}

/// Active power of a power text in W ("0.75 kW", "60 W").
pub fn parse_power(text: &str) -> Option<f32> {
    value_with_unit(text, "kW").map(|v| v * 1000.0).or_else(|| value_with_unit(text, "W"))
}

/// Apparent power of a text in VA ("100 VA", "1.6 kVA").
fn parse_apparent(text: &str) -> Option<f32> {
    value_with_unit(text, "kVA").map(|v| v * 1000.0).or_else(|| value_with_unit(text, "VA"))
}

/// Second voltage of a text: the output of "230 V AC / 24 V DC".
fn output_voltage(text: &str) -> Option<f32> {
    text.split_once('/').and_then(|(_, b)| parse_voltage(b.trim()))
}

/// Active and apparent power (W, VA) of a mains load.
fn demand(p: &Part, kind: FnKind, u: f32, three_phase: bool) -> Option<(f32, f32)> {
    let (cos, eta) = match kind {
        FnKind::Motor3 | FnKind::Motor1 | FnKind::SoftStarter => (0.8, 0.85),
        FnKind::FrequencyConverter => (0.95, 0.9),
        FnKind::PowerSupply => (0.95, 0.9),
        FnKind::Transformer => (0.8, 1.0),
        _ => (1.0, 1.0),
    };
    let cos = tech_fraction(p, &["Power factor", "cos phi"]).unwrap_or(cos);
    let eta = tech_fraction(p, &["Efficiency"]).unwrap_or(eta);
    let root = if three_phase { 3f32.sqrt() } else { 1.0 };
    match kind {
        FnKind::PowerSupply => {
            let out = parse_power(&p.power).or_else(|| Some(output_voltage(&p.voltage)? * parse_current(&p.current)?))?;
            let w = out / eta;
            Some((w, w / cos))
        }
        FnKind::Transformer => {
            let va = parse_apparent(&p.power).or_else(|| parse_apparent(&p.description))?;
            Some((va * cos, va))
        }
        FnKind::SocketOutlet => parse_power(&p.power).map(|w| (w, w)),
        _ => {
            if let Some(i) = parse_current(&p.current) {
                let va = root * u * i;
                return Some((va * cos, va));
            }
            let w = parse_power(&p.power)? / eta;
            Some((w, w / cos))
        }
    }
}

/// The part of a device with a function of this kind.
fn part_with<'a>(prj: &'a Project, tag: &str, kind: FnKind) -> Option<&'a Part> {
    let parts = device_parts(prj, tag);
    parts.iter().find(|p| p.functions.iter().any(|f| f.kind == kind)).or(parts.first()).copied()
}

/// Consumption of a device on a power supply in A, at the voltage `u`.
fn consumption(prj: &Project, tag: &str, u: f32) -> Option<f32> {
    for p in device_parts(prj, tag) {
        if let Some(v) = p.tech_value("Current consumption") {
            if let Some(ma) = value_with_unit(v, "mA") {
                return Some(ma / 1000.0);
            }
            if let Some(a) = parse_current(v) {
                return Some(a);
            }
        }
        let kinds: Vec<FnKind> = p.functions.iter().map(|f| f.kind).collect();
        if kinds.contains(&FnKind::MotorDc) {
            if let Some(i) = parse_current(&p.current) {
                return Some(i);
            }
        }
        if kinds.iter().any(|k| matches!(k, FnKind::Coil | FnKind::Valve | FnKind::Brake)) {
            let w = p.tech_value("Coil power").and_then(parse_power).or_else(|| value_with_unit(&p.description, "W")).or_else(|| p.data_value("Power loss").and_then(|v| parse_power(&v)));
            if let Some(w) = w {
                return Some(w / u.max(1.0));
            }
        }
    }
    None
}

/// Devices that are expected to draw from a power supply.
fn is_consumer(prj: &Project, tag: &str) -> bool {
    device_parts(prj, tag).iter().any(|p| p.functions.iter().any(|f| matches!(f.kind, FnKind::Coil | FnKind::Valve | FnKind::Brake | FnKind::Signal | FnKind::MotorDc)))
}

/// Symbols the control voltage passes through: contacts, switches, protective devices.
fn switches(prj: &Project, s: &SymbolInst) -> bool {
    passes_current(prj, s)
        || is_protective(&s.sym)
        || prj.symbol(&s.sym).is_some_and(|d| matches!(d.func, Func::Aux(_)))
        || matches!(
            kind_of_symbol(&s.sym),
            Some(
                FnKind::No
                    | FnKind::Nc
                    | FnKind::Changeover
                    | FnKind::NoOnDelay
                    | FnKind::NcOnDelay
                    | FnKind::NoOffDelay
                    | FnKind::NcOffDelay
                    | FnKind::PushbuttonNo
                    | FnKind::PushbuttonNc
                    | FnKind::SwitchNo
                    | FnKind::SwitchNc
                    | FnKind::Sensor
                    | FnKind::SensorNc
            )
        )
}

/// Load budgets of the power supplies: devices on the nets fed from their positive
/// output through contacts and protective devices.
fn budgets(prj: &Project, nl: &Netlist) -> Vec<Budget> {
    let mut out: Vec<Budget> = Vec::new();
    for (pi, pg) in prj.pages.iter().enumerate().filter(|(_, p)| p.kind == PageKind::Schematic) {
        for s in &pg.symbols {
            if kind_of_symbol(&s.sym) != Some(FnKind::PowerSupply) || s.tag.is_empty() || out.iter().any(|b| b.device == s.tag) {
                continue;
            }
            let Some(p) = part_with(prj, &s.tag, FnKind::PowerSupply) else { continue };
            let Some(rated) = parse_current(&p.current) else { continue };
            let voltage = output_voltage(&p.voltage).unwrap_or(24.0);
            let mut todo: Vec<usize> = (0..s.pins.len())
                .filter(|&i| {
                    let pin = s.pins[i].trim();
                    let net = nl.pin_net.get(&(s.id, i));
                    matches!(pin, "L+" | "+" | "V+" | "24V" | "+24V") || net.is_some_and(|&n| crate::electrical::net_kind(prj, &nl.nets[n].name) == PotentialKind::DcPlus)
                })
                .filter_map(|i| nl.pin_net.get(&(s.id, i)).copied())
                .collect();
            let mut seen = BTreeSet::new();
            let mut tags: BTreeSet<String> = BTreeSet::new();
            while let Some(n) = todo.pop() {
                if !seen.insert(n) {
                    continue;
                }
                for pr in &nl.nets[n].pins {
                    let o = &prj.pages[pr.page].symbols[pr.sym];
                    if o.tag == s.tag || o.tag.is_empty() {
                        continue;
                    }
                    if switches(prj, o) {
                        todo.extend((0..o.pins.len()).filter(|&j| j != pr.pin).filter_map(|j| nl.pin_net.get(&(o.id, j)).copied()));
                    }
                    if prj.symbol(&o.sym).is_some_and(|d| !d.func.is_feedthrough()) && !is_protective(&o.sym) {
                        tags.insert(o.tag.clone());
                    }
                }
            }
            let mut b = Budget { device: s.tag.clone(), target: (pi, s.id), voltage, rated, current: 0.0, loads: Vec::new(), without_data: Vec::new() };
            for t in tags {
                match consumption(prj, &t, voltage) {
                    Some(i) => {
                        b.current += i;
                        b.loads.push((t, i));
                    }
                    None if is_consumer(prj, &t) => b.without_data.push(t),
                    None => {}
                }
            }
            out.push(b);
        }
    }
    out
}

/// Loads, feeders, demand and power supply budgets of the project.
pub fn calculate(prj: &Project, nl: &Netlist) -> LoadCalc {
    let el = &prj.electrical;
    let mains = mains(prj, nl);
    let cs = circuits(prj, nl);
    let mut calc = LoadCalc::default();
    let mut seen = BTreeSet::new();
    for (pi, pg) in prj.pages.iter().enumerate().filter(|(_, p)| p.kind == PageKind::Schematic) {
        for s in &pg.symbols {
            let Some(kind) = kind_of_symbol(&s.sym).filter(|k| MAINS_LOADS.contains(k)) else { continue };
            let nets: Vec<usize> = (0..s.pins.len()).filter_map(|i| nl.pin_net.get(&(s.id, i)).copied()).collect();
            let phases: BTreeSet<String> = nets.iter().filter_map(|n| mains.get(n).cloned()).collect();
            if s.tag.is_empty() || phases.is_empty() || !seen.insert(s.tag.clone()) {
                continue;
            }
            let Some(part) = part_with(prj, &s.tag, kind) else { continue };
            let three_phase = matches!(kind, FnKind::Motor3 | FnKind::FrequencyConverter | FnKind::SoftStarter) || phases.len() >= 3;
            let u = if three_phase { el.supply.voltage } else { el.supply.u0() };
            let Some((w, va)) = demand(part, kind, u, three_phase) else { continue };
            let root = if three_phase { 3f32.sqrt() } else { 1.0 };
            let feeder = cs.iter().find(|c| nets.iter().any(|n| c.nets.contains(n))).map(|c| c.device.clone()).unwrap_or_default();
            calc.loads.push(Load {
                device: s.tag.clone(),
                target: (pi, s.id),
                kind,
                power: w / 1000.0,
                apparent: va / 1000.0,
                current: va / (root * u.max(1.0)),
                three_phase,
                phases,
                feeder,
            });
        }
    }
    let diversity = |n: usize| if el.diversity > 0.0 { el.diversity.min(1.0) } else { rated_diversity(n) };
    for (i, c) in cs.iter().enumerate() {
        // Loads behind the device and behind the devices downstream of it.
        let below: BTreeSet<&str> = (0..cs.len())
            .filter(|&j| {
                let mut p = Some(j);
                let mut depth = 0;
                while let Some(k) = p {
                    if k == i {
                        return true;
                    }
                    p = cs[k].parent;
                    depth += 1;
                    if depth > cs.len() {
                        break;
                    }
                }
                false
            })
            .map(|j| cs[j].device.as_str())
            .collect();
        let loads: Vec<&Load> = calc.loads.iter().filter(|l| below.contains(l.feeder.as_str())).collect();
        if loads.is_empty() {
            continue;
        }
        let sum: f32 = loads.iter().map(|l| l.current).sum();
        calc.feeders.push(Feeder {
            device: c.device.clone(),
            target: (c.page, c.symbol),
            rated: c.protection.rated,
            loads: loads.len(),
            current: sum * if loads.len() > 1 { diversity(loads.len()) } else { 1.0 },
        });
    }
    calc.installed = (calc.loads.iter().map(|l| l.power).sum(), calc.loads.iter().map(|l| l.apparent).sum());
    calc.diversity = diversity(calc.loads.len());
    calc.demand = calc.installed.1 * calc.diversity;
    calc.demand_current = calc.demand * 1000.0 / (3f32.sqrt() * el.supply.voltage.max(1.0));
    for l in &calc.loads {
        if l.three_phase {
            for p in &l.phases {
                *calc.phases.entry(p.clone()).or_default() += l.current;
            }
        } else if let Some(p) = l.phases.iter().next() {
            *calc.phases.entry(p.clone()).or_default() += l.current;
        }
    }
    if calc.phases.len() >= 3 {
        let mean = calc.phases.values().sum::<f32>() / calc.phases.len() as f32;
        if mean > 0.0 {
            calc.unbalance = calc.phases.values().map(|i| (i - mean).abs()).fold(0.0, f32::max) / mean * 100.0;
        }
    }
    calc.budgets = budgets(prj, nl);
    calc
}

/// Check messages: overloaded protective devices, supply and power supplies, phase
/// unbalance.
pub fn problems(prj: &Project, nl: &Netlist) -> Vec<Issue> {
    let limits = crate::rules::limits(prj);
    let usable = limits.usable();
    let calc = calculate(prj, nl);
    let mut out = Vec::new();
    for f in &calc.feeders {
        if f.current > f.rated * usable + 0.01 {
            out.push(crate::issue!(Severity::Warning, Some(f.target), "{}: load {} A above the usable {} A", f.device, format!("{:.1}", f.current), format!("{:.1}", f.rated * usable)));
        }
    }
    let rated = prj.electrical.supply.rated_current;
    if rated > 0.0 && calc.demand_current > rated * usable + 0.01 {
        out.push(crate::issue!(Severity::Warning, None, "Demand {} A above the usable {} A of the supply", format!("{:.1}", calc.demand_current), format!("{:.1}", rated * usable)));
    }
    if limits.max_unbalance > 0.0 && calc.unbalance > limits.max_unbalance {
        out.push(crate::issue!(Severity::Warning, None, "Phase unbalance {} % above {} %", format!("{:.0}", calc.unbalance), limits.max_unbalance));
    }
    for b in &calc.budgets {
        if b.current > b.rated * usable + 1e-3 {
            out.push(crate::issue!(Severity::Warning, Some(b.target), "{}: load {} A above the usable {} A of the power supply", b.device, format!("{:.2}", b.current), format!("{:.2}", b.rated * usable)));
        }
        if !b.without_data.is_empty() {
            out.push(crate::issue!(Severity::Info, Some(b.target), "{}: no consumption data for {}", b.device, b.without_data.join(", ")));
        }
    }
    out
}

#[cfg(test)]
mod tests;
