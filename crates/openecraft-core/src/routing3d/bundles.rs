//! Cable bundles and their branches: stretches that several routed items share. The
//! routes are traced through a grid; where the same items run along consecutive
//! cells they form one bundle, and where items join or leave there is a branch. In a
//! bundle the items lie side by side, each on a lane of its own ([`spread`]).

use super::{Class, RoutedItem};
use crate::parts::vec3::{add, cross, dot, norm, sub, V3};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Stretch shared by two or more routed items.
#[derive(Clone, Debug, PartialEq)]
pub struct Bundle {
    /// Indices into the routed items.
    pub items: Vec<usize>,
    pub points: Vec<V3>,
    pub length: f32,
    /// Outer diameter of the bundle, mm.
    pub diameter: f32,
    pub classes: Vec<Class>,
}

/// Where items join or leave bundles.
#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    pub at: V3,
    /// Bundles that meet there (indices).
    pub bundles: Vec<usize>,
    /// Items passing through the branch point.
    pub items: usize,
}

type Key = [i32; 3];

/// Diameter of a bundle of items with the given outer diameters (packed round, 15 %
/// gaps).
pub fn diameter(d: impl Iterator<Item = f32>) -> f32 {
    d.map(|x| x * x).sum::<f32>().sqrt() * 1.15
}

type Edge = (Key, Key);

fn key_of(q: V3, cell: f32) -> Key {
    [0, 1, 2].map(|i| (q[i] / cell).round() as i32)
}

/// The cells a stretch runs through, in order.
fn keys_along(points: &[V3], cell: f32) -> Vec<Key> {
    let mut keys: Vec<Key> = Vec::new();
    for s in points.windows(2) {
        let len = ((s[1][0] - s[0][0]).powi(2) + (s[1][1] - s[0][1]).powi(2) + (s[1][2] - s[0][2]).powi(2)).sqrt();
        let steps = (len / (cell * 0.5)).ceil().max(1.0) as usize;
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            let q = key_of([0, 1, 2].map(|j| s[0][j] + (s[1][j] - s[0][j]) * t), cell);
            if keys.last() != Some(&q) {
                keys.push(q);
            }
        }
    }
    keys
}

/// The edges between neighbouring cells along the keys (only neighbours along one
/// axis: no jumps).
fn edges_along(keys: &[Key]) -> impl Iterator<Item = Edge> + '_ {
    keys.windows(2).filter(|w| (0..3).map(|j| (w[0][j] - w[1][j]).abs()).sum::<i32>() == 1).map(|w| (w[0].min(w[1]), w[0].max(w[1])))
}

/// The edges between consecutive cells with the items along them, and the chains of
/// edges two or more items share (the nodes, the items).
#[allow(clippy::type_complexity)]
fn chains(items: &[RoutedItem], cell: f32) -> (BTreeMap<Edge, BTreeSet<usize>>, Vec<(Vec<Key>, Vec<usize>)>) {
    let mut edges: BTreeMap<Edge, BTreeSet<usize>> = BTreeMap::new();
    for (i, it) in items.iter().enumerate().filter(|(_, it)| it.routed()) {
        for (_, points) in it.stretches() {
            for e in edges_along(&keys_along(points, cell)) {
                edges.entry(e).or_default().insert(i);
            }
        }
    }
    let shared: BTreeMap<Edge, &BTreeSet<usize>> = edges.iter().filter(|(_, s)| s.len() >= 2).map(|(k, s)| (*k, s)).collect();
    let mut at: BTreeMap<Key, Vec<Edge>> = BTreeMap::new();
    for e in edges.keys() {
        at.entry(e.0).or_default().push(*e);
        at.entry(e.1).or_default().push(*e);
    }
    // A node in the middle of a bundle has two edges with the same items.
    let through = |n: Key| -> bool {
        let list = &at[&n];
        list.len() == 2 && edges[&list[0]] == edges[&list[1]]
    };
    let mut out = Vec::new();
    let mut seen: BTreeSet<Edge> = BTreeSet::new();
    for (&e, set) in &shared {
        if seen.contains(&e) {
            continue;
        }
        seen.insert(e);
        let mut chain: Vec<Key> = vec![e.0, e.1];
        // Extend at both ends while the next node continues the same bundle.
        for front in [false, true] {
            loop {
                let (end, before) = if front { (chain[0], chain[1]) } else { (chain[chain.len() - 1], chain[chain.len() - 2]) };
                if !through(end) {
                    break;
                }
                let Some(next) = at[&end].iter().find(|x| **x != (before.min(end), before.max(end))).copied() else { break };
                if seen.contains(&next) {
                    break;
                }
                seen.insert(next);
                let other = if next.0 == end { next.1 } else { next.0 };
                if front {
                    chain.insert(0, other);
                } else {
                    chain.push(other);
                }
            }
        }
        out.push((chain, set.iter().copied().collect()));
    }
    (edges, out)
}

