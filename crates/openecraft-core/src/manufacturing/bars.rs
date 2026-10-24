//! Copper bars: flat bars bent flat-wise or edgewise and drilled, with the developed
//! length (DIN 6935), the positions of bends and holes on the flat bar, springback,
//! an order of the bends the machine can make, checks of short flanges and of holes
//! in the bending zone, and the cutting of all bars of the layout (also the bars of
//! the busbar systems) from stock lengths.

use super::machines::{Machine, MachineKind, ToolKind};
use super::num;
use crate::model::Project;
use crate::panel::ItemKind;
use openecraft_i18n::tr_fmt;
use std::fmt::Write;
pub use crate::model::manufacturing::{BarHole, BarRules, Bend, CopperBar};

impl Default for BarRules {
    fn default() -> Self {
        BarRules { k_factor: 0.0, radius: 1.0, min_radius: 1.0, min_flange: 25.0, springback: 2.0, stock_length: 4000.0, kerf: 3.0, end_distance: 1.0 }
    }
}

/// A bend on the flat bar.
#[derive(Clone, Debug, PartialEq)]
pub struct FlatBend {
    /// Index of the bend in the bar.
    pub index: usize,
    /// Centre of the bending zone from the start of the flat bar, mm.
    pub at: f32,
    /// Length of the bending zone (bend allowance), mm.
    pub zone: f32,
    pub angle: f32,
    /// Angle to set on the machine (with springback).
    pub set_angle: f32,
    pub radius: f32,
    pub edgewise: bool,
}

/// A copper bar developed flat.
#[derive(Clone, Debug, PartialEq)]
pub struct Flat {
    pub length: f32,
    pub bends: Vec<FlatBend>,
    /// Hole positions from the start of the flat bar and diameters.
    pub holes: Vec<(f32, f32)>,
    /// Order of the bends (indices into `bends`).
    pub sequence: Vec<usize>,
    pub problems: Vec<String>,
}

/// Correction factor of the neutral fibre after DIN 6935 (r / s < 5), else 1.
pub fn din_factor(r: f32, s: f32) -> f32 {
    if s <= 0.0 || r / s >= 5.0 {
        return 1.0;
    }
    (0.65 + 0.5 * (r / s).max(0.01).log10()).clamp(0.2, 1.0)
}

/// Bend allowance and setback of one bend: (allowance, setback) for inner radius `r`,
/// thickness in the bending plane `s` and angle `deg`.
pub fn bend_allowance(r: f32, s: f32, deg: f32, k_factor: f32) -> (f32, f32) {
    let beta = deg.abs().min(179.0);
    let k = if k_factor > 0.0 { 2.0 * k_factor } else { din_factor(r, s) };
    let allowance = std::f32::consts::PI * (r + s / 2.0 * k) * beta / 180.0;
    let setback = (r + s) * (beta.to_radians() / 2.0).tan();
    (allowance, setback)
}

impl CopperBar {
    /// Thickness in the bending plane and inner radius of a bend.
    fn bend_geometry(&self, b: &Bend, rules: &BarRules) -> (f32, f32) {
        let s = if b.edgewise { self.width } else { self.thickness };
        let r = if b.radius > 0.0 { b.radius } else { rules.radius * s };
        (s, r)
    }

