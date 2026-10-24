//! Plugs: pin order, the two sides of each pin (socket and pin side), unused pins
//! of the plug parts and placement problems (pins placed twice, more pins than the
//! parts have).

use openecraft_i18n::tr_fmt;
use crate::model::*;
use crate::netlist::{pin_designation, Netlist};
use crate::parts::FnKind;
use crate::symbols::{Func, PinKind};
pub use crate::model::plugs::{Pin, PlugData};

fn pin_kind(prj: &Project, s: &SymbolInst) -> Option<PinKind> {
    match prj.symbol(&s.sym).map(|d| d.func) {
        Some(Func::Plug(k)) => Some(k),
        _ => None,
    }
}

/// Tags of all plugs.
pub fn plug_tags(prj: &Project) -> Vec<String> {
    let mut v: Vec<String> = prj
        .pages
        .iter()
        .filter(|p| p.kind == PageKind::Schematic)
        .flat_map(|p| p.symbols.iter())
        .filter(|s| pin_kind(prj, s).is_some() && !s.tag.is_empty())
        .map(|s| s.tag.clone())
        .collect();
    v.sort_by(|a, b| natural_cmp(a, b));
    v.dedup();
    v
}

/// Pin symbols of a plug in plug order: the stored order first, the rest sorted by
/// designation (then schematic order).
pub fn pins(prj: &Project, plug: &str) -> Vec<Pin> {
    let mut v = Vec::new();
    for (pi, pg) in prj.pages.iter().enumerate().filter(|(_, p)| p.kind == PageKind::Schematic) {
        for (si, s) in pg.symbols.iter().enumerate() {
            if let (true, Some(kind)) = (s.tag == plug, pin_kind(prj, s)) {
                v.push(Pin { page: pi, sym: si, id: s.id, label: s.label.clone(), kind });
            }
        }
    }
    let order = prj.plugs.get(plug).map(|d| d.order.as_slice()).unwrap_or(&[]);
    let rank = |p: &Pin| order.iter().position(|o| *o == p.label).unwrap_or(usize::MAX);
    v.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| natural_cmp(&a.label, &b.label)).then((a.page, a.sym).cmp(&(b.page, b.sym))));
    v
}

/// Pin designations in plug order (each once).
pub fn designations(prj: &Project, plug: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for p in pins(prj, plug) {
        if !v.contains(&p.label) {
            v.push(p.label);
        }
    }
    v
}

/// Moves a pin designation one place up or down and stores the resulting order.
pub fn move_pin(prj: &mut Project, plug: &str, index: usize, up: bool) {
    let mut labels = designations(prj, plug);
    let other = if up { index.checked_sub(1) } else { Some(index + 1).filter(|i| *i < labels.len()) };
    if let (Some(o), true) = (other, index < labels.len()) {
        labels.swap(index, o);
        prj.plugs.entry(plug.to_string()).or_default().order = labels;
    }
}

/// Forgets the stored order: pins sorted by designation.
pub fn sort(prj: &mut Project, plug: &str) {
    prj.plugs.remove(plug);
}

/// Number of pins of the plug's parts (the part with the most `Plug pin` functions,
/// as male and female housings each carry all pins); `None` without such parts.
pub fn capacity(prj: &Project, plug: &str) -> Option<usize> {
    let data = prj.devices.get(plug)?;
    data.parts
        .iter()
        .filter_map(|r| prj.parts.iter().chain(crate::parts::catalog()).find(|p| p.number == r.number))
        .map(|p| p.functions.iter().filter(|f| f.kind == FnKind::PlugPin).count())
        .filter(|n| *n > 0)
        .max()
}

/// One pin designation of a plug with what is connected on both sides.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub label: String,
    /// Placed symbols of this pin; empty for unused pins of the plug parts.
    pub pins: Vec<Pin>,
    /// Connection points wired to the socket side (female) and to the pin side (male).
    pub socket: String,
    pub pin: String,
    pub potential: String,
    pub location: String,
    pub function_text: String,
}

fn targets(prj: &Project, open: &Netlist, ends: &[(u64, usize)], own: &[(usize, usize)]) -> String {
    let mut v: Vec<String> = ends
        .iter()
        .filter_map(|e| open.pin_net.get(e))
        .flat_map(|&n| open.nets[n].pins.iter())
        .filter(|p| !own.contains(&(p.page, p.sym)))
        .map(|p| pin_designation(prj, *p))
        .collect();
    v.sort_by(|a, b| natural_cmp(a, b));
    v.dedup();
    v.join(", ")
}

/// Plug diagram rows: one per pin designation in plug order, then the unused pins of
/// the plug parts. `nl`: potentials, `open`: plug sides apart.
pub fn rows(prj: &Project, plug: &str, nl: &Netlist, open: &Netlist) -> Vec<Row> {
    let all = pins(prj, plug);
    let own: Vec<(usize, usize)> = all.iter().map(|p| (p.page, p.sym)).collect();
    let mut out = Vec::new();
    for label in designations(prj, plug) {
        let pins: Vec<Pin> = all.iter().filter(|p| p.label == label).cloned().collect();
        let (mut socket, mut pin) = (Vec::new(), Vec::new());
        for p in &pins {
            match p.kind {
                PinKind::Pair => {
                    socket.push((p.id, 0));
                    pin.push((p.id, 1));
                }
                PinKind::Female => socket.push((p.id, 0)),
                PinKind::Male => pin.push((p.id, 0)),
            }
        }
        let first = &prj.pages[pins[0].page].symbols[pins[0].sym];
        let mut locations: Vec<String> = pins.iter().map(|p| crate::frame::reference(prj, p.page, prj.pages[p.page].symbols[p.sym].pos)).collect();
        locations.dedup();
        out.push(Row {
            socket: targets(prj, open, &socket, &own),
            pin: targets(prj, open, &pin, &own),
            potential: nl.pin_net.get(&(first.id, 0)).map(|&n| nl.nets[n].name.clone()).unwrap_or_default(),
            location: locations.join(", "),
            function_text: first.function_text.replace('\n', " "),
            label,
            pins,
        });
    }
    if let Some(n) = capacity(prj, plug) {
        for i in 1..=n {
            let label = i.to_string();
            if !out.iter().any(|r| r.label == label) {
                out.push(Row { label, pins: Vec::new(), socket: String::new(), pin: String::new(), potential: String::new(), location: String::new(), function_text: "spare".into() });
            }
        }
    }
    out
}

/// Placement problems of all plugs: (message, jump target).
pub fn problems(prj: &Project) -> Vec<(String, Option<(usize, u64)>)> {
    let mut v = Vec::new();
    for plug in plug_tags(prj) {
        let all = pins(prj, &plug);
        let mut labels: Vec<&str> = Vec::new();
        for p in &all {
            if labels.contains(&p.label.as_str()) {
                continue;
            }
            labels.push(&p.label);
            let same: Vec<&Pin> = all.iter().filter(|x| x.label == p.label).collect();
            let males = same.iter().filter(|x| x.kind != PinKind::Female).count();
            let females = same.iter().filter(|x| x.kind != PinKind::Male).count();
            if males > 1 || females > 1 {
                let target = same.get(1).map(|x| (x.page, x.id));
                v.push((tr_fmt!("{}: pin {} placed {} times", plug, p.label, males.max(females)), target));
            }
        }
        if let Some(n) = capacity(prj, &plug).filter(|n| labels.len() > *n) {
            let target = all.first().map(|x| (x.page, x.id));
            v.push((tr_fmt!("{}: {} pins used, the plug parts have {}", plug, labels.len(), n), target));
        }
    }
    v
}

#[cfg(test)]
mod tests;
