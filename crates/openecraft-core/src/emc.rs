//! EMC checks (IEC 60204-1 4.4.2, IEC 61800-3, IEC 61000-6-2/-4): motor cables of
//! frequency converters shielded at both ends, analogue signals in shielded cables,
//! no power and signal conductors in the same cable, a surge protective device at
//! the supply. Segregation of the routes and the termination of shields are checked
//! by the 3D routing ([`crate::routing3d`]).

use crate::electrical::{mains, net_kind};
use crate::model::*;
use crate::netlist::{Connection, Netlist};
use crate::parts::{kind_of_symbol, FnKind};
use crate::potentials::PotentialKind;
use crate::issue::{Issue, Severity};
use crate::routing3d::{is_shielded, ShieldEnds};
use std::collections::{BTreeMap, BTreeSet};

/// What a cable carries as far as EMC is concerned.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CableClass {
    pub cable: String,
    /// Output of a frequency converter (device tag).
    pub drive: Option<String>,
    /// Analogue signals (device tags).
    pub analog: BTreeSet<String>,
    /// Conductors of the mains.
    pub power: bool,
    /// Signal, DC and analogue conductors.
    pub signal: bool,
    pub shielded: bool,
    pub both_ends: bool,
}

/// Output pins of a frequency converter.
fn is_drive_output(pin: &str) -> bool {
    matches!(pin.trim(), "U" | "V" | "W" | "U2" | "V2" | "W2" | "T1" | "T2" | "T3")
}

/// EMC class of every cable.
pub fn classes(prj: &Project, nl: &Netlist, conns: &[Connection]) -> Vec<CableClass> {
    let mains = mains(prj, nl);
    let mut by: BTreeMap<&str, CableClass> = BTreeMap::new();
    for c in conns.iter().filter(|c| !c.cable.is_empty()) {
        let e = by.entry(c.cable.as_str()).or_insert_with(|| CableClass { cable: c.cable.clone(), ..Default::default() });
        let kind = net_kind(prj, &c.net);
        for p in [c.a, c.b] {
            let s = &prj.pages[p.page].symbols[p.sym];
            let k = kind_of_symbol(&s.sym);
            let pin = s.pins.get(p.pin).map(String::as_str).unwrap_or("");
            if k == Some(FnKind::FrequencyConverter) && is_drive_output(pin) {
                e.drive = Some(s.tag.clone());
            }
            if matches!(k, Some(FnKind::PlcAnalogInput | FnKind::PlcAnalogOutput | FnKind::Transmitter)) {
                e.analog.insert(s.tag.clone());
            }
            if nl.pin_net.get(&(s.id, p.pin)).is_some_and(|n| mains.contains_key(n)) {
                e.power = true;
            }
        }
        if matches!(kind, PotentialKind::Signal | PotentialKind::DcPlus | PotentialKind::DcMinus) || !e.analog.is_empty() {
            e.signal = true;
        }
    }
    for e in by.values_mut() {
        if let Some(c) = prj.cable(&e.cable) {
            e.shielded = is_shielded(c);
            e.both_ends = c.shield_ends == ShieldEnds::Both;
        }
        // The drive output is no mains, but power all the same.
        e.power |= e.drive.is_some();
    }
    by.into_values().collect()
}

/// Check messages of the EMC rules.
pub fn problems(prj: &Project, nl: &Netlist, conns: &[Connection]) -> Vec<Issue> {
    let limits = crate::rules::limits(prj);
    let mut out = Vec::new();
    for c in classes(prj, nl, conns) {
        if let (Some(d), true) = (&c.drive, limits.shield_drive_cables) {
            if !c.shielded {
                out.push(crate::issue!(Severity::Warning, None, "Motor cable {} of frequency converter {} must be shielded", c.cable, d));
            } else if !c.both_ends {
                out.push(crate::issue!(Severity::Warning, None, "Shield of motor cable {} must be connected at both ends", c.cable));
            }
        }
        if limits.shield_analog_cables && !c.analog.is_empty() && !c.shielded {
            out.push(crate::issue!(Severity::Warning, None, "Cable {} carries analogue signals of {} and must be shielded", c.cable, c.analog.iter().cloned().collect::<Vec<_>>().join(", ")));
        }
        if c.power && c.signal {
            out.push(crate::issue!(Severity::Warning, None, "Cable {} carries power and signal conductors together", c.cable));
        }
    }
    let has_spd = prj.pages.iter().flat_map(|p| p.symbols.iter()).any(|s| kind_of_symbol(&s.sym) == Some(FnKind::SurgeProtection));
    if limits.surge_protection && !has_spd && !mains(prj, nl).is_empty() {
        out.push(crate::issue!(Severity::Warning, None, "No surge protective device at the supply (IEC 60364-4-44)"));
    }
    out
}

#[cfg(test)]
mod tests;
