//! Busbar systems and the devices mounted on them. Every bar of a system is one
//! conductor: the devices on a system (terminals for the supply cables, fuse switch
//! disconnectors and circuit breakers with their feed side on the bars) may join no
//! more conductors than it has bars, otherwise the bars short-circuit them.

use super::{ItemKind, PanelItem};
use crate::issue::{Issue, Severity};
use crate::model::*;
use crate::netlist::Netlist;
use crate::parts::vec3::{box_triangles, Tri};
use crate::parts::FnKind;
use crate::symbols::Func;
use std::collections::{BTreeMap, BTreeSet};

/// The conductors a busbar system joins: the nets on its bars (bridged netlist).
#[derive(Clone, Debug, PartialEq)]
pub struct BusbarUse {
    pub enclosure: usize,
    pub busbar: u64,
    pub bars: usize,
    pub nets: BTreeSet<usize>,
    /// The devices on the system.
    pub devices: Vec<String>,
}

/// Devices mounted on the busbar item `bar`: on its face, overlapping it, with a part
/// mounted on busbars.
fn mounted_on<'a>(prj: &Project, items: &'a [PanelItem], bar: &PanelItem) -> Vec<&'a PanelItem> {
    let on_busbar = |it: &PanelItem| super::part_by_number(prj, &it.part).is_some_and(|p| p.mounting.to_lowercase().contains("busbar"));
    items
        .iter()
        .filter(|it| it.kind == ItemKind::Device && it.face == bar.face && !it.tag.is_empty() && on_busbar(it))
        .filter(|it| it.x < bar.x + bar.w && bar.x < it.x + it.w && it.y < bar.y + bar.h && bar.y < it.y + it.h)
        .collect()
}

/// Nets that are one conductor: the primary of a current transformer passes it
/// through (a transformer clipped in behind a strip, a set in the busbar). Each net to
/// the first net of its conductor.
fn conductors(prj: &Project, nl: &Netlist) -> BTreeMap<usize, usize> {
    let mut parent: BTreeMap<usize, usize> = BTreeMap::new();
    fn root(parent: &BTreeMap<usize, usize>, mut n: usize) -> usize {
        while let Some(&p) = parent.get(&n).filter(|&&p| p != n) {
            n = p;
        }
        n
    }
    let symbols = prj.pages.iter().filter(|p| p.kind == PageKind::Schematic).flat_map(|p| &p.symbols);
    for s in symbols.filter(|s| crate::parts::kind_of_symbol(&s.sym) == Some(FnKind::CurrentTransformer)) {
        if let (Some(&a), Some(&b)) = (nl.pin_net.get(&(s.id, 0)), nl.pin_net.get(&(s.id, 1))) {
            let (ra, rb) = (root(&parent, a), root(&parent, b));
            if ra != rb {
                parent.insert(ra.max(rb), ra.min(rb));
            }
        }
    }
    let keys: Vec<usize> = parent.keys().copied().collect();
    keys.into_iter().map(|n| (n, root(&parent, n))).collect()
}

