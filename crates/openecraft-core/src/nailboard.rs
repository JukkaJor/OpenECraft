//! Nailboard (formboard) drawing of a harness to scale: the bundle as a star of
//! branches from a common node to the connectors, each branch as long as the wire
//! lengths require (least squares fit of the wire lengths between connectors), drawn
//! as thick as its bundle, with nails at the node and the branch ends, the branch
//! length, and at each connector its pin table (pin, wire, colour, cross-section,
//! other end). Branches too short to keep their tables apart are drawn longer with a
//! break mark (not to scale), as on paper formboards.

use crate::analysis::Analysis;
use crate::geom::{p, P};
use crate::harness::{self, HarnessWire};
use crate::model::{natural_cmp, Project};
use crate::render::{Draw, Ink, VAnchor};
use crate::symbols::Anchor;
use openecraft_i18n::{tr, tr_fmt};

pub use crate::harness::topology::{branch_lengths, DEFAULT_BRANCH};

/// Shortest branch on the sheet, mm; shorter ones are drawn with a break mark.
const MIN_DRAWN: f32 = 60.0;
/// Standard drawing scales (1 : n).
pub const SCALES: [f32; 9] = [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0];
/// Drawing size (A3 landscape), mm.
pub const SHEET: (f32, f32) = (420.0, 297.0);

/// A connector end of the harness on the board.
#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    pub device: String,
    /// From the node to the connector, mm.
    pub length: f32,
    /// No wire of the connector has a length: the length is assumed.
    pub assumed: bool,
    /// Direction in degrees (0 = right, counter-clockwise on the sheet).
    pub angle: f32,
    /// Diameter of the bundle in this branch, mm.
    pub diameter: f32,
    /// Pin, wire number, colour, cross-section, other end.
    pub pins: Vec<[String; 5]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Nailboard {
    pub tag: String,
    pub description: String,
    pub part: String,
    pub wires: usize,
    /// Diameter of the whole bundle at the node, mm.
    pub diameter: f32,
    pub branches: Vec<Branch>,
}

/// The nailboard of a harness; `None` if it has no wires.
pub fn layout(prj: &Project, an: &Analysis, tag: &str) -> Option<Nailboard> {
    let wires = harness::wires(prj, an, tag);
    if wires.is_empty() {
        return None;
    }
    let h = prj.harnesses.iter().find(|h| h.tag == tag);
    Some(from_wires(tag, h.map(|h| h.description.as_str()).unwrap_or(""), h.map(|h| h.part.as_str()).unwrap_or(""), &wires))
}

pub fn from_wires(tag: &str, description: &str, part: &str, wires: &[HarnessWire]) -> Nailboard {
    let mut devices: Vec<String> = wires.iter().flat_map(|w| [w.from.0.clone(), w.to.0.clone()]).collect();
    devices.sort_by(|a, b| natural_cmp(a, b));
    devices.dedup();
    let at = |d: &str| devices.iter().position(|x| x == d).unwrap_or(0);
    let pairs: Vec<(usize, usize, f32)> = wires.iter().map(|w| (at(&w.from.0), at(&w.to.0), w.length)).collect();
    let lengths = branch_lengths(devices.len(), &pairs);
    let n = devices.len();
    // Around the node long and short branches alternate, so neighbouring tables stay apart.
    let mut by_length: Vec<usize> = (0..n).collect();
    by_length.sort_by(|a, b| lengths[*b].0.total_cmp(&lengths[*a].0));
    let mut around = Vec::with_capacity(n);
    let (mut lo, mut hi) = (0, n);
    while lo < hi {
        around.push(by_length[lo]);
        lo += 1;
        if lo < hi {
            hi -= 1;
            around.push(by_length[hi]);
        }
    }
    let slot = |i: usize| around.iter().position(|x| *x == i).unwrap_or(0);
    let branches = devices
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let own: Vec<&HarnessWire> = wires.iter().filter(|w| w.from.0 == *d || w.to.0 == *d).collect();
            let mut pins: Vec<[String; 5]> = own
                .iter()
                .map(|w| {
                    let (me, other) = if w.from.0 == *d { (&w.from, &w.to) } else { (&w.to, &w.from) };
                    let other = if other.1.is_empty() { other.0.clone() } else { format!("{}:{}", other.0, other.1) };
                    [me.1.clone(), w.number.clone(), w.color.clone(), w.cross_section.clone(), other]
                })
                .collect();
            pins.sort_by(|a, b| natural_cmp(&a[0], &b[0]));
            let bundle: Vec<HarnessWire> = own.into_iter().cloned().collect();
            // Two connectors: a straight line; more: spread around the node, the first to the left.
            let angle = if n == 2 { if i == 0 { 180.0 } else { 0.0 } } else { 180.0 - 360.0 * slot(i) as f32 / n as f32 };
            Branch { device: d.clone(), length: lengths[i].0, assumed: lengths[i].1, angle, diameter: harness::bundle_diameter(&bundle), pins }
        })
        .collect();
    Nailboard { tag: tag.into(), description: description.into(), part: part.into(), wires: wires.len(), diameter: harness::bundle_diameter(wires), branches }
}