    /// The bar developed flat with its bends, holes and bending order.
    pub fn flat(&self, rules: &BarRules) -> Flat {
        let mut problems = Vec::new();
        if self.bends.len() + 1 != self.segments.len() {
            problems.push(tr_fmt!("Copper bar {}: {} segments need {} bends", self.name, self.segments.len(), self.segments.len().saturating_sub(1)));
        }
        let n = self.bends.len().min(self.segments.len().saturating_sub(1));
        let geo: Vec<(f32, f32, f32, f32)> = self.bends[..n]
            .iter()
            .map(|b| {
                let (s, r) = self.bend_geometry(b, rules);
                let (a, sb) = bend_allowance(r, s, b.angle, rules.k_factor);
                (s, r, a, sb)
            })
            .collect();
        // Straight lengths on the flat bar: segment less the setbacks at its ends.
        let straight: Vec<f32> = (0..self.segments.len())
            .map(|i| self.segments[i] - if i > 0 && i - 1 < n { geo[i - 1].3 } else { 0.0 } - if i < n { geo[i].3 } else { 0.0 })
            .collect();
        let mut bends = Vec::new();
        let mut at = 0.0;
        for i in 0..self.segments.len() {
            at += straight[i];
            if i < n {
                let (s, r, a, _) = geo[i];
                let b = &self.bends[i];
                if r < rules.min_radius * s - 1e-3 {
                    problems.push(tr_fmt!("Copper bar {}: bend {} has a radius of {} mm, at least {} mm", self.name, i + 1, num(r), num(rules.min_radius * s)));
                }
                let set = b.angle.signum() * (b.angle.abs() * (1.0 + rules.springback / 100.0)).min(179.0);
                bends.push(FlatBend { index: i, at: at + a / 2.0, zone: a, angle: b.angle, set_angle: set, radius: r, edgewise: b.edgewise });
                at += a;
            }
        }
        let length = at;
        for (i, len) in straight.iter().enumerate() {
            if n > 0 && *len < rules.min_flange - 1e-3 {
                problems.push(tr_fmt!("Copper bar {}: segment {} is {} mm straight, at least {} mm are needed for bending", self.name, i + 1, num(len.max(0.0)), num(rules.min_flange)));
            }
        }
        let start_of = |seg: usize| -> f32 {
            let mut x = 0.0;
            for k in 0..seg.min(self.segments.len()) {
                x += straight[k] + if k < n { geo[k].2 } else { 0.0 };
            }
            // Outer dimensions start at the virtual sharp, before the bending zone.
            x - if seg > 0 && seg - 1 < n { geo[seg - 1].3 } else { 0.0 }
        };
        let mut holes: Vec<(f32, f32)> = self.holes.iter().filter(|h| h.segment < self.segments.len()).map(|h| (start_of(h.segment) + h.at, h.diameter)).collect();
        if self.end_holes > 0.0 {
            let d = (rules.end_distance.clamp(0.5, 2.0) * self.width).min(length / 2.0);
            holes.push((d, self.end_holes));
            holes.push((length - d, self.end_holes));
        }
        holes.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (x, d) in &holes {
            if *x - d / 2.0 < 0.0 || *x + d / 2.0 > length {
                problems.push(tr_fmt!("Copper bar {}: hole Ø{} at {} mm is outside the bar", self.name, num(*d), num(*x)));
                continue;
            }
            if d + 2.0 > self.width {
                problems.push(tr_fmt!("Copper bar {}: hole Ø{} is too large for a {} mm wide bar", self.name, num(*d), num(self.width)));
            }
            // Holes deform within the bending zone and one thickness next to it.
            if let Some(b) = bends.iter().find(|b| (x - b.at).abs() < b.zone / 2.0 + self.thickness + d / 2.0) {
                problems.push(tr_fmt!("Copper bar {}: hole Ø{} at {} mm is too close to bend {}", self.name, num(*d), num(*x), b.index + 1));
            }
        }
        let sequence = bend_sequence(&bends, length);
        Flat { length, bends, holes, sequence, problems }
    }
}

/// Order of the bends: from the bar ends inwards (the long remaining part stays straight
/// and is held by the machine), flat bends before edgewise ones at equal distance.
pub fn bend_sequence(bends: &[FlatBend], length: f32) -> Vec<usize> {
    let mut v: Vec<usize> = (0..bends.len()).collect();
    v.sort_by(|&a, &b| {
        let da = bends[a].at.min(length - bends[a].at);
        let db = bends[b].at.min(length - bends[b].at);
        da.total_cmp(&db).then(bends[a].edgewise.cmp(&bends[b].edgewise))
    });
    v
}

/// Bar profile of a busbar part: data `Bar size: 12 x 5`, or `Cu 12 x 5 mm` in its
/// description; 12 × 5 mm without data.
pub fn bar_profile(part: Option<&crate::parts::Part>) -> (f32, f32) {
    let parse = |s: &str| -> Option<(f32, f32)> {
        let (a, b) = s.split_once(['x', 'X', '×'])?;
        let w: String = a.chars().rev().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect::<Vec<_>>().into_iter().rev().collect();
        let t: String = b.trim_start().chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
        Some((w.replace(',', ".").parse().ok()?, t.replace(',', ".").parse().ok()?))
    };
    part.and_then(|p| p.tech_value("Bar size").and_then(parse).or_else(|| p.description.split_once("Cu").and_then(|(_, r)| parse(r))))
        .unwrap_or((12.0, 5.0))
}

