//! Short-circuit currents and protection coordination of the mains (IEC 60909-0
//! simplified, IEC 60364-4-41, -4-43): the highest short-circuit current at each
//! protective device against its breaking capacity, the lowest fault current at the
//! end of its circuit against the disconnection time, the energy let through against
//! the short-circuit withstand of the cables (k²S²) and the selectivity of each
//! device with the one upstream.

use openecraft_i18n::tr_fmt;
use crate::electrical::{circuits, Circuit, Earthing};
use crate::model::*;
use crate::netlist::{Connection, Netlist};
use crate::protection::{Protection, NO_TRIP};
use crate::issue::{Issue, Severity};
use std::collections::BTreeMap;

/// Voltage factors of IEC 60909-0 for low voltage (tolerance ±10 %).
const C_MAX: f32 = 1.1;
const C_MIN: f32 = 0.95;
/// Reactance of a conductor, Ω/m.
const X_CABLE: f32 = 0.08e-3;
/// R/X of the supply.
const RX_SUPPLY: f32 = 0.1;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Z {
    r: f32,
    x: f32,
}

impl Z {
    fn abs(self) -> f32 {
        self.r.hypot(self.x)
    }

    fn add(self, o: Z) -> Z {
        Z { r: self.r + o.r, x: self.x + o.x }
    }

    fn scale(self, f: f32) -> Z {
        Z { r: self.r * f, x: self.x * f }
    }

    /// Impedance of magnitude `abs` with the R/X of the supply.
    fn supply(abs: f32) -> Z {
        let x = abs / (1.0 + RX_SUPPLY * RX_SUPPLY).sqrt();
        Z { r: x * RX_SUPPLY, x }
    }
}

/// Short-circuit data of a circuit.
#[derive(Clone, Debug, PartialEq)]
pub struct Fault {
    pub device: String,
    pub target: (usize, u64),
    pub protection: Protection,
    /// Highest short-circuit current at the device, A.
    pub ik_max: f32,
    /// Cable at the end of the circuit (highest impedance), empty: none.
    pub cable: String,
    pub cross_section: f32,
    /// Highest short-circuit current at the end of the circuit, A.
    pub ik_max_end: f32,
    /// Lowest fault current (line to PE) at the end of the circuit, A.
    pub ik_min_end: f32,
    /// Longest permitted disconnection time, s (`None`: IT system, no disconnection
    /// at the first fault).
    pub limit: Option<f32>,
    /// Current that disconnects within the limit, A.
    pub trip_current: f32,
    /// Longest disconnection time at the lowest fault current, s.
    pub time: f32,
    /// Rated residual current of a residual current device protecting the circuit, mA.
    pub residual: Option<f32>,
    /// TT system without a residual current device: the earth fault loop is unknown.
    pub needs_rcd: bool,
    /// Energy let through and what the end cable withstands (k²S²), A²s.
    pub energy: Option<(f32, f32)>,
}

impl Fault {
    /// Breaking capacity at least the short-circuit current (`None`: no data).
    pub fn breaks(&self) -> Option<bool> {
        self.protection.breaking.map(|b| b * 1000.0 + 0.5 >= self.ik_max)
    }

    pub fn disconnects(&self) -> bool {
        !self.needs_rcd && (self.limit.is_none() || self.residual.is_some() || self.limit.is_some_and(|l| self.time <= l))
    }

    pub fn withstands(&self) -> bool {
        self.energy.is_none_or(|(e, k)| e <= k)
    }
}

/// Cables in the nets of a circuit with their resistance factor (length / mm²).
fn circuit_cables<'a>(prj: &'a Project, nl: &Netlist, conns: &[Connection], c: &Circuit) -> Vec<(&'a Cable, f32)> {
    let mut tags: Vec<&str> = conns
        .iter()
        .filter(|k| !k.cable.is_empty())
        .filter(|k| {
            [k.a, k.b].iter().any(|p| nl.pin_net.get(&(prj.pages[p.page].symbols[p.sym].id, p.pin)).is_some_and(|n| c.nets.contains(n)))
        })
        .map(|k| k.cable.as_str())
        .collect();
    tags.sort();
    tags.dedup();
    tags.into_iter()
        .filter_map(|t| prj.cable(t))
        .filter_map(|cb| crate::production::cross_section_mm2(&cb.cross_section).filter(|a| *a > 0.0 && cb.length > 0.0).map(|a| (cb, a)))
        .collect()
}

