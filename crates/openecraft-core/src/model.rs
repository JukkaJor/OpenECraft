//! Project data model: project → pages → placed symbols, wires, texts and graphics.
//!
//! Devices are identified by their device tag (e.g. `-K1`). All symbols that share a
//! tag belong to the same device: one *main function* (e.g. the coil) and any number
//! of *auxiliary functions* (e.g. its contacts). Cross references are derived from that.

use openecraft_i18n::{tr, tr_fmt};
use crate::geom::{p, P};
pub use crate::panel::Layout;
pub use openecraft_parts::natural_cmp;
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
    /// Panel layout (enclosures, mounting plates, rails, ducts, devices).
    pub layout: Layout,
    pub cables: Vec<Cable>,
    pub wire_numbering: WireNumbering,
    /// Terminal strip data (order, jumpers, spare terminals) keyed by strip tag.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub strips: BTreeMap<String, StripData>,
    /// PLC card data (start address, rack / slot) keyed by card tag.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub plc_cards: BTreeMap<String, PlcCard>,
    /// Released revisions, oldest first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub revisions: Vec<crate::revision::Revision>,
    /// Wire harnesses with the connections they contain.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub harnesses: Vec<crate::harness::Harness>,
    /// Project symbol library (user symbols).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<crate::symlib::UserSymbol>,
    /// Title blocks and report forms.
    pub forms: crate::forms::Forms,
    pub next_id: u64,
}

