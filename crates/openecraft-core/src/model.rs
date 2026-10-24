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
    /// Potential definitions (kind, voltage, wire colour and cross-section).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub potentials: Vec<crate::potentials::PotentialDef>,
    /// Wire colour and cross-section of nets without a defined potential.
    #[serde(skip_serializing_if = "crate::potentials::WireDefaults::is_default")]
    pub wire_defaults: crate::potentials::WireDefaults,
    /// Wires are drawn in their colour (schematic, PDF, SVG).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub wire_colors: bool,
    /// Terminal strip data (order, jumpers, spare terminals) keyed by strip tag.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub strips: BTreeMap<String, StripData>,
    /// Plug data (pin order) keyed by plug tag.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub plugs: BTreeMap<String, crate::plugs::PlugData>,
    /// PLC card data (start address, rack / slot) keyed by card tag.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub plc_cards: BTreeMap<String, PlcCard>,
    /// PLC stations, bus systems, bus connections and device descriptions.
    #[serde(skip_serializing_if = "crate::plc_hw::Hardware::is_empty")]
    pub plc: crate::plc_hw::Hardware,
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
    /// Report blocks: report pages generated and updated together.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub report_blocks: Vec<crate::report_blocks::ReportBlock>,
    /// Options and generation rules of the configurator.
    #[serde(skip_serializing_if = "crate::config::Configuration::is_empty")]
    pub configuration: crate::config::Configuration,
    /// Marker texts and materials.
    #[serde(skip_serializing_if = "crate::marking::Settings::is_default")]
    pub marking: crate::marking::Settings,
    /// Defaults and numbering of hoses / pipes.
    #[serde(skip_serializing_if = "crate::fluid::FluidSettings::is_default")]
    pub fluid: crate::fluid::FluidSettings,
    /// PCT loops of the pre-planning.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pct_loops: Vec<crate::process::PctLoop>,
    /// Source and target languages of the project texts with their dictionary.
    #[serde(skip_serializing_if = "crate::translation::Languages::is_default")]
    pub languages: crate::translation::Languages,
    /// Wired / problem marks of the wiring assistant.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub wiring_marks: Vec<crate::wiring::WiringMark>,
    /// Structure management: defined identifiers, aspects of the pages, work areas.
    #[serde(skip_serializing_if = "crate::structure::Structures::is_default")]
    pub structures: crate::structure::Structures,
    /// Page templates defined for the project.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub page_templates: Vec<crate::page_templates::PageTemplate>,
    /// Layers of graphics, texts, dimensions and images (visibility, locking, print).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<crate::graphics::Layer>,
    /// Approvers of revisions (release workflow).
    #[serde(skip_serializing_if = "crate::revision::Workflow::is_default")]
    pub workflow: crate::revision::Workflow,
    /// Stored states of the whole project (compare, take over).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub baselines: Vec<crate::compare::Baseline>,
    /// Comments, redline / greenline markup, approvals and change requests.
    #[serde(skip_serializing_if = "crate::review::Review::is_empty")]
    pub review: crate::review::Review,
    /// User defined properties, function definitions and placement schemes.
    #[serde(skip_serializing_if = "crate::properties::Library::is_empty")]
    pub property_db: crate::properties::Library,
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
            potentials: Vec::new(),
            wire_defaults: Default::default(),
            wire_colors: false,
            strips: BTreeMap::new(),
            plugs: BTreeMap::new(),
            plc_cards: BTreeMap::new(),
            plc: Default::default(),
            revisions: Vec::new(),
            harnesses: Vec::new(),
            symbols: Vec::new(),
            forms: Default::default(),
            report_blocks: Vec::new(),
            marking: Default::default(),
            configuration: Default::default(),
            fluid: Default::default(),
            pct_loops: Vec::new(),
            wiring_marks: Vec::new(),
            languages: Default::default(),
            structures: Default::default(),
            page_templates: Vec::new(),
            layers: Vec::new(),
            workflow: Default::default(),
            baselines: Vec::new(),
            review: Default::default(),
            property_db: Default::default(),
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
    /// Project property values ([`crate::properties`]) and free project properties,
    /// shown in title blocks as `{prop:Name}`.
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
    /// Overview drawing (single-line, block diagram): symbols and wires are drawn
    /// but not evaluated.
    Overview,
    /// Free graphics (layout sketches, notes): not evaluated.
    Graphic,
    /// An embedded file (PDF, Office document, image).
    Document,
}

