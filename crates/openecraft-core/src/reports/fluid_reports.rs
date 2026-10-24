//! Fluid reports: fluid netlist, hose configuration, tubing / piping list, fluid
//! device groups and the electro-fluid cross reference.

use super::Table;
use crate::electrofluid;
use crate::fluid::{self, length_text};
use crate::index::Index;
use crate::model::*;
use crate::netlist::Netlist;
use openecraft_i18n::{tr, tr_const, tr_fmt};

fn pressure_text(bar: f32) -> String {
    if bar > 0.0 { format!("{bar}") } else { "–".into() }
}

pub(super) fn netlist(prj: &Project, open: &Netlist) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Net"), 0.5),
        (tr_const!("Hose no."), 0.6),
        (tr_const!("Line"), 1.1),
        (tr_const!("Medium"), 1.0),
        (tr_const!("Nominal size"), 0.7),
        (tr_const!("Ports"), 3.0),
        (tr_const!("Branches"), 0.6),
    ]);
    for n in fluid::fluid_nets(prj, open) {
        let ports: Vec<String> = n.ports.iter().map(|p| format!("{} ({})", p.designation, p.kind.title())).collect();
        let target = n.ports.first().map(|p| (p.pin.page, prj.pages[p.pin.page].symbols[p.pin.sym].id));
        let branches = if n.branches() > 0 { n.branches().to_string() } else { String::new() };
        t.push(vec![n.net.clone(), n.number.clone(), n.line.title().into(), n.medium.title().into(), n.size.clone(), ports.join(", "), branches], target);
    }
    t
}

pub(super) fn configuration(prj: &Project, open: &Netlist) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Hose no."), 0.6),
        (tr_const!("Type"), 0.6),
        (tr_const!("From"), 1.1),
        (tr_const!("Fitting"), 1.0),
        (tr_const!("To"), 1.1),
        (tr_const!("Fitting"), 1.0),
        (tr_const!("Nominal size"), 0.7),
        (tr_const!("Part"), 1.0),
        (tr_const!("Length (m)"), 0.7),
        (tr_const!("Cut length (m)"), 0.7),
        (tr_const!("Max. pressure (bar)"), 0.7),
    ]);
    for h in fluid::hoses(prj, open) {
        let c = &h.config;
        let length = match &h.routed {
            Some(r) if h.length <= 0.0 => tr_fmt!("{} (routed)", length_text(r.length)),
            _ => length_text(h.length),
        };
        let target = Some((h.a.page, prj.pages[h.a.page].symbols[h.a.sym].id));
        t.push(
            vec![
                h.number.clone(),
                c.line.title().into(),
                h.from.clone(),
                c.fitting_a.clone(),
                h.to.clone(),
                c.fitting_b.clone(),
                h.size.clone(),
                c.part.clone(),
                length,
                length_text(h.cut_length(prj.fluid.allowance)),
                pressure_text(c.pressure),
            ],
            target,
        );
    }
    t
}

pub(super) fn tubing(prj: &Project, open: &Netlist) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Type"), 0.6),
        (tr_const!("Nominal size"), 0.7),
        (tr_const!("Medium"), 1.0),
        (tr_const!("Part"), 1.1),
        (tr_const!("Pieces"), 0.5),
        (tr_const!("Total length (m)"), 0.8),
        (tr_const!("Bends"), 0.5),
        (tr_const!("Fittings"), 2.2),
    ]);
    for m in fluid::tubing(prj, &fluid::hoses(prj, open)) {
        let fittings: Vec<String> = m.fittings.iter().map(|(f, n)| format!("{n}× {f}")).collect();
        t.push(
            vec![m.line.title().into(), m.size, m.medium.title().into(), m.part, m.pieces.to_string(), length_text(m.length), m.bends.to_string(), fittings.join(", ")],
            None,
        );
    }
    t
}

pub(super) fn groups(prj: &Project, idx: &Index) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Group"), 0.7),
        (tr_const!("Kind"), 1.0),
        (tr_const!("Place"), 0.4),
        (tr_const!("Device"), 0.7),
        (tr_const!("Function"), 1.4),
        (tr_const!("Actuated by"), 1.0),
        (tr_const!("Location"), 0.7),
        (tr_const!("Part"), 1.0),
    ]);
    for g in &prj.fluid.groups {
        t.push(vec![g.tag.clone(), g.kind.title().into(), String::new(), String::new(), g.description.clone(), String::new(), String::new(), g.part.clone()], None);
        for (i, m) in g.members.iter().enumerate() {
            let main = electrofluid::main_of(prj, idx, m);
            let function = main.as_ref().map(|(d, s, _)| if s.function_text.is_empty() { tr(d.name).to_string() } else { s.function_text.clone() }).unwrap_or_default();
            let location = main.as_ref().map(|(_, _, l)| l.reference(prj)).unwrap_or_else(|| "?".into());
            let parts: Vec<String> = prj.devices.get(m).map(|d| d.parts.iter().map(|p| p.number.clone()).collect()).unwrap_or_default();
            t.push(
                vec![String::new(), String::new(), (i + 1).to_string(), m.clone(), function, electrofluid::linked(prj, m).join(", "), location, parts.join(", ")],
                main.map(|(_, _, l)| (l.page, l.sym_id)),
            );
        }
    }
    t
}

pub(super) fn electro_fluid(prj: &Project, idx: &Index) -> Table {
    let mut t = Table::with_widths(&[
        (tr_const!("Fluid device"), 0.8),
        (tr_const!("Function"), 1.2),
        (tr_const!("Location"), 0.7),
        (tr_const!("Electrical device"), 0.8),
        (tr_const!("Function"), 1.2),
        (tr_const!("Location"), 0.7),
        (tr_const!("Status"), 1.2),
    ]);
    for e in electrofluid::entries(prj, idx) {
        let mut first = true;
        let head = |first: bool| if first { (e.tag.clone(), e.function.clone(), e.location.clone()) } else { Default::default() };
        if e.electrical.is_empty() {
            let (a, b, c) = head(true);
            t.push(vec![a, b, c, String::new(), String::new(), String::new(), tr("Solenoid not linked").into()], e.target);
            continue;
        }
        for (tag, function, location) in &e.electrical {
            let (a, b, c) = head(first);
            let status = if location.is_empty() { tr("Not in the schematic") } else { tr("Linked") };
            t.push(vec![a, b, c, tag.clone(), function.clone(), location.clone(), status.into()], e.target);
            first = false;
        }
    }
    t
}