/// Size of a connector's pin table on the sheet, mm.
fn table_size(b: &Branch) -> (f32, f32) {
    (TABLE_W, 6.0 + 3.6 * b.pins.len() as f32)
}

const TABLE_W: f32 = 78.0;
const HEADER: f32 = 22.0;
const MARGIN: f32 = 10.0;

fn dir(angle: f32) -> P {
    let a = angle.to_radians();
    p(a.cos(), -a.sin())
}

/// Top left corner of a connector's table next to the branch end `end`.
fn table_at(b: &Branch, end: P) -> P {
    let (w, h) = table_size(b);
    let d = dir(b.angle);
    let x = if d.x < -0.3 { end.x - w - 3.0 } else if d.x > 0.3 { end.x + 3.0 } else { end.x - w / 2.0 };
    let y = if d.y < -0.3 { end.y - h - 3.0 } else if d.y > 0.3 { end.y + 3.0 } else { end.y - h / 2.0 };
    p(x, y)
}

/// Length of a branch on the sheet at scale `1 : n`.
fn drawn(b: &Branch, n: f32) -> f32 {
    (b.length / n).max(MIN_DRAWN)
}

/// Branch ends and the top left corners of the tables at scale `1 : n`, node at the
/// origin; tables that would overlap are moved apart vertically.
fn placement(nb: &Nailboard, n: f32) -> Vec<(P, P)> {
    let mut v: Vec<(P, P)> = nb
        .branches
        .iter()
        .map(|b| {
            let d = dir(b.angle);
            let end = p(d.x * drawn(b, n), d.y * drawn(b, n));
            (end, table_at(b, end))
        })
        .collect();
    // Tables below the node move down, tables above it move up (away from the branches).
    for up in [false, true] {
        let mut order: Vec<usize> = (0..v.len()).filter(|i| (v[*i].0.y < 0.0) == up).collect();
        order.sort_by(|a, b| if up { v[*b].1.y.total_cmp(&v[*a].1.y) } else { v[*a].1.y.total_cmp(&v[*b].1.y) });
        for k in 0..order.len() {
            let i = order[k];
            let (wi, hi) = table_size(&nb.branches[i]);
            for _ in 0..order.len() {
                let hit = order[..k].iter().find(|&&j| {
                    let (wj, hj) = table_size(&nb.branches[j]);
                    let (a, b) = (v[i].1, v[j].1);
                    a.x < b.x + wj + 2.0 && b.x < a.x + wi + 2.0 && a.y < b.y + hj + 2.0 && b.y < a.y + hi + 2.0
                });
                let Some(&j) = hit else { break };
                let (_, hj) = table_size(&nb.branches[j]);
                v[i].1.y = if up { v[j].1.y - hi - 2.0 } else { v[j].1.y + hj + 2.0 };
            }
        }
    }
    v
}

/// Extent of the drawing at scale `1 : n` around the node at the origin: min and max.
fn extent(nb: &Nailboard, n: f32) -> (P, P) {
    let (mut lo, mut hi) = (p(0.0, 0.0), p(0.0, 0.0));
    for (b, (end, t)) in nb.branches.iter().zip(placement(nb, n)) {
        let (w, h) = table_size(b);
        for q in [end, t, p(t.x + w, t.y + h)] {
            lo = p(lo.x.min(q.x), lo.y.min(q.y));
            hi = p(hi.x.max(q.x), hi.y.max(q.y));
        }
    }
    (lo, hi)
}

/// The smallest standard scale (1 : n) at which the board fits the sheet.
pub fn scale(nb: &Nailboard) -> f32 {
    let (w, h) = (SHEET.0 - 2.0 * MARGIN, SHEET.1 - 2.0 * MARGIN - HEADER);
    SCALES
        .into_iter()
        .find(|n| {
            let (lo, hi) = extent(nb, *n);
            hi.x - lo.x <= w && hi.y - lo.y <= h
        })
        .unwrap_or(SCALES[SCALES.len() - 1])
}

fn nail(d: &mut dyn Draw, at: P) {
    d.circle(at, 1.2, 0.25, Ink::Line, false);
    d.line(p(at.x - 1.8, at.y), p(at.x + 1.8, at.y), 0.18, Ink::Line, false);
    d.line(p(at.x, at.y - 1.8), p(at.x, at.y + 1.8), 0.18, Ink::Line, false);
}

