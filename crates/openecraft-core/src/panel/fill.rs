//! Fill levels of the cable ducts: the cross sections of the routed wires (with their
//! insulation) against the cross section of the duct, at the fullest stretch.

use super::{profile, ItemKind, RoutedWire};
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
        for r in w.route.runs.iter().filter(|r| r.to - r.from > 0.01) {
            runs.entry((w.enclosure, r.duct)).or_default().push(Stretch { from: r.from, to: r.to, area });
        }
    }
    let mut out = Vec::new();
    for (ei, e) in prj.layout.enclosures.iter().enumerate() {
        for d in e.items.iter().filter(|i| i.kind == ItemKind::Duct) {
            let Some(list) = runs.get(&(ei, d.id)) else { continue };
            let (w, depth) = profile(prj, ItemKind::Duct, &d.part);
            let capacity = w * depth;
            // Sweep along the duct; at the same position wires leave before others
            // enter, so stretches that only touch do not count as side by side.
            let mut events: Vec<(f32, bool, f32)> = Vec::new();
            for s in list {
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

#[cfg(test)]
mod tests;