/// Bundles and branches of the routed items (`cell`: grid edge length).
pub fn find(items: &[RoutedItem], cell: f32) -> (Vec<Bundle>, Vec<Branch>) {
    let centre = |k: Key| -> V3 { [0, 1, 2].map(|i| k[i] as f32 * cell) };
    let (edges, chains) = chains(items, cell);
    let shared: BTreeMap<Edge, &BTreeSet<usize>> = edges.iter().filter(|(_, s)| s.len() >= 2).map(|(k, s)| (*k, s)).collect();
    let mut at: BTreeMap<Key, Vec<Edge>> = BTreeMap::new();
    for e in edges.keys() {
        at.entry(e.0).or_default().push(*e);
        at.entry(e.1).or_default().push(*e);
    }
    let through = |n: Key| -> bool {
        let list = &at[&n];
        list.len() == 2 && edges[&list[0]] == edges[&list[1]]
    };
    let mut bundles: Vec<Bundle> = Vec::new();
    for (chain, members) in chains {
        let points = super::grid::simplify(chain.iter().map(|k| centre(*k)).collect());
        let mut classes: Vec<Class> = members.iter().map(|&i| items[i].class).collect();
        classes.sort();
        classes.dedup();
        bundles.push(Bundle {
            length: (chain.len() - 1) as f32 * cell,
            diameter: diameter(members.iter().map(|&i| items[i].diameter)),
            items: members,
            points,
            classes,
        });
    }
    // Branches: nodes where bundles end and items go different ways.
    let mut branches = Vec::new();
    for (&n, list) in &at {
        if through(n) || list.len() < 2 || !list.iter().any(|e| shared.contains_key(e)) {
            continue;
        }
        let c = centre(n);
        let touching: Vec<usize> = (0..bundles.len()).filter(|&b| bundles[b].points.first().is_some_and(|q| same(*q, c)) || bundles[b].points.last().is_some_and(|q| same(*q, c))).collect();
        let passing: BTreeSet<usize> = list.iter().flat_map(|e| edges[e].iter().copied()).collect();
        if !touching.is_empty() && passing.len() >= 2 {
            branches.push(Branch { at: c, bundles: touching, items: passing.len() });
        }
    }
    (bundles, branches)
}

