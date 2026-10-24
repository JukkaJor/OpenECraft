//! Connectivity analysis: builds electrical nets (potentials) from wires, connection
//! points, terminals, potential connection points and interruption points. Wires
//! between the ports of fluid power and process symbols form fluid nets (hoses,
//! pipes) that are kept apart from the electrical ones.

use crate::geom::{on_segment_interior, P};
use crate::model::{Page, PageKind, Project};
use crate::symbols::Func;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PinRef {
    pub page: usize,
    /// Index into `page.symbols`.
    pub sym: usize,
    pub pin: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Net {
    pub name: String,
    /// True if the name comes from a potential or interruption point.
    pub named: bool,
    pub pins: Vec<PinRef>,
    /// (page index, wire id)
    pub wires: Vec<(usize, u64)>,
    /// Connects ports of fluid symbols (hose / pipe).
    pub fluid: bool,
    /// Connects fluid ports with electrical connection points (an error).
    pub mixed: bool,
}

#[derive(Default)]
pub struct Netlist {
    pub nets: Vec<Net>,
    pub wire_net: HashMap<u64, usize>,
    /// (symbol id, pin index) -> net
    pub pin_net: HashMap<(u64, usize), usize>,
    /// (symbol id, pin index) of pins that touch nothing else.
    pub unconnected: Vec<(usize, u64, usize)>,
}

struct Uf {
    parent: Vec<usize>,
}

impl Uf {
    fn add(&mut self) -> usize {
        self.parent.push(self.parent.len());
        self.parent.len() - 1
    }
    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[rb] = ra;
        }
    }
}

struct Builder {
    uf: Uf,
    points: HashMap<(usize, i64, i64), usize>,
    named: HashMap<String, usize>,
}

impl Builder {
    fn point(&mut self, page: usize, q: P) -> usize {
        let (x, y) = q.key();
        if let Some(&n) = self.points.get(&(page, x, y)) {
            return n;
        }
        let n = self.uf.add();
        self.points.insert((page, x, y), n);
        n
    }
    fn named(&mut self, name: String) -> usize {
        if let Some(&n) = self.named.get(&name) {
            return n;
        }
        let n = self.uf.add();
        self.named.insert(name, n);
        n
    }
}

/// End points of all wires on a page plus the number of wires ending at each point.
fn wire_ends(page: &Page) -> HashMap<(i64, i64), (P, usize)> {
    let mut ends: HashMap<(i64, i64), (P, usize)> = HashMap::new();
    for w in &page.wires {
        for e in [w.a, w.b] {
            ends.entry(e.key()).or_insert((e, 0)).1 += 1;
        }
    }
    ends
}

/// Positions where a connection dot must be drawn (three or more connections).
pub fn junctions(prj: &Project, page: &Page) -> Vec<P> {
    let ends = wire_ends(page);
    let mut count: HashMap<(i64, i64), (P, usize)> = ends.clone();
    // Ends by their x and y key: a horizontal or vertical wire (the usual case) only
    // passes through ends in its own row or column (±1 key for the 0.01 mm tolerance
    // of `on_segment_interior`), so only those are tested against it.
    let mut by_x: HashMap<i64, Vec<(i64, i64)>> = HashMap::new();
    let mut by_y: HashMap<i64, Vec<(i64, i64)>> = HashMap::new();
    for k in ends.keys() {
        by_x.entry(k.0).or_default().push(*k);
        by_y.entry(k.1).or_default().push(*k);
    }
    let all: Vec<(i64, i64)> = ends.keys().copied().collect();
    let near = |by: &HashMap<i64, Vec<(i64, i64)>>, c: i64| (c - 1..=c + 1).flat_map(|c| by.get(&c).into_iter().flatten().copied()).collect::<Vec<_>>();
    for w in &page.wires {
        let (a, b) = (w.a.key(), w.b.key());
        let candidates = if a.0 == b.0 {
            near(&by_x, a.0)
        } else if a.1 == b.1 {
            near(&by_y, a.1)
        } else {
            all.clone()
        };
        for k in candidates {
            if on_segment_interior(ends[&k].0, w.a, w.b) {
                count.get_mut(&k).unwrap().1 += 2;
            }
        }
    }
    for s in &page.symbols {
        if let Some(def) = prj.symbol(&s.sym) {
            for i in 0..def.pins.len() {
                let q = def.pin_world(i, s.pos, s.rot, s.mirror);
                if let Some(c) = count.get_mut(&q.key()) {
                    c.1 += 1;
                }
            }
        }
    }
    count.into_values().filter(|(_, n)| *n >= 3).map(|(q, _)| q).collect()
}

