//! Fill levels of the cable ducts: the cross sections of the routed wires (with their
//! insulation) against the cross section of the duct, at the fullest stretch.

use super::{profile, ItemKind, Route, RoutedWire};
use crate::model::Project;
use std::collections::HashMap;

/// Fill level of one cable duct.
#[derive(Clone, Debug, PartialEq)]
pub struct DuctFill {
    pub enclosure: usize,
    pub duct: u64,
    /// Cross section of the duct (width × depth) in mm².
    pub capacity: f32,
    /// Highest fill level in percent.
    pub percent: f32,
    /// Where it is reached: the stretch along the duct's axis (x of horizontal, y of
    /// vertical ducts).
    pub from: f32,
    pub to: f32,
    /// Wires in that stretch.
    pub wires: usize,
    /// All wires that run through the duct somewhere.
    pub total_wires: usize,
}

/// A wire's stretch in a duct.
struct Stretch {
    from: f32,
    to: f32,
    area: f32,
}

/// The fullest stretch of a duct: (wire area, from, to, number of wires). At the same
/// position wires leave before others enter, so stretches that only touch do not
/// count as side by side.
fn fullest<'a>(stretches: impl Iterator<Item = &'a Stretch>) -> (f32, f32, f32, usize) {
    let mut events: Vec<(f32, bool, f32)> = Vec::new();
    // Stretches without length (nodes at the same place) are never side by side.
    for s in stretches.filter(|s| s.to - s.from > 1e-3) {
        events.push((s.from, true, s.area));
        events.push((s.to, false, s.area));
    }
    events.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let (mut area, mut count) = (0.0f32, 0usize);
    let mut best = (0.0f32, 0.0f32, 0.0f32, 0usize);
    for (i, &(pos, enter, a)) in events.iter().enumerate() {
        if enter {
            area += a;
            count += 1;
        } else {
            area -= a;
            count -= 1;
        }
        let next = events.get(i + 1).map(|e| e.0).unwrap_or(pos);
        if next > pos && area > best.0 + 1e-3 {
            best = (area, pos, next, count);
        } else if next > pos && (area - best.0).abs() <= 1e-3 && (best.2 - pos).abs() < 0.01 {
            best.2 = next;
        }
    }
    best
}

/// Cross section of a wire including its insulation (from its outer diameter).
pub fn wire_area(cross_section: &str) -> f32 {
    let d = crate::harness::outer_diameter(cross_section);
    std::f32::consts::PI / 4.0 * d * d
}

/// Fill levels of all ducts that wires run through, by enclosure and duct order.
pub fn duct_fill(prj: &Project, wiring: &[RoutedWire]) -> Vec<DuctFill> {
    // (enclosure, duct id) -> stretches of the wires
    let mut runs: HashMap<(usize, u64), Vec<Stretch>> = HashMap::new();
    for w in wiring {
        let area = wire_area(&w.cross_section);
        let door = [(w.enclosure, &w.door_routes.0), (w.last_enclosure(), &w.door_routes.1)].into_iter().filter_map(|(e, r)| r.as_ref().map(|r| (e, r)));
        for (ei, route) in w.stretches().chain(door) {
            for r in route.runs.iter().filter(|r| r.to - r.from > 0.01) {
                runs.entry((ei, r.duct)).or_default().push(Stretch { from: r.from, to: r.to, area });
            }
        }
    }
    let mut out = Vec::new();
    for (ei, e) in prj.layout.enclosures.iter().enumerate() {
        for d in e.items.iter().filter(|i| i.kind == ItemKind::Duct) {
            let Some(list) = runs.get(&(ei, d.id)) else { continue };
            let (w, depth) = profile(prj, ItemKind::Duct, &d.part);
            let capacity = w * depth;
            let best = fullest(list.iter());
            out.push(DuctFill {
                enclosure: ei,
                duct: d.id,
                capacity,
                percent: if capacity > 0.0 { best.0 / capacity * 100.0 } else { 0.0 },
                from: best.1,
                to: best.2,
                wires: best.3,
                total_wires: list.len(),
            });
        }
    }
    out
}

/// A stretch that would go above the fill limit counts this many times its length
/// when routing: a detour up to that factor is preferred to overfilling a duct.
pub const OVERFILL_COST: f32 = 10.0;

/// Wires routed so far, to keep further wires out of stretches that would be filled
/// above the limit.
pub struct DuctLoads {
    /// Cross section per duct id.
    capacity: HashMap<u64, f32>,
    runs: HashMap<u64, Vec<Stretch>>,
    limit: f32,
}

impl DuctLoads {
    pub fn new(prj: &Project) -> DuctLoads {
        let capacity = prj
            .layout
            .enclosures
            .iter()
            .flat_map(|e| e.items.iter().filter(|i| i.kind == ItemKind::Duct))
            .map(|d| {
                let (w, depth) = profile(prj, ItemKind::Duct, &d.part);
                (d.id, w * depth)
            })
            .collect();
        DuctLoads { capacity, runs: HashMap::new(), limit: prj.layout.duct_fill_limit }
    }

    /// Wire area in the fullest part of a stretch of a duct.
    pub fn load(&self, duct: u64, from: f32, to: f32) -> f32 {
        let Some(list) = self.runs.get(&duct) else { return 0.0 };
        let clipped: Vec<Stretch> = list
            .iter()
            .filter(|s| s.from < to && s.to > from)
            .map(|s| Stretch { from: s.from.max(from), to: s.to.min(to), area: s.area })
            .collect();
        fullest(clipped.iter()).0
    }

    /// Cost factor of running a wire of cross section `area` along a stretch:
    /// [`OVERFILL_COST`] if the stretch would be filled above the limit, else 1.
    pub fn cost(&self, duct: u64, from: f32, to: f32, area: f32) -> f32 {
        let capacity = self.capacity.get(&duct).copied().unwrap_or(0.0);
        if capacity > 0.0 && (self.load(duct, from, to) + area) / capacity * 100.0 > self.limit {
            OVERFILL_COST
        } else {
            1.0
        }
    }

    pub fn add(&mut self, route: &Route, area: f32) {
        for r in route.runs.iter().filter(|r| r.to - r.from > 0.01) {
            self.runs.entry(r.duct).or_default().push(Stretch { from: r.from, to: r.to, area });
        }
    }
}

#[cfg(test)]
mod tests;