impl Default for Project {
    fn default() -> Self {
        let mut prj = Project {
            info: ProjectInfo::default(),
            pages: Vec::new(),
            parts: Vec::new(),
            devices: BTreeMap::new(),
            layout: Layout::default(),
            cables: Vec::new(),
            wire_numbering: WireNumbering::default(),
            strips: BTreeMap::new(),
            plc_cards: BTreeMap::new(),
            revisions: Vec::new(),
            harnesses: Vec::new(),
            symbols: Vec::new(),
            forms: Default::default(),
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
    /// User defined project properties, shown in title blocks as `{prop:Name}`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, String>,
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
            properties: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageKind {
    Schematic,
    Cover,
    Report(ReportKind),
    /// 2D model view of the mounting plate of an enclosure (enclosure id).
    Mounting(u64),
}

impl PageKind {
    pub fn label(self) -> String {
        match self {
            PageKind::Schematic => tr("Schematic (multi-line)").into(),
            PageKind::Cover => tr("Cover sheet").into(),
            PageKind::Report(r) => tr_fmt!("Report: {}", r.title()),
            PageKind::Mounting(_) => tr("Mounting layout").into(),
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
    PanelWiring,
    CableOverview,
    CableDiagram,
    RevisionOverview,
    HarnessData,
}

impl ReportKind {
    pub const ALL: [ReportKind; 12] = [
        ReportKind::TableOfContents,
        ReportKind::DeviceList,
        ReportKind::BillOfMaterials,
        ReportKind::TerminalDiagram,
        ReportKind::ConnectionList,
        ReportKind::PlcAssignment,
        ReportKind::PotentialList,
        ReportKind::PanelWiring,
        ReportKind::CableOverview,
        ReportKind::CableDiagram,
        ReportKind::RevisionOverview,
        ReportKind::HarnessData,
    ];

    pub fn title(self) -> &'static str {
        match self {
            ReportKind::TableOfContents => tr("Table of contents"),
            ReportKind::DeviceList => tr("Device list"),
            ReportKind::BillOfMaterials => tr("Bill of materials"),
            ReportKind::TerminalDiagram => tr("Terminal diagram"),
            ReportKind::ConnectionList => tr("Connection list"),
            ReportKind::PlcAssignment => tr("PLC assignment list"),
            ReportKind::PotentialList => tr("Potential overview"),
            ReportKind::PanelWiring => tr("Wire list (panel routing)"),
            ReportKind::CableOverview => tr("Cable overview"),
            ReportKind::CableDiagram => tr("Cable diagram"),
            ReportKind::RevisionOverview => tr("Revision overview (changes)"),
            ReportKind::HarnessData => tr("Harness data"),
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
    /// Structure of the page (IEC 81346) without prefix signs: function (`=`) and
    /// location (`+`). Devices placed on the page inherit it.
    pub function: String,
    pub location: String,
    /// Released page: no changes until it is unlocked.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
    /// Title block of this page; empty = the project's default.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title_block: String,
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
    /// Its content can be changed: a schematic page that is not released. Report,
    /// cover and mounting pages are generated.
    pub fn is_editable(&self) -> bool {
        self.kind == PageKind::Schematic && !self.locked
    }

    pub fn new(id: u64, name: &str, description: &str, kind: PageKind) -> Page {
        Page {
            id,
            name: name.into(),
            description: description.into(),
            function: String::new(),
            location: String::new(),
            locked: false,
            title_block: String::new(),
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
    /// Wire (connection) number; all wires of a connection carry the same number.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub number: String,
    /// Set manually: automatic numbering keeps it.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub number_fixed: bool,
    /// Tag of the cable this wire is a core of, and the core designation.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cable: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub core: String,
}

/// A cable; its cores are assigned to wires (`Wire::cable` / `Wire::core`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Cable {
    pub id: u64,
    /// Complete device tag, e.g. `=A1-W1`.
    pub tag: String,
    /// Cable type, e.g. "H07RN-F 4G1.5".
    pub cable_type: String,
    /// Core designations separated by blanks, e.g. "BN BK GY GNYE" or "1 2 3 GNYE".
    pub cores: String,
    pub cross_section: String,
    /// Length in metres.
    pub length: f32,
    /// Part number (bill of materials: length × part).
    pub part: String,
    pub description: String,
}

impl Cable {
    pub fn core_list(&self) -> Vec<String> {
        self.cores.split_whitespace().map(String::from).collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NumberScheme {
    /// 1, 2, 3 … in schematic order.
    #[default]
    Sequential,
    /// Page, column and counter: `3.4.1`.
    PageColumn,
    /// Named potentials (L1, +24V …) keep their name, other connections are counted.
    Potential,
}

impl NumberScheme {
    pub const ALL: [NumberScheme; 3] = [NumberScheme::Sequential, NumberScheme::PageColumn, NumberScheme::Potential];

    pub fn title(self) -> &'static str {
        match self {
            NumberScheme::Sequential => tr("Sequential (1, 2, 3 …)"),
            NumberScheme::PageColumn => tr("Page / column (3.4.1)"),
            NumberScheme::Potential => tr("Potential name (L1, +24V, otherwise counted)"),
        }
    }
}

/// Terminal strip data beyond the placed terminals.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StripData {
    /// Terminal designations in strip order; terminals not listed follow sorted.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<String>,
    /// Terminal designation -> jumper (bridge) number; terminals with the same
    /// number are connected.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub jumpers: BTreeMap<String, u32>,
    /// Spare terminals (not in the schematic) at the end of the strip.
    pub spares: u32,
}

/// PLC card (I/O module) data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PlcCard {
    /// First byte of the card's inputs / outputs (I<start>.0, Q<start>.0).
    pub start_byte: u32,
    pub rack: String,
    pub slot: String,
    pub description: String,
}

/// Settings of the automatic wire numbering.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WireNumbering {
    pub scheme: NumberScheme,
    /// Put in front of counted numbers, e.g. "W".
    pub prefix: String,
    pub start: u32,
}

impl Default for WireNumbering {
    fn default() -> Self {
        WireNumbering { scheme: NumberScheme::Sequential, prefix: String::new(), start: 1 }
    }
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
    /// Reads a project file and prepares it for editing ([`Project::normalize`]).
    pub fn from_json(text: &str) -> Result<Project, String> {
        let mut prj: Project = serde_json::from_str(text).map_err(|e| tr_fmt!("Not a valid project file: {}", e))?;
        prj.normalize();
        Ok(prj)
    }

    /// Content of a project file.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn normalize(&mut self) {
        crate::symlib::register(&self.symbols);
        let mut max = 0;
        for pg in &self.pages {
            max = max.max(pg.id);
            max = pg.symbols.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.wires.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.texts.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.shapes.iter().map(|s| s.id).fold(max, u64::max);
        }
        for e in &self.layout.enclosures {
            max = max.max(e.id);
            max = e.items.iter().map(|i| i.id).fold(max, u64::max);
        }
        max = self.cables.iter().map(|c| c.id).fold(max, u64::max);
        max = self.harnesses.iter().map(|h| h.id).fold(max, u64::max);
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

    /// Next free device tag for an identifying letter within a structure, e.g.
    /// `=A1-K3` (counted per structure, as `=A1-K1` and `=A2-K1` are different devices).
    pub fn next_tag(&self, structure: &str, letter: &str) -> String {
        let prefix = format!("{structure}-{letter}");
        let tags = self.pages.iter().flat_map(|pg| pg.symbols.iter().map(|s| s.tag.as_str()));
        let max = tags
            .chain(self.cables.iter().map(|c| c.tag.as_str()))
            .chain(self.harnesses.iter().map(|h| h.tag.as_str()))
            .filter_map(|t| t.strip_prefix(&prefix)?.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        format!("{prefix}{}", max + 1)
    }

    pub fn cable(&self, tag: &str) -> Option<&Cable> {
        self.cables.iter().find(|c| c.tag == tag)
    }
}


#[cfg(test)]
mod tests;