/// Short-circuit data of every circuit of the mains.
pub fn faults(prj: &Project, nl: &Netlist, conns: &[Connection]) -> Vec<Fault> {
    let el = &prj.electrical;
    let s = &el.supply;
    let limits = crate::rules::limits(prj);
    let ik3 = s.ik_max.max(0.01) * 1000.0;
    let z_src = Z::supply(C_MAX * s.voltage / (3f32.sqrt() * ik3));
    let z_loop = if s.ik_min > 0.0 { Z::supply(C_MIN * s.u0() / (s.ik_min * 1000.0)) } else { z_src.scale(2.0) };
    let rho20 = el.conditions.material.rho20();
    let rho_hot = el.conditions.rho_hot();
    let k = el.conditions.k();
    circuits(prj, nl)
        .into_iter()
        .map(|c| {
            let cables = circuit_cables(prj, nl, conns, &c);
            let end = cables.iter().max_by(|a, b| (a.0.length / a.1).total_cmp(&(b.0.length / b.1)));
            let (cable, mm2, len) = end.map(|(cb, a)| (cb.tag.clone(), *a, cb.length)).unwrap_or((String::new(), 0.0, 0.0));
            let conductor = |rho: f32| if mm2 > 0.0 { Z { r: rho * len / mm2, x: X_CABLE * len } } else { Z::default() };
            let ik_max_end = C_MAX * s.voltage / (3f32.sqrt() * z_src.add(conductor(rho20)).abs());
            let ik_min_end = C_MIN * s.u0() / z_loop.add(conductor(rho_hot).scale(2.0)).abs();
            let limit = match s.earthing {
                Earthing::It => None,
                Earthing::Tt if c.is_final() => Some(limits.disconnect_final.min(0.2)),
                Earthing::Tt => Some(limits.disconnect_distribution.min(1.0)),
                _ if c.is_final() => Some(limits.disconnect_final),
                _ => Some(limits.disconnect_distribution),
            };
            let p = &c.protection;
            let trip_current = limit.map_or(0.0, |l| p.current_for(l));
            let time = p.trip_time(ik_min_end).1;
            // The cable must withstand what the device lets through at the lowest and
            // the highest fault current.
            let energy = (mm2 > 0.0)
                .then(|| [ik_min_end, ik_max_end].iter().filter_map(|&i| p.energy(i)).reduce(f32::max))
                .flatten()
                .map(|e| (e, (k * mm2).powi(2)));
            Fault {
                device: c.device.clone(),
                target: (c.page, c.symbol),
                protection: c.protection.clone(),
                ik_max: ik3,
                cable,
                cross_section: mm2,
                ik_max_end,
                ik_min_end,
                limit,
                trip_current,
                time,
                residual: c.residual,
                needs_rcd: s.earthing == Earthing::Tt && c.residual.is_none(),
                energy,
            }
        })
        .collect()
}

/// Selectivity of a protective device with the one upstream.
#[derive(Clone, Debug, PartialEq)]
pub struct Selectivity {
    pub upstream: String,
    pub downstream: String,
    pub target: (usize, u64),
    /// The devices are selective up to this current, A.
    pub limit: f32,
    /// Highest short-circuit current at the downstream device, A.
    pub ik: f32,
}

impl Selectivity {
    pub fn is_total(&self) -> bool {
        self.limit >= self.ik
    }

    /// Not even overloads of the downstream device are selective.
    pub fn is_none(&self, down: &Protection) -> bool {
        self.limit < down.rated * 2.0
    }
}

/// The downstream device trips before the upstream device at the current `i`.
fn selective_at(up: &Protection, down: &Protection, i: f32) -> bool {
    let (up_min, _) = up.trip_time(i);
    let (_, down_max) = down.trip_time(i);
    (up_min == NO_TRIP && down_max == NO_TRIP) || up_min > down_max
}

