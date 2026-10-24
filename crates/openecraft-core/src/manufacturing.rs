//! Panel manufacturing (in the spirit of EPLAN Pro Panel Production): CNC machines
//! with their tools and controller dialects ([`machines`]), drilling, milling and
//! punching strategies for the faces of an enclosure ([`strategy`]), the programs of
//! the machines ([`post`]), bending and drilling of copper bars with cut optimisation
//! ([`bars`]), the exchange with enclosure-system configurators ([`configurator`]),
//! the hand-off of manufacturing jobs to the machines ([`handoff`]) and the feedback
//! of the panel production ([`feedback`]). The drilling pattern itself is in
//! [`crate::production`].

pub mod bars;
pub mod configurator;
pub mod feedback;
pub mod handoff;
pub mod machines;
pub mod post;
pub mod strategy;

pub use bars::{BarRules, Bend, CopperBar};
pub use feedback::Feedback;
pub use handoff::Job;
pub use machines::{Dialect, Machine, MachineKind, Tool, ToolKind};
pub use strategy::{Method, OpKind, Operation, Plan, Strategy};

use crate::model::Project;
use crate::panel::Face;
use std::collections::BTreeMap;
pub use crate::model::manufacturing::Manufacturing;

impl Default for Manufacturing {
    fn default() -> Self {
        Manufacturing {
            machines: Vec::new(),
            sheet_machine: machines::DEFAULT_SHEET.into(),
            face_machines: BTreeMap::new(),
            bar_machine: machines::DEFAULT_BAR.into(),
            strategy: Strategy::default(),
            bars: Vec::new(),
            bar_rules: BarRules::default(),
            jobs: Vec::new(),
            feedback: Vec::new(),
        }
    }
}

impl Manufacturing {
    pub fn is_default(&self) -> bool {
        *self == Manufacturing::default()
    }

    /// Id of the machine a face is made on.
    pub fn machine_of(&self, face: Face) -> &str {
        self.face_machines.get(face.key()).unwrap_or(&self.sheet_machine)
    }
}

/// Index of an enclosure by name.
pub fn enclosure_index(prj: &Project, name: &str) -> Option<usize> {
    prj.layout.enclosures.iter().position(|e| e.name == name)
}

/// File name part of a text (enclosure names such as "+CAB1").
pub fn file_stem(s: &str) -> String {
    let v: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
    v.trim_matches('_').to_string()
}

/// Checksum of a program (FNV-1a, 64 bit, hex): programs changed since their hand-off.
pub fn checksum(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// Numbers in machine files: three decimals at most, no trailing zeros.
pub fn num(v: f32) -> String {
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_string() }
}

#[cfg(test)]
mod tests;