/// Each routed item's stretches ([`RoutedItem::stretches`]) drawn apart from the items
/// it shares them with, with the bend radius of each corner: in a bundle every item has
/// a lane of its own, the lanes side by side across the bundle as far apart as its
/// thickest item and turned with it round its corners. A straight stretch of a route
/// lies on its lane of the bundle it shares most of, with a short step where it joins
/// another; the ends stay where they are.
pub fn spread(items: &[RoutedItem], cell: f32) -> Vec<Vec<(Vec<V3>, Vec<f32>)>> {
    let (_, chains) = chains(items, cell);
    // The lane of each item along each edge of its bundles: the bundle and the offset.
    let mut lanes: HashMap<(Edge, usize), (usize, V3)> = HashMap::new();
    for (b, (chain, members)) in chains.iter().enumerate() {
        let gap = members.iter().map(|&i| items[i].diameter).fold(0.0, f32::max) + 1.0;
        let cols = (members.len() as f32).sqrt().ceil() as usize;
        let rows = members.len().div_ceil(cols);
        let dir = |j: usize| -> V3 { [0, 1, 2].map(|k| (chain[j + 1][k] - chain[j][k]) as f32) };
        let first = dir(0);
        let axis = (0..3).find(|&k| first[k] != 0.0).unwrap_or(0);
        let unit = |k: usize| -> V3 { [0, 1, 2].map(|i| if i == k { 1.0 } else { 0.0 }) };
        let (mut u, mut v) = (unit((axis + 1) % 3), unit((axis + 2) % 3));
        for j in 0..chain.len() - 1 {
            let d = dir(j);
            if j > 0 && d != dir(j - 1) {
                // Turned with the bundle: what pointed along it now points along the
                // new way, what pointed the new way now points back.
                let (before, w) = (dir(j - 1), cross(dir(j - 1), d));
                let turn = |x: V3| add(add(scale(d, dot(x, before)), scale(before, -dot(x, d))), scale(w, dot(x, w)));
                (u, v) = (turn(u), turn(v));
            }
            let e = (chain[j].min(chain[j + 1]), chain[j].max(chain[j + 1]));
            for (k, &i) in members.iter().enumerate() {
                let (c, r) = ((k % cols) as f32 - (cols - 1) as f32 / 2.0, (k / cols) as f32 - (rows - 1) as f32 / 2.0);
                lanes.insert((e, i), (b, add(scale(u, c * gap), scale(v, r * gap))));
            }
        }
    }
    items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let radii = it.stretch_radii();
            it.stretches()
                .into_iter()
                .enumerate()
                .map(|(s, (_, points))| {
                    let radii = radii.get(s).copied().unwrap_or_default();
                    // The lane of each straight stretch: of the bundle it shares most cells with.
                    let offsets: Vec<V3> = points
                        .windows(2)
                        .map(|seg| {
                            let mut count: BTreeMap<usize, (usize, V3)> = BTreeMap::new();
                            for e in edges_along(&keys_along(seg, cell)) {
                                if let Some(&(b, o)) = lanes.get(&(e, i)) {
                                    count.entry(b).or_insert((0, o)).0 += 1;
                                }
                            }
                            count.values().max_by_key(|(n, _)| *n).map_or([0.0; 3], |(_, o)| *o)
                        })
                        .collect();
                    lay(points, radii, &offsets)
                })
                .collect()
        })
        .collect()
}

fn scale(a: V3, f: f32) -> V3 {
    [a[0] * f, a[1] * f, a[2] * f]
}

/// The polyline with its straight stretches moved by `offsets` (one per stretch), the
/// ends kept: a corner where both moved stretches meet, otherwise both of their ends
/// with the step between them.
fn lay(points: &[V3], radii: &[f32], offsets: &[V3]) -> (Vec<V3>, Vec<f32>) {
    let n = points.len();
    if n < 3 || offsets.iter().all(|o| *o == [0.0; 3]) {
        return (points.to_vec(), radii.to_vec());
    }
    let (mut out, mut bends) = (vec![points[0]], vec![0.0]);
    for j in 1..n - 1 {
        let (o1, o2) = (offsets[j - 1], offsets[j]);
        let (d1, d2) = (norm(sub(points[j], points[j - 1])), norm(sub(points[j + 1], points[j])));
        let r = radii.get(j).copied().unwrap_or(0.0);
        let corner = add(points[j], add(o1, scale(d1, dot(sub(o2, o1), d1))));
        let off_line = cross(sub(corner, add(points[j], o2)), d2);
        if dot(off_line, off_line) < 0.25 {
            out.push(corner);
            bends.push(r);
        } else {
            out.extend([add(points[j], o1), add(points[j], o2)]);
            bends.extend([r, r]);
        }
    }
    out.push(points[n - 1]);
    bends.push(0.0);
    (out, bends)
}

fn same(a: V3, b: V3) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3)
}

#[cfg(test)]
mod tests;