/// Draws the nailboard on an A3 sheet; returns the scale (1 : n).
pub fn draw(d: &mut dyn Draw, nb: &Nailboard) -> f32 {
    let n = scale(nb);
    let (w, h) = SHEET;
    d.rect(p(MARGIN / 2.0, MARGIN / 2.0), p(w - MARGIN / 2.0, h - MARGIN / 2.0), 0.5, Ink::Frame, false);
    d.text(p(MARGIN, MARGIN + 4.0), &tr_fmt!("Nailboard {}", nb.tag), 5.0, Anchor::Left, VAnchor::Middle, Ink::Text);
    let mut info = vec![tr_fmt!("{} wires", nb.wires), tr_fmt!("Bundle Ø {} mm", format!("{:.1}", nb.diameter)), tr_fmt!("Scale 1:{}", n)];
    if !nb.part.is_empty() {
        info.insert(0, tr_fmt!("Part {}", nb.part));
    }
    if !nb.description.is_empty() {
        info.insert(0, nb.description.clone());
    }
    d.text(p(MARGIN, MARGIN + 11.0), &info.join("  ·  "), 2.8, Anchor::Left, VAnchor::Middle, Ink::Text);
    let mut notes = Vec::new();
    if nb.branches.iter().any(|b| b.assumed) {
        notes.push(tr("~ length assumed: no wire of the connector has a length"));
    }
    if nb.branches.iter().any(|b| b.length / n < MIN_DRAWN) {
        notes.push(tr("// branch not to scale"));
    }
    d.text(p(MARGIN, MARGIN + 16.0), &notes.join("  ·  "), 2.2, Anchor::Left, VAnchor::Middle, Ink::Error);
    // Node: centre of the free area, shifted so the extent fits.
    let (lo, hi) = extent(nb, n);
    let area_min = p(MARGIN, MARGIN + HEADER);
    let area = (w - 2.0 * MARGIN, h - 2.0 * MARGIN - HEADER);
    let node = p(area_min.x + (area.0 - (hi.x - lo.x)) / 2.0 - lo.x, area_min.y + (area.1 - (hi.y - lo.y)) / 2.0 - lo.y);
    let places = placement(nb, n);
    for (b, (_, rel_t)) in nb.branches.iter().zip(places) {
        let dv = dir(b.angle);
        let len_drawn = drawn(b, n);
        let end = p(node.x + dv.x * len_drawn, node.y + dv.y * len_drawn);
        // The bundle as two lines its diameter apart (at least 1 mm on the sheet).
        let half = (b.diameter / n).max(1.0) / 2.0;
        let nrm = p(-dv.y * half, dv.x * half);
        d.line(p(node.x + nrm.x, node.y + nrm.y), p(end.x + nrm.x, end.y + nrm.y), 0.35, Ink::Wire, false);
        d.line(p(node.x - nrm.x, node.y - nrm.y), p(end.x - nrm.x, end.y - nrm.y), 0.35, Ink::Wire, false);
        nail(d, end);
        if len_drawn > b.length / n + 0.5 {
            // Break mark: not to scale.
            let c = p(node.x + dv.x * len_drawn * 0.7, node.y + dv.y * len_drawn * 0.7);
            let side = p(-dv.y * (half + 2.0), dv.x * (half + 2.0));
            for k in [-1.0f32, 1.0] {
                let o = p(c.x + dv.x * k * 0.8, c.y + dv.y * k * 0.8);
                d.line(p(o.x - side.x + dv.x, o.y - side.y + dv.y), p(o.x + side.x - dv.x, o.y + side.y - dv.y), 0.3, Ink::Line, false);
            }
        }
        let mid = p((node.x + end.x) / 2.0 - dv.y * (half + 2.5), (node.y + end.y) / 2.0 + dv.x * (half + 2.5));
        let len = format!("{}{:.0} mm", if b.assumed { "~" } else { "" }, b.length);
        d.text(mid, &len, 2.5, Anchor::Center, VAnchor::Middle, if b.assumed { Ink::Error } else { Ink::Text });
        // Connector with its pin table (a leader line if it was moved away from the end).
        let t = p(node.x + rel_t.x, node.y + rel_t.y);
        let (tw, th) = table_size(b);
        if t != table_at(b, end) {
            let near = p(end.x.clamp(t.x, t.x + tw), end.y.clamp(t.y, t.y + th));
            d.line(end, near, 0.18, Ink::Line, true);
        }
        d.rect(t, p(t.x + tw, t.y + th), 0.35, Ink::Line, false);
        d.text(p(t.x + 2.0, t.y + 3.0), &b.device, 2.8, Anchor::Left, VAnchor::Middle, Ink::Text);
        d.line(p(t.x, t.y + 6.0), p(t.x + tw, t.y + 6.0), 0.25, Ink::Line, false);
        let cols = [2.0, 12.0, 24.0, 38.0, 50.0];
        for (k, pin) in b.pins.iter().enumerate() {
            let y = t.y + 7.8 + 3.6 * k as f32;
            for (c, x) in pin.iter().zip(cols) {
                d.text(p(t.x + x, y), c, 2.0, Anchor::Left, VAnchor::Middle, Ink::Text);
            }
        }
    }
    nail(d, node);
    d.circle(node, 2.2, 0.35, Ink::Line, false);
    n
}

/// The nailboard as an SVG drawing (A3 landscape).
pub fn svg(nb: &Nailboard) -> String {
    crate::svg::drawing_svg(SHEET.0, SHEET.1, |d| {
        draw(d, nb);
    })
}

#[cfg(test)]
mod tests;