/// A piece to cut: (label, width, thickness, length).
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub label: String,
    pub enclosure: String,
    pub width: f32,
    pub thickness: f32,
    pub length: f32,
    /// A bar of a busbar system (straight, no holes).
    pub system: bool,
}

/// All bars to cut: the copper bars (developed, each `count` times) and the bars of the
/// busbar systems in the layout (one per pole, straight).
pub fn pieces(prj: &Project) -> Vec<Piece> {
    let m = &prj.layout.manufacturing;
    let mut v = Vec::new();
    for b in &m.bars {
        let flat = b.flat(&m.bar_rules);
        for k in 0..b.count.max(1) {
            let label = if b.count > 1 { format!("{}/{}", b.name, k + 1) } else { b.name.clone() };
            v.push(Piece { label, enclosure: b.enclosure.clone(), width: b.width, thickness: b.thickness, length: flat.length, system: false });
        }
    }
    for e in &prj.layout.enclosures {
        for it in e.items.iter().filter(|i| i.kind == ItemKind::Busbar) {
            let part = prj.parts.iter().chain(crate::parts::catalog()).find(|p| p.number == it.part);
            let (w, t) = bar_profile(part);
            for k in 0..crate::production::bar_count(&prj.parts, &it.part) {
                v.push(Piece { label: format!("{} {} L{}", e.name, it.id, k + 1), enclosure: e.name.clone(), width: w, thickness: t, length: it.length(), system: true });
            }
        }
    }
    v
}

/// One stock bar with the pieces cut from it.
#[derive(Clone, Debug, PartialEq)]
pub struct StockBar {
    pub width: f32,
    pub thickness: f32,
    pub pieces: Vec<Piece>,
    /// Remaining length, mm.
    pub rest: f32,
}

/// Cutting plan: the pieces of each profile on stock bars (first fit decreasing) and
/// the pieces longer than the stock.
pub fn cutting(pieces: &[Piece], rules: &BarRules) -> (Vec<StockBar>, Vec<Piece>) {
    let mut sorted: Vec<&Piece> = pieces.iter().collect();
    sorted.sort_by(|a, b| a.width.total_cmp(&b.width).then(a.thickness.total_cmp(&b.thickness)).then(b.length.total_cmp(&a.length)));
    let mut bars: Vec<StockBar> = Vec::new();
    let mut too_long = Vec::new();
    for p in sorted {
        if p.length > rules.stock_length + 1e-3 {
            too_long.push(p.clone());
            continue;
        }
        let same = |b: &StockBar| (b.width - p.width).abs() < 0.01 && (b.thickness - p.thickness).abs() < 0.01;
        let need = |b: &StockBar| p.length + if b.pieces.is_empty() { 0.0 } else { rules.kerf };
        match bars.iter_mut().find(|b| same(b) && b.rest + 1e-3 >= need(b)) {
            Some(b) => {
                b.rest -= need(b);
                b.pieces.push(p.clone());
            }
            None => bars.push(StockBar { width: p.width, thickness: p.thickness, pieces: vec![p.clone()], rest: rules.stock_length - p.length }),
        }
    }
    (bars, too_long)
}

