//! 3D topology of a harness: nodes (connector positions, branch points) and the
//! segments between them with their centre curves, the way of every wire through the
//! segments and the bundle diameter of each segment. A topology stored in the harness
//! (edited by the user) takes precedence; otherwise it is derived from the 3D routes of
//! the panel layout, and connectors outside the layout are placed around the common
//! node like on the nailboard (branch lengths fitted to the wire lengths).

use super::{bundle_diameter, HarnessWire};
use crate::analysis::Analysis;
use crate::model::harness::{Harness, Node as StoredNode, Segment as StoredSegment};
use crate::model::{natural_cmp, Project};
use openecraft_parts::vec3::{add, dist, norm, sub, V3};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

/// Branch length used when no wire of a connector has a length, mm.
pub const DEFAULT_BRANCH: f32 = 300.0;
/// Shortest branch, mm.
const MIN_BRANCH: f32 = 50.0;

/// Branch lengths `b` with `b[i] + b[j]` close to the length of every wire between
/// connectors `i` and `j` (least squares, by Gauss-Seidel iteration); connectors
/// without lengths get [`DEFAULT_BRANCH`].
pub fn branch_lengths(n: usize, wires: &[(usize, usize, f32)]) -> Vec<(f32, bool)> {
    let known: Vec<&(usize, usize, f32)> = wires.iter().filter(|w| w.2 > 0.0 && w.0 != w.1).collect();
    let mut b: Vec<f32> = (0..n)
        .map(|i| {
            let l: Vec<f32> = known.iter().filter(|w| w.0 == i || w.1 == i).map(|w| w.2 / 2.0).collect();
            if l.is_empty() { DEFAULT_BRANCH } else { l.iter().sum::<f32>() / l.len() as f32 }
        })
        .collect();
    for _ in 0..200 {
        for i in 0..n {
            let est: Vec<f32> = known.iter().filter_map(|w| if w.0 == i { Some(w.2 - b[w.1]) } else if w.1 == i { Some(w.2 - b[w.0]) } else { None }).collect();
            if !est.is_empty() {
                b[i] = (est.iter().sum::<f32>() / est.len() as f32).max(MIN_BRANCH);
            }
        }
    }
    (0..n).map(|i| (b[i], !known.iter().any(|w| w.0 == i || w.1 == i))).collect()
}

/// Where a topology comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Stored in the harness.
    Stored,
    /// Derived from the 3D routes of the panel layout (connectors outside it placed
    /// around the common node).
    Layout,
    /// Generated as a star around the common node (no connector in the layout).
    Generated,
}