impl PageKind {
    pub fn label(self) -> String {
        match self {
            PageKind::Schematic => tr("Schematic (multi-line)").into(),
            PageKind::Cover => tr("Cover sheet").into(),
            PageKind::Report(r) => tr_fmt!("Report: {}", r.title()),
            PageKind::Mounting(_) => tr("Mounting layout").into(),
            PageKind::Overview => tr("Overview (not evaluated)").into(),
            PageKind::Graphic => tr("Graphics").into(),
            PageKind::Document => tr("Document (embedded file)").into(),
        }
    }

    /// Pages drawn by hand (symbols, wires, texts, graphics).
    pub fn is_drawing(self) -> bool {
        matches!(self, PageKind::Schematic | PageKind::Overview | PageKind::Graphic)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ReportKind {
    TableOfContents,
    DeviceList,
    BillOfMaterials,
    TerminalDiagram,
    TerminalLineup,
    TerminalStripOverview,
    TerminalConnections,
    PlugDiagram,
    PinConnections,
    ConnectionList,
    DeviceConnections,
    PlcAssignment,
    PotentialList,
    PanelWiring,
    CableOverview,
    CableDiagram,
    CableConnections,
    CableSizing,
    RevisionOverview,
    HarnessData,
    DuctFill,
    PanelLegend,
    Climate,
    Marking,
    HoseList,
    PctLoops,
    FluidNetlist,
    HoseConfiguration,
    TubingList,
    FluidDeviceGroups,
    ElectroFluid,
    RoutingList,
    BundleList,
}

impl ReportKind {
    pub const ALL: [ReportKind; 33] = [
        ReportKind::TableOfContents,
        ReportKind::DeviceList,
        ReportKind::BillOfMaterials,
        ReportKind::TerminalDiagram,
        ReportKind::TerminalLineup,
        ReportKind::TerminalStripOverview,
        ReportKind::TerminalConnections,
        ReportKind::PlugDiagram,
        ReportKind::PinConnections,
        ReportKind::ConnectionList,
        ReportKind::DeviceConnections,
        ReportKind::PlcAssignment,
        ReportKind::PotentialList,
        ReportKind::PanelWiring,
        ReportKind::CableOverview,
        ReportKind::CableDiagram,
        ReportKind::CableConnections,
        ReportKind::CableSizing,
        ReportKind::RevisionOverview,
        ReportKind::HarnessData,
        ReportKind::DuctFill,
        ReportKind::PanelLegend,
        ReportKind::Climate,
        ReportKind::Marking,
        ReportKind::HoseList,
        ReportKind::PctLoops,
        ReportKind::FluidNetlist,
        ReportKind::HoseConfiguration,
        ReportKind::TubingList,
        ReportKind::FluidDeviceGroups,
        ReportKind::ElectroFluid,
        ReportKind::RoutingList,
        ReportKind::BundleList,
    ];

    pub fn title(self) -> &'static str {
        match self {
            ReportKind::TableOfContents => tr("Table of contents"),
            ReportKind::DeviceList => tr("Device list"),
            ReportKind::BillOfMaterials => tr("Bill of materials"),
            ReportKind::TerminalDiagram => tr("Terminal diagram"),
            ReportKind::TerminalLineup => tr("Terminal line-up"),
            ReportKind::TerminalStripOverview => tr("Terminal strip overview"),
            ReportKind::TerminalConnections => tr("Terminal connection diagram"),
            ReportKind::PlugDiagram => tr("Plug diagram"),
            ReportKind::PinConnections => tr("Pin connection diagram"),
            ReportKind::DeviceConnections => tr("Device connection diagram"),
            ReportKind::ConnectionList => tr("Connection list"),
            ReportKind::PlcAssignment => tr("PLC assignment list"),
            ReportKind::PotentialList => tr("Potential overview"),
            ReportKind::PanelWiring => tr("Wire list (panel routing)"),
            ReportKind::CableOverview => tr("Cable overview"),
            ReportKind::CableDiagram => tr("Cable diagram"),
            ReportKind::CableConnections => tr("Cable connection diagram"),
            ReportKind::PanelLegend => tr("Panel legend"),
            ReportKind::CableSizing => tr("Cable sizing (voltage drop)"),
            ReportKind::RevisionOverview => tr("Revision overview (changes)"),
            ReportKind::HarnessData => tr("Harness data"),
            ReportKind::DuctFill => tr("Duct fill levels"),
            ReportKind::Climate => tr("Enclosure climate"),
            ReportKind::Marking => tr("Marking"),
            ReportKind::HoseList => tr("Hose list"),
            ReportKind::PctLoops => tr("PCT loop overview"),
            ReportKind::FluidNetlist => tr("Fluid netlist"),
            ReportKind::HoseConfiguration => tr("Hose configuration"),
            ReportKind::TubingList => tr("Tubing / piping list"),
            ReportKind::FluidDeviceGroups => tr("Fluid device groups"),
            ReportKind::ElectroFluid => tr("Electro-fluid cross reference"),
            ReportKind::RoutingList => tr("Routing list (3D)"),
            ReportKind::BundleList => tr("Bundle list"),
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
    /// Structure of the page (IEC 81346) without prefix signs: function (`=`),
    /// installation site (`++`), location (`+`) and higher-level product (`-`).
    /// Devices placed on the page inherit it.
    pub function: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub installation: String,
    pub location: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub product: String,
    /// Released page: no changes until it is unlocked.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
    /// Inserted by the configurator: replaced when it generates again.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub generated: bool,
    /// Title block of this page; empty = the project's default.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title_block: String,
    /// Plot frame (paper format, columns, rows); empty = the project's default.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub frame: String,
    /// File of a document page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<crate::document::Document>,
    pub kind: PageKind,
    /// Report pages: what part of the report the page shows (report blocks, pages
    /// output from other projects); none: the first rows of the whole report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<crate::report_blocks::ReportPart>,
    pub symbols: Vec<SymbolInst>,
    pub wires: Vec<Wire>,
    pub texts: Vec<TextItem>,
    pub shapes: Vec<Shape>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dimensions: Vec<Dimension>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<ImageItem>,
    /// Connection definition points: define number, colour, cross-section and
    /// cable core of the connection they sit on.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conn_defs: Vec<ConnDef>,
    /// Cable definition lines: the wires they cross are cores of the cable.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cable_defs: Vec<CableDef>,
    /// Page property values.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: crate::properties::Values,
}

impl Default for Page {
    fn default() -> Self {
        Page::new(0, "1", "", PageKind::Schematic)
    }
}

impl Page {
    /// Its content can be changed: a schematic, overview or graphics page that is
    /// not released. Report, cover and mounting pages are generated.
    pub fn is_editable(&self) -> bool {
        self.kind.is_drawing() && !self.locked
    }

    pub fn new(id: u64, name: &str, description: &str, kind: PageKind) -> Page {
        Page {
            id,
            name: name.into(),
            description: description.into(),
            function: String::new(),
            installation: String::new(),
            location: String::new(),
            product: String::new(),
            locked: false,
            generated: false,
            title_block: String::new(),
            frame: String::new(),
            document: None,
            kind,
            report: None,
            symbols: Vec::new(),
            wires: Vec::new(),
            texts: Vec::new(),
            shapes: Vec::new(),
            dimensions: Vec::new(),
            images: Vec::new(),
            conn_defs: Vec::new(),
            cable_defs: Vec::new(),
            values: BTreeMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
            && self.wires.is_empty()
            && self.texts.is_empty()
            && self.shapes.is_empty()
            && self.dimensions.is_empty()
            && self.images.is_empty()
            && self.conn_defs.is_empty()
            && self.cable_defs.is_empty()
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
    /// Tag block placed by hand: position of its top-left corner relative to `pos`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_offset: Option<P>,
    /// Further properties shown beside the symbol (part number, description …).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<PlacedProp>,
    /// Placement scheme the placed properties were taken from.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub scheme: String,
    /// Function definition; empty = the symbol's built-in one.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub function_def: String,
    /// Function, terminal and PLC connection point property values.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: crate::properties::Values,
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
            tag_offset: None,
            props: Vec::new(),
            scheme: String::new(),
            function_def: String::new(),
            values: BTreeMap::new(),
        }
    }
}

/// Fixed placeable properties of older files (now property keys, see
/// [`PropKind::key`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PropKind {
    #[default]
    Description,
    PartNumber,
    TypeNumber,
    Manufacturer,
    Label,
    Pins,
    Structure,
}

/// A property shown at a position relative to a symbol.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlacedProp {
    /// Property key (with index), see [`crate::properties`].
    pub key: String,
    /// Anchor point of the text relative to the symbol position.
    pub offset: P,
    pub size: f32,
    #[serde(skip_serializing_if = "crate::props::is_left", default = "crate::props::default_anchor")]
    pub anchor: crate::symbols::Anchor,
    /// Shown as `Name: value`.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub name: bool,
    /// Fixed property of older files, converted on loading.
    #[serde(rename = "kind", skip_serializing)]
    pub legacy: Option<PropKind>,
}

impl Default for PlacedProp {
    fn default() -> Self {
        PlacedProp { key: String::new(), offset: p(4.0, 4.0), size: 2.0, anchor: crate::symbols::Anchor::Left, name: false, legacy: None }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Wire {
    pub id: u64,
    pub a: P,
    pub b: P,
    /// Wire colour set manually (e.g. "BK", "BU", "GNYE"); empty = from the
    /// potential definition or the project default.
    pub color: String,
    /// Cross-section set manually (e.g. "1.5 mm²"); empty = automatic. Hoses and
    /// pipes (fluid nets): nominal size, e.g. "8 mm" or "DN25".
    pub cross_section: String,
    /// Medium of a hose / pipe; None = the project's default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<crate::fluid::Medium>,
    /// Length of a hose / pipe in metres; 0 = not set.
    #[serde(skip_serializing_if = "is_zero")]
    pub length: f32,
    /// Hose / tube / pipe configuration (line type, part, fittings, rating).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hose: Option<crate::fluid::HoseConfig>,
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

/// `skip_serializing_if` for numbers at their default.
pub(crate) fn is_zero<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
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
    /// Network / bus cable for this bus system (`None`: ordinary cable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<crate::devdesc::BusKind>,
    /// The cable has an overall shield; `None`: from the cable type and the bus.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shielded: Option<bool>,
    /// Ends at which the shield is connected.
    #[serde(skip_serializing_if = "crate::routing3d::ShieldEnds::is_both")]
    pub shield_ends: crate::routing3d::ShieldEnds,
    /// Outer diameter in mm; 0 = estimated from cores and cross-section.
    #[serde(skip_serializing_if = "is_zero")]
    pub diameter: f32,
    /// Cable property values.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: crate::properties::Values,
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
    /// End brackets and end plates are not placed automatically.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub manual_accessories: bool,
}

/// PLC card (I/O module) data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PlcCard {
    /// First byte of the card's inputs / outputs (I<start>.0, Q<start>.0).
    pub start_byte: u32,
    /// First byte of the outputs when it differs from the inputs' (mixed modules).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_byte: Option<u32>,
    pub rack: String,
    pub slot: String,
    pub description: String,
    /// Station of the hardware configuration the card is plugged into.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub station: String,
    /// Device description (overrides the one from the parts).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub device: String,
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
    #[serde(skip_serializing_if = "String::is_empty")]
    pub layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
}

impl Default for TextItem {
    fn default() -> Self {
        TextItem { id: 0, pos: P::default(), text: String::new(), size: 3.0, layer: String::new(), color: None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ShapeKind {
    #[default]
    Line,
    Rect,
    /// Centre `a`, a point of the circle `b`.
    Circle,
    /// Inside the rectangle `a` – `b`.
    Ellipse,
    /// Centre `a`, start point `b`, `sweep` degrees counter-clockwise.
    Arc,
    /// Open line through `pts`.
    Polyline,
    /// Closed area through `pts`.
    Polygon,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Shape {
    pub id: u64,
    pub kind: ShapeKind,
    pub a: P,
    pub b: P,
    pub dashed: bool,
    /// Corner points of polylines and polygons.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pts: Vec<P>,
    /// Arc angle in degrees.
    #[serde(skip_serializing_if = "is_zero")]
    pub sweep: f32,
    /// Line width in mm; 0 = the normal symbol line.
    #[serde(skip_serializing_if = "is_zero")]
    pub width: f32,
    /// Line (and fill) colour; none = the normal line colour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
    /// Closed shapes are filled.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub fill: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub layer: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DimKind {
    /// Parallel to the measured points.
    #[default]
    Aligned,
    Horizontal,
    Vertical,
}

/// A dimension between the points `a` and `b`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Dimension {
    pub id: u64,
    pub kind: DimKind,
    pub a: P,
    pub b: P,
    /// Distance of the dimension line from the measured points (along the normal
    /// of the measuring direction).
    pub offset: f32,
    /// Shown instead of the measured value when not empty.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    /// Real length per millimetre on the page (drawing scale).
    pub scale: f32,
    pub decimals: u8,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub unit: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub layer: String,
}

impl Default for Dimension {
    fn default() -> Self {
        Dimension {
            id: 0,
            kind: DimKind::Aligned,
            a: P::default(),
            b: P::default(),
            offset: 8.0,
            text: String::new(),
            scale: 1.0,
            decimals: 0,
            unit: String::new(),
            layer: String::new(),
        }
    }
}

/// A bitmap (PNG, JPEG) shown in the rectangle `a` – `b`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ImageItem {
    pub id: u64,
    pub a: P,
    pub b: P,
    pub file: crate::document::Document,
    /// Pixel size of the bitmap (keeps the aspect ratio when resized).
    pub px: [u32; 2],
    #[serde(skip_serializing_if = "String::is_empty")]
    pub layer: String,
}

/// Connection definition point on a wire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ConnDef {
    pub id: u64,
    pub pos: P,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub number: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub color: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cross_section: String,
    /// Cable and core of the wire it sits on.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cable: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub core: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// Cable definition line from `a` to `b`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CableDef {
    pub id: u64,
    pub a: P,
    pub b: P,
    /// Complete tag of the cable.
    pub cable: String,
    /// Draw a shield (ellipse around the crossing).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub shield: bool,
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
    /// Device property values.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: crate::properties::Values,
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
        self.fix_next_id();
        // Make sure each symbol has a designation for each pin.
        for pg in &mut self.pages {
            for s in &mut pg.symbols {
                if let Some(def) = crate::symbols::get(&s.sym) {
                    while s.pins.len() < def.pins.len() {
                        s.pins.push(def.pins[s.pins.len()].label.to_string());
                    }
                    s.pins.truncate(def.pins.len());
                }
                crate::props::migrate(s);
            }
        }
        self.migrate_part_assignments();
        if self.pages.is_empty() {
            let id = self.new_id();
            self.pages.push(Page::new(id, "1", "Schematic", PageKind::Schematic));
        }
    }

    /// Makes `next_id` larger than every id in use.
    pub fn fix_next_id(&mut self) {
        let mut max = 0;
        for pg in &self.pages {
            max = max.max(pg.id);
            max = pg.symbols.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.wires.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.texts.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.shapes.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.dimensions.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.images.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.conn_defs.iter().map(|s| s.id).fold(max, u64::max);
            max = pg.cable_defs.iter().map(|s| s.id).fold(max, u64::max);
        }
        for e in &self.layout.enclosures {
            max = max.max(e.id);
            max = e.items.iter().map(|i| i.id).fold(max, u64::max);
        }
        max = self.cables.iter().map(|c| c.id).fold(max, u64::max);
        max = self.harnesses.iter().map(|h| h.id).fold(max, u64::max);
        self.next_id = self.next_id.max(max + 1);
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