impl Netlist {
    /// Builds the netlist. With `bridge_terminals == false` the two sides of a terminal
    /// are kept as separate nets (used for terminal diagrams and wire numbers); with
    /// `true` the jumpers of terminal strips connect their terminals as well.
    pub fn build(prj: &Project, bridge_terminals: bool) -> Netlist {
        let mut b = Builder { uf: Uf { parent: Vec::new() }, points: HashMap::new(), named: HashMap::new() };
        let mut pin_nodes: Vec<(PinRef, u64, usize, bool)> = Vec::new();
        let mut wire_nodes: Vec<(usize, u64, usize)> = Vec::new();
        // Number of things touching each point node (to find unconnected pins).
        let mut degree: HashMap<usize, usize> = HashMap::new();
        // (strip, terminal designation) -> node, for jumpers
        let mut terminals: HashMap<(String, String), usize> = HashMap::new();
        // (plug, pin designation) -> nodes of its pins (male and female are mated)
        let mut plug_pins: HashMap<(String, String), Vec<usize>> = HashMap::new();

        for (pi, page) in prj.pages.iter().enumerate() {
            if page.kind != PageKind::Schematic {
                continue;
            }
            let ends = wire_ends(page);
            for w in &page.wires {
                let na = b.point(pi, w.a);
                let nb = b.point(pi, w.b);
                b.uf.union(na, nb);
                *degree.entry(na).or_default() += 1;
                *degree.entry(nb).or_default() += 1;
                for (q, _) in ends.values() {
                    if on_segment_interior(*q, w.a, w.b) {
                        let nq = b.point(pi, *q);
                        b.uf.union(na, nq);
                        *degree.entry(nq).or_default() += 2;
                    }
                }
                wire_nodes.push((pi, w.id, na));
            }
            for (si, s) in page.symbols.iter().enumerate() {
                let Some(def) = prj.symbol(&s.sym) else { continue };
                if def.func == Func::Overview {
                    continue;
                }
                let mut nodes = Vec::new();
                for i in 0..def.pins.len() {
                    let q = def.pin_world(i, s.pos, s.rot, s.mirror);
                    let n = b.point(pi, q);
                    *degree.entry(n).or_default() += 1;
                    nodes.push(n);
                    pin_nodes.push((PinRef { page: pi, sym: si, pin: i }, s.id, n, def.is_fluid()));
                }
                if bridge_terminals || !def.func.is_feedthrough() {
                    for &(x, y) in &def.bridges {
                        b.uf.union(nodes[x], nodes[y]);
                    }
                }
                if def.func == Func::Terminal && !nodes.is_empty() {
                    terminals.insert((s.tag.clone(), s.label.clone()), nodes[0]);
                }
                if matches!(def.func, Func::Plug(_)) && !nodes.is_empty() && !s.tag.is_empty() {
                    plug_pins.entry((s.tag.clone(), s.label.clone())).or_default().push(nodes[0]);
                }
                match def.func {
                    Func::Potential if !s.label.is_empty() => {
                        let n = b.named(format!("pot:{}", s.label));
                        b.uf.union(n, nodes[0]);
                    }
                    Func::Interruption if !s.label.is_empty() => {
                        let n = b.named(format!("int:{}", s.label));
                        b.uf.union(n, nodes[0]);
                    }
                    _ => {}
                }
            }
        }

        if bridge_terminals {
            for nodes in plug_pins.values() {
                for w in nodes.windows(2) {
                    b.uf.union(w[0], w[1]);
                }
            }
            for (strip, labels) in crate::strips::jumper_groups(prj) {
                let nodes: Vec<usize> = labels.iter().filter_map(|l| terminals.get(&(strip.clone(), l.clone())).copied()).collect();
                for w in nodes.windows(2) {
                    b.uf.union(w[0], w[1]);
                }
            }
        }

        let mut nl = Netlist::default();
        let mut root_net: HashMap<usize, usize> = HashMap::new();
        let mut net_for = |uf: &mut Uf, nets: &mut Vec<Net>, node: usize| -> usize {
            let r = uf.find(node);
            *root_net.entry(r).or_insert_with(|| {
                nets.push(Net::default());
                nets.len() - 1
            })
        };
        let mut electric = vec![];
        for (pin, sym_id, node, fluid) in &pin_nodes {
            let ni = net_for(&mut b.uf, &mut nl.nets, *node);
            nl.nets[ni].pins.push(*pin);
            electric.resize(nl.nets.len(), false);
            if *fluid {
                nl.nets[ni].fluid = true;
            } else {
                electric[ni] = true;
            }
            nl.pin_net.insert((*sym_id, pin.pin), ni);
            if degree.get(node).copied().unwrap_or(0) <= 1 {
                nl.unconnected.push((pin.page, *sym_id, pin.pin));
            }
        }
        for (pi, wid, node) in &wire_nodes {
            let ni = net_for(&mut b.uf, &mut nl.nets, *node);
            nl.nets[ni].wires.push((*pi, *wid));
            nl.wire_net.insert(*wid, ni);
        }
        for (ni, net) in nl.nets.iter_mut().enumerate() {
            net.mixed = net.fluid && electric.get(ni).copied().unwrap_or(false);
        }
        // Names: potentials win over interruption points. Two different potentials in
        // one net stay visible as "A / B" (short circuit).
        let mut pots: HashMap<usize, Vec<String>> = HashMap::new();
        let mut ints: HashMap<usize, Vec<String>> = HashMap::new();
        for (name, node) in &b.named {
            let r = b.uf.find(*node);
            if let Some(&ni) = root_net.get(&r) {
                match name.split_once(':') {
                    Some(("pot", n)) => pots.entry(ni).or_default().push(n.to_string()),
                    Some((_, n)) => ints.entry(ni).or_default().push(n.to_string()),
                    None => {}
                }
            }
        }
        for (ni, net) in nl.nets.iter_mut().enumerate() {
            if let Some(v) = pots.get_mut(&ni) {
                v.sort();
                net.name = v.join(" / ");
                net.named = true;
            } else if let Some(v) = ints.get_mut(&ni) {
                v.sort();
                net.name = v[0].clone();
                net.named = true;
            }
        }
        let (mut counter, mut fluid) = (0, 0);
        for net in &mut nl.nets {
            if net.named {
            } else if net.fluid && !net.mixed {
                fluid += 1;
                net.name = format!("F{fluid}");
            } else {
                counter += 1;
                net.name = format!("N{counter}");
            }
        }
        nl
    }

