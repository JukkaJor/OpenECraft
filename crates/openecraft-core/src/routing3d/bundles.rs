//! Cable bundles and their branches: stretches that several routed items share. The
//! routes are traced through a grid; where the same items run along consecutive
//! cells they form one bundle, and where items join or leave there is a branch. Ways
//! running together lie side by side, each on a lane of its own ([`spread_ways`]).

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

/// The edges between consecutive cells with what runs along them (the owners of the
/// `stretches`), and the chains of edges two or more share (the nodes, the owners).
#[allow(clippy::type_complexity)]
fn chains(stretches: &[(usize, &[V3])], cell: f32) -> (BTreeMap<Edge, BTreeSet<usize>>, Vec<(Vec<Key>, Vec<usize>)>) {
    let mut edges: BTreeMap<Edge, BTreeSet<usize>> = BTreeMap::new();
    for &(owner, points) in stretches {
        for e in edges_along(&keys_along(points, cell)) {
            edges.entry(e).or_default().insert(owner);
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
    let stretches: Vec<(usize, &[V3])> = items.iter().enumerate().filter(|(_, it)| it.routed()).flat_map(|(i, it)| it.stretches().into_iter().map(move |(_, p)| (i, p))).collect();
    let (edges, chains) = chains(&stretches, cell);
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

/// A polyline laid out with others by [`spread_ways`]: its corners, their bend radii, its
/// outer diameter, and the surface it runs along (the mounting plate): the direction
/// away from it (none: free) and how far it may come nearer to it.
pub struct Way<'a> {
    pub points: &'a [V3],
    pub radii: &'a [f32],
    pub diameter: f32,
    pub rest: V3,
    pub below: f32,
}

/// The ways drawn apart from those they run together with, with the bend radius of each
/// corner. Straight stretches along the same axis that overlap and lie closer than
/// their thicknesses together (or to one that does) form a group with a lane for each,
/// the lanes side by side around the middle of the group as far apart as its thickest
/// way, so that no two ways run inside each other. Along a surface (the mounting plate)
/// the lanes lie in rows away from it as far as the ways may move towards it and away
/// (as deep as they lie in front of it), each row side by side along the surface;
/// elsewhere they form a square turned like the bundles round their corners (traced
/// through a grid of `cell`). At its corners a way steps from lane to lane, its ends
/// stay where they are, the stretches at its ends too.
pub fn spread_ways(ways: &[Way], cell: f32) -> Vec<(Vec<V3>, Vec<f32>)> {
    let stretches: Vec<(usize, &[V3])> = ways.iter().enumerate().map(|(i, w)| (i, w.points)).collect();
    let (_, chains) = chains(&stretches, cell);
    // Across each line of the grid the directions of the bundles on it, turned with
    // them round their corners.
    let line_of = |e: &Edge| -> Line {
        let axis = (0..3).find(|&k| e.0[k] != e.1[k]).unwrap_or(0);
        let mut at = e.0;
        at[axis] = 0;
        (axis, at)
    };
    let mut across: BTreeMap<Line, (V3, V3)> = BTreeMap::new();
    for (chain, _) in &chains {
        let dir = |j: usize| -> V3 { [0, 1, 2].map(|k| (chain[j + 1][k] - chain[j][k]) as f32) };
        let first = dir(0);
        let axis = (0..3).find(|&k| first[k] != 0.0).unwrap_or(0);
        let (mut u, mut v) = (unit((axis + 1) % 3), unit((axis + 2) % 3));
        for j in 0..chain.len() - 1 {
            let d = dir(j);
            if j > 0 && d != dir(j - 1) {
                // What pointed along the bundle now points along the new way, what
                // pointed the new way now points back.
                let (before, w) = (dir(j - 1), cross(dir(j - 1), d));
                let turn = |x: V3| add(add(scale(d, dot(x, before)), scale(before, -dot(x, d))), scale(w, dot(x, w)));
                (u, v) = (turn(u), turn(v));
            }
            across.entry(line_of(&(chain[j].min(chain[j + 1]), chain[j].max(chain[j + 1])))).or_insert((u, v));
        }
    }
    // The straight stretches along an axis: way, stretch, axis, from, to, the point. The
    // stretches at the ends (from the connection points) stay where they are.
    let mut runs: Vec<(usize, usize, usize, f32, f32, V3)> = Vec::new();
    for (i, way) in ways.iter().enumerate() {
        let last = way.points.len().saturating_sub(2);
        for (j, s) in way.points.windows(2).enumerate().filter(|(j, _)| *j > 0 && *j < last) {
            let moving: Vec<usize> = (0..3).filter(|&k| (s[0][k] - s[1][k]).abs() > 1e-3).collect();
            if let [axis] = moving[..] {
                runs.push((i, j, axis, s[0][axis].min(s[1][axis]), s[0][axis].max(s[1][axis]), s[0]));
            }
        }
    }
    // Groups of runs that lie inside each other (joined, through the ones in between).
    let mut root: Vec<usize> = (0..runs.len()).collect();
    fn find(root: &mut [usize], mut i: usize) -> usize {
        while root[i] != i {
            root[i] = root[root[i]];
            i = root[i];
        }
        i
    }
    let thickest = ways.iter().map(|w| w.diameter).fold(0.0, f32::max);
    let mut order: Vec<usize> = (0..runs.len()).collect();
    let side = |r: &(usize, usize, usize, f32, f32, V3)| r.5[(r.2 + 1) % 3];
    order.sort_by(|&a, &b| runs[a].2.cmp(&runs[b].2).then(side(&runs[a]).total_cmp(&side(&runs[b]))));
    for (n, &a) in order.iter().enumerate() {
        let ra = runs[a];
        for &b in &order[n + 1..] {
            let rb = runs[b];
            if rb.2 != ra.2 || side(&rb) - side(&ra) > thickest + 1.0 {
                break;
            }
            let reach = (ways[ra.0].diameter + ways[rb.0].diameter) / 2.0;
            let apart = (0..3).filter(|&k| k != ra.2).map(|k| (ra.5[k] - rb.5[k]).powi(2)).sum::<f32>().sqrt();
            if ra.0 != rb.0 && apart < reach && ra.4.min(rb.4) - ra.3.max(rb.3) > 1.0 {
                let (x, y) = (find(&mut root, a), find(&mut root, b));
                root[x] = y;
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for r in 0..runs.len() {
        let g = find(&mut root, r);
        groups.entry(g).or_default().push(r);
    }
    // The move of each run onto its lane, across its axis.
    let mut moves: HashMap<(usize, usize), V3> = HashMap::new();
    for members in groups.values().filter(|m| m.len() >= 2) {
        let axis = runs[members[0]].2;
        let middle = scale(members.iter().fold([0.0; 3], |m, &r| add(m, runs[r].5)), 1.0 / members.len() as f32);
        let line = (axis, {
            let mut k = key_of(middle, cell);
            k[axis] = 0;
            k
        });
        let gap = members.iter().map(|&r| ways[runs[r].0].diameter).fold(0.0, f32::max) + 1.0;
        // The surface the ways run along, across their axis.
        let rest = members.iter().map(|&r| ways[runs[r].0].rest).find(|n| *n != [0.0; 3]).map(|mut n| {
            n[axis] = 0.0;
            n
        });
        let (u, v, cols, rows) = match rest.filter(|n| dot(*n, *n) > 0.25).map(norm) {
            Some(n) => {
                let below = members.iter().map(|&r| ways[runs[r].0].below).fold(f32::INFINITY, f32::min).max(0.0);
                let rows = ((2.0 * below / gap).floor() as usize + 1).min(members.len());
                (norm(cross(unit(axis), n)), n, members.len().div_ceil(rows), rows)
            }
            None => {
                let (u, v) = across.get(&line).copied().unwrap_or((unit((axis + 1) % 3), unit((axis + 2) % 3)));
                let cols = (members.len() as f32).sqrt().ceil() as usize;
                (u, v, cols, members.len().div_ceil(cols))
            }
        };
        let mut tos: Vec<V3> = members
            .iter()
            .enumerate()
            .map(|(k, &r)| {
                let (c, row) = ((k % cols) as f32 - (cols - 1) as f32 / 2.0, (k / cols) as f32 - (rows - 1) as f32 / 2.0);
                let mut to = sub(add(middle, add(scale(u, c * gap), scale(v, row * gap))), runs[r].5);
                to[axis] = 0.0;
                to
            })
            .collect();
        // Away from the surface the ways run along as far as one of them would come too
        // near to it.
        let rest = members.iter().map(|&r| ways[runs[r].0].rest).find(|n| *n != [0.0; 3]).map(|mut n| {
            n[axis] = 0.0;
            n
        });
        if let Some(n) = rest.filter(|n| dot(*n, *n) > 0.25).map(norm) {
            let shift = members.iter().zip(&tos).map(|(&r, to)| -ways[runs[r].0].below - dot(*to, n)).fold(0.0, f32::max);
            for to in &mut tos {
                *to = add(*to, scale(n, shift));
            }
        }
        for (&r, to) in members.iter().zip(tos) {
            moves.insert((runs[r].0, runs[r].1), to);
        }
    }
    ways.iter()
        .enumerate()
        .map(|(i, way)| {
            let offsets: Vec<V3> = (0..way.points.len().saturating_sub(1)).map(|j| moves.get(&(i, j)).copied().unwrap_or([0.0; 3])).collect();
            lay(way.points, way.radii, &offsets)
        })
        .collect()
}

/// A line of the grid: its axis and the cell across it (the axis left at 0).
type Line = (usize, Key);

fn unit(k: usize) -> V3 {
    [0, 1, 2].map(|i| if i == k { 1.0 } else { 0.0 })
}

fn scale(a: V3, f: f32) -> V3 {
    [a[0] * f, a[1] * f, a[2] * f]
}

/// The polyline with its straight stretches moved by `offsets` (one per stretch), the
/// ends kept: a corner where both moved stretches meet, otherwise the corner on the first
/// one and a step across both onto the second (never back).
fn lay(points: &[V3], radii: &[f32], offsets: &[V3]) -> (Vec<V3>, Vec<f32>) {
    let n = points.len();
    if n < 3 || offsets.iter().all(|o| *o == [0.0; 3]) {
        return (points.to_vec(), radii.to_vec());
    }
    let mut out: Vec<(V3, f32)> = vec![(points[0], 0.0)];
    for j in 1..n - 1 {
        let (o1, o2) = (offsets[j - 1], offsets[j]);
        let (d1, d2) = (norm(sub(points[j], points[j - 1])), norm(sub(points[j + 1], points[j])));
        let r = radii.get(j).copied().unwrap_or(0.0);
        let corner = add(points[j], add(o1, scale(d1, dot(sub(o2, o1), d1))));
        let to_second = sub(add(points[j], o2), corner);
        let step = sub(to_second, scale(d2, dot(to_second, d2)));
        out.push((corner, r));
        if dot(step, step) >= 0.25 {
            out.push((add(corner, step), r));
        }
    }
    out.push((points[n - 1], 0.0));
    out.dedup_by(|a, b| dot(sub(a.0, b.0), sub(a.0, b.0)) < 0.0025);
    out.into_iter().unzip()
}

fn same(a: V3, b: V3) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3)
}

#[cfg(test)]
mod tests;
