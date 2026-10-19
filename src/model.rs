//! Project data model: project → pages → placed symbols, wires, texts and graphics.
//!
//! Devices are identified by their device tag (e.g. `-K1`). All symbols that share a
//! tag belong to the same device: one *main function* (e.g. the coil) and any number
//! of *auxiliary functions* (e.g. its contacts). Cross references are derived from that.

use crate::geom::{p, P};
pub use crate::parts::{Part, PartRef};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PAGE_W: f32 = 420.0;
pub const PAGE_H: f32 = 297.0;
/// Outer drawing frame.
pub const FRAME_X0: f32 = 10.0;
pub const FRAME_Y0: f32 = 10.0;
pub const FRAME_X1: f32 = 410.0;
pub const FRAME_Y1: f32 = 287.0;
/// Inner area (inside column / row strips).
pub const AREA_X0: f32 = 15.0;
pub const AREA_Y0: f32 = 15.0;
/// Top edge of the title block.
pub const TITLE_Y0: f32 = 255.0;
pub const COLUMNS: usize = 10;
pub const ROWS: usize = 6;

pub fn column_width() -> f32 {
    (FRAME_X1 - AREA_X0) / COLUMNS as f32
}

pub fn row_height() -> f32 {
    (TITLE_Y0 - AREA_Y0) / ROWS as f32
}

/// Column (1-based) of a page position, as used for cross references (`/page.column`).
pub fn column_of(pos: P) -> usize {
    let c = ((pos.x - AREA_X0) / column_width()).floor() as i64 + 1;
    c.clamp(1, COLUMNS as i64) as usize
}

/// Row letter of a page position.
pub fn row_of(pos: P) -> char {
    let r = ((pos.y - AREA_Y0) / row_height()).floor() as i64;
    (b'A' + r.clamp(0, ROWS as i64 - 1) as u8) as char
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub info: ProjectInfo,
    pub pages: Vec<Page>,
    pub parts: Vec<Part>,
    /// Device level data (article assignment etc.) keyed by device tag.
    pub devices: BTreeMap<String, DeviceData>,
    pub next_id: u64,
}

