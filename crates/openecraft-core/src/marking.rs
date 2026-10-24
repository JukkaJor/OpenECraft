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
pub use crate::model::marking::{Category, Column, Scheme, Settings, WireText};

impl Default for Settings {
    fn default() -> Self {
        Settings {
            wire_text: WireText::Number,
            device_material: "GEN.MARK-DEV".into(),
            door_material: "GEN.MARK-LEG".into(),
            terminal_material: "GEN.MARK-TB-5".into(),
            wire_material: "GEN.MARK-WIRE".into(),
            cable_material: "GEN.MARK-CABLE".into(),
            scheme: String::new(),
            schemes: Vec::new(),
        }
    }
}

impl Settings {
    pub fn is_default(&self) -> bool {
        *self == Settings::default()
    }
}

impl Category {
    pub const ALL: [Category; 4] = [Category::Device, Category::Terminal, Category::Wire, Category::Cable];

    pub fn parse(code: &str) -> Option<Category> {
        Category::ALL.into_iter().find(|c| c.code() == code.trim())
    }

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

/// Enclosure of a placed device (or of the valve terminal / valve it sits on) and
/// whether it sits on the door.
fn placement(prj: &Project, tag: &str) -> (String, bool) {
    match crate::panel::locate(prj, tag) {
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

impl Column {
    pub const ALL: [Column; 7] = [Column::Material, Column::Category, Column::Text, Column::Count, Column::Enclosure, Column::Group, Column::Number];

    /// Header in the export and code in commands (not translated).
    pub fn code(self) -> &'static str {
        match self {
            Column::Material => "Material",
            Column::Category => "Category",
            Column::Text => "Text",
            Column::Count => "Count",
            Column::Enclosure => "Enclosure",
            Column::Group => "Group",
            Column::Number => "No",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Column::Material => tr("Material"),
            Column::Category => tr("Category"),
            Column::Text => tr("Text"),
            Column::Count => tr("Count"),
            Column::Enclosure => tr("Enclosure"),
            Column::Group => tr("Group"),
            Column::Number => tr("Number"),
        }
    }

    pub fn parse(code: &str) -> Option<Column> {
        Column::ALL.into_iter().find(|c| c.code().eq_ignore_ascii_case(code.trim()))
    }
}

/// Name of the standard scheme (all columns, semicolon separated).
pub const STANDARD: &str = "Standard";

/// The built-in labeling schemes: the standard layout and import tables for common
/// marking programs (one label per row; adjust a copy if a program version expects
/// another layout).
pub fn builtin_schemes() -> Vec<Scheme> {
    let per_label = |name: &str, columns: Vec<Column>, separator: &str, header: bool| Scheme {
        name: name.into(),
        columns,
        separator: separator.into(),
        header,
        bom: false,
        one_row_per_label: true,
        categories: Vec::new(),
    };
    vec![
        Scheme { name: STANDARD.into(), ..Scheme::default() },
        per_label("Phoenix Contact CLIP PROJECT", vec![Column::Material, Column::Text], ";", true),
        per_label("Weidmüller M-Print PRO", vec![Column::Text], "\t", false),
        per_label("Brady", vec![Column::Number, Column::Text, Column::Material], ",", true),
        per_label("Murrplastik", vec![Column::Material, Column::Text], ";", true),
        per_label("Partex", vec![Column::Text], "\t", false),
    ]
}

/// The schemes available in the project: the built-in ones (replaced by project schemes
/// of the same name), then the other project schemes.
pub fn schemes(s: &Settings) -> Vec<Scheme> {
    let mut v = builtin_schemes();
    for own in &s.schemes {
        match v.iter_mut().find(|b| b.name == own.name) {
            Some(b) => *b = own.clone(),
            None => v.push(own.clone()),
        }
    }
    v
}

pub fn is_builtin(name: &str) -> bool {
    builtin_schemes().iter().any(|s| s.name == name)
}

/// The scheme of that name (the selected one with `None`; the standard scheme if none
/// is selected).
pub fn scheme(s: &Settings, name: Option<&str>) -> Option<Scheme> {
    let name = name.unwrap_or(&s.scheme);
    let all = schemes(s);
    if name.is_empty() {
        return all.into_iter().next();
    }
    all.into_iter().find(|x| x.name == name)
}

/// Markers grouped by material in the order of first use.
fn by_material(markers: &[Marker]) -> Vec<&Marker> {
    let mut sorted: Vec<&Marker> = markers.iter().collect();
    let order: Vec<String> = material(markers).into_iter().map(|m| m.0).collect();
    sorted.sort_by_key(|m| order.iter().position(|o| *o == m.material).unwrap_or(usize::MAX));
    sorted
}

/// The rows of a scheme (with the header row if it has one).
pub fn rows(markers: &[Marker], scheme: &Scheme) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    if scheme.header {
        out.push(scheme.columns.iter().map(|c| c.code().to_string()).collect());
    }
    let mut n = 0;
    for m in by_material(markers).into_iter().filter(|m| scheme.categories.is_empty() || scheme.categories.contains(&m.category)) {
        let (repeat, count) = if scheme.one_row_per_label { (m.count, 1) } else { (1, m.count) };
        for _ in 0..repeat {
            n += 1;
            let cell = |c: &Column| match c {
                Column::Material => m.material.clone(),
                Column::Category => m.category.code().into(),
                Column::Text => m.text.clone(),
                Column::Count => count.to_string(),
                Column::Enclosure => m.enclosure.clone(),
                Column::Group => m.group.clone(),
                Column::Number => n.to_string(),
            };
            out.push(scheme.columns.iter().map(cell).collect());
        }
    }
    out
}

/// Text export of a scheme; cells holding the separator, quotes or line breaks are quoted.
pub fn text(markers: &[Marker], scheme: &Scheme) -> String {
    let sep = if scheme.separator.is_empty() { ";" } else { scheme.separator.as_str() };
    let esc = |s: &str| if s.contains(sep) || s.contains(['"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
    let mut out = String::from(if scheme.bom { "\u{feff}" } else { "" });
    for row in rows(markers, scheme) {
        out.push_str(&row.iter().map(|c| esc(c)).collect::<Vec<_>>().join(sep));
        out.push('\n');
    }
    out
}

/// Excel export of a scheme (one sheet).
pub fn xlsx(markers: &[Marker], scheme: &Scheme) -> Vec<u8> {
    crate::xlsx::write(&[("Marking".into(), rows(markers, scheme))])
}

/// CSV for marking software in the standard scheme: semicolon separated, UTF-8 with
/// byte order mark, grouped by material in the order of application. The header and the
/// category codes are not translated.
pub fn csv(markers: &[Marker]) -> String {
    text(markers, &builtin_schemes()[0])
}

#[cfg(test)]
mod tests;