/// Program of the busbar machine (all enclosures or one): per copper bar the cut, the
/// punched holes (tool of the diameter) and the bends in their order, then the cuts of
/// the busbar systems' bars; one row per operation.
pub fn program(prj: &Project, m: &Machine, enclosure: Option<&str>) -> (String, Vec<String>) {
    let mine = |name: &str| enclosure.is_none_or(|e| e == name);
    let mf = &prj.layout.manufacturing;
    let mut problems = Vec::new();
    if m.kind != MachineKind::Busbar {
        problems.push(tr_fmt!("{} is not a busbar machine", m.name));
    }
    let mut out = String::from("Bar;Enclosure;Profile;Count;Operation;Position;Value;Tool;Edgewise\n");
    for b in mf.bars.iter().filter(|b| mine(&b.enclosure)) {
        let f = b.flat(&mf.bar_rules);
        problems.extend(f.problems.iter().cloned());
        if b.thickness > m.max_thickness + 1e-4 {
            problems.push(tr_fmt!("Copper bar {}: {} mm are too thick for {}", b.name, num(b.thickness), m.name));
        }
        let profile = format!("{}x{}", num(b.width), num(b.thickness));
        let row = |out: &mut String, op: &str, pos: f32, value: String, tool: String, edge: &str| {
            let _ = writeln!(out, "{};{};{profile};{};{op};{};{value};{tool};{edge}", b.name, b.enclosure, b.count.max(1), num(pos));
        };
        row(&mut out, "cut", f.length, String::new(), String::new(), "");
        for (x, d) in &f.holes {
            let tool = match m.tool(ToolKind::PunchRound, *d, 0.1) {
                Some(t) => format!("T{}", t.number),
                None => {
                    problems.push(tr_fmt!("Copper bar {}: no punch Ø{} on {}, drill by hand", b.name, num(*d), m.name));
                    String::new()
                }
            };
            row(&mut out, "punch", *x, num(*d), tool, "");
        }
        for &i in &f.sequence {
            let fb = &f.bends[i];
            row(&mut out, "bend", fb.at, num(fb.set_angle), format!("R{}", num(fb.radius)), if fb.edgewise { "1" } else { "0" });
        }
    }
    for p in pieces(prj).iter().filter(|p| p.system && mine(&p.enclosure)) {
        let _ = writeln!(out, "{};{};{}x{};1;cut;{};;;", p.label, p.enclosure, num(p.width), num(p.thickness), num(p.length));
    }
    (out, problems)
}

/// Problems of all copper bars (project check).
pub fn problems(prj: &Project) -> Vec<String> {
    let mf = &prj.layout.manufacturing;
    let mut v: Vec<String> = mf.bars.iter().flat_map(|b| b.flat(&mf.bar_rules).problems).collect();
    let (_, too_long) = cutting(&pieces(prj), &mf.bar_rules);
    v.extend(too_long.iter().map(|p| tr_fmt!("Copper bar {}: {} mm are longer than the stock length of {} mm", p.label, num(p.length), num(mf.bar_rules.stock_length))));
    v
}

/// Parses segment lengths: `200, 150, 300`.
pub fn parse_segments(text: &str) -> Result<Vec<f32>, String> {
    text.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()).map(|s| s.parse::<f32>().ok().filter(|v| *v > 0.0).ok_or_else(|| tr_fmt!("Invalid length {}", s))).collect()
}

/// Parses bends: `90; -90 e; 45 r10` (angle, `e` edgewise, `r` inner radius).
pub fn parse_bends(text: &str) -> Result<Vec<Bend>, String> {
    let mut v = Vec::new();
    for entry in text.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let mut b = Bend::default();
        let mut words = entry.split_whitespace();
        b.angle = words.next().and_then(|a| a.parse().ok()).filter(|a: &f32| a.abs() > 0.0 && a.abs() < 180.0).ok_or_else(|| tr_fmt!("Invalid bend {}", entry))?;
        for w in words {
            let w = w.to_ascii_lowercase();
            if w == "e" {
                b.edgewise = true;
            } else if let Some(r) = w.strip_prefix('r').and_then(|r| r.parse().ok()) {
                b.radius = r;
            } else {
                return Err(tr_fmt!("Invalid bend {}", entry));
            }
        }
        v.push(b);
    }
    Ok(v)
}

/// Parses holes: `0:20:9; 2:30:11` (segment, distance from its start, diameter).
pub fn parse_holes(text: &str) -> Result<Vec<BarHole>, String> {
    text.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let v: Vec<&str> = s.split(':').map(str::trim).collect();
            let hole = || match v[..] {
                [seg, at, d] => Some(BarHole { segment: seg.parse().ok()?, at: at.parse().ok()?, diameter: d.parse().ok().filter(|d: &f32| *d > 0.0)? }),
                _ => None,
            };
            hole().ok_or_else(|| tr_fmt!("Invalid hole {}", s))
        })
        .collect()
}

#[cfg(test)]
mod tests;