/// Current up to which two devices are selective (`ik` when they are throughout).
pub fn selectivity_limit(up: &Protection, down: &Protection, ik: f32) -> f32 {
    // Fuses with a rated current ratio of 1.6 are selective (IEC 60269-1, gG).
    if up.characteristic.is_fuse() && down.characteristic.is_fuse() && up.rated >= 1.6 * down.rated - 1e-3 {
        return ik;
    }
    let start = down.rated * 1.05;
    if ik <= start {
        return ik;
    }
    let steps = 400;
    let f = (ik / start).powf(1.0 / steps as f32);
    let mut i = start;
    let mut last = start;
    for _ in 0..=steps {
        if !selective_at(up, down, i) {
            return last;
        }
        last = i;
        i *= f;
    }
    ik
}

/// Selectivity of every protective device with the device upstream.
pub fn selectivity(prj: &Project, nl: &Netlist, conns: &[Connection]) -> Vec<Selectivity> {
    let cs = circuits(prj, nl);
    let ik: BTreeMap<String, f32> = faults(prj, nl, conns).into_iter().map(|f| (f.device, f.ik_max)).collect();
    cs.iter()
        .filter_map(|c| {
            let up = &cs[c.parent?];
            let ik = ik.get(&c.device).copied().unwrap_or(0.0);
            Some(Selectivity {
                upstream: up.device.clone(),
                downstream: c.device.clone(),
                target: (c.page, c.symbol),
                limit: selectivity_limit(&up.protection, &c.protection, ik),
                ik,
            })
        })
        .collect()
}

fn ka(a: f32) -> String {
    format!("{:.2}", a / 1000.0)
}

fn amps(a: f32) -> String {
    if a.is_finite() {
        format!("{a:.0}")
    } else {
        "–".into()
    }
}

/// Check messages: breaking capacity, disconnection, withstand, selectivity.
pub fn problems(prj: &Project, nl: &Netlist, conns: &[Connection]) -> Vec<Issue> {
    let limits = crate::rules::limits(prj);
    let mut out = Vec::new();
    let mut issue = |severity, message, target| out.push(Issue { severity, message, target: Some(target) });
    let faults = faults(prj, nl, conns);
    let mut without = 0;
    for f in &faults {
        match f.breaks() {
            Some(false) => issue(
                Severity::Error,
                tr_fmt!("{}: breaking capacity {} kA below the short-circuit current {} kA", f.device, format!("{}", f.protection.breaking.unwrap_or(0.0)), ka(f.ik_max)),
                f.target,
            ),
            None => without += 1,
            _ => {}
        }
        if f.needs_rcd {
            issue(Severity::Error, tr_fmt!("{}: a TT system needs a residual current device", f.device), f.target);
        } else if !f.disconnects() {
            let at = if f.cable.is_empty() { f.device.clone() } else { f.cable.clone() };
            issue(
                Severity::Error,
                tr_fmt!(
                    "{}: fault current {} A at {} does not disconnect within {} s (needs {} A)",
                    f.device,
                    amps(f.ik_min_end),
                    at,
                    format!("{}", f.limit.unwrap_or(0.0)),
                    amps(f.trip_current)
                ),
                f.target,
            );
        }
        if let Some((e, k)) = f.energy.filter(|_| !f.withstands()) {
            issue(Severity::Error, tr_fmt!("{}: lets through {} A²s, cable {} withstands {} A²s", f.device, format!("{e:.0}"), f.cable, format!("{k:.0}")), f.target);
        }
    }
    if without > 0 {
        out.push(Issue { severity: Severity::Info, message: tr_fmt!("{} protective devices of the mains have no breaking capacity data", without), target: None });
    }
    let downs: BTreeMap<&str, &Protection> = faults.iter().map(|f| (f.device.as_str(), &f.protection)).collect();
    for s in selectivity(prj, nl, conns) {
        if s.is_total() {
            continue;
        }
        let none = downs.get(s.downstream.as_str()).is_some_and(|p| s.is_none(p));
        let severity = if none || limits.total_selectivity { Severity::Warning } else { Severity::Info };
        let message = if none {
            tr_fmt!("{} and {} are not selective", s.upstream, s.downstream)
        } else {
            tr_fmt!("{} is selective to {} only up to {} A (short-circuit current {} A)", s.downstream, s.upstream, amps(s.limit), amps(s.ik))
        };
        out.push(Issue { severity, message, target: Some(s.target) });
    }
    out
}

#[cfg(test)]
mod tests;
