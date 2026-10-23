//! Wire numbers. A number belongs to a connection: a net with the terminal sides
//! kept apart (`Netlist::build(prj, false)`), so the wires on both sides of a
//! terminal get their own numbers. All wire segments of a connection carry it.

use crate::model::*;
use crate::netlist::{Net, Netlist};
use std::collections::{HashMap, HashSet};

/// The wire segments of a net.
pub fn wires_of<'a>(prj: &'a Project, net: &Net) -> Vec<&'a Wire> {
    let mut v: Vec<&Wire> =
        net.wires.iter().filter_map(|(pi, id)| prj.pages[*pi].wires.iter().find(|w| w.id == *id)).collect();
    v.sort_by_key(|w| w.id);
    v
}

/// Number of a connection (the first number found on its wires).
pub fn number_of(prj: &Project, net: &Net) -> String {
    wires_of(prj, net).iter().find(|w| !w.number.is_empty()).map(|w| w.number.clone()).unwrap_or_default()
}

/// Different numbers on the wires of one connection (e.g. after joining two).
pub fn numbers_of(prj: &Project, net: &Net) -> Vec<String> {
    let mut v: Vec<String> = wires_of(prj, net).iter().filter(|w| !w.number.is_empty()).map(|w| w.number.clone()).collect();
    v.sort_by(|a, b| natural_cmp(a, b));
    v.dedup();
    v
}

/// Connections that get a number (they have wires and connect something), in
/// schematic order (page, column, top to bottom), with the page and column where
/// they start.
fn connections_in_order(prj: &Project, open: &Netlist) -> Vec<(usize, usize, usize)> {
    let mut v: Vec<((usize, usize, i64, i64), usize)> = Vec::new();
    for (ni, net) in open.nets.iter().enumerate() {
        if net.wires.is_empty() || net.pins.is_empty() {
            continue;
        }
        let start = net
            .wires
            .iter()
            .filter_map(|(pi, id)| prj.pages[*pi].wires.iter().find(|w| w.id == *id).map(|w| (*pi, w)))
            .flat_map(|(pi, w)| {
                let frame = crate::frame::of_page(prj, pi);
                [w.a, w.b].map(|q| (pi, frame.column_index(q) + 1, (q.y * 10.0) as i64, (q.x * 10.0) as i64))
            })
            .min();
        if let Some(start) = start {
            v.push((start, ni));
        }
    }
    v.sort();
    v.into_iter().map(|((page, col, _, _), ni)| (ni, page, col)).collect()
}

/// Numbers all connections according to the project's settings; hoses and pipes
/// (fluid nets) are counted apart with the fluid prefix (`H1`, `H2` …). Manually set
/// numbers stay unless `overwrite_fixed`. Returns the number of numbered connections.
pub fn renumber(prj: &mut Project, open: &Netlist, bridged: &Netlist, overwrite_fixed: bool) -> usize {
    let settings = prj.wire_numbering.clone();
    let order = connections_in_order(prj, open);
    let fixed = |prj: &Project, net: &Net| wires_of(prj, net).iter().any(|w| w.number_fixed && !w.number.is_empty());
    let mut used: HashSet<String> = HashSet::new();
    let mut todo = Vec::new();
    for (ni, page, col) in order {
        let net = &open.nets[ni];
        if !overwrite_fixed && fixed(prj, net) {
            used.insert(number_of(prj, net));
        } else {
            todo.push((ni, page, col));
        }
    }
    let mut counter = settings.start;
    let mut next_counted = |used: &HashSet<String>| loop {
        let n = format!("{}{counter}", settings.prefix);
        counter += 1;
        if !used.contains(&n) {
            return n;
        }
    };
    let mut per_column: HashMap<(usize, usize), u32> = HashMap::new();
    let mut numbers = Vec::new();
    let mut hoses = 0;
    for (ni, page, col) in todo {
        let net = &open.nets[ni];
        let number = match settings.scheme {
            _ if net.fluid => loop {
                hoses += 1;
                let n = format!("{}{hoses}", prj.fluid.prefix);
                if !used.contains(&n) {
                    break n;
                }
            },
            NumberScheme::Sequential => next_counted(&used),
            NumberScheme::PageColumn => loop {
                let k = per_column.entry((page, col)).or_insert(0);
                *k += 1;
                let n = format!("{}{}.{col}.{k}", settings.prefix, prj.pages[page].name);
                if !used.contains(&n) {
                    break n;
                }
            },
            NumberScheme::Potential => {
                let pin = net.pins[0];
                let sym = prj.pages[pin.page].symbols[pin.sym].id;
                match bridged.pin_net.get(&(sym, pin.pin)).map(|&n| &bridged.nets[n]) {
                    // Shared on purpose by all connections of the potential.
                    Some(pot) if pot.named => pot.name.clone(),
                    _ => next_counted(&used),
                }
            }
        };
        used.insert(number.clone());
        numbers.push((ni, number));
    }
    let count = numbers.len();
    for (ni, number) in numbers {
        set(prj, &open.nets[ni], &number, false);
    }
    count
}

/// Sets the number of all wires of a connection.
pub fn set(prj: &mut Project, net: &Net, number: &str, fixed: bool) {
    for (pi, id) in &net.wires {
        if let Some(w) = prj.pages[*pi].wires.iter_mut().find(|w| w.id == *id) {
            w.number = number.to_string();
            w.number_fixed = fixed && !number.is_empty();
        }
    }
}

/// Removes wire numbers (manually set ones only with `include_fixed`). Returns the
/// number of wires changed.
pub fn clear(prj: &mut Project, include_fixed: bool) -> usize {
    let mut n = 0;
    for w in prj.pages.iter_mut().flat_map(|pg| pg.wires.iter_mut()) {
        if !w.number.is_empty() && (include_fixed || !w.number_fixed) {
            w.number.clear();
            w.number_fixed = false;
            n += 1;
        }
    }
    n
}

/// Wire of a connection on a page that carries its label (the longest one).
pub fn label_wire<'a>(page: &'a Page, page_i: usize, net: &Net) -> Option<&'a Wire> {
    net.wires
        .iter()
        .filter(|(pi, _)| *pi == page_i)
        .filter_map(|(_, id)| page.wires.iter().find(|w| w.id == *id))
        .max_by(|a, b| (a.b - a.a).len().total_cmp(&(b.b - b.a).len()).then(b.id.cmp(&a.id)))
}

#[cfg(test)]
mod tests;
