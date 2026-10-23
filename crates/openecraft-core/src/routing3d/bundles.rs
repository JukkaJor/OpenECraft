//! Cable bundles and their branches: stretches that several routed items share. The
//! routes are traced through a grid; where the same items run along consecutive
//! cells they form one bundle, and where items join or leave there is a branch.

use super::{Class, RoutedItem};
use crate::parts::vec3::V3;
use std::collections::{BTreeMap, BTreeSet};

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

/// Bundles and branches of the routed items (`cell`: grid edge length).
pub fn find(items: &[RoutedItem], cell: f32) -> (Vec<Bundle>, Vec<Branch>) {
    let key = |q: V3| -> Key { [0, 1, 2].map(|i| (q[i] / cell).round() as i32) };
    let centre = |k: Key| -> V3 { [0, 1, 2].map(|i| k[i] as f32 * cell) };
    // Edges between consecutive cells and the items along them.
    let mut edges: BTreeMap<(Key, Key), BTreeSet<usize>> = BTreeMap::new();
    for (i, it) in items.iter().enumerate().filter(|(_, it)| it.routed()) {
        let mut keys: Vec<Key> = Vec::new();
        for s in it.points.windows(2) {
            let len = ((s[1][0] - s[0][0]).powi(2) + (s[1][1] - s[0][1]).powi(2) + (s[1][2] - s[0][2]).powi(2)).sqrt();
            let steps = (len / (cell * 0.5)).ceil().max(1.0) as usize;
            for k in 0..=steps {
                let t = k as f32 / steps as f32;
                let q = key([0, 1, 2].map(|j| s[0][j] + (s[1][j] - s[0][j]) * t));
                if keys.last() != Some(&q) {
                    keys.push(q);
                }
            }
        }
        for w in keys.windows(2) {
            // Only neighbours along one axis are edges (no jumps).
            if (0..3).map(|j| (w[0][j] - w[1][j]).abs()).sum::<i32>() == 1 {
                edges.entry((w[0].min(w[1]), w[0].max(w[1]))).or_default().insert(i);
            }
        }
    }
    let shared: BTreeMap<(Key, Key), &BTreeSet<usize>> = edges.iter().filter(|(_, s)| s.len() >= 2).map(|(k, s)| (*k, s)).collect();
    let mut at: BTreeMap<Key, Vec<(Key, Key)>> = BTreeMap::new();
    for e in edges.keys() {
        at.entry(e.0).or_default().push(*e);
        at.entry(e.1).or_default().push(*e);
    }
    // A node in the middle of a bundle has two edges with the same items.
    let through = |n: Key| -> bool {
        let list = &at[&n];
        list.len() == 2 && edges[&list[0]] == edges[&list[1]]
    };
    let mut bundles: Vec<Bundle> = Vec::new();
    let mut seen: BTreeSet<(Key, Key)> = BTreeSet::new();
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
        let points = super::grid::simplify(chain.iter().map(|k| centre(*k)).collect());
        let members: Vec<usize> = set.iter().copied().collect();
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

fn same(a: V3, b: V3) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3)
}

#[cfg(test)]
mod tests;
