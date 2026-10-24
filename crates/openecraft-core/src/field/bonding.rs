//! Equipotential bonding: conductors between the main earthing terminal, the
//! enclosures, the conductive machine frames and the metal cable carriers. Every
//! conductive part must reach the main earthing terminal; metal carriers meeting at
//! a node count as connected ([`super::FieldRules::continuity`]).

use super::Field;
use crate::model::Project;
use crate::issue::{Issue, Severity};
use openecraft_i18n::tr;
use std::collections::HashMap;
pub use crate::model::field::{Bond, BondKind};

impl BondKind {
    pub const ALL: [BondKind; 3] = [BondKind::Main, BondKind::Supplementary, BondKind::Functional];

    pub fn title(self) -> &'static str {
        match self {
            BondKind::Main => tr("Main bonding"),
            BondKind::Supplementary => tr("Supplementary bonding"),
            BondKind::Functional => tr("Functional bonding"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            BondKind::Main => "main",
            BondKind::Supplementary => "supplementary",
            BondKind::Functional => "functional",
        }
    }

    pub fn from_key(key: &str) -> Option<BondKind> {
        BondKind::ALL.into_iter().find(|k| k.key() == key.trim().to_ascii_lowercase())
    }

    /// Smallest copper cross-section in mm².
    pub fn minimum(self) -> f32 {
        match self {
            BondKind::Main => 6.0,
            BondKind::Supplementary => 2.5,
            BondKind::Functional => 0.0,
        }
    }
}

/// What a bonding point is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointKind {
    MainEarth,
    Enclosure,
    Frame,
    Carrier,
}

impl PointKind {
    pub fn title(self) -> &'static str {
        match self {
            PointKind::MainEarth => tr("Main earthing terminal"),
            PointKind::Enclosure => tr("Enclosure"),
            PointKind::Frame => tr("Machine frame"),
            PointKind::Carrier => tr("Metal cable carrier"),
        }
    }
}

/// A conductive part with its connection to the main earthing terminal.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub name: String,
    pub kind: PointKind,
    pub bonded: bool,
}

/// Bonding points: the main earthing terminal, enclosures, conductive outlines and
/// metal carriers (segments by name).
pub fn names(prj: &Project) -> Vec<(String, PointKind)> {
    let f = &prj.field;
    let mut v = vec![(f.rules.main_earth.clone(), PointKind::MainEarth)];
    v.extend(prj.layout.enclosures.iter().map(|e| (e.name.clone(), PointKind::Enclosure)));
    v.extend(f.outlines.iter().filter(|o| o.conductive).map(|o| (o.name.clone(), PointKind::Frame)));
    v.extend(f.segments.iter().filter(|s| s.kind.metallic()).map(|s| (f.segment_name(s.id), PointKind::Carrier)));
    v
}

fn root(p: &mut [usize], mut i: usize) -> usize {
    while p[i] != i {
        p[i] = p[p[i]];
        i = p[i];
    }
    i
}

/// The bonding points and whether they reach the main earthing terminal.
pub fn points(prj: &Project) -> Vec<Point> {
    let f: &Field = &prj.field;
    let all = names(prj);
    let idx: HashMap<&str, usize> = all.iter().enumerate().map(|(i, (n, _))| (n.as_str(), i)).collect();
    let mut parent: Vec<usize> = (0..all.len()).collect();
    let join = |p: &mut Vec<usize>, a: usize, b: usize| {
        let (ra, rb) = (root(p, a), root(p, b));
        p[ra] = rb;
    };
    for b in &f.bonds {
        if let (Some(&a), Some(&c)) = (idx.get(b.a.as_str()), idx.get(b.b.as_str())) {
            join(&mut parent, a, c);
        }
    }
    if f.rules.continuity {
        let metal: Vec<&super::Segment> = f.segments.iter().filter(|s| s.kind.metallic()).collect();
        for (i, s) in metal.iter().enumerate() {
            for t in &metal[i + 1..] {
                if [s.a, s.b].iter().any(|n| *n == t.a || *n == t.b) {
                    if let (Some(&a), Some(&b)) = (idx.get(f.segment_name(s.id).as_str()), idx.get(f.segment_name(t.id).as_str())) {
                        join(&mut parent, a, b);
                    }
                }
            }
        }
    }
    let earth = root(&mut parent, 0);
    (0..all.len()).map(|i| Point { name: all[i].0.clone(), kind: all[i].1, bonded: root(&mut parent, i) == earth }).collect()
}

/// Cross-section in mm² of a text like "6", "6 mm²", "2,5".
fn mm2(s: &str) -> Option<f32> {
    crate::production::cross_section_mm2(s).or_else(|| s.trim().trim_end_matches("mm²").trim().replace(',', ".").parse().ok())
}

/// Conductive parts not connected to the main earthing terminal, conductors to unknown
/// points and conductors below the smallest cross-section.
pub fn problems(prj: &Project) -> Vec<Issue> {
    let f = &prj.field;
    let mut v = Vec::new();
    let known = names(prj);
    for b in &f.bonds {
        for end in [&b.a, &b.b] {
            if !known.iter().any(|(n, _)| n == end) {
                v.push(crate::issue!(Severity::Warning, None, "Bonding conductor {} refers to the unknown bonding point {}", b.id, end));
            }
        }
        match mm2(&b.cross_section) {
            Some(cs) if cs + 1e-3 < b.kind.minimum() => v.push(crate::issue!(Severity::Warning, None, "Bonding conductor {} – {} has {} mm², at least {} mm² are required", b.a, b.b, format!("{cs}"), format!("{}", b.kind.minimum()))),
            None if b.kind != BondKind::Functional => v.push(crate::issue!(Severity::Warning, None, "Bonding conductor {} – {} has no cross-section", b.a, b.b)),
            _ => {}
        }
    }
    // Only checked once bonding is planned at all.
    if !f.bonds.is_empty() {
        for p in points(prj).into_iter().filter(|p| !p.bonded) {
            v.push(crate::issue!(Severity::Warning, None, "{} ({}) is not connected to the equipotential bonding", p.name, p.kind.title()));
        }
    }
    v
}