    pub fn net_of_wire(&self, id: u64) -> Option<&Net> {
        self.wire_net.get(&id).map(|&i| &self.nets[i])
    }
}

/// A point-to-point connection between two device connection points.
#[derive(Clone, Debug)]
pub struct Connection {
    /// Potential (from the netlist with bridged terminals).
    pub net: String,
    pub a: PinRef,
    pub b: PinRef,
    pub color: String,
    pub cross_section: String,
    pub number: String,
    /// Cable tag and core, if the wires are a cable core.
    pub cable: String,
    pub core: String,
    /// Harness the connection belongs to.
    pub harness: String,
}

/// Point-to-point connections: the device connection points of each connection of
/// `open` (terminal sides apart) are chained in schematic order (potentials and
/// connectors are not physical wire ends). Devices placed in the same enclosure are
/// chained first, so the wiring stays inside the cabinet and one wire leaves for the
/// devices outside. Potential names come from `nl`. Fluid nets are left out (see
/// [`crate::fluid::hoses`]).
pub fn connections(prj: &Project, nl: &Netlist, open: &Netlist) -> Vec<Connection> {
    let mut out = Vec::new();
    for net in open.nets.iter().filter(|n| !n.fluid) {
        let mut pins = chained_pins(prj, net);
        let enclosure = |p: &PinRef| crate::panel::locate(prj, &prj.pages[p.page].symbols[p.sym].tag).map(|(e, _)| e).unwrap_or(usize::MAX);
        pins.sort_by_key(enclosure);
        if pins.len() < 2 {
            continue;
        }
        let wires = crate::wirenum::wires_of(prj, net);
        let props = crate::potentials::props_of(prj, nl, net);
        let (color, cross_section) = (props.color, props.cross_section);
        let (cable, core) = wires.iter().find(|w| !w.cable.is_empty()).map(|w| (w.cable.clone(), w.core.clone())).unwrap_or_default();
        let number = crate::wirenum::number_of(prj, net);
        let first = pins[0];
        let potential = nl
            .pin_net
            .get(&(prj.pages[first.page].symbols[first.sym].id, first.pin))
            .map(|&n| nl.nets[n].name.clone())
            .unwrap_or_else(|| net.name.clone());
        for pair in pins.windows(2) {
            let harness = crate::harness::of(prj, &pin_designation(prj, pair[0]), &pin_designation(prj, pair[1]));
            out.push(Connection {
                net: potential.clone(),
                a: pair[0],
                b: pair[1],
                color: color.clone(),
                cross_section: cross_section.clone(),
                number: number.clone(),
                cable: cable.clone(),
                core: core.clone(),
                harness,
            });
        }
    }
    out
}