impl Source {
    pub fn key(self) -> &'static str {
        match self {
            Source::Stored => "stored",
            Source::Layout => "layout",
            Source::Generated => "generated",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    pub pos: V3,
    /// Device tags of the connectors at the node.
    pub connectors: Vec<String>,
    pub bend_radius: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub id: String,
    /// Node indices.
    pub start: usize,
    pub end: usize,
    /// Centre curve from the start node to the end node, mm.
    pub points: Vec<V3>,
    /// Length of the centre curve, mm.
    pub physical: f32,
    /// Formboard length, mm.
    pub length: f32,
    /// Indices into the harness wires running through the segment.
    pub wires: Vec<usize>,
    /// Bundle diameter, mm.
    pub diameter: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Topology {
    pub source: Source,
    pub nodes: Vec<Node>,
    pub segments: Vec<Segment>,
    /// Per harness wire the segments from its first to its second end; `None`: the
    /// wire cannot be routed (an end has no node, the nodes are not connected).
    pub routes: Vec<Option<Vec<usize>>>,
}

impl Topology {
    pub fn node_of(&self, device: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.connectors.iter().any(|c| c == device))
    }

    pub fn node_index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    pub fn segment_index(&self, id: &str) -> Option<usize> {
        self.segments.iter().position(|s| s.id == id)
    }

    /// Length of a wire along its segments, mm.
    pub fn wire_length(&self, wire: usize) -> Option<f32> {
        self.routes.get(wire)?.as_ref().map(|r| r.iter().map(|&s| self.segments[s].length).sum())
    }

    /// Sum of all segment lengths, mm.
    pub fn length(&self) -> f32 {
        self.segments.iter().map(|s| s.length).sum()
    }

    /// Unit vector of the centre curve at the start and at the end of a segment.
    pub fn vectors(&self, seg: usize) -> (V3, V3) {
        let p = &self.segments[seg].points;
        let dir = |a: V3, b: V3| if dist(a, b) < 1e-6 { [1.0, 0.0, 0.0] } else { norm(sub(b, a)) };
        match p.len() {
            0 | 1 => ([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
            n => (dir(p[0], p[1]), dir(p[n - 2], p[n - 1])),
        }
    }

    /// Smallest and largest coordinates of all points.
    pub fn bounds(&self) -> (V3, V3) {
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for q in self.nodes.iter().map(|n| n.pos).chain(self.segments.iter().flat_map(|s| s.points.iter().copied())) {
            for k in 0..3 {
                min[k] = min[k].min(q[k]);
                max[k] = max[k].max(q[k]);
            }
        }
        if min[0] > max[0] { ([0.0; 3], [0.0; 3]) } else { (min, max) }
    }
}

/// Length of a polyline.
pub fn polyline_length(points: &[V3]) -> f32 {
    points.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// Point at the fraction `t` (0 … 1) of the length of a polyline.
pub fn point_at(points: &[V3], t: f32) -> V3 {
    let total = polyline_length(points);
    let mut rest = t.clamp(0.0, 1.0) * total;
    for w in points.windows(2) {
        let l = dist(w[0], w[1]);
        if rest <= l && l > 0.0 {
            return add(w[0], sub(w[1], w[0]).map(|c| c * rest / l));
        }
        rest -= l;
    }
    points.last().copied().unwrap_or([0.0; 3])
}

/// The topology of a harness: stored, or derived from the layout and generated.
pub fn of(prj: &Project, an: &Analysis, tag: &str) -> Option<Topology> {
    let h = prj.harnesses.iter().find(|h| h.tag == tag)?;
    let ws = super::wires(prj, an, tag);
    Some(build(prj, an, h, &ws))
}

pub fn build(prj: &Project, an: &Analysis, h: &Harness, wires: &[HarnessWire]) -> Topology {
    if h.nodes.is_empty() { derive(prj, an, wires) } else { stored(h, wires) }
}

/// Stores the derived topology in the harness for editing; returns the number of
/// nodes and segments (`None`: no harness with the tag).
pub fn store(prj: &mut Project, an: &Analysis, tag: &str) -> Option<(usize, usize)> {
    let ws = super::wires(prj, an, tag);
    let t = derive(prj, an, &ws);
    let h = prj.harnesses.iter_mut().find(|h| h.tag == tag)?;
    h.nodes = t.nodes.iter().map(|n| StoredNode { id: n.id.clone(), pos: n.pos.map(round), connectors: n.connectors.clone(), bend_radius: n.bend_radius }).collect();
    h.segments = t
        .segments
        .iter()
        .map(|s| StoredSegment {
            id: s.id.clone(),
            start: t.nodes[s.start].id.clone(),
            end: t.nodes[s.end].id.clone(),
            points: s.points[1..s.points.len() - 1].iter().map(|q| q.map(round)).collect(),
            length: 0.0,
        })
        .collect();
    Some((h.nodes.len(), h.segments.len()))
}

fn round(v: f32) -> f32 {
    (v * 10.0).round() / 10.0
}

/// Drops the stored topology (the derived one is used again).
pub fn clear(prj: &mut Project, tag: &str) -> bool {
    match prj.harnesses.iter_mut().find(|h| h.tag == tag) {
        Some(h) => {
            let had = !h.nodes.is_empty() || !h.segments.is_empty();
            h.nodes.clear();
            h.segments.clear();
            had
        }
        None => false,
    }
}

fn finish(source: Source, nodes: Vec<Node>, mut segments: Vec<Segment>, routes: Vec<Option<Vec<usize>>>, wires: &[HarnessWire]) -> Topology {
    for (i, s) in segments.iter_mut().enumerate() {
        s.wires = (0..wires.len()).filter(|&w| routes[w].as_ref().is_some_and(|r| r.contains(&i))).collect();
        let own: Vec<HarnessWire> = s.wires.iter().map(|&w| wires[w].clone()).collect();
        s.diameter = if own.is_empty() { 0.0 } else { bundle_diameter(&own) };
    }
    Topology { source, nodes, segments, routes }
}

fn stored(h: &Harness, wires: &[HarnessWire]) -> Topology {
    let nodes: Vec<Node> = h.nodes.iter().map(|n| Node { id: n.id.clone(), pos: n.pos, connectors: n.connectors.clone(), bend_radius: n.bend_radius }).collect();
    let at = |id: &str| nodes.iter().position(|n| n.id == id);
    let segments: Vec<Segment> = h
        .segments
        .iter()
        .filter_map(|s| {
            let (a, b) = (at(&s.start)?, at(&s.end)?);
            let points: Vec<V3> = std::iter::once(nodes[a].pos).chain(s.points.iter().copied()).chain(std::iter::once(nodes[b].pos)).collect();
            let physical = polyline_length(&points);
            Some(Segment { id: s.id.clone(), start: a, end: b, physical, length: if s.length > 0.0 { s.length } else { physical }, points, wires: Vec::new(), diameter: 0.0 })
        })
        .collect();
    let node_of = |dev: &str| nodes.iter().position(|n| n.connectors.iter().any(|c| c == dev));
    let routes = wires.iter().map(|w| shortest(&nodes, &segments, node_of(&w.from.0)?, node_of(&w.to.0)?)).collect();
    finish(Source::Stored, nodes, segments, routes, wires)
}

#[derive(PartialEq)]
struct Visit(f32, usize);

impl Eq for Visit {}

impl PartialOrd for Visit {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Visit {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.0.total_cmp(&self.0).then(other.1.cmp(&self.1))
    }
}

/// Segments of the shortest way between two nodes (Dijkstra).
fn shortest(nodes: &[Node], segments: &[Segment], from: usize, to: usize) -> Option<Vec<usize>> {
    let mut best = vec![f32::MAX; nodes.len()];
    let mut via: Vec<Option<usize>> = vec![None; nodes.len()];
    let mut heap = BinaryHeap::new();
    best[from] = 0.0;
    heap.push(Visit(0.0, from));
    while let Some(Visit(d, n)) = heap.pop() {
        if n == to {
            break;
        }
        if d > best[n] {
            continue;
        }
        for (si, s) in segments.iter().enumerate() {
            let other = if s.start == n { s.end } else if s.end == n { s.start } else { continue };
            let nd = d + s.length.max(0.0);
            if nd < best[other] {
                best[other] = nd;
                via[other] = Some(si);
                heap.push(Visit(nd, other));
            }
        }
    }
    if best[to] == f32::MAX {
        return None;
    }
    let mut route = Vec::new();
    let mut n = to;
    while n != from {
        let si = via[n]?;
        route.push(si);
        n = if segments[si].start == n { segments[si].end } else { segments[si].start };
    }
    route.reverse();
    Some(route)
}

/// The 3D route of a harness wire in the panel layout, from its first to its second
/// end (cable cores: the route of their cable).
fn layout_points(an: &Analysis, w: &HarnessWire) -> Option<Vec<V3>> {
    let (a, b) = (super::joined(&w.from), super::joined(&w.to));
    let item = if w.cable.is_empty() {
        let (i, rw) = an.wiring.iter().enumerate().find(|(_, rw)| (rw.from == a && rw.to == b) || (rw.from == b && rw.to == a))?;
        let it = an.routing.items.iter().find(|it| it.wire == Some(i) && it.routed())?;
        (it, rw.from != a)
    } else {
        let it = an.routing.items.iter().find(|it| it.wire.is_none() && it.key == w.cable && it.routed())?;
        (it, it.ends[0].tag != w.from.0)
    };
    let mut points = item.0.points.clone();
    if item.1 {
        points.reverse();
    }
    (points.len() >= 2).then_some(points)
}

type Key = [i32; 3];

/// Keys of the grid cells along a polyline. Each stretch is sampled from its smaller
/// end, so that wires on the same line hit the same cells in either direction.
fn cells(points: &[V3], cell: f32) -> Vec<Key> {
    let key = |q: V3| -> Key { [0, 1, 2].map(|i| (q[i] / cell).round() as i32) };
    let mut out: Vec<Key> = Vec::new();
    for s in points.windows(2) {
        let flip = s[1] < s[0];
        let (a, b) = if flip { (s[1], s[0]) } else { (s[0], s[1]) };
        let steps = (dist(a, b) / (cell * 0.5)).ceil().max(1.0) as usize;
        let mut v: Vec<Key> = (0..=steps).map(|k| key(add(a, sub(b, a).map(|c| c * k as f32 / steps as f32)))).collect();
        if flip {
            v.reverse();
        }
        for k in v {
            if out.last() != Some(&k) {
                out.push(k);
            }
        }
    }
    out
}

/// Positions of the connectors: averaged route ends in the layout, the others around
/// the common node at their fitted branch length.
fn connector_positions(devices: &[String], wires: &[HarnessWire], polys: &[Option<Vec<V3>>]) -> (Vec<Option<V3>>, V3, bool) {
    let mut sums: Vec<(V3, usize)> = vec![([0.0; 3], 0); devices.len()];
    let at = |d: &str| devices.iter().position(|x| x == d).unwrap_or(0);
    for (w, p) in wires.iter().zip(polys) {
        if let Some(p) = p {
            for (dev, q) in [(&w.from.0, p[0]), (&w.to.0, p[p.len() - 1])] {
                let s = &mut sums[at(dev)];
                s.0 = add(s.0, q);
                s.1 += 1;
            }
        }
    }
    let placed: Vec<Option<V3>> = sums.iter().map(|(s, n)| (*n > 0).then(|| s.map(|c| c / *n as f32))).collect();
    let known: Vec<V3> = placed.iter().flatten().copied().collect();
    let hub = if known.is_empty() { [0.0; 3] } else { known.iter().fold([0.0; 3], |a, q| add(a, *q)).map(|c| c / known.len() as f32) };
    (placed, hub, !known.is_empty())
}

fn derive(prj: &Project, an: &Analysis, wires: &[HarnessWire]) -> Topology {
    let mut devices: Vec<String> = wires.iter().flat_map(|w| [w.from.0.clone(), w.to.0.clone()]).collect();
    devices.sort_by(|a, b| natural_cmp(a, b));
    devices.dedup();
    let at = |d: &str| devices.iter().position(|x| x == d).unwrap_or(0);
    let polys: Vec<Option<Vec<V3>>> = wires.iter().map(|w| layout_points(an, w)).collect();
    let (placed, hub, in_layout) = connector_positions(&devices, wires, &polys);
    // Connectors outside the layout: around the common node, as on the nailboard.
    let pairs: Vec<(usize, usize, f32)> = wires.iter().map(|w| (at(&w.from.0), at(&w.to.0), w.length)).collect();
    let lengths = branch_lengths(devices.len(), &pairs);
    let free: Vec<usize> = (0..devices.len()).filter(|&i| placed[i].is_none()).collect();
    let pos: Vec<V3> = (0..devices.len())
        .map(|i| {
            placed[i].unwrap_or_else(|| {
                let k = free.iter().position(|&f| f == i).unwrap_or(0);
                let angle = if free.len() == 2 && !in_layout { if k == 0 { 180.0f32 } else { 0.0 } } else { 180.0 - 360.0 * k as f32 / free.len() as f32 };
                let a = angle.to_radians();
                add(hub, [a.cos() * lengths[i].0, -a.sin() * lengths[i].0, 0.0])
            })
        })
        .collect();
    if !in_layout {
        return star(&devices, &pos, hub, wires);
    }
    let lines: Vec<Vec<V3>> = wires
        .iter()
        .zip(&polys)
        .map(|(w, p)| {
            let (a, b) = (pos[at(&w.from.0)], pos[at(&w.to.0)]);
            match p {
                Some(p) => std::iter::once(a).chain(p[1..p.len() - 1].iter().copied()).chain(std::iter::once(b)).collect(),
                None if placed[at(&w.from.0)].is_some() && placed[at(&w.to.0)].is_some() => vec![a, b],
                None => vec![a, hub, b],
            }
        })
        .collect();
    let cell = prj.layout.routing.grid.max(5.0);
    let key = |q: V3| -> Key { [0, 1, 2].map(|i| (q[i] / cell).round() as i32) };
    let keys: Vec<Vec<Key>> = lines.iter().map(|l| cells(l, cell)).collect();
    // Edges between neighbouring cells and the wires along them.
    let mut edges: BTreeMap<(Key, Key), BTreeSet<usize>> = BTreeMap::new();
    let mut adj: BTreeMap<Key, BTreeSet<Key>> = BTreeMap::new();
    for (i, k) in keys.iter().enumerate() {
        for e in k.windows(2) {
            edges.entry((e[0].min(e[1]), e[0].max(e[1]))).or_default().insert(i);
            adj.entry(e[0]).or_default().insert(e[1]);
            adj.entry(e[1]).or_default().insert(e[0]);
        }
    }
    let edge = |a: Key, b: Key| (a.min(b), a.max(b));
    // Nodes: connectors, branch points and where the wires along the bundle change.
    let mut node_at: BTreeMap<Key, usize> = BTreeMap::new();
    let mut nodes: Vec<Node> = Vec::new();
    for (i, d) in devices.iter().enumerate() {
        let k = key(pos[i]);
        match node_at.get(&k) {
            Some(&n) => nodes[n].connectors.push(d.clone()),
            None => {
                node_at.insert(k, nodes.len());
                nodes.push(Node { id: String::new(), pos: pos[i], connectors: vec![d.clone()], bend_radius: 0.0 });
            }
        }
    }
    for (k, nb) in &adj {
        let n: Vec<&Key> = nb.iter().collect();
        let through = n.len() == 2 && edges[&edge(*k, *n[0])] == edges[&edge(*k, *n[1])];
        if !through && !node_at.contains_key(k) {
            node_at.insert(*k, nodes.len());
            nodes.push(Node { id: String::new(), pos: k.map(|c| c as f32 * cell), connectors: Vec::new(), bend_radius: 0.0 });
        }
    }
    for (i, n) in nodes.iter_mut().enumerate() {
        n.id = format!("N{}", i + 1);
    }
    // Segments: the chains of cells between two nodes.
    let mut seg_of: BTreeMap<(Key, Key), usize> = BTreeMap::new();
    let mut segments: Vec<Segment> = Vec::new();
    let starts: Vec<(Key, usize)> = node_at.iter().map(|(k, n)| (*k, *n)).collect();
    let mut by_node: Vec<(Key, usize)> = starts;
    by_node.sort_by_key(|(_, n)| *n);
    for (k, n) in by_node {
        for next in adj.get(&k).cloned().unwrap_or_default() {
            if seg_of.contains_key(&edge(k, next)) {
                continue;
            }
            let mut chain = vec![k, next];
            while !node_at.contains_key(chain.last().unwrap()) {
                let (last, before) = (chain[chain.len() - 1], chain[chain.len() - 2]);
                let Some(&step) = adj[&last].iter().find(|x| **x != before) else { break };
                if seg_of.contains_key(&edge(last, step)) {
                    break;
                }
                chain.push(step);
            }
            let Some(&end) = node_at.get(chain.last().unwrap()) else { continue };
            let si = segments.len();
            for e in chain.windows(2) {
                seg_of.insert(edge(e[0], e[1]), si);
            }
            let inner = chain[1..chain.len() - 1].iter().map(|c| c.map(|v| v as f32 * cell));
            let points = simplify(std::iter::once(nodes[n].pos).chain(inner).chain(std::iter::once(nodes[end].pos)).collect());
            let physical = polyline_length(&points);
            segments.push(Segment { id: format!("S{}", si + 1), start: n, end, points, physical, length: physical, wires: Vec::new(), diameter: 0.0 });
        }
    }
    let routes = keys
        .iter()
        .map(|k| {
            let mut r: Vec<usize> = Vec::new();
            for e in k.windows(2) {
                let s = *seg_of.get(&edge(e[0], e[1]))?;
                if r.last() != Some(&s) {
                    r.push(s);
                }
            }
            Some(r)
        })
        .collect();
    let (nodes, segments, routes) = merge_short(nodes, segments, routes, 1.5 * cell);
    finish(Source::Layout, nodes, segments, routes, wires)
}

/// The star of the nailboard: a branch from the common node to every connector.
fn star(devices: &[String], pos: &[V3], hub: V3, wires: &[HarnessWire]) -> Topology {
    let mut nodes: Vec<Node> = devices.iter().zip(pos).map(|(d, p)| Node { id: String::new(), pos: *p, connectors: vec![d.clone()], bend_radius: 0.0 }).collect();
    let common = devices.len();
    nodes.push(Node { id: String::new(), pos: hub, connectors: Vec::new(), bend_radius: 0.0 });
    for (i, n) in nodes.iter_mut().enumerate() {
        n.id = format!("N{}", i + 1);
    }
    let segments: Vec<Segment> = (0..devices.len())
        .map(|i| {
            let points = vec![hub, pos[i]];
            let physical = polyline_length(&points);
            Segment { id: format!("S{}", i + 1), start: common, end: i, points, physical, length: physical, wires: Vec::new(), diameter: 0.0 }
        })
        .collect();
    let at = |d: &str| devices.iter().position(|x| x == d).unwrap_or(0);
    let routes = wires.iter().map(|w| Some(if w.from.0 == w.to.0 { Vec::new() } else { vec![at(&w.from.0), at(&w.to.0)] })).collect();
    finish(Source::Generated, nodes, segments, routes, wires)
}

type Parts = (Vec<Node>, Vec<Segment>, Vec<Option<Vec<usize>>>);

/// Removes segments shorter than `min` that end at a node without connectors: that
/// node is merged into the other end (cells next to a branch point or a connector).
fn merge_short(mut nodes: Vec<Node>, mut segments: Vec<Segment>, mut routes: Vec<Option<Vec<usize>>>, min: f32) -> Parts {
    loop {
        let found = segments.iter().position(|s| s.start == s.end || (s.physical < min && (nodes[s.start].connectors.is_empty() || nodes[s.end].connectors.is_empty())));
        let Some(si) = found else { break };
        let s = segments.remove(si);
        for r in routes.iter_mut().flatten() {
            r.retain(|&x| x != si);
            for x in r.iter_mut() {
                if *x > si {
                    *x -= 1;
                }
            }
            r.dedup();
        }
        if s.start == s.end {
            continue;
        }
        let (keep, drop) = if nodes[s.start].connectors.is_empty() { (s.end, s.start) } else { (s.start, s.end) };
        let at = nodes[keep].pos;
        let moved = std::mem::take(&mut nodes[drop].connectors);
        nodes[keep].connectors.extend(moved);
        nodes.remove(drop);
        let fix = |n: usize| if n == drop { if keep > drop { keep - 1 } else { keep } } else if n > drop { n - 1 } else { n };
        for t in segments.iter_mut() {
            if t.start == drop {
                t.points[0] = at;
            }
            if t.end == drop {
                let last = t.points.len() - 1;
                t.points[last] = at;
            }
            t.start = fix(t.start);
            t.end = fix(t.end);
            t.points = simplify(std::mem::take(&mut t.points));
            if t.points.len() < 2 {
                t.points = vec![at, at];
            }
            t.physical = polyline_length(&t.points);
            t.length = t.physical;
        }
    }
    for (i, n) in nodes.iter_mut().enumerate() {
        n.id = format!("N{}", i + 1);
    }
    for (i, s) in segments.iter_mut().enumerate() {
        s.id = format!("S{}", i + 1);
    }
    (nodes, segments, routes)
}

/// Drops points on a straight line between their neighbours.
fn simplify(points: Vec<V3>) -> Vec<V3> {
    let mut out: Vec<V3> = Vec::with_capacity(points.len());
    for q in points {
        if out.last().is_some_and(|l| dist(*l, q) < 1e-3) {
            continue;
        }
        if out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            let (u, v) = (sub(b, a), sub(q, b));
            let c = openecraft_parts::vec3::cross(u, v);
            if c.iter().all(|x| x.abs() < 1e-3 * (1.0 + dist(a, q))) && openecraft_parts::vec3::dot(u, v) > 0.0 {
                out.pop();
            }
        }
        out.push(q);
    }
    out
}

/// A problem of a stored topology or of the parts on its segments.
#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    /// Two nodes, segments, protections or fixings have the same id.
    DuplicateId(String),
    /// A segment ends at a node that does not exist: (segment, node).
    UnknownNode(String, String),
    /// A protection or fixing lies on a segment that does not exist: (id, segment).
    UnknownSegment(String, String),
    /// A location along a segment is outside 0 … 1: (id).
    OutOfRange(String),
    /// A connector of the harness is at no node.
    ConnectorWithoutNode(String),
    /// A wire cannot be routed through the segments: (wire number, from, to).
    NotRouted(String, String, String),
    /// The bend radius of a node is below the smallest bend radius of its bundle:
    /// (node, radius, smallest), mm.
    BendRadius(String, f32, f32),
}

/// Problems of the stored topology of a harness and of its protections and fixings.
pub fn problems(prj: &Project, an: &Analysis, h: &Harness) -> Vec<Problem> {
    let mut v = Vec::new();
    let ws = super::wires(prj, an, &h.tag);
    let t = build(prj, an, h, &ws);
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for id in h.nodes.iter().map(|n| &n.id).chain(h.segments.iter().map(|s| &s.id)).chain(h.protections.iter().map(|p| &p.id)).chain(h.fixings.iter().map(|f| &f.id)) {
        if !id.is_empty() && !ids.insert(id) {
            v.push(Problem::DuplicateId(id.clone()));
        }
    }
    for s in &h.segments {
        for n in [&s.start, &s.end] {
            if !h.nodes.iter().any(|x| x.id == *n) {
                v.push(Problem::UnknownNode(s.id.clone(), n.clone()));
            }
        }
    }
    let on_segment = h.protections.iter().map(|p| (&p.id, &p.segment, vec![p.start, p.end])).chain(h.fixings.iter().map(|f| (&f.id, &f.segment, vec![f.location])));
    for (id, seg, at) in on_segment {
        if !seg.is_empty() && t.segment_index(seg).is_none() {
            v.push(Problem::UnknownSegment(id.clone(), seg.clone()));
        }
        if at.iter().any(|x| !(0.0..=1.0).contains(x)) {
            v.push(Problem::OutOfRange(id.clone()));
        }
    }
    if t.source == Source::Stored {
        let mut devices: Vec<&String> = ws.iter().flat_map(|w| [&w.from.0, &w.to.0]).collect();
        devices.sort_by(|a, b| natural_cmp(a, b));
        devices.dedup();
        for d in devices.into_iter().filter(|d| t.node_of(d).is_none()) {
            v.push(Problem::ConnectorWithoutNode(d.clone()));
        }
        for (w, r) in ws.iter().zip(&t.routes) {
            if r.is_none() && t.node_of(&w.from.0).is_some() && t.node_of(&w.to.0).is_some() {
                v.push(Problem::NotRouted(w.number.clone(), super::joined(&w.from), super::joined(&w.to)));
            }
        }
    }
    let factor = prj.layout.routing.bend_wire;
    for (i, n) in t.nodes.iter().enumerate().filter(|(_, n)| n.bend_radius > 0.0) {
        let d = t.segments.iter().filter(|s| s.start == i || s.end == i).map(|s| s.diameter).fold(0.0, f32::max);
        if n.bend_radius + 1e-3 < factor * d {
            v.push(Problem::BendRadius(n.id.clone(), n.bend_radius, factor * d));
        }
    }
    v
}

#[cfg(test)]
mod tests;
