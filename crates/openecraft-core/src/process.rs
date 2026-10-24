//! Pre-planning of process plants: PCT loops (process control technology, IEC 62424)
//! planned before the electrical engineering. The instruments of a P&ID carry the
//! function letters and the loop number in their label ("TIC 101"); their device tag
//! links them with the devices of the schematic. A loop is engineered when all its
//! devices have functions in the schematic.

use openecraft_i18n::{tr, tr_fmt};
use crate::index::Index;
use crate::model::*;
use crate::symbols::{self, Func};
pub use crate::model::process::PctLoop;

/// An instrument placed in a P&ID.
#[derive(Clone, Debug, PartialEq)]
pub struct Instrument {
    pub tag: String,
    /// Function letters, e.g. "TIC".
    pub letters: String,
    pub loop_name: String,
    pub page: usize,
    pub sym_id: u64,
}

/// Function letters and loop number of an instrument label: "TIC 101", "PI-12",
/// "LSH102A" → ("LSH", "102A").
pub fn parse_label(label: &str) -> Option<(String, String)> {
    let s = label.trim();
    let letters: String = s.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let rest = s[letters.len()..].trim_start_matches([' ', '-', '_']).trim();
    (!letters.is_empty() && rest.starts_with(|c: char| c.is_ascii_digit())).then(|| (letters.to_ascii_uppercase(), rest.to_string()))
}

/// Measured variable of the first function letter (ISA 5.1 / IEC 62424).
pub fn variable(letters: &str) -> &'static str {
    match letters.chars().next().map(|c| c.to_ascii_uppercase()) {
        Some('A') => tr("Analysis"),
        Some('D') => tr("Density"),
        Some('E') => tr("Voltage"),
        Some('F') => tr("Flow"),
        Some('G') => tr("Position"),
        Some('H') => tr("Hand"),
        Some('I') => tr("Current"),
        Some('J') => tr("Power"),
        Some('K') => tr("Time"),
        Some('L') => tr("Level"),
        Some('M') => tr("Moisture"),
        Some('P') => tr("Pressure"),
        Some('Q') => tr("Quantity"),
        Some('R') => tr("Radiation"),
        Some('S') => tr("Speed"),
        Some('T') => tr("Temperature"),
        Some('V') => tr("Vibration"),
        Some('W') => tr("Weight"),
        _ => "–",
    }
}

/// Instruments of all schematic pages, in page order.
pub fn instruments(prj: &Project) -> Vec<Instrument> {
    let mut v = Vec::new();
    for (pi, pg) in prj.pages.iter().enumerate().filter(|(_, pg)| pg.kind == PageKind::Schematic) {
        for s in &pg.symbols {
            let Some(def) = prj.symbol(&s.sym) else { continue };
            if def.category != symbols::Category::Process || def.func != Func::Overview {
                continue;
            }
            let (letters, loop_name) = parse_label(&s.label).unwrap_or_default();
            v.push(Instrument { tag: s.tag.clone(), letters, loop_name, page: pi, sym_id: s.id });
        }
    }
    v
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// No device of the loop is in the schematic yet.
    Planned,
    /// Some devices are in the schematic.
    InProgress,
    /// All devices are in the schematic.
    Engineered,
}

impl Status {
    pub fn title(self) -> &'static str {
        match self {
            Status::Planned => tr("Planned"),
            Status::InProgress => tr("In engineering"),
            Status::Engineered => tr("Engineered"),
        }
    }
}

/// A loop with what the P&ID and the schematic say about it.
#[derive(Clone, Debug, PartialEq)]
pub struct LoopInfo {
    pub name: String,
    pub description: String,
    pub range: String,
    /// Defined in the pre-planning (not only found in the P&ID).
    pub defined: bool,
    /// Function letters of its instruments, e.g. ["TIC", "TSH"].
    pub functions: Vec<String>,
    /// Devices of the loop: instrument tags and the listed devices.
    pub devices: Vec<String>,
    /// Devices without functions in the schematic.
    pub missing: Vec<String>,
    pub status: Status,
    /// First instrument (jump target).
    pub target: Option<(usize, u64)>,
}

/// The device has functions in the schematic besides overview representations.
fn engineered(idx: &Index, tag: &str) -> bool {
    idx.devices.get(tag).is_some_and(|d| !d.mains.is_empty() || !d.auxs.is_empty() || !d.others.is_empty())
}

/// Defined loops and loops found in the P&ID, in natural order of their numbers.
pub fn loops(prj: &Project, idx: &Index) -> Vec<LoopInfo> {
    let insts = instruments(prj);
    let mut names: Vec<String> = prj.pct_loops.iter().map(|l| l.name.clone()).collect();
    names.extend(insts.iter().filter(|i| !i.loop_name.is_empty()).map(|i| i.loop_name.clone()));
    names.sort_by(|a, b| natural_cmp(a, b));
    names.dedup();
    names
        .into_iter()
        .map(|name| {
            let def = prj.pct_loops.iter().find(|l| l.name == name);
            let mine: Vec<&Instrument> = insts.iter().filter(|i| i.loop_name == name).collect();
            let mut functions: Vec<String> = mine.iter().map(|i| i.letters.clone()).collect();
            functions.dedup();
            let mut devices: Vec<String> = mine.iter().map(|i| i.tag.clone()).filter(|t| !t.is_empty()).collect();
            devices.extend(def.map(|d| d.devices.clone()).unwrap_or_default());
            devices.sort_by(|a, b| natural_cmp(a, b));
            devices.dedup();
            let missing: Vec<String> = devices.iter().filter(|t| !engineered(idx, t)).cloned().collect();
            let status = if devices.is_empty() || missing.len() == devices.len() {
                Status::Planned
            } else if missing.is_empty() {
                Status::Engineered
            } else {
                Status::InProgress
            };
            LoopInfo {
                description: def.map(|d| d.description.clone()).unwrap_or_default(),
                range: def.map(|d| d.range.clone()).unwrap_or_default(),
                defined: def.is_some(),
                functions,
                devices,
                missing,
                status,
                target: mine.first().map(|i| (i.page, i.sym_id)),
                name,
            }
        })
        .collect()
}

/// Pre-planning problems: (message, warning (else info), jump target).
pub fn problems(prj: &Project, idx: &Index) -> Vec<Problem> {
    let mut v = Vec::new();
    for i in instruments(prj).iter().filter(|i| i.loop_name.is_empty()) {
        let label = &prj.pages[i.page].symbols.iter().find(|s| s.id == i.sym_id).map(|s| s.label.clone()).unwrap_or_default();
        v.push((tr_fmt!("Instrument {}: label '{}' has no function letters and loop number (e.g. TIC 101)", i.tag, label), true, Some((i.page, i.sym_id))));
    }
    for l in loops(prj, idx) {
        if !l.defined {
            v.push((tr_fmt!("PCT loop {} is in the P&ID but not in the pre-planning", l.name), false, l.target));
        } else if l.target.is_none() {
            v.push((tr_fmt!("PCT loop {} has no instrument in the P&ID", l.name), true, None));
        }
        if l.status == Status::InProgress {
            v.push((tr_fmt!("PCT loop {}: devices not in the schematic yet: {}", l.name, l.missing.join(", ")), false, l.target));
        }
    }
    v
}

/// (message, warning (else info), jump target)
pub type Problem = (String, bool, Option<(usize, u64)>);

#[cfg(test)]
mod tests;
