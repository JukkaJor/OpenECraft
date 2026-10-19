//! Parts management modelled after EPLAN: a global parts database (independent of
//! projects) with product groups, commercial / technical / mounting data and
//! function templates; projects keep copies of the parts they use ("project parts").

use crate::model::{natural_cmp, Project};
use crate::symbols::{self, Func};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TopGroup {
    #[default]
    Electrical,
    Fluid,
    Mechanics,
}

impl TopGroup {
    pub const ALL: [TopGroup; 3] = [TopGroup::Electrical, TopGroup::Fluid, TopGroup::Mechanics];

    pub fn title(self) -> &'static str {
        match self {
            TopGroup::Electrical => "Electrical engineering",
            TopGroup::Fluid => "Fluid power",
            TopGroup::Mechanics => "Mechanics",
        }
    }
}

/// Suggested product groups / subgroups (free text is allowed as well).
pub const PRODUCT_GROUPS: &[(&str, &[&str])] = &[
    ("Relays, contactors", &["Power contactor", "Auxiliary contact block", "Relay", "Timer relay"]),
    ("Safety devices", &["Motor protection switch", "Circuit breaker", "Fuse", "Overload relay", "Auxiliary switch"]),
    ("Sensors, switches, pushbuttons", &["Pushbutton", "Emergency stop", "Selector switch", "Proximity sensor", "Position switch", "Contact block"]),
    ("Signal devices", &["Indicator lamp", "Horn"]),
    ("Motors", &["Three-phase motor", "Single-phase motor"]),
    ("Power supplies, transformers", &["Power supply", "Control transformer"]),
    ("Terminals", &["Feed-through terminal", "PE terminal", "Accessories"]),
    ("PLC", &["Digital input module", "Digital output module", "CPU"]),
    ("Valves", &["Solenoid valve"]),
    ("Passive components", &["Resistor", "Capacitor", "Diode"]),
    ("Mounting", &["Mounting rail", "Cable duct", "Enclosure"]),
];

/// Kinds of functions in a function template. Placed symbols map onto these kinds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FnKind {
    Coil,
    MainContacts3,
    MainContact,
    No,
    Nc,
    Changeover,
    PushbuttonNo,
    PushbuttonNc,
    SwitchNo,
    Sensor,
    Breaker1,
    Breaker3,
    MotorProtection,
    Overload,
    Fuse,
    Motor3,
    Motor1,
    Signal,
    PowerSupply,
    Transformer,
    Terminal,
    PlcInput,
    PlcOutput,
    Valve,
    #[default]
    Generic,
}

impl FnKind {
    pub const ALL: [FnKind; 25] = [
        FnKind::Coil,
        FnKind::MainContacts3,
        FnKind::MainContact,
        FnKind::No,
        FnKind::Nc,
        FnKind::Changeover,
        FnKind::PushbuttonNo,
        FnKind::PushbuttonNc,
        FnKind::SwitchNo,
        FnKind::Sensor,
        FnKind::Breaker1,
        FnKind::Breaker3,
        FnKind::MotorProtection,
        FnKind::Overload,
        FnKind::Fuse,
        FnKind::Motor3,
        FnKind::Motor1,
        FnKind::Signal,
        FnKind::PowerSupply,
        FnKind::Transformer,
        FnKind::Terminal,
        FnKind::PlcInput,
        FnKind::PlcOutput,
        FnKind::Valve,
        FnKind::Generic,
    ];