/// The two sides a device can have on the bars: the conductors of its feed side (odd
/// designations, N) and of its outgoing side (even designations, N'). Terminals have
/// one side.
fn sides(prj: &Project, nl: &Netlist, alias: &BTreeMap<usize, usize>, tag: &str) -> Vec<BTreeSet<usize>> {
    let symbols: Vec<&SymbolInst> = prj.pages.iter().filter(|p| p.kind == PageKind::Schematic).flat_map(|p| &p.symbols).filter(|s| s.tag == tag).collect();
    let kind = |s: &SymbolInst| crate::parts::kind_of_symbol(&s.sym);
    let canon = |n: usize| *alias.get(&n).unwrap_or(&n);
    let (mut feed, mut out, mut terminals) = (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
    for s in &symbols {
        let Some(def) = prj.symbol(&s.sym) else { continue };
        if def.func == Func::Terminal {
            terminals.extend((0..def.pins.len()).filter_map(|i| nl.pin_net.get(&(s.id, i)).map(|&n| canon(n))));
            continue;
        }
        let power = matches!(
            kind(s),
            Some(
                FnKind::Fuse
                    | FnKind::Fuse3
                    | FnKind::Breaker1
                    | FnKind::Breaker2
                    | FnKind::Breaker3
                    | FnKind::Breaker4
                    | FnKind::Disconnector3
                    | FnKind::MotorProtection
                    | FnKind::ResidualCurrent
                    | FnKind::MainContacts3
                    | FnKind::MainContacts4
            )
        );
        if !power {
            continue;
        }
        for i in 0..def.pins.len() {
            let Some(&n) = nl.pin_net.get(&(s.id, i)) else { continue };
            let label = s.pins.get(i).map(|p| p.trim()).unwrap_or_default();
            let outgoing = match label.parse::<u32>().ok() {
                Some(d) => d % 2 == 0,
                None => label.ends_with('\''),
            };
            if outgoing { &mut out } else { &mut feed }.insert(canon(n));
        }
    }
    if !terminals.is_empty() {
        return vec![terminals];
    }
    [feed, out].into_iter().filter(|s| !s.is_empty()).collect()
}

/// The busbar systems of the panel layout and the conductors on their bars: of each
/// device the side that shares its conductors with the other devices on the system
/// (the feed side when none does: a feeder strip of an incoming cable is turned).
pub fn uses(prj: &Project, nl: &Netlist) -> Vec<BusbarUse> {
    let mut out = Vec::new();
    let alias = conductors(prj, nl);
    for (ei, e) in prj.layout.enclosures.iter().enumerate() {
        for bar in e.items.iter().filter(|i| i.kind == ItemKind::Busbar) {
            let devices: Vec<&PanelItem> = mounted_on(prj, &e.items, bar);
            let mut tags: Vec<String> = devices.iter().map(|d| d.tag.clone()).collect();
            tags.dedup();
            let all: Vec<Vec<BTreeSet<usize>>> = tags.iter().map(|t| sides(prj, nl, &alias, t)).collect();
            let mut nets = BTreeSet::new();
            for (k, own) in all.iter().enumerate() {
                let others: BTreeSet<usize> = all.iter().enumerate().filter(|(j, _)| *j != k).flat_map(|(_, s)| s.iter().flatten().copied()).collect();
                let pick = own.iter().max_by_key(|s| (s.intersection(&others).count(), std::cmp::Reverse(own.iter().position(|o| o == *s))));
                if let Some(side) = pick {
                    nets.extend(side.iter().copied());
                }
            }
            out.push(BusbarUse { enclosure: ei, busbar: bar.id, bars: crate::production::bar_count(&prj.parts, &bar.part), nets, devices: tags });
        }
    }
    out
}

/// The bars of a busbar system for the 3D view: each bar on edge at the front of the
/// system, evenly spaced across it like in the 2D view (part data `Bars`, `Bar: Cu 30 x
/// 10 mm`), on insulating holders behind them.
pub fn mesh(prj: &Project, it: &PanelItem) -> Vec<(Tri, Option<[u8; 3]>)> {
    const COPPER: [u8; 3] = [200, 120, 60];
    const HOLDER: [u8; 3] = [70, 72, 78];
    let n = crate::production::bar_count(&prj.parts, &it.part);
    let section = super::part_by_number(prj, &it.part).and_then(|p| p.tech_value("Bar")).map(numbers).unwrap_or_default();
    let (big, small) = match section.as_slice() {
        [a, b, ..] => (a.max(*b), a.min(*b)),
        _ => (30.0, 10.0),
    };
    let horizontal = it.w >= it.h;
    let (along, across) = if horizontal { (it.w, it.h) } else { (it.h, it.w) };
    let pitch = across / n as f32;
    let thick = small.min(pitch * 0.5);
    let depth = big.min(it.d * 0.6);
    // Along / across the system and depth to plate coordinates.
    let at = |a: f32, c: f32, z: f32| if horizontal { [it.x + a, it.y + c, it.z + z] } else { [it.x + c, it.y + a, it.z + z] };
    let block = |a0: f32, a1: f32, c0: f32, c1: f32, z0: f32, z1: f32| {
        let (p, q) = (at(a0, c0, z0), at(a1, c1, z1));
        box_triangles([p[0].min(q[0]), p[1].min(q[1]), p[2]], [p[0].max(q[0]), p[1].max(q[1]), q[2]])
    };
    let mut out = Vec::new();
    for k in 0..n {
        let c = pitch * (k as f32 + 0.5);
        out.extend(block(0.0, along, c - thick / 2.0, c + thick / 2.0, it.d - depth, it.d).into_iter().map(|t| (t, Some(COPPER))));
    }
    let holders = ((along / 400.0).ceil() as usize).max(1) + 1;
    for k in 0..holders {
        let a = (along - 30.0) * k as f32 / (holders - 1) as f32;
        out.extend(block(a, a + 30.0, 0.0, across, 0.0, it.d - depth).into_iter().map(|t| (t, Some(HOLDER))));
    }
    out
}

/// The numbers in a text (`Cu 30 x 10 mm` → 30, 10).
fn numbers(text: &str) -> Vec<f32> {
    text.split(|c: char| !(c.is_ascii_digit() || c == '.')).filter_map(|s| s.parse().ok()).collect()
}

/// Busbar systems joining more conductors than they have bars.
pub fn problems(prj: &Project, nl: &Netlist) -> Vec<Issue> {
    let mut v = Vec::new();
    for u in uses(prj, nl).into_iter().filter(|u| u.nets.len() > u.bars) {
        let e = &prj.layout.enclosures[u.enclosure];
        let names: Vec<String> = u.nets.iter().map(|&n| nl.nets[n].name.clone()).collect();
        let target = prj.pages.iter().enumerate().find_map(|(pi, pg)| pg.symbols.iter().find(|s| u.devices.contains(&s.tag)).map(|s| (pi, s.id)));
        v.push(crate::issue!(
            Severity::Error,
            target,
            "{}: the busbar system with {} bars joins {} conductors ({}): short circuit",
            e.name,
            u.bars,
            u.nets.len(),
            names.join(", ")
        ));
    }
    v
}

#[cfg(test)]
mod tests;