/// Connection points of the devices in a net in schematic order (page, then left to
/// right, top to bottom); potentials and connectors are left out.
pub fn chained_pins(prj: &Project, net: &Net) -> Vec<PinRef> {
    let mut pins: Vec<PinRef> = net
        .pins
        .iter()
        .filter(|p| {
            let s = &prj.pages[p.page].symbols[p.sym];
            prj.symbol(&s.sym).map(|d| d.func.has_tag()).unwrap_or(false)
        })
        .copied()
        .collect();
    let at = |p: &PinRef| {
        let s = &prj.pages[p.page].symbols[p.sym];
        prj.symbol(&s.sym).map(|d| d.pin_world(p.pin, s.pos, s.rot, s.mirror)).unwrap_or(s.pos)
    };
    pins.sort_by(|a, b| {
        let (qa, qb) = (at(a), at(b));
        a.page.cmp(&b.page).then(qa.x.total_cmp(&qb.x)).then(qa.y.total_cmp(&qb.y))
    });
    pins
}

/// Human readable designation of a pin: `-K1:A1`, `=L1`, `-X1:3`.
pub fn pin_designation(prj: &Project, pin: PinRef) -> String {
    let s = &prj.pages[pin.page].symbols[pin.sym];
    let Some(def) = prj.symbol(&s.sym) else { return "?".into() };
    match def.func {
        Func::Potential => format!("={}", s.label),
        Func::Interruption => format!("»{}", s.label),
        Func::Terminal | Func::Plug(_) => format!("{}:{}", s.tag, s.label),
        Func::Plc => format!("{}:{}", s.tag, s.label),
        _ => {
            let pl = s.pins.get(pin.pin).map(String::as_str).unwrap_or("");
            if pl.is_empty() {
                s.tag.clone()
            } else {
                format!("{}:{}", s.tag, pl)
            }
        }
    }
}

#[cfg(test)]
mod tests;
