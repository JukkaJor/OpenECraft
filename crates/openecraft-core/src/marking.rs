//! Marking: the labels of devices, terminals, wires and cables to print on marker
//! material, grouped by material in the order they are applied, and the CSV export
//! for marking software.

use crate::index::Index;
use crate::model::Project;
use crate::netlist::{pin_designation, Connection};
use crate::panel::Face;
use crate::strips::{self, ElementKind};
use crate::structure::Dt;
use openecraft_i18n::tr;
use serde::{Deserialize, Serialize};

/// What a wire marker shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireText {
    /// The wire number, the same on both ends.
    #[default]
    Number,
    /// At each end the connection point at the other end (destination marking).
    Destination,
}

/// Marking settings of a project: wire marker text and the marker material per kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub wire_text: WireText,
    pub device_material: String,
    /// Legend plates of door devices.
    pub door_material: String,
    pub terminal_material: String,
    pub wire_material: String,
    pub cable_material: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            wire_text: WireText::Number,
            device_material: "GEN.MARK-DEV".into(),
            door_material: "GEN.MARK-LEG".into(),
            terminal_material: "GEN.MARK-TB-5".into(),
            wire_material: "GEN.MARK-WIRE".into(),
            cable_material: "GEN.MARK-CABLE".into(),
        }
    }
}

impl Settings {
    pub fn is_default(&self) -> bool {
        *self == Settings::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Device,
    Terminal,
    Wire,
    Cable,
}

impl Category {
    /// Code in the CSV export.
    pub fn code(self) -> &'static str {
        match self {
            Category::Device => "device",
            Category::Terminal => "terminal",
            Category::Wire => "wire",
            Category::Cable => "cable",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Category::Device => tr("Device marker"),
            Category::Terminal => tr("Terminal marker"),
            Category::Wire => tr("Wire marker"),
            Category::Cable => tr("Cable marker"),
        }
    }
}

/// A label (or several equal ones) to print.
#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub category: Category,
    /// Part number of the marker material.
    pub material: String,
    pub text: String,
    pub count: u32,
    /// Location designation of the enclosure the marked item is in; empty in the field.
    pub enclosure: String,
    /// What the marker belongs to: device tag, terminal strip, connection, cable.
    pub group: String,
}

/// A device tag as marked in its enclosure: without the location of the enclosure.
fn marked_tag(tag: &str, enclosure: &str) -> String {
    let mut dt = Dt::parse(tag);
    if !dt.location.is_empty() && dt.location == enclosure.trim_start_matches('+') {
        dt.location.clear();
    }
    dt.tag()
}

/// Enclosure of a placed device and whether it sits on the door.
fn placement(prj: &Project, tag: &str) -> (String, bool) {
    match prj.layout.find_device(tag) {
        Some((ei, it)) => (prj.layout.enclosures[ei].name.clone(), it.face == Face::Door),
        None => (String::new(), false),
    }
}

/// All markers of a project in the order they are applied: devices, terminal strips
/// (strip marker, then the terminals in rail order), wires (two per connection; cable
/// cores are marked by their cable), cables (two each). `conns`: the point-to-point
/// connections of the analysis.
pub fn markers(prj: &Project, conns: &[Connection]) -> Vec<Marker> {
    let s = &prj.marking;
    let strip_tags = strips::strip_tags(prj);
    let mut out = Vec::new();
    let mut push = |category, material: &str, text: String, count, enclosure: &str, group: &str| {
        if !text.is_empty() {
            out.push(Marker { category, material: material.into(), text, count, enclosure: enclosure.into(), group: group.into() });
        }
    };
    let idx = Index::build(prj);
    for tag in idx.sorted_tags().into_iter().filter(|t| !strip_tags.contains(t)) {
        let (enclosure, door) = placement(prj, tag);
        let material = if door { &s.door_material } else { &s.device_material };
        push(Category::Device, material, marked_tag(tag, &enclosure), 1, &enclosure, tag);
    }
    for strip in &strip_tags {
        let (enclosure, _) = placement(prj, strip);
        push(Category::Terminal, &s.terminal_material, marked_tag(strip, &enclosure), 1, &enclosure, strip);
        for e in strips::strip_layout(prj, strip) {
            if let ElementKind::Terminal(label) | ElementKind::Spare(label) = e.kind {
                push(Category::Terminal, &s.terminal_material, label, 1, &enclosure, strip);
            }
        }
    }
    for c in conns.iter().filter(|c| c.cable.is_empty()) {
        let (from, to) = (pin_designation(prj, c.a), pin_designation(prj, c.b));
        let tag = &prj.pages[c.a.page].symbols[c.a.sym].tag;
        let (enclosure, _) = placement(prj, tag);
        match s.wire_text {
            WireText::Number => push(Category::Wire, &s.wire_material, c.number.clone(), 2, &enclosure, &c.number),
            WireText::Destination => {
                let group = format!("{from} – {to}");
                push(Category::Wire, &s.wire_material, to.clone(), 1, &enclosure, &group);
                push(Category::Wire, &s.wire_material, from.clone(), 1, &enclosure, &group);
            }
        }
    }
    for c in &prj.cables {
        push(Category::Cable, &s.cable_material, c.tag.clone(), 2, "", &c.tag);
    }
    out
}

/// Pieces of marker material per part number, in the order of first use.
pub fn material(markers: &[Marker]) -> Vec<(String, u32)> {
    let mut v: Vec<(String, u32)> = Vec::new();
    for m in markers.iter().filter(|m| !m.material.is_empty()) {
        match v.iter_mut().find(|x| x.0 == m.material) {
            Some(x) => x.1 += m.count,
            None => v.push((m.material.clone(), m.count)),
        }
    }
    v
}

/// CSV for marking software: semicolon separated, UTF-8 with byte order mark, one row
/// per label, grouped by material in the order of application. The header and the
/// category codes are not translated.
pub fn csv(markers: &[Marker]) -> String {
    let esc = |s: &str| if s.contains([';', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
    let mut sorted: Vec<&Marker> = markers.iter().collect();
    let order: Vec<String> = material(markers).into_iter().map(|m| m.0).collect();
    sorted.sort_by_key(|m| order.iter().position(|o| *o == m.material).unwrap_or(usize::MAX));
    let mut out = String::from("\u{feff}Material;Category;Text;Count;Enclosure;Group\n");
    for m in sorted {
        out.push_str(&[esc(&m.material), m.category.code().into(), esc(&m.text), m.count.to_string(), esc(&m.enclosure), esc(&m.group)].join(";"));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests;
