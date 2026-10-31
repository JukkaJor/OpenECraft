//! Cable bundles and their branches: stretches that several routed items share. The
//! routes are traced through a grid; where the same items run along consecutive
//! cells they form one bundle, and where items join or leave there is a branch. Ways
//! running together lie side by side, each on a lane of its own ([`spread_ways`]);
//! where they still run inside or through each other they are pushed apart
//! ([`separate`]).

use super::{Class, RoutedItem};
use crate::parts::vec3::{add, cross, dist, dot, fillet_each, norm, segment_in_box, sub, V3};
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

/// The inside of a cable duct the ways may run along: its box and the direction away
/// from the surface it stands on (its base).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Channel {
    pub inside: (V3, V3),
    pub floor: V3,
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
    /// Points per quarter turn of its bends where [`lay_out`] bends it round its corners.
    pub steps: usize,
}

/// The ways drawn apart from those they run together with, with the bend radius of each
/// corner. Straight stretches along the same axis that overlap and lie closer than
/// their thicknesses together (or to one that does) form a group with a lane for each,
/// the lanes side by side around the middle of the group as far apart as its thickest
/// way, so that no two ways run inside each other. Along a surface (the mounting plate)
/// the lanes lie in rows away from it as far as the ways may move towards it and away
/// (as deep as they lie in front of it), each row side by side along the surface;
/// elsewhere they form a square turned like the bundles round their corners (traced
/// through a grid of `cell`). In one of the `channels` (along it) they lie in it as
/// [`crate::panel::duct_lanes`] packs them. Elsewhere the lanes keep out of the
/// `obstacles` (boxes) a way did not run through before and in the `insides` (boxes,
/// the cabinets) it ran in: centred on the group, otherwise all to one side of it (or a
/// corner), otherwise (along a surface) in one stack away from it; where none of them
/// does, the group stays as it is. At its corners a way steps from lane to lane; its
/// ends stay where they are, the stretches at its ends run from their lanes to them.
pub fn spread_ways(ways: &[Way], cell: f32, obstacles: &[(V3, V3)], channels: &[Channel], insides: &[(V3, V3)]) -> Vec<(Vec<V3>, Vec<f32>)> {
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
    // stretches at the ends (from the connection points) too: they run from their lanes
    // to the end points.
    let mut runs: Vec<Run> = Vec::new();
    for (i, way) in ways.iter().enumerate() {
        for (j, s) in way.points.windows(2).enumerate() {
            let moving: Vec<usize> = (0..3).filter(|&k| (s[0][k] - s[1][k]).abs() > 1e-3).collect();
            if let [axis] = moving[..] {
                runs.push((i, j, axis, s[0][axis].min(s[1][axis]), s[0][axis].max(s[1][axis]), s[0]));
            }
        }
    }
    let groups = groups(ways, &runs);
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
        // In a duct along it: packed into it.
        let inside = |c: &&Channel| (0..3).all(|k| middle[k] >= c.inside.0[k] - 1.0 && middle[k] <= c.inside.1[k] + 1.0);
        let along = |c: &&Channel| (0..3).all(|k| c.inside.1[axis] - c.inside.0[axis] >= c.inside.1[k] - c.inside.0[k]);
        if let Some(c) = channels.iter().find(|c| inside(c) && along(c)) {
            let at: Vec<(V3, f32)> = members.iter().map(|&r| (runs[r].5, ways[runs[r].0].diameter)).collect();
            let frame = across.get(&line).copied().unwrap_or((unit((axis + 1) % 3), unit((axis + 2) % 3)));
            for (&r, to) in members.iter().zip(packed(c, axis, &at, frame)) {
                moves.insert((runs[r].0, runs[r].1), to);
            }
            continue;
        }
        let gap = members.iter().map(|&r| ways[runs[r].0].diameter).fold(0.0, f32::max) + 1.0;
        // The surface the ways run along, across their axis.
        let rest = members.iter().map(|&r| ways[runs[r].0].rest).find(|n| *n != [0.0; 3]).map(|mut n| {
            n[axis] = 0.0;
            n
        });
        let rest = rest.filter(|n| dot(*n, *n) > 0.25).map(norm);
        let (u, v, cols, rows) = match rest {
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
        // The moves of the members onto lanes of `cols` × `rows` around the middle,
        // shifted by `aside` lanes along `u` and `v`, away from the surface where they
        // come too near to it.
        let layout = |cols: usize, rows: usize, aside: (f32, f32)| -> Vec<V3> {
            let mut tos: Vec<V3> = members
                .iter()
                .enumerate()
                .map(|(k, &r)| {
                    let (c, row) = ((k % cols) as f32 - (cols - 1) as f32 / 2.0 + aside.0, (k / cols) as f32 - (rows - 1) as f32 / 2.0 + aside.1);
                    let mut to = sub(add(middle, add(scale(u, c * gap), scale(v, row * gap))), runs[r].5);
                    to[axis] = 0.0;
                    to
                })
                .collect();
            if let Some(n) = rest {
                let shift = members.iter().zip(&tos).map(|(&r, to)| -ways[runs[r].0].below - dot(*to, n)).fold(0.0, f32::max);
                for to in &mut tos {
                    *to = add(*to, scale(n, shift));
                }
            }
            tos
        };
        // Moved, no run enters an obstacle it did not run through before or comes further
        // out of an inside it ran in.
        let clear = |tos: &[V3]| {
            members.iter().zip(tos).all(|(&r, to)| {
                let (i, _, axis, from, till, at) = runs[r];
                let (mut a, mut b) = (at, at);
                (a[axis], b[axis]) = (from, till);
                let half = ways[i].diameter / 2.0;
                let kept = |o: V3, (lo, hi): (V3, V3)| {
                    let q = add(o, *to);
                    !within(o, (lo, hi)) || (0..3).all(|k| q[k] >= (lo[k] + half).min(o[k]) - 1e-3 && q[k] <= (hi[k] - half).max(o[k]) + 1e-3)
                };
                insides.iter().all(|inside| kept(a, *inside) && kept(b, *inside))
                    && obstacles.iter().all(|(lo, hi)| {
                        let bx = (lo.map(|x| x - half), hi.map(|x| x + half));
                        segment_in_box(add(a, *to), add(b, *to), bx).is_none() || segment_in_box(a, b, bx).is_some()
                    })
            })
        };
        let (su, sv) = ((cols - 1) as f32 / 2.0, (rows - 1) as f32 / 2.0);
        let mut tries = vec![(cols, rows, (0.0, 0.0))];
        for (a, b) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            let aside = (a * su, b * sv);
            if aside != (0.0, 0.0) && !tries.iter().any(|t| t.2 == aside) {
                tries.push((cols, rows, aside));
            }
        }
        if cols > 1 && rest.is_some() {
            tries.push((1, members.len(), (0.0, 0.0)));
        }
        if let Some(tos) = tries.into_iter().map(|(c, r, aside)| layout(c, r, aside)).find(|tos| clear(tos)) {
            for (&r, to) in members.iter().zip(tos) {
                moves.insert((runs[r].0, runs[r].1), to);
            }
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

/// A straight stretch of a way along an axis: way, stretch, axis, from, to, a point.
type Run = (usize, usize, usize, f32, f32, V3);

/// Groups of runs that lie inside each other (joined, through the ones in between).
fn groups(ways: &[Way], runs: &[Run]) -> BTreeMap<usize, Vec<usize>> {
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
    let side = |r: &Run| r.5[(r.2 + 1) % 3];
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
    groups
}

/// The moves of ways along `axis` (where they are, their diameter) into the lanes of
/// the channel `c` (across it): in their order across the directions `frame` turned with
/// the bundles round their corners, so that they do not cross where they turn into the
/// next channel.
fn packed(c: &Channel, axis: usize, ways: &[(V3, f32)], frame: (V3, V3)) -> Vec<V3> {
    let up = (0..3).find(|&k| k != axis && c.floor[k].abs() > 0.5).unwrap_or((axis + 1) % 3);
    let across = 3 - axis - up;
    let (lo, hi) = c.inside;
    let diameters: Vec<f32> = ways.iter().map(|w| w.1).collect();
    let side = if frame.0[across].abs() >= frame.1[across].abs() { frame.0[across] } else { frame.1[across] };
    let mut order: Vec<usize> = (0..ways.len()).collect();
    if side < 0.0 {
        order.reverse();
    }
    let (lanes, _) = crate::panel::duct_lanes_in_order(hi[across] - lo[across], hi[up] - lo[up], &diameters, &order);
    let (base, sign) = if c.floor[up] > 0.0 { (lo[up], 1.0) } else { (hi[up], -1.0) };
    ways.iter()
        .zip(lanes)
        .map(|(&(at, _), [x, y])| {
            let mut lane = at;
            lane[across] = (lo[across] + hi[across]) / 2.0 + x;
            lane[up] = base + sign * y;
            let mut to = sub(lane, at);
            to[axis] = 0.0;
            to
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

/// A wire, cable or tube drawn apart from the others by [`separate`]: its points (bent
/// round its corners), its outer diameter, the surface it runs along (as [`Way`]) and
/// how long the stretches at its ends are (with their bends): where its end point lies
/// as close to another one's end as their thicknesses (wires on one terminal, terminals
/// closer than the wires are thick) they converge, and the stretch stays as it is.
pub struct Strand<'a> {
    pub points: &'a [V3],
    pub diameter: f32,
    pub rest: V3,
    pub below: f32,
    pub tails: [f32; 2],
}

/// Overlap (mm) of two strands [`separate`] leaves (lanes touch).
pub const STRAND_GAP: f32 = 0.05;

/// Rounds of [`separate`] at most.
pub const ROUNDS: usize = 60;

/// Overlap (mm) [`separate`] leaves more with every round.
const SLACK_GROWTH: f32 = 0.01;

/// Parts the segments of a round of [`separate`] are searched in (on the threads there are).
const PUSH_PARTS: usize = 16;

/// How far a strand is pushed from where it was routed at most ([`separate`]): this many
/// times its thickness and [`MOST_MOVE_EXTRA`] mm more.
const MOST_MOVE: f32 = 4.0;
const MOST_MOVE_EXTRA: f32 = 20.0;

/// The ways as the 3D view draws them: laid side by side ([`spread_ways`] on a grid of
/// `cell`), bent round their corners ([`Way::steps`] points per quarter turn) and pushed
/// apart ([`separate`]); `progress` as for [`separate_with`].
pub fn lay_out(ways: &[Way], cell: f32, obstacles: &[(V3, V3)], channels: &[Channel], insides: &[(V3, V3)], progress: &dyn Fn(usize, usize) -> bool) -> Option<Vec<Vec<V3>>> {
    let laid = spread_ways(ways, cell, obstacles, channels, insides);
    let bent: Vec<Vec<V3>> = ways.iter().zip(&laid).map(|(w, (points, radii))| fillet_each(points, radii, w.steps)).collect();
    let tails = |points: &[V3], radii: &[f32]| -> [f32; 2] {
        let n = points.len();
        let r = |i: usize| radii.get(i).copied().unwrap_or(0.0);
        if n < 2 {
            return [0.0; 2];
        }
        [dist(points[0], points[1]) + r(1), dist(points[n - 2], points[n - 1]) + r(n - 2)]
    };
    let strands: Vec<Strand> = ways.iter().zip(&laid).zip(&bent).map(|((w, (points, radii)), bent)| Strand { points: bent, diameter: w.diameter, rest: w.rest, below: w.below, tails: tails(points, radii) }).collect();
    separate_with(&strands, obstacles, channels, insides, progress)
}

/// The strands pushed apart where they still run inside or through each other: sampled
/// along their length, each piece moved off the others by its share of their overlap
/// (one over the other where they cross) and the move spread along the strand, round
/// after round until none overlap (or [`ROUNDS`]). Their ends (twice their thickness
/// from the connection points, the stretches at the ends where they converge with
/// others: [`Strand::tails`]) and what lies inside one of the `obstacles` stay; none
/// moves further than [`MOST_MOVE`] times its thickness from where it was, no sample enters an obstacle it kept out of, comes further out of a channel's inside or
/// one of the `insides` (the cabinets) it was in, or nearer to the surface it runs along
/// than `below`. Straight where they did not move.
pub fn separate(strands: &[Strand], obstacles: &[(V3, V3)], channels: &[Channel], insides: &[(V3, V3)]) -> Vec<Vec<V3>> {
    separate_with(strands, obstacles, channels, insides, &|_, _| true).unwrap_or_default()
}

/// Like [`separate`], telling `progress` how many of at most [`ROUNDS`] rounds are done
/// before each; `None` when it answers false (stopped).
#[allow(clippy::too_many_lines)]
pub fn separate_with(strands: &[Strand], obstacles: &[(V3, V3)], channels: &[Channel], insides: &[(V3, V3)], progress: &dyn Fn(usize, usize) -> bool) -> Option<Vec<Vec<V3>>> {
    let radius: Vec<f32> = strands.iter().map(|s| s.diameter.max(0.0) / 2.0).collect();
    let Samples { mut at, owner, mut fixed, arc, spans } = samples(strands, obstacles);
    let was = at.clone();
    // The boxes a sample stays in (where it was in one) and those it keeps out of, by the
    // cells of a grid they reach into (the obstacles grown by the thickest strand).
    let walls: Vec<(V3, V3)> = channels.iter().map(|c| c.inside).chain(insides.iter().copied()).collect();
    let thickest = radius.iter().fold(0.0f32, |m, r| m.max(*r));
    let grown: Vec<(V3, V3)> = obstacles.iter().map(|(lo, hi)| (lo.map(|v| v - thickest), hi.map(|v| v + thickest))).collect();
    let (wall_cells, obstacle_cells) = (BoxGrid::new(&walls), BoxGrid::new(&grown));
    let keep = |i: usize, mut p: V3| -> V3 {
        let (o, r, st) = (was[i], radius[owner[i]], &strands[owner[i]]);
        // Never further than a few times its thickness from where it was routed.
        let (moved, most) = (sub(p, o), MOST_MOVE * st.diameter + MOST_MOVE_EXTRA);
        let far = dot(moved, moved).sqrt();
        if far > most {
            p = add(o, scale(moved, most / far));
        }
        if st.rest != [0.0; 3] {
            let (h, least) = (dot(sub(p, o), st.rest), -st.below.max(0.0));
            if h < least {
                p = add(p, scale(st.rest, least - h));
            }
        }
        for b in wall_cells.at(o).iter().map(|&w| walls[w as usize]).filter(|b| within(o, *b)) {
            for k in 0..3 {
                p[k] = p[k].clamp((b.0[k] + r).min(o[k]), (b.1[k] - r).max(o[k]));
            }
        }
        for &(lo, hi) in obstacle_cells.at(p).iter().map(|&b| &obstacles[b as usize]) {
            let (lo, hi) = (lo.map(|v| v - r), hi.map(|v| v + r));
            if within(p, (lo, hi)) && !within(o, (lo, hi)) {
                // Out the nearest side.
                let (k, low) = (0..3).flat_map(|k| [(k, true), (k, false)]).min_by(|a, b| {
                    let depth = |(k, low): (usize, bool)| if low { p[k] - lo[k] } else { hi[k] - p[k] };
                    depth(*a).total_cmp(&depth(*b))
                }).unwrap_or((0, true));
                p[k] = if low { lo[k] - 0.01 } else { hi[k] + 0.01 };
            }
        }
        p
    };
    // Segments (by their first sample) in every cell of a grid they come within their
    // thickness of: the thin ones in small cells, the others in cells as large as the
    // thickest.
    let thin = 4.0f32;
    let coarse = (2.0 * thickest).max(2.0 * thin);
    let level = |r: f32| usize::from(r > thin);
    let size = [2.0 * thin, coarse];
    let reach = |l: usize, a: V3, b: V3, r: f32| -> ([i32; 3], [i32; 3]) {
        let key = |v: f32| (v / size[l]).floor() as i32;
        ([0, 1, 2].map(|k| key(a[k].min(b[k]) - r)), [0, 1, 2].map(|k| key(a[k].max(b[k]) + r)))
    };
    let mut grids: [HashMap<[i32; 3], Vec<u32>>; 2] = [HashMap::new(), HashMap::new()];
    let segment = |i: usize| i + 1 < owner.len() && owner[i] == owner[i + 1];
    let mut placed: Vec<([i32; 3], [i32; 3])> = vec![([0; 3], [-1; 3]); at.len()];
    for i in (0..at.len()).filter(|&i| segment(i)) {
        let r = radius[owner[i]];
        placed[i] = reach(level(r), at[i], at[i + 1], r);
        for c in cells(placed[i]) {
            grids[level(r)].entry(c).or_default().push(i as u32);
        }
    }
    // Sums of the pushes and moves of each sample in a round (and which have one).
    let mut pushes: Vec<(V3, f32)> = vec![([0.0; 3], 0.0); at.len()];
    let mut spread: Vec<(V3, f32)> = vec![([0.0; 3], 0.0); at.len()];
    let (mut pushed, mut moved): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
    let add_to = |sums: &mut [(V3, f32)], touched: &mut Vec<usize>, k: usize, v: V3, w: f32| {
        if sums[k].1 == 0.0 {
            touched.push(k);
        }
        sums[k] = (add(sums[k].0, scale(v, w)), sums[k].1 + w);
    };
    // How often a sample was kept from moving: twice, and it stays (the others give way).
    let mut stuck = vec![0u8; at.len()];
    let mut active: Vec<usize> = (0..at.len()).filter(|&i| !fixed[i]).collect();
    for round in 0..ROUNDS {
        if !progress(round, ROUNDS) {
            return None;
        }
        // Each segment off the segments of the others it overlaps: by half the overlap
        // (all of it off one that stays), its ends as far as they are near.
        let mut segments: Vec<usize> = active.iter().flat_map(|&i| [i.wrapping_sub(1), i]).filter(|&i| i != usize::MAX && segment(i)).collect();
        segments.dedup();
        // The overlap left grows a little with every round, so that they come to rest.
        let slack = STRAND_GAP + SLACK_GROWTH * round as f32;
        // Searched in parts on the threads there are, added up in order.
        let parts: Vec<&[usize]> = segments.chunks(segments.len().div_ceil(PUSH_PARTS).max(64)).collect();
        let found = crate::parallel::par_map(&parts, |part| {
            let mut out: Vec<(usize, V3, f32)> = Vec::new();
            let mut seen: Vec<usize> = Vec::new();
            for &i in *part {
                let (a, b, s) = (at[i], at[i + 1], owner[i]);
                seen.clear();
                for (l, grid) in grids.iter().enumerate() {
                    for c in cells(reach(l, a, b, radius[s])) {
                        for &j in grid.get(&c).map(Vec::as_slice).unwrap_or(&[]) {
                            let j = j as usize;
                            if owner[j] == s || seen.contains(&j) {
                                continue;
                            }
                            seen.push(j);
                            let need = radius[s] + radius[owner[j]];
                            let (own, other) = (sub(b, a), sub(at[j + 1], at[j]));
                            let (u, t) = closest(a, own, at[j], other);
                            let (p, q) = (add(a, scale(own, u)), add(at[j], scale(other, t)));
                            let d = dist(p, q);
                            if d >= need - slack {
                                continue;
                            }
                            // Across each other: one over the other (along both), otherwise
                            // apart.
                            let over = cross(own, other);
                            let crossing = dot(over, over) > 0.2 * dot(own, own) * dot(other, other);
                            let (away, gap) = if crossing {
                                let n = norm(over);
                                let h = dot(sub(p, q), n);
                                let up = if h.abs() > 1e-3 { h > 0.0 } else { (dot(n, strands[s].rest) >= 0.0) == (s < owner[j]) };
                                (scale(n, if up { 1.0 } else { -1.0 }), h.abs())
                            } else if d > 1e-4 {
                                (scale(sub(p, q), 1.0 / d), d)
                            } else {
                                (aside(other, strands[s].rest, s < owner[j]), 0.0)
                            };
                            let share = if fixed[j] && fixed[j + 1] { 1.0 } else { 0.5 };
                            let push = scale(away, (need - gap).max(0.0) * share);
                            out.extend([(i, push, 1.0 - u), (i + 1, push, u)]);
                        }
                    }
                }
            }
            out
        });
        for (k, push, w) in found.into_iter().flatten() {
            add_to(&mut pushes, &mut pushed, k, push, w);
        }
        pushed.sort_unstable();
        pushed.dedup();
        let moves: Vec<(usize, V3)> = pushed.iter().map(|&k| (k, pushes[k])).filter(|(k, (_, w))| !fixed[*k] && *w > 1e-3).map(|(k, (sum, w))| (k, scale(sum, 1.0 / w.max(0.5)))).collect();
        for k in pushed.drain(..) {
            pushes[k] = ([0.0; 3], 0.0);
        }
        if moves.is_empty() {
            break;
        }
        // Each move spread along its strand, fading out over three times its thickness
        // (so that it bends away); where moves meet, their mean.
        for (i, m) in moves {
            let (span, reach) = (spans[owner[i]].clone(), 3.0 * strands[owner[i]].diameter + 10.0);
            let near = |k: &usize| (arc[*k] - arc[i]).abs() < reach;
            for k in (span.start..i).rev().take_while(near).chain((i..span.end).take_while(near)).filter(|&k| !fixed[k]) {
                add_to(&mut spread, &mut moved, k, m, 1.0 - (arc[k] - arc[i]).abs() / reach);
            }
        }
        moved.sort_unstable();
        moved.dedup();
        let mut changed: Vec<usize> = Vec::new();
        for k in moved.drain(..) {
            let (sum, w) = std::mem::replace(&mut spread[k], ([0.0; 3], 0.0));
            let want = scale(sum, 1.0 / w.max(1.0));
            let to = keep(k, add(at[k], want));
            if dist(to, at[k]) < 0.5 * dot(want, want).sqrt() {
                stuck[k] += 1;
                fixed[k] |= stuck[k] >= 2;
            }
            if dist(to, at[k]) > 0.01 {
                at[k] = to;
                changed.push(k);
            }
        }
        // The grid follows the segments that moved; their samples and those next to
        // them are checked again.
        let mut next: Vec<usize> = Vec::new();
        for &i in &changed {
            for j in [i.wrapping_sub(1), i] {
                if j == usize::MAX || !segment(j) {
                    continue;
                }
                let r = radius[owner[j]];
                let l = level(r);
                let now = reach(l, at[j], at[j + 1], r);
                if now != placed[j] {
                    for c in cells(placed[j]) {
                        if let Some(list) = grids[l].get_mut(&c) {
                            list.retain(|&x| x as usize != j);
                        }
                    }
                    for c in cells(now) {
                        grids[l].entry(c).or_default().push(j as u32);
                    }
                    placed[j] = now;
                }
            }
            next.extend([i.wrapping_sub(1), i, i + 1].into_iter().filter(|&m| m < at.len() && owner[m] == owner[i] && !fixed[m]));
        }
        next.sort_unstable();
        next.dedup();
        active = next;
    }
    Some(spans.into_iter().map(|span| straighten(&at[span], 0.05)).collect())
}

/// Boxes by the cells of a grid they reach into, to find those around a point.
struct BoxGrid {
    cells: HashMap<[i32; 3], Vec<u32>>,
}

/// Edge (mm) of the cells of a [`BoxGrid`].
const BOX_CELL: f32 = 100.0;

impl BoxGrid {
    fn new(boxes: &[(V3, V3)]) -> BoxGrid {
        let mut cells: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
        for (i, (lo, hi)) in boxes.iter().enumerate() {
            for c in cells_of(*lo, *hi) {
                cells.entry(c).or_default().push(i as u32);
            }
        }
        BoxGrid { cells }
    }

    /// The boxes reaching into the cell of `p` (all that may contain it).
    fn at(&self, p: V3) -> &[u32] {
        self.cells.get(&p.map(|v| (v / BOX_CELL).floor() as i32)).map_or(&[], Vec::as_slice)
    }
}

/// The cells of a [`BoxGrid`] a box reaches into.
fn cells_of(lo: V3, hi: V3) -> impl Iterator<Item = [i32; 3]> {
    let key = |v: f32| (v / BOX_CELL).floor() as i32;
    cells((lo.map(key), hi.map(key)))
}

/// The points of the strands for [`separate`], every three quarters of their thickness
/// (at least 3 mm): where they are, their strand, whether they stay (near the ends, at
/// an end converging with others, or inside an obstacle), how far along their strand,
/// the samples of each strand.
struct Samples {
    at: Vec<V3>,
    owner: Vec<usize>,
    fixed: Vec<bool>,
    arc: Vec<f32>,
    spans: Vec<std::ops::Range<usize>>,
}

fn samples(strands: &[Strand], obstacles: &[(V3, V3)]) -> Samples {
    let mut out = Samples { at: Vec::new(), owner: Vec::new(), fixed: Vec::new(), arc: Vec::new(), spans: Vec::new() };
    // The ends that lie as close to an end of another strand as their thicknesses.
    let ends: Vec<(usize, V3)> = strands.iter().enumerate().flat_map(|(s, st)| st.points.first().into_iter().chain(st.points.last()).map(move |q| (s, *q))).collect();
    let crowded = |s: usize, q: V3| ends.iter().any(|&(t, e)| t != s && dist(q, e) < (strands[s].diameter + strands[t].diameter) / 2.0);
    let grid = BoxGrid::new(obstacles);
    for (s, st) in strands.iter().enumerate() {
        let start = out.at.len();
        let step = (st.diameter * 0.75).max(3.0);
        let ends = 2.0 * st.diameter + 10.0;
        let total: f32 = st.points.windows(2).map(|w| dist(w[0], w[1])).sum();
        let held = [st.points.first(), st.points.last()].map(|q| q.is_some_and(|q| crowded(s, *q)));
        let hold = [0, 1].map(|e| if held[e] { st.tails[e].max(ends) } else { ends });
        let mut run = 0.0;
        for (k, &q) in st.points.iter().enumerate() {
            let a = if k == 0 { q } else { st.points[k - 1] };
            let len = dist(a, q);
            let n = if k == 0 { 1 } else { (len / step).ceil().max(1.0) as usize };
            for i in 1..=n {
                let t = i as f32 / n as f32;
                let p = add(a, scale(sub(q, a), t));
                let along = run + len * t;
                let inside = grid.at(p).iter().any(|&b| within(p, obstacles[b as usize]));
                out.at.push(p);
                out.owner.push(s);
                out.arc.push(along);
                out.fixed.push(along < hold[0] || total - along < hold[1] || inside);
            }
            run += len;
        }
        out.spans.push(start..out.at.len());
    }
    out
}

/// Where the segments from `a` along `da` and from `b` along `db` come nearest: the
/// share of each.
fn closest(a: V3, da: V3, b: V3, db: V3) -> (f32, f32) {
    let r = sub(a, b);
    let (la, lb, f) = (dot(da, da), dot(db, db), dot(db, r));
    if la <= 1e-9 || lb <= 1e-9 {
        return match (la <= 1e-9, lb <= 1e-9) {
            (true, true) => (0.0, 0.0),
            (true, false) => (0.0, (f / lb).clamp(0.0, 1.0)),
            _ => ((-dot(da, r) / la).clamp(0.0, 1.0), 0.0),
        };
    }
    let (c, k) = (dot(da, r), dot(da, db));
    let denom = la * lb - k * k;
    let s = if denom > 1e-9 { ((k * f - c * lb) / denom).clamp(0.0, 1.0) } else { 0.0 };
    let t = (k * s + f) / lb;
    if t < 0.0 {
        ((-c / la).clamp(0.0, 1.0), 0.0)
    } else if t > 1.0 {
        (((k - c) / la).clamp(0.0, 1.0), 1.0)
    } else {
        (s, t)
    }
}

/// The cells of a grid from `lo` to `hi`.
fn cells((lo, hi): ([i32; 3], [i32; 3])) -> impl Iterator<Item = [i32; 3]> {
    (lo[0]..=hi[0]).flat_map(move |x| (lo[1]..=hi[1]).flat_map(move |y| (lo[2]..=hi[2]).map(move |z| [x, y, z])))
}

/// Inside the box `b`.
fn within(p: V3, b: (V3, V3)) -> bool {
    (0..3).all(|k| p[k] >= b.0[k] && p[k] <= b.1[k])
}

/// Across the segment `along` lying on another one: along the surface the strand runs
/// on (or across the axis it runs least along), to one side or the other.
fn aside(along: V3, rest: V3, first: bool) -> V3 {
    let least = (0..3).min_by(|&a, &b| along[a].abs().total_cmp(&along[b].abs())).unwrap_or(0);
    let mut d = cross(along, if rest != [0.0; 3] { rest } else { unit(least) });
    if dot(d, d) < 1e-8 {
        d = cross(along, unit((least + 1) % 3));
    }
    scale(norm(d), if first { 1.0 } else { -1.0 })
}

/// The points without those that lie within `tolerance` of the line through their
/// neighbours kept (Douglas–Peucker).
fn straighten(points: &[V3], tolerance: f32) -> Vec<V3> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    let (first, last) = (0, points.len() - 1);
    keep[first] = true;
    keep[last] = true;
    let mut stack = vec![(first, last)];
    while let Some((a, b)) = stack.pop() {
        let ab = sub(points[b], points[a]);
        let l2 = dot(ab, ab);
        let off = |q: V3| {
            let t = if l2 > 0.0 { (dot(sub(q, points[a]), ab) / l2).clamp(0.0, 1.0) } else { 0.0 };
            dist(q, add(points[a], scale(ab, t)))
        };
        if let Some((m, d)) = (a + 1..b).map(|m| (m, off(points[m]))).max_by(|x, y| x.1.total_cmp(&y.1)) {
            if d > tolerance {
                keep[m] = true;
                stack.extend([(a, m), (m, b)]);
            }
        }
    }
    points.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

#[cfg(test)]
mod tests;