    pub fn title(self) -> &'static str {
        match self {
            FnKind::Coil => "Coil",
            FnKind::MainContacts3 => "Main contacts 3-pole",
            FnKind::MainContact => "Main contact 1-pole",
            FnKind::No => "NO contact",
            FnKind::Nc => "NC contact",
            FnKind::Changeover => "Changeover contact",
            FnKind::PushbuttonNo => "Pushbutton NO",
            FnKind::PushbuttonNc => "Pushbutton NC",
            FnKind::SwitchNo => "Switch NO",
            FnKind::Sensor => "Sensor",
            FnKind::Breaker1 => "Circuit breaker 1-pole",
            FnKind::Breaker3 => "Circuit breaker 3-pole",
            FnKind::MotorProtection => "Motor protection switch",
            FnKind::Overload => "Overload relay",
            FnKind::Fuse => "Fuse",
            FnKind::Motor3 => "Motor 3-phase",
            FnKind::Motor1 => "Motor 1-phase",
            FnKind::Signal => "Signal device",
            FnKind::PowerSupply => "Power supply",
            FnKind::Transformer => "Transformer",
            FnKind::Terminal => "Terminal",
            FnKind::PlcInput => "PLC input",
            FnKind::PlcOutput => "PLC output",
            FnKind::Valve => "Valve coil",
            FnKind::Generic => "Generic function",
        }
    }

    /// Function kind represented by a library symbol.
    pub fn of_symbol(key: &str) -> Option<FnKind> {
        Some(match key {
            "coil" | "coil_on" | "coil_off" => FnKind::Coil,
            "valve" => FnKind::Valve,
            "no" => FnKind::No,
            "nc" | "ol_nc" => FnKind::Nc,
            "co" => FnKind::Changeover,
            "main3" => FnKind::MainContacts3,
            "main1" => FnKind::MainContact,
            "pb_no" => FnKind::PushbuttonNo,
            "pb_nc" | "estop" => FnKind::PushbuttonNc,
            "sel" => FnKind::SwitchNo,
            "prox" | "limit" => FnKind::Sensor,
            "fuse" => FnKind::Fuse,
            "cb1" => FnKind::Breaker1,
            "cb3" => FnKind::Breaker3,
            "mps3" => FnKind::MotorProtection,
            "ol3" => FnKind::Overload,
            "motor3" => FnKind::Motor3,
            "motor1" => FnKind::Motor1,
            "lamp" | "horn" => FnKind::Signal,
            "psu" => FnKind::PowerSupply,
            "trafo" => FnKind::Transformer,
            "term" | "term_pe" => FnKind::Terminal,
            "plc_in" => FnKind::PlcInput,
            "plc_out" => FnKind::PlcOutput,
            "res" | "cap" | "diode" => FnKind::Generic,
            _ => return None,
        })
    }

    fn short(self) -> &'static str {
        match self {
            FnKind::Coil => "Coil",
            FnKind::MainContacts3 => "Main3",
            FnKind::MainContact => "Main1",
            FnKind::No => "NO",
            FnKind::Nc => "NC",
            FnKind::Changeover => "CO",
            FnKind::PushbuttonNo => "PBNO",
            FnKind::PushbuttonNc => "PBNC",
            FnKind::SwitchNo => "SWNO",
            FnKind::Sensor => "Sensor",
            FnKind::Breaker1 => "CB1",
            FnKind::Breaker3 => "CB3",
            FnKind::MotorProtection => "MPS",
            FnKind::Overload => "OL",
            FnKind::Fuse => "Fuse",
            FnKind::Motor3 => "M3",
            FnKind::Motor1 => "M1",
            FnKind::Signal => "Signal",
            FnKind::PowerSupply => "PSU",
            FnKind::Transformer => "Trafo",
            FnKind::Terminal => "Terminal",
            FnKind::PlcInput => "DI",
            FnKind::PlcOutput => "DO",
            FnKind::Valve => "Valve",
            FnKind::Generic => "Generic",
        }
    }

    fn from_short(s: &str) -> Option<FnKind> {
        FnKind::ALL.into_iter().find(|k| k.short().eq_ignore_ascii_case(s.trim()))
    }
}

/// One function of a function template.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FunctionTemplate {
    pub kind: FnKind,
    /// Connection point designations in symbol order, separated by blanks ("13 14").
    pub pins: String,
    pub description: String,
}

impl FunctionTemplate {
    pub fn new(kind: FnKind, pins: &str) -> FunctionTemplate {
        FunctionTemplate { kind, pins: pins.into(), description: String::new() }
    }

