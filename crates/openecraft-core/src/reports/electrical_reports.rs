//! Reports of the electrical calculations: short circuit, selectivity, loads, power
//! supply budgets, safety functions and the rule set verification.

use super::Table;
use crate::model::*;
use crate::netlist::{Connection, Netlist};
use openecraft_i18n::{tr, tr_fmt};

fn num(v: f32, digits: usize) -> String {
    if v.is_finite() {
        format!("{v:.digits$}")
    } else {
        "–".into()
    }
}

fn ok(good: bool) -> String {
    if good { tr("OK") } else { tr("Not OK") }.to_string()
}

pub(super) fn short_circuit(prj: &Project, nl: &Netlist, conns: &[Connection]) -> Table {
    let mut t = Table::new(&[
        (tr("Device"), 0.8),
        (tr("Characteristic"), 0.7),
        (tr("Rated current (A)"), 0.7),
        (tr("Breaking capacity (kA)"), 0.8),
        (tr("Ik max (kA)"), 0.7),
        (tr("Cable"), 0.7),
        (tr("Ik1 min (A)"), 0.7),
        (tr("Disconnection (s)"), 0.8),
        (tr("Permitted (s)"), 0.7),
        (tr("Result"), 0.6),
    ]);
    for f in crate::shortcircuit::faults(prj, nl, conns) {
        let p = &f.protection;
        let disconnection = if f.residual.is_some() { tr_fmt!("RCD {} mA", f.residual.unwrap_or(0.0)) } else { num(f.time, 2) };
        t.push(
            vec![
                f.device.clone(),
                p.characteristic.title().into(),
                num(p.rated, 1),
                p.breaking.map(|b| num(b, 1)).unwrap_or_else(|| "–".into()),
                num(f.ik_max / 1000.0, 2),
                f.cable.clone(),
                num(f.ik_min_end, 0),
                disconnection,
                f.limit.map(|l| num(l, 1)).unwrap_or_else(|| "IT".into()),
                ok(f.breaks() != Some(false) && f.disconnects() && f.withstands()),
            ],
            Some(f.target),
        );
    }
    t
}

pub(super) fn selectivity(prj: &Project, nl: &Netlist, conns: &[Connection]) -> Table {
    let mut t = Table::new(&[(tr("Upstream"), 0.8), (tr("Downstream"), 0.8), (tr("Selective up to (A)"), 0.9), (tr("Ik max (A)"), 0.8), (tr("Selectivity"), 0.8)]);
    for s in crate::shortcircuit::selectivity(prj, nl, conns) {
        let grade = if s.is_total() { tr("Total") } else { tr("Partial") };
        t.push(vec![s.upstream.clone(), s.downstream.clone(), num(s.limit, 0), num(s.ik, 0), grade.into()], Some(s.target));
    }
    t
}

pub(super) fn load_calculation(prj: &Project, nl: &Netlist) -> Table {
    let mut t = Table::new(&[
        (tr("Device"), 0.8),
        (tr("Kind"), 1.0),
        (tr("Protective device"), 0.8),
        (tr("Phases"), 0.7),
        (tr("Power (kW)"), 0.6),
        (tr("Apparent power (kVA)"), 0.8),
        (tr("Current (A)"), 0.6),
    ]);
    let c = crate::loads::calculate(prj, nl);
    for l in &c.loads {
        let phases = l.phases.iter().cloned().collect::<Vec<_>>().join(" ");
        t.push(vec![l.device.clone(), l.kind.title().into(), l.feeder.clone(), phases, num(l.power, 2), num(l.apparent, 2), num(l.current, 2)], Some(l.target));
    }
    for f in &c.feeders {
        t.push(vec![f.device.clone(), tr_fmt!("Loads behind it: {}", f.loads), String::new(), String::new(), String::new(), num(f.rated, 1), num(f.current, 2)], Some(f.target));
    }
    let mut row = |label: String, kw: String, kva: String, a: String| t.push(vec![label, String::new(), String::new(), String::new(), kw, kva, a], None);
    row(tr("Installed").into(), num(c.installed.0, 2), num(c.installed.1, 2), String::new());
    row(tr_fmt!("Demand (diversity factor {})", num(c.diversity, 2)), String::new(), num(c.demand, 2), num(c.demand_current, 1));
    for (p, i) in &c.phases {
        row(tr_fmt!("Phase {}", p), String::new(), String::new(), num(*i, 2));
    }
    if c.phases.len() >= 3 {
        row(tr_fmt!("Unbalance {} %", num(c.unbalance, 0)), String::new(), String::new(), String::new());
    }
    t
}

pub(super) fn power_supply_budget(prj: &Project, nl: &Netlist) -> Table {
    let mut t = Table::new(&[(tr("Power supply"), 0.8), (tr("Device"), 0.8), (tr("Current (A)"), 0.6), (tr("Rated current (A)"), 0.7), (tr("Utilisation (%)"), 0.7)]);
    for b in crate::loads::calculate(prj, nl).budgets {
        for (tag, i) in &b.loads {
            t.push(vec![b.device.clone(), tag.clone(), num(*i, 3), String::new(), String::new()], Some(b.target));
        }
        for tag in &b.without_data {
            t.push(vec![b.device.clone(), tag.clone(), "?".into(), String::new(), String::new()], Some(b.target));
        }
        t.push(vec![b.device.clone(), tr("Total").into(), num(b.current, 2), num(b.rated, 1), num(b.current / b.rated.max(0.01) * 100.0, 0)], Some(b.target));
    }
    t
}

pub(super) fn safety_functions(prj: &Project) -> Table {
    let mut t = Table::new(&[
        (tr("Safety function"), 1.0),
        (tr("Subsystem"), 0.9),
        (tr("Category"), 0.5),
        (tr("DCavg (%)"), 0.5),
        (tr("MTTFd (years)"), 0.6),
        (tr("PL"), 0.6),
        (tr("Required"), 0.6),
        (tr("Result"), 0.5),
    ]);
    for f in &prj.electrical.safety {
        let e = crate::safety::evaluate(prj, f);
        for (s, r) in f.subsystems.iter().zip(&e.subsystems) {
            t.push(
                vec![f.name.clone(), s.name.clone(), s.category.title().into(), num(s.dc, 0), r.mttfd.map(|m| num(m, 1)).unwrap_or_else(|| "–".into()), crate::safety::pl_text(r.pl), String::new(), String::new()],
                None,
            );
        }
        t.push(vec![f.name.clone(), tr("Safety function").into(), String::new(), String::new(), String::new(), crate::safety::pl_text(e.achieved), crate::safety::pl_text(Some(f.required)), ok(e.is_met())], None);
    }
    t
}

pub(super) fn verification(prj: &Project, conns: &[Connection]) -> Table {
    let mut t = Table::new(&[(tr("Rule set"), 0.8), (tr("Rule"), 2.0), (tr("Checked"), 0.5), (tr("Violations"), 0.5), (tr("Items"), 1.6)]);
    for r in crate::rules::verify(prj, conns) {
        let mut items: Vec<String> = r.failed.iter().take(8).map(|i| i.label.clone()).collect();
        if r.failed.len() > 8 {
            items.push("…".into());
        }
        let target = r.failed.iter().find_map(|i| i.target.filter(|t| t.1 != 0));
        t.push(vec![r.set.clone(), r.check.message.clone(), r.checked.to_string(), r.failed.len().to_string(), items.join(", ")], target);
    }
    t
}