impl Default for Project {
    fn default() -> Self {
        let mut prj = Project {
            info: ProjectInfo::default(),
            pages: Vec::new(),
            parts: Vec::new(),
            devices: BTreeMap::new(),
            next_id: 1,
        };
        let id = prj.new_id();
        prj.pages.push(Page::new(id, "1", "Schematic", PageKind::Schematic));
        prj
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectInfo {
    pub name: String,
    pub description: String,
    pub company: String,
    pub creator: String,
    pub date: String,
    pub revision: String,
}

impl Default for ProjectInfo {
    fn default() -> Self {
        ProjectInfo {
            name: "New project".into(),
            description: String::new(),
            company: String::new(),
            creator: String::new(),
            date: String::new(),
            revision: "A".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageKind {
    Schematic,
    Cover,
    Report(ReportKind),
}

impl PageKind {
    pub fn label(self) -> String {
        match self {
            PageKind::Schematic => "Schematic (multi-line)".into(),
            PageKind::Cover => "Cover sheet".into(),
            PageKind::Report(r) => format!("Report: {}", r.title()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ReportKind {
    TableOfContents,
    DeviceList,
    BillOfMaterials,
    TerminalDiagram,
    ConnectionList,
    PlcAssignment,
    PotentialList,
}

impl ReportKind {
    pub const ALL: [ReportKind; 7] = [
        ReportKind::TableOfContents,
        ReportKind::DeviceList,
        ReportKind::BillOfMaterials,
        ReportKind::TerminalDiagram,
        ReportKind::ConnectionList,
        ReportKind::PlcAssignment,
        ReportKind::PotentialList,
    ];

    pub fn title(self) -> &'static str {
        match self {
            ReportKind::TableOfContents => "Table of contents",
            ReportKind::DeviceList => "Device list",
            ReportKind::BillOfMaterials => "Bill of materials",
            ReportKind::TerminalDiagram => "Terminal diagram",
            ReportKind::ConnectionList => "Connection list",
            ReportKind::PlcAssignment => "PLC assignment list",
            ReportKind::PotentialList => "Potential overview",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Page {
    pub id: u64,
    /// Page number / name as shown in cross references, e.g. "3".
    pub name: String,
    pub description: String,
    pub kind: PageKind,
    pub symbols: Vec<SymbolInst>,
    pub wires: Vec<Wire>,
    pub texts: Vec<TextItem>,
    pub shapes: Vec<Shape>,
}

impl Default for Page {
    fn default() -> Self {
        Page::new(0, "1", "", PageKind::Schematic)
    }
}

impl Page {
    pub fn new(id: u64, name: &str, description: &str, kind: PageKind) -> Page {
        Page {
            id,
            name: name.into(),
            description: description.into(),
            kind,
            symbols: Vec::new(),
            wires: Vec::new(),
            texts: Vec::new(),
            shapes: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty() && self.wires.is_empty() && self.texts.is_empty() && self.shapes.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SymbolInst {
    pub id: u64,
    /// Key into the symbol library.
    pub sym: String,
    pub pos: P,
    /// Quarter turns clockwise.
    pub rot: u8,
    pub mirror: bool,
    /// Device tag, e.g. `-K1`.
    pub tag: String,
    /// Symbol specific designation: terminal number, potential name, PLC address,
    /// interruption point name, ...
    pub label: String,
    pub function_text: String,
    /// Connection point designations, one per symbol pin.
    pub pins: Vec<String>,
}

impl Default for SymbolInst {
    fn default() -> Self {
        SymbolInst {
            id: 0,
            sym: String::new(),
            pos: p(0.0, 0.0),
            rot: 0,
            mirror: false,
            tag: String::new(),
            label: String::new(),
            function_text: String::new(),
            pins: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Wire {
    pub id: u64,
    pub a: P,
    pub b: P,
    /// Optional wire colour (e.g. "BK", "BU", "GNYE").
    pub color: String,
    /// Optional cross-section (e.g. "1.5 mm²").
    pub cross_section: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextItem {
    pub id: u64,
    pub pos: P,
    pub text: String,
    pub size: f32,
}

impl Default for TextItem {
    fn default() -> Self {
        TextItem { id: 0, pos: P::default(), text: String::new(), size: 3.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ShapeKind {
    #[default]
    Line,
    Rect,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Shape {
    pub id: u64,
    pub kind: ShapeKind,
    pub a: P,
    pub b: P,
    pub dashed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceData {
    /// Assigned parts (e.g. contactor + auxiliary contact block).
    pub parts: Vec<PartRef>,
    pub description: String,
    /// Single part number of files written by older versions.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub part: String,
}

impl Project {
    pub fn new_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Repairs ids after loading (e.g. hand-edited files).
    pub fn normalize(&mut self) {
        let mut max = 0;
        for pg in &self.pages {
            max = max.max(pg.id);
            max = pg.symbols.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.wires.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.texts.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.shapes.iter().map(|s| s.id).fold(max, u64::max);
        }
        self.next_id = self.next_id.max(max + 1);
        // Make sure each symbol has a designation for each pin.
        for pg in &mut self.pages {
            for s in &mut pg.symbols {
                if let Some(def) = crate::symbols::get(&s.sym) {
                    while s.pins.len() < def.pins.len() {
                        s.pins.push(def.pins[s.pins.len()].label.to_string());
                    }
                    s.pins.truncate(def.pins.len());
                }
            }
        }
        self.migrate_part_assignments();
        if self.pages.is_empty() {
            let id = self.new_id();
            self.pages.push(Page::new(id, "1", "Schematic", PageKind::Schematic));
        }
    }

    /// Converts single part assignments of older files to part lists; old generic
    /// part numbers are mapped onto the catalog ("CT-9A-24DC" → "GEN.CT-9A-24DC").
    fn migrate_part_assignments(&mut self) {
        let catalog = crate::parts::default_catalog();
        let mut remapped = Vec::new();
        for dev in self.devices.values_mut() {
            let legacy = std::mem::take(&mut dev.part);
            if legacy.is_empty() || !dev.parts.is_empty() {
                continue;
            }
            let gen = format!("GEN.{legacy}");
            let number = if catalog.iter().any(|p| p.number == gen) {
                remapped.push((legacy.clone(), gen.clone()));
                gen
            } else {
                legacy
            };
            dev.parts.push(PartRef { number, count: 1 });
        }
        for (old, new) in remapped {
            self.parts.retain(|p| p.number != old);
            if !self.parts.iter().any(|p| p.number == new) {
                if let Some(p) = catalog.iter().find(|p| p.number == new) {
                    self.parts.push(p.clone());
                }
            }
        }
    }

    /// All device tags used in the project.
    pub fn all_tags(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .pages
            .iter()
            .flat_map(|pg| pg.symbols.iter())
            .filter(|s| !s.tag.is_empty())
            .map(|s| s.tag.clone())
            .collect();
        v.sort_by(|a, b| natural_cmp(a, b));
        v.dedup();
        v
    }

    /// Next free device tag for an identifying letter, e.g. `-K3`.
    pub fn next_tag(&self, letter: &str) -> String {
        let prefix = format!("-{letter}");
        let mut max = 0u32;
        for pg in &self.pages {
            for s in &pg.symbols {
                if let Some(rest) = s.tag.strip_prefix(&prefix) {
                    if let Ok(n) = rest.parse::<u32>() {
                        max = max.max(n);
                    }
                }
            }
        }
        format!("{prefix}{}", max + 1)
    }
}

/// Comparison that orders "-K2" before "-K10".
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn chunks(s: &str) -> Vec<(bool, String)> {
        let mut out: Vec<(bool, String)> = Vec::new();
        for c in s.chars() {
            let d = c.is_ascii_digit();
            match out.last_mut() {
                Some((ld, buf)) if *ld == d => buf.push(c),
                _ => out.push((d, c.to_string())),
            }
        }
        out
    }
    let (ca, cb) = (chunks(a), chunks(b));
    for (x, y) in ca.iter().zip(cb.iter()) {
        let ord = if x.0 && y.0 {
            let nx: u64 = x.1.parse().unwrap_or(0);
            let ny: u64 = y.1.parse().unwrap_or(0);
            nx.cmp(&ny).then_with(|| x.1.cmp(&y.1))
        } else {
            x.1.cmp(&y.1)
        };
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    ca.len().cmp(&cb.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order() {
        let mut v = vec!["-K10", "-K2", "-F1", "-K1"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["-F1", "-K1", "-K2", "-K10"]);
    }

    #[test]
    fn columns() {
        assert_eq!(column_of(p(AREA_X0 + 1.0, 50.0)), 1);
        assert_eq!(column_of(p(FRAME_X1 - 1.0, 50.0)), COLUMNS);
        assert_eq!(row_of(p(100.0, AREA_Y0 + 1.0)), 'A');
    }
}