    pub fn pin_list(&self) -> Vec<String> {
        self.pins.split_whitespace().map(String::from).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Part {
    // General
    /// Unique key, conventionally `MANUFACTURER.TYPE`.
    pub number: String,
    pub type_number: String,
    pub order_number: String,
    pub manufacturer: String,
    pub supplier: String,
    pub description: String,
    pub note: String,
    pub top_group: TopGroup,
    pub group: String,
    pub subgroup: String,
    /// Identifying letter of devices using this part (device selection).
    pub identifier: String,
    // Commercial
    pub price: f32,
    pub currency: String,
    /// The price refers to this many units.
    pub price_unit: u32,
    pub pack_quantity: u32,
    pub quantity_unit: String,
    // Technical
    pub voltage: String,
    pub current: String,
    pub power: String,
    pub tech_data: String,
    pub certificates: String,
    pub datasheet: String,
    // Mounting
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub weight: f32,
    pub mounting: String,
    // Logic
    pub functions: Vec<FunctionTemplate>,
    /// Part numbers of accessories.
    pub accessories: Vec<String>,
}

impl Default for Part {
    fn default() -> Self {
        Part {
            number: String::new(),
            type_number: String::new(),
            order_number: String::new(),
            manufacturer: String::new(),
            supplier: String::new(),
            description: String::new(),
            note: String::new(),
            top_group: TopGroup::Electrical,
            group: String::new(),
            subgroup: String::new(),
            identifier: String::new(),
            price: 0.0,
            currency: "EUR".into(),
            price_unit: 1,
            pack_quantity: 1,
            quantity_unit: "pcs".into(),
            voltage: String::new(),
            current: String::new(),
            power: String::new(),
            tech_data: String::new(),
            certificates: String::new(),
            datasheet: String::new(),
            width: 0.0,
            height: 0.0,
            depth: 0.0,
            weight: 0.0,
            mounting: String::new(),
            functions: Vec::new(),
            accessories: Vec::new(),
        }
    }
}

impl Part {
    /// Price of `qty` units, respecting the price unit.
    pub fn total_price(&self, qty: f32) -> f32 {
        self.price / self.price_unit.max(1) as f32 * qty
    }

    pub fn matches_text(&self, f: &str) -> bool {
        if f.is_empty() {
            return true;
        }
        let f = f.to_lowercase();
        [&self.number, &self.type_number, &self.order_number, &self.manufacturer, &self.description, &self.group, &self.subgroup]
            .iter()
            .any(|s| s.to_lowercase().contains(&f))
    }

    /// Short summary of the function template, e.g. "Coil, Main3, NO".
    pub fn template_summary(&self) -> String {
        let mut counts: BTreeMap<FnKind, usize> = BTreeMap::new();
        for f in &self.functions {
            *counts.entry(f.kind).or_default() += 1;
        }
        counts
            .iter()
            .map(|(k, n)| if *n > 1 { format!("{n}× {}", k.title()) } else { k.title().to_string() })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Reference from a device to a part.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PartRef {
    pub number: String,
    pub count: u32,
}

impl Default for PartRef {
    fn default() -> Self {
        PartRef { number: String::new(), count: 1 }
    }
}

/// The global parts database.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PartsDb {
    pub parts: Vec<Part>,
}

impl Default for PartsDb {
    fn default() -> Self {
        PartsDb { parts: default_catalog() }
    }
}

impl PartsDb {
    pub fn get(&self, number: &str) -> Option<&Part> {
        self.parts.iter().find(|p| p.number == number)
    }

    /// Inserts or replaces a part (by part number).
    pub fn upsert(&mut self, part: Part) {
        upsert(&mut self.parts, part);
    }

    pub fn sort(&mut self) {
        self.parts.sort_by(|a, b| natural_cmp(&a.number, &b.number));
    }
}

pub fn upsert(list: &mut Vec<Part>, part: Part) {
    match list.iter_mut().find(|p| p.number == part.number) {
        Some(existing) => *existing = part,
        None => list.push(part),
    }
}

/// A part number that does not exist yet, based on `base`.
pub fn unique_number(list: &[Part], base: &str) -> String {
    if !list.iter().any(|p| p.number == base) {
        return base.to_string();
    }
    (1..)
        .map(|i| format!("{base}_{i}"))
        .find(|n| !list.iter().any(|p| &p.number == n))
        .unwrap()
}

// ---------------------------------------------------------------------- project

/// Copies a part from the database into the project parts (if not there yet).
pub fn ensure_in_project(prj: &mut Project, db: &PartsDb, number: &str) {
    if prj.parts.iter().any(|p| p.number == number) {
        return;
    }
    if let Some(p) = db.get(number) {
        prj.parts.push(p.clone());
        // Accessories come along so the BOM can resolve them later.
        for acc in p.accessories.clone() {
            if !prj.parts.iter().any(|x| x.number == acc) {
                if let Some(a) = db.get(&acc) {
                    prj.parts.push(a.clone());
                }
            }
        }
    }
}

/// Resolves a part: project parts first, then the database.
pub fn lookup<'a>(prj: &'a Project, db: Option<&'a PartsDb>, number: &str) -> Option<&'a Part> {
    prj.parts.iter().find(|p| p.number == number).or_else(|| db.and_then(|d| d.get(number)))
}

/// Updates all project parts from the database. Returns the number of updated parts.
pub fn sync_project_from_db(prj: &mut Project, db: &PartsDb) -> usize {
    let mut n = 0;
    // Make sure every referenced part is present.
    let refs: Vec<String> =
        prj.devices.values().flat_map(|d| d.parts.iter().map(|r| r.number.clone())).collect();
    for r in refs {
        ensure_in_project(prj, db, &r);
    }
    for p in prj.parts.iter_mut() {
        if let Some(src) = db.get(&p.number) {
            if p != src {
                *p = src.clone();
                n += 1;
            }
        }
    }
    n
}

/// Adds project parts that are unknown in the database. Returns the number added.
pub fn transfer_project_to_db(prj: &Project, db: &mut PartsDb) -> usize {
    let mut n = 0;
    for p in &prj.parts {
        if db.get(&p.number).is_none() {
            db.parts.push(p.clone());
            n += 1;
        }
    }
    n
}

/// Removes project parts that no device uses any more.
pub fn purge_unused(prj: &mut Project) -> usize {
    let mut used: Vec<String> = prj.devices.values().flat_map(|d| d.parts.iter().map(|r| r.number.clone())).collect();
    let acc: Vec<String> = prj
        .parts
        .iter()
        .filter(|p| used.contains(&p.number))
        .flat_map(|p| p.accessories.clone())
        .collect();
    used.extend(acc);
    let before = prj.parts.len();
    prj.parts.retain(|p| used.contains(&p.number));
    before - prj.parts.len()
}

// ---------------------------------------------------------------------- devices

/// Functions placed for a device: (kind, connection designations, symbol id).
pub fn placed_functions(prj: &Project, tag: &str) -> Vec<(FnKind, Vec<String>, u64)> {
    let mut v = Vec::new();
    for pg in &prj.pages {
        for s in pg.symbols.iter().filter(|s| s.tag == tag) {
            if let Some(k) = FnKind::of_symbol(&s.sym) {
                v.push((k, s.pins.clone(), s.id));
            }
        }
    }
    v
}

/// Functions available from the parts of a device (templates × count).
pub fn available_functions(prj: &Project, db: Option<&PartsDb>, tag: &str) -> Vec<FunctionTemplate> {
    let Some(dev) = prj.devices.get(tag) else { return Vec::new() };
    let mut v = Vec::new();
    for r in &dev.parts {
        if let Some(p) = lookup(prj, db, &r.number) {
            for _ in 0..r.count.max(1) {
                v.extend(p.functions.iter().cloned());
            }
        }
    }
    v
}

/// Template functions not used by any placed function (shown as reserve contacts).
pub fn reserve_functions(prj: &Project, db: Option<&PartsDb>, tag: &str) -> Vec<FunctionTemplate> {
    let mut avail = available_functions(prj, db, tag);
    let placed = placed_functions(prj, tag);
    // First match by kind and designations, then by kind only.
    let mut rest = Vec::new();
    for (k, pins, _) in &placed {
        if let Some(i) = avail.iter().position(|f| f.kind == *k && &f.pin_list() == pins) {
            avail.remove(i);
        } else {
            rest.push(*k);
        }
    }
    for k in rest {
        if let Some(i) = avail.iter().position(|f| f.kind == k) {
            avail.remove(i);
        }
    }
    avail
}

/// Per function kind: (placed, available) for devices that have parts assigned.
pub fn usage(prj: &Project, db: Option<&PartsDb>, tag: &str) -> BTreeMap<FnKind, (usize, usize)> {
    let mut m: BTreeMap<FnKind, (usize, usize)> = BTreeMap::new();
    for (k, _, _) in placed_functions(prj, tag) {
        m.entry(k).or_default().0 += 1;
    }
    for f in available_functions(prj, db, tag) {
        m.entry(f.kind).or_default().1 += 1;
    }
    m
}

/// Assigns connection designations from the function templates to the placed
/// functions of a device (in page / position order). Returns how many changed.
pub fn apply_designations(prj: &mut Project, db: Option<&PartsDb>, tag: &str) -> usize {
    let mut avail = available_functions(prj, db, tag);
    let mut placed: Vec<(usize, usize)> = Vec::new();
    for (pi, pg) in prj.pages.iter().enumerate() {
        for (si, s) in pg.symbols.iter().enumerate() {
            if s.tag == tag {
                placed.push((pi, si));
            }
        }
    }
    placed.sort_by(|a, b| {
        let (sa, sb) = (&prj.pages[a.0].symbols[a.1], &prj.pages[b.0].symbols[b.1]);
        a.0.cmp(&b.0).then(sa.pos.x.total_cmp(&sb.pos.x)).then(sa.pos.y.total_cmp(&sb.pos.y))
    });
    let mut changed = 0;
    for (pi, si) in placed {
        let s = &mut prj.pages[pi].symbols[si];
        let Some(kind) = FnKind::of_symbol(&s.sym) else { continue };
        let n_pins = symbols::get(&s.sym).map(|d| d.pins.len()).unwrap_or(0);
        if let Some(i) = avail.iter().position(|f| f.kind == kind && f.pin_list().len() == n_pins) {
            let pins = avail.remove(i).pin_list();
            if s.pins != pins {
                s.pins = pins;
                changed += 1;
            }
        }
    }
    changed
}

/// Device selection: does the part fit a device with these placed functions?
/// The part must provide the main function (or, without one, any placed function).
pub fn fits_device(part: &Part, placed: &[(FnKind, Vec<String>, u64)], main: Option<FnKind>) -> bool {
    if part.functions.is_empty() {
        return false;
    }
    match main {
        Some(m) => part.functions.iter().any(|f| f.kind == m),
        None => placed.iter().any(|(k, _, _)| part.functions.iter().any(|f| f.kind == *k)),
    }
}

/// Main function kind of a device, if one is placed.
pub fn main_kind(prj: &Project, tag: &str) -> Option<FnKind> {
    prj.pages
        .iter()
        .flat_map(|pg| pg.symbols.iter())
        .filter(|s| s.tag == tag)
        .find(|s| symbols::get(&s.sym).map(|d| matches!(d.func, Func::Main { .. })).unwrap_or(false))
        .and_then(|s| FnKind::of_symbol(&s.sym))
}

// ---------------------------------------------------------------------- CSV

const CSV_FIELDS: [&str; 29] = [
    "Part number",
    "Type number",
    "Order number",
    "Manufacturer",
    "Supplier",
    "Description",
    "Note",
    "Product top group",
    "Product group",
    "Product subgroup",
    "Identifier",
    "Price",
    "Currency",
    "Price unit",
    "Pack quantity",
    "Quantity unit",
    "Voltage",
    "Current",
    "Power",
    "Technical data",
    "Certificates",
    "Datasheet",
    "Width",
    "Height",
    "Depth",
    "Weight",
    "Mounting",
    "Functions",
    "Accessories",
];

fn csv_escape(s: &str) -> String {
    if s.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn to_csv(parts: &[Part]) -> String {
    let mut out = CSV_FIELDS.join(";");
    out.push('\n');
    for p in parts {
        let functions = p
            .functions
            .iter()
            .map(|f| format!("{}:{}", f.kind.short(), f.pins))
            .collect::<Vec<_>>()
            .join("|");
        let top = match p.top_group {
            TopGroup::Electrical => "Electrical",
            TopGroup::Fluid => "Fluid",
            TopGroup::Mechanics => "Mechanics",
        };
        let row = [
            p.number.clone(),
            p.type_number.clone(),
            p.order_number.clone(),
            p.manufacturer.clone(),
            p.supplier.clone(),
            p.description.clone(),
            p.note.clone(),
            top.to_string(),
            p.group.clone(),
            p.subgroup.clone(),
            p.identifier.clone(),
            format!("{:.2}", p.price),
            p.currency.clone(),
            p.price_unit.to_string(),
            p.pack_quantity.to_string(),
            p.quantity_unit.clone(),
            p.voltage.clone(),
            p.current.clone(),
            p.power.clone(),
            p.tech_data.clone(),
            p.certificates.clone(),
            p.datasheet.clone(),
            p.width.to_string(),
            p.height.to_string(),
            p.depth.to_string(),
            p.weight.to_string(),
            p.mounting.clone(),
            functions,
            p.accessories.join(","),
        ];
        out.push_str(&row.iter().map(|c| csv_escape(c)).collect::<Vec<_>>().join(";"));
        out.push('\n');
    }
    out
}

/// Splits CSV text (separator `;` or `,`) into records, honouring quotes.
fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let sep = if text.lines().next().map(|l| l.contains(';')).unwrap_or(true) { ';' } else { ',' };
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if c == sep && !quoted => row.push(std::mem::take(&mut cell)),
            '\r' if !quoted => {}
            '\n' if !quoted => {
                row.push(std::mem::take(&mut cell));
                if row.iter().any(|c| !c.is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            c => cell.push(c),
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows
}

/// Reads parts from CSV. Columns are matched by header name; unknown columns are ignored.
pub fn from_csv(text: &str) -> Result<Vec<Part>, String> {
    let rows = parse_csv(text.trim_start_matches('\u{feff}'));
    let Some((header, data)) = rows.split_first() else { return Err("empty file".into()) };
    let col = |name: &str| header.iter().position(|h| h.trim().eq_ignore_ascii_case(name));
    if col("Part number").is_none() {
        return Err("column 'Part number' missing".into());
    }
    let idx: Vec<Option<usize>> = CSV_FIELDS.iter().map(|f| col(f)).collect();
    let mut parts = Vec::new();
    for r in data {
        let get = |i: usize| idx[i].and_then(|c| r.get(c)).map(|s| s.trim().to_string()).unwrap_or_default();
        let num = |i: usize| get(i).replace(',', ".").parse::<f32>().unwrap_or(0.0);
        let int = |i: usize, d: u32| get(i).parse::<u32>().unwrap_or(d);
        let number = get(0);
        if number.is_empty() {
            continue;
        }
        let top_group = match get(7).to_lowercase().as_str() {
            s if s.starts_with("fluid") => TopGroup::Fluid,
            s if s.starts_with("mech") => TopGroup::Mechanics,
            _ => TopGroup::Electrical,
        };
        let functions = get(27)
            .split('|')
            .filter(|s| !s.trim().is_empty())
            .filter_map(|s| {
                let (k, pins) = s.split_once(':').unwrap_or((s, ""));
                FnKind::from_short(k).map(|k| FunctionTemplate::new(k, pins.trim()))
            })
            .collect();
        parts.push(Part {
            number,
            type_number: get(1),
            order_number: get(2),
            manufacturer: get(3),
            supplier: get(4),
            description: get(5),
            note: get(6),
            top_group,
            group: get(8),
            subgroup: get(9),
            identifier: get(10),
            price: num(11),
            currency: if get(12).is_empty() { "EUR".into() } else { get(12) },
            price_unit: int(13, 1).max(1),
            pack_quantity: int(14, 1).max(1),
            quantity_unit: if get(15).is_empty() { "pcs".into() } else { get(15) },
            voltage: get(16),
            current: get(17),
            power: get(18),
            tech_data: get(19),
            certificates: get(20),
            datasheet: get(21),
            width: num(22),
            height: num(23),
            depth: num(24),
            weight: num(25),
            mounting: get(26),
            functions,
            accessories: get(28).split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
        });
    }
    Ok(parts)
}

/// Imports a parts file (JSON database / part list or CSV). Returns the parts.
pub fn import(text: &str) -> Result<Vec<Part>, String> {
    let t = text.trim_start_matches('\u{feff}').trim_start();
    if t.starts_with('{') {
        serde_json::from_str::<PartsDb>(t).map(|d| d.parts).map_err(|e| e.to_string())
    } else if t.starts_with('[') {
        serde_json::from_str::<Vec<Part>>(t).map_err(|e| e.to_string())
    } else {
        from_csv(t)
    }
}

// ---------------------------------------------------------------------- catalog

struct C {
    parts: Vec<Part>,
}

#[allow(clippy::too_many_arguments)]
impl C {
    fn add(
        &mut self,
        number: &str,
        desc: &str,
        group: &str,
        subgroup: &str,
        ident: &str,
        price: f32,
        dims: (f32, f32, f32, f32),
        functions: &[(FnKind, &str)],
    ) -> &mut Part {
        let (top, rest) = number.split_once('.').unwrap_or(("GEN", number));
        let manufacturer = if top == "GEN" { "Generic" } else { top };
        self.parts.push(Part {
            number: number.into(),
            type_number: rest.into(),
            order_number: rest.into(),
            manufacturer: manufacturer.into(),
            supplier: "Generic distribution".into(),
            description: desc.into(),
            top_group: if group == "Mounting" { TopGroup::Mechanics } else if group == "Valves" { TopGroup::Fluid } else { TopGroup::Electrical },
            group: group.into(),
            subgroup: subgroup.into(),
            identifier: ident.into(),
            price,
            width: dims.0,
            height: dims.1,
            depth: dims.2,
            weight: dims.3,
            mounting: if dims.0 > 0.0 { "Mounting rail 35 mm".into() } else { String::new() },
            functions: functions.iter().map(|(k, p)| FunctionTemplate::new(*k, p)).collect(),
            ..Default::default()
        });
        self.parts.last_mut().unwrap()
    }
}

/// Generic example catalog with function templates.
pub fn default_catalog() -> Vec<Part> {
    use FnKind::*;
    let mut c = C { parts: Vec::new() };
    let rc = "Relays, contactors";
    let sd = "Safety devices";
    let sw = "Sensors, switches, pushbuttons";
    let main3 = "1 3 5 2 4 6";

    let p = c.add("GEN.CT-9A-24DC", "Contactor 3P 9 A, coil 24 V DC, 1 NO aux", rc, "Power contactor", "K", 38.5, (45.0, 74.0, 73.0, 0.32), &[(Coil, "A1 A2"), (MainContacts3, main3), (No, "13 14")]);
    p.voltage = "24 V DC".into();
    p.current = "9 A (AC-3)".into();
    p.power = "4 kW / 400 V".into();
    p.accessories = vec!["GEN.AUX-2NO2NC".into()];
    let p = c.add("GEN.CT-18A-24DC", "Contactor 3P 18 A, coil 24 V DC, 1 NO + 1 NC aux", rc, "Power contactor", "K", 52.0, (45.0, 84.0, 87.0, 0.45), &[(Coil, "A1 A2"), (MainContacts3, main3), (No, "13 14"), (Nc, "21 22")]);
    p.voltage = "24 V DC".into();
    p.current = "18 A (AC-3)".into();
    p.power = "7.5 kW / 400 V".into();
    p.accessories = vec!["GEN.AUX-2NO2NC".into()];
    c.add("GEN.AUX-2NO2NC", "Auxiliary contact block 2 NO + 2 NC, front mounting", rc, "Auxiliary contact block", "K", 14.6, (45.0, 40.0, 40.0, 0.05), &[(No, "53 54"), (Nc, "61 62"), (Nc, "71 72"), (No, "83 84")]);
    c.add("GEN.AUX-1NO1NC", "Auxiliary contact block 1 NO + 1 NC", rc, "Auxiliary contact block", "K", 9.8, (45.0, 40.0, 40.0, 0.03), &[(No, "53 54"), (Nc, "61 62")]);
    let p = c.add("GEN.RLY-24DC-2CO", "Interface relay 24 V DC, 2 CO, with socket", rc, "Relay", "K", 12.4, (15.5, 78.0, 70.0, 0.04), &[(Coil, "A1 A2"), (Changeover, "11 12 14"), (Changeover, "21 22 24")]);
    p.voltage = "24 V DC".into();
    p.current = "8 A".into();
    let p = c.add("GEN.TMR-24DC-OND", "Timer relay on-delay 24 V DC, 0.05 s - 100 h, 1 CO", rc, "Timer relay", "K", 34.9, (17.5, 90.0, 64.0, 0.07), &[(Coil, "A1 A2"), (Changeover, "15 16 18")]);
    p.voltage = "24 V DC".into();

    let p = c.add("GEN.MPS-2.5-4A", "Motor protection switch 2.5 - 4 A", sd, "Motor protection switch", "Q", 61.0, (45.0, 97.0, 97.0, 0.3), &[(MotorProtection, main3)]);
    p.current = "2.5 - 4 A".into();
    p.accessories = vec!["GEN.MPS-AUX-1NO1NC".into()];
    let p = c.add("GEN.MPS-4-6.3A", "Motor protection switch 4 - 6.3 A", sd, "Motor protection switch", "Q", 63.0, (45.0, 97.0, 97.0, 0.3), &[(MotorProtection, main3)]);
    p.current = "4 - 6.3 A".into();
    p.accessories = vec!["GEN.MPS-AUX-1NO1NC".into()];
    c.add("GEN.MPS-AUX-1NO1NC", "Auxiliary switch for motor protection switch, 1 NO + 1 NC", sd, "Auxiliary switch", "Q", 12.9, (9.0, 90.0, 60.0, 0.02), &[(No, "13 14"), (Nc, "21 22")]);
    let p = c.add("GEN.MCB-1P-C6", "Miniature circuit breaker 1P C6", sd, "Circuit breaker", "F", 8.7, (18.0, 90.0, 70.0, 0.12), &[(Breaker1, "1 2")]);
    p.current = "6 A".into();
    let p = c.add("GEN.MCB-1P-B10", "Miniature circuit breaker 1P B10", sd, "Circuit breaker", "F", 8.7, (18.0, 90.0, 70.0, 0.12), &[(Breaker1, "1 2")]);
    p.current = "10 A".into();
    let p = c.add("GEN.MCB-3P-C16", "Miniature circuit breaker 3P C16", sd, "Circuit breaker", "Q", 27.3, (54.0, 90.0, 70.0, 0.36), &[(Breaker3, main3)]);
    p.current = "16 A".into();
    let p = c.add("GEN.FUSE-5X20-2A", "Fuse holder 5x20 mm with fuse 2 A", sd, "Fuse", "F", 4.2, (8.0, 60.0, 50.0, 0.02), &[(Fuse, "1 2")]);
    p.current = "2 A".into();
    let p = c.add("GEN.OL-2.8-4A", "Thermal overload relay 2.8 - 4 A, 1 NO + 1 NC", sd, "Overload relay", "F", 44.0, (45.0, 70.0, 80.0, 0.15), &[(Overload, main3), (Nc, "95 96"), (No, "97 98")]);
    p.current = "2.8 - 4 A".into();

    c.add("GEN.PB-GN-1NO", "Pushbutton green, 1 NO", sw, "Pushbutton", "S", 11.2, (30.0, 30.0, 45.0, 0.03), &[(PushbuttonNo, "13 14")]).accessories = vec!["GEN.CB-1NO".into(), "GEN.CB-1NC".into()];
    c.add("GEN.PB-RD-1NC", "Pushbutton red, 1 NC", sw, "Pushbutton", "S", 11.2, (30.0, 30.0, 45.0, 0.03), &[(PushbuttonNc, "11 12")]).accessories = vec!["GEN.CB-1NO".into(), "GEN.CB-1NC".into()];
    c.add("GEN.ESTOP-1NC", "Emergency stop mushroom 40 mm, twist release, 1 NC", sw, "Emergency stop", "S", 24.6, (40.0, 40.0, 60.0, 0.06), &[(PushbuttonNc, "11 12")]).accessories = vec!["GEN.CB-1NC".into()];
    c.add("GEN.SEL-2POS-1NO", "Selector switch 2 positions, 1 NO", sw, "Selector switch", "S", 15.9, (30.0, 30.0, 45.0, 0.03), &[(SwitchNo, "13 14")]);
    c.add("GEN.CB-1NO", "Contact block 1 NO for front mounting", sw, "Contact block", "S", 4.9, (10.0, 30.0, 40.0, 0.01), &[(No, "13 14")]);
    c.add("GEN.CB-1NC", "Contact block 1 NC for front mounting", sw, "Contact block", "S", 4.9, (10.0, 30.0, 40.0, 0.01), &[(Nc, "21 22")]);
    let p = c.add("GEN.PROX-M12-PNP", "Inductive proximity sensor M12, PNP NO, Sn 4 mm", sw, "Proximity sensor", "B", 29.0, (12.0, 12.0, 60.0, 0.05), &[(Sensor, "BN BK")]);
    p.voltage = "10 - 30 V DC".into();
    c.add("GEN.LIMIT-1NO1NC", "Position switch roller plunger, 1 NO + 1 NC", sw, "Position switch", "B", 32.0, (31.0, 60.0, 33.0, 0.1), &[(Sensor, "13 14"), (Nc, "21 22")]);

    let p = c.add("GEN.LMP-GN-24", "Indicator lamp green, LED 24 V", "Signal devices", "Indicator lamp", "P", 9.1, (30.0, 30.0, 45.0, 0.02), &[(Signal, "X1 X2")]);
    p.voltage = "24 V AC/DC".into();
    let p = c.add("GEN.LMP-RD-24", "Indicator lamp red, LED 24 V", "Signal devices", "Indicator lamp", "P", 9.1, (30.0, 30.0, 45.0, 0.02), &[(Signal, "X1 X2")]);
    p.voltage = "24 V AC/DC".into();
    c.add("GEN.HORN-24", "Panel buzzer 24 V, 85 dB", "Signal devices", "Horn", "P", 18.0, (30.0, 30.0, 50.0, 0.03), &[(Signal, "1 2")]);

    let p = c.add("GEN.MOT-0.75KW", "Three-phase motor 0.75 kW 400 V, 1400 rpm", "Motors", "Three-phase motor", "M", 180.0, (0.0, 0.0, 0.0, 9.5), &[(Motor3, "U1 V1 W1 PE")]);
    p.voltage = "400 V".into();
    p.current = "1.9 A".into();
    p.power = "0.75 kW".into();
    let p = c.add("GEN.MOT-1PH-0.25KW", "Single-phase motor 0.25 kW 230 V", "Motors", "Single-phase motor", "M", 140.0, (0.0, 0.0, 0.0, 5.0), &[(Motor1, "L N PE")]);
    p.power = "0.25 kW".into();

    let p = c.add("GEN.PS-24DC-5A", "Power supply 230 V AC / 24 V DC 5 A", "Power supplies, transformers", "Power supply", "T", 89.0, (40.0, 125.0, 115.0, 0.6), &[(PowerSupply, "L N PE L+ M")]);
    p.voltage = "230 V AC / 24 V DC".into();
    p.current = "5 A".into();
    c.add("GEN.TR-230-24-100VA", "Control transformer 230 V / 24 V 100 VA", "Power supplies, transformers", "Control transformer", "T", 46.0, (78.0, 84.0, 70.0, 1.6), &[(Transformer, "1.1 1.2 2.1 2.2")]);

    let p = c.add("GEN.TB-2.5", "Feed-through terminal 2.5 mm², grey", "Terminals", "Feed-through terminal", "X", 0.95, (5.2, 48.0, 36.0, 0.006), &[(Terminal, "")]);
    p.accessories = vec!["GEN.TB-END".into()];
    c.add("GEN.TB-PE-2.5", "Protective earth terminal 2.5 mm², green-yellow", "Terminals", "PE terminal", "X", 1.8, (5.2, 48.0, 36.0, 0.01), &[(Terminal, "")]);
    c.add("GEN.TB-END", "End plate for terminal 2.5 mm²", "Terminals", "Accessories", "X", 0.3, (1.5, 48.0, 36.0, 0.002), &[]);

    let di: Vec<(FnKind, String)> = (0..16).map(|i| (PlcInput, format!("I{}.{}", i / 8, i % 8))).collect();
    let di: Vec<(FnKind, &str)> = di.iter().map(|(k, s)| (*k, s.as_str())).collect();
    c.add("GEN.PLC-DI16", "PLC digital input module 16 x 24 V DC", "PLC", "Digital input module", "A", 145.0, (25.0, 100.0, 75.0, 0.15), &di);
    let dq: Vec<(FnKind, String)> = (0..16).map(|i| (PlcOutput, format!("Q{}.{}", i / 8, i % 8))).collect();
    let dq: Vec<(FnKind, &str)> = dq.iter().map(|(k, s)| (*k, s.as_str())).collect();
    c.add("GEN.PLC-DO16", "PLC digital output module 16 x 24 V DC 0.5 A", "PLC", "Digital output module", "A", 165.0, (25.0, 100.0, 75.0, 0.15), &dq);

    let p = c.add("GEN.SV-24DC", "Solenoid valve coil 24 V DC, 4.8 W", "Valves", "Solenoid valve", "Y", 22.5, (0.0, 0.0, 0.0, 0.1), &[(Valve, "A1 A2")]);
    p.voltage = "24 V DC".into();
    p.power = "4.8 W".into();

    c.add("GEN.RES-1K", "Resistor 1 kΩ 0.6 W", "Passive components", "Resistor", "R", 0.1, (0.0, 0.0, 0.0, 0.0), &[(Generic, "1 2")]);
    c.add("GEN.CAP-100N", "Capacitor 100 nF 63 V", "Passive components", "Capacitor", "C", 0.15, (0.0, 0.0, 0.0, 0.0), &[(Generic, "1 2")]);
    c.add("GEN.DIODE-1N4007", "Rectifier diode 1 A 1000 V", "Passive components", "Diode", "V", 0.08, (0.0, 0.0, 0.0, 0.0), &[(Generic, "A K")]);

    let p = c.add("GEN.RAIL-TS35-1M", "Mounting rail TS 35 x 7.5, perforated, 1 m", "Mounting", "Mounting rail", "", 6.5, (1000.0, 35.0, 7.5, 0.35), &[]);
    p.quantity_unit = "m".into();
    p.mounting.clear();
    let p = c.add("GEN.DUCT-40X60", "Cable duct 40 x 60 mm, 2 m", "Mounting", "Cable duct", "", 11.0, (2000.0, 40.0, 60.0, 0.9), &[]);
    p.pack_quantity = 10;
    p.mounting.clear();
    let p = c.add("GEN.ENC-600X800", "Steel enclosure 600 x 800 x 250 mm, IP66", "Mounting", "Enclosure", "", 320.0, (600.0, 800.0, 250.0, 28.0), &[]);
    p.mounting.clear();

    let mut parts = c.parts;
    parts.sort_by(|a, b| natural_cmp(&a.number, &b.number));
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_roundtrip() {
        let parts = default_catalog();
        let csv = to_csv(&parts);
        let back = from_csv(&csv).unwrap();
        assert_eq!(parts.len(), back.len());
        for (a, b) in parts.iter().zip(&back) {
            assert_eq!(a.number, b.number);
            assert_eq!(a.functions, b.functions);
            assert_eq!(a.accessories, b.accessories);
            assert_eq!(a.description, b.description);
        }
    }

    #[test]
    fn catalog_numbers_unique_and_accessories_exist() {
        let parts = default_catalog();
        for (i, p) in parts.iter().enumerate() {
            assert!(parts[i + 1..].iter().all(|q| q.number != p.number), "duplicate {}", p.number);
            for a in &p.accessories {
                assert!(parts.iter().any(|q| &q.number == a), "{} accessory {a} missing", p.number);
            }
        }
    }

    #[test]
    fn legacy_single_part_is_migrated() {
        let json = r#"{"devices": {"-K1": {"part": "CT-9A-24DC", "description": ""}},
                      "parts": [{"number": "CT-9A-24DC", "manufacturer": "Generic", "description": "old", "price": 1.0}]}"#;
        let mut prj: Project = serde_json::from_str(json).unwrap();
        prj.normalize();
        assert_eq!(prj.devices["-K1"].parts, vec![PartRef { number: "GEN.CT-9A-24DC".into(), count: 1 }]);
        assert!(prj.parts.iter().any(|p| p.number == "GEN.CT-9A-24DC" && !p.functions.is_empty()));
        assert!(!prj.parts.iter().any(|p| p.number == "CT-9A-24DC"));
    }

    #[test]
    fn designations_from_template_and_device_selection() {
        let mut prj = crate::sample::demo_project();
        // Scramble the contact numbers of -K1, then take them from the parts again.
        for pg in &mut prj.pages {
            for s in pg.symbols.iter_mut().filter(|s| s.tag == "-K1" && s.sym == "no") {
                s.pins = vec!["x".into(), "y".into()];
            }
        }
        assert_eq!(apply_designations(&mut prj, None, "-K1"), 3);
        let mut pins: Vec<String> = placed_functions(&prj, "-K1")
            .into_iter()
            .filter(|f| f.0 == FnKind::No)
            .map(|f| f.1.join("/"))
            .collect();
        pins.sort();
        assert_eq!(pins, vec!["13/14", "53/54", "83/84"]);

        let db = PartsDb::default();
        let placed = placed_functions(&prj, "-K1");
        let main = main_kind(&prj, "-K1");
        assert_eq!(main, Some(FnKind::Coil));
        assert!(fits_device(db.get("GEN.CT-9A-24DC").unwrap(), &placed, main));
        assert!(!fits_device(db.get("GEN.MCB-1P-C6").unwrap(), &placed, main));
    }

    #[test]
    fn json_import() {
        let db = PartsDb::default();
        let json = serde_json::to_string(&db).unwrap();
        assert_eq!(import(&json).unwrap().len(), db.parts.len());
    }
}
