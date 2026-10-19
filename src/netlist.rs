//! Connectivity analysis: builds electrical nets (potentials) from wires, connection
//! points, terminals, potential connection points and interruption points.

use crate::geom::{on_segment_interior, P};
use crate::model::{Page, PageKind, Project};
use crate::symbols::{self, Func};
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
pub fn junctions(page: &Page) -> Vec<P> {
    let ends = wire_ends(page);
    let mut count: HashMap<(i64, i64), (P, usize)> = ends.clone();
    for w in &page.wires {
        for (k, (q, _)) in &ends {
            if on_segment_interior(*q, w.a, w.b) {
                count.get_mut(k).unwrap().1 += 2;
            }
        }
    }
    for s in &page.symbols {
        if let Some(def) = symbols::get(&s.sym) {
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
    /// are kept as separate nets (used for terminal diagrams).
    pub fn build(prj: &Project, bridge_terminals: bool) -> Netlist {
        let mut b = Builder { uf: Uf { parent: Vec::new() }, points: HashMap::new(), named: HashMap::new() };
        let mut pin_nodes: Vec<(PinRef, u64, usize)> = Vec::new();
        let mut wire_nodes: Vec<(usize, u64, usize)> = Vec::new();
        // Number of things touching each point node (to find unconnected pins).
        let mut degree: HashMap<usize, usize> = HashMap::new();

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
                let Some(def) = symbols::get(&s.sym) else { continue };
                let mut nodes = Vec::new();
                for i in 0..def.pins.len() {
                    let q = def.pin_world(i, s.pos, s.rot, s.mirror);
                    let n = b.point(pi, q);
                    *degree.entry(n).or_default() += 1;
                    nodes.push(n);
                    pin_nodes.push((PinRef { page: pi, sym: si, pin: i }, s.id, n));
                }
                if bridge_terminals || def.func != Func::Terminal {
                    for &(x, y) in &def.bridges {
                        b.uf.union(nodes[x], nodes[y]);
                    }
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

        let mut nl = Netlist::default();
        let mut root_net: HashMap<usize, usize> = HashMap::new();
        let mut net_for = |uf: &mut Uf, nets: &mut Vec<Net>, node: usize| -> usize {
            let r = uf.find(node);
            *root_net.entry(r).or_insert_with(|| {
                nets.push(Net::default());
                nets.len() - 1
            })
        };
        for (pin, sym_id, node) in &pin_nodes {
            let ni = net_for(&mut b.uf, &mut nl.nets, *node);
            nl.nets[ni].pins.push(*pin);
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
        let mut counter = 0;
        for net in &mut nl.nets {
            if !net.named {
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
    pub net: String,
    pub a: PinRef,
    pub b: PinRef,
    pub color: String,
    pub cross_section: String,
}

/// Point-to-point connections: the device connection points of each net are chained
/// in schematic order (potentials and connectors are not physical wire ends).
pub fn connections(prj: &Project, nl: &Netlist) -> Vec<Connection> {
    let mut out = Vec::new();
    for net in &nl.nets {
        let mut pins: Vec<&PinRef> = net
            .pins
            .iter()
            .filter(|p| {
                let s = &prj.pages[p.page].symbols[p.sym];
                symbols::get(&s.sym).map(|d| d.func.has_tag()).unwrap_or(false)
            })
            .collect();
        if pins.len() < 2 {
            continue;
        }
        pins.sort_by(|a, b| {
            let (sa, sb) = (&prj.pages[a.page].symbols[a.sym], &prj.pages[b.page].symbols[b.sym]);
            a.page.cmp(&b.page).then(sa.pos.x.total_cmp(&sb.pos.x)).then(sa.pos.y.total_cmp(&sb.pos.y))
        });
        let (color, cross_section) = net
            .wires
            .iter()
            .find_map(|(pi, wid)| {
                prj.pages[*pi].wires.iter().find(|w| w.id == *wid && (!w.color.is_empty() || !w.cross_section.is_empty()))
            })
            .map(|w| (w.color.clone(), w.cross_section.clone()))
            .unwrap_or_default();
        for pair in pins.windows(2) {
            out.push(Connection {
                net: net.name.clone(),
                a: *pair[0],
                b: *pair[1],
                color: color.clone(),
                cross_section: cross_section.clone(),
            });
        }
    }
    out
}

/// Human readable designation of a pin: `-K1:A1`, `=L1`, `-X1:3`.
pub fn pin_designation(prj: &Project, pin: PinRef) -> String {
    let s = &prj.pages[pin.page].symbols[pin.sym];
    let Some(def) = symbols::get(&s.sym) else { return "?".into() };
    match def.func {
        Func::Potential => format!("={}", s.label),
        Func::Interruption => format!("»{}", s.label),
        Func::Terminal => format!("{}:{}", s.tag, s.label),
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
