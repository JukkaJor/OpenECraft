//! Part data: product groups, function templates, commercial / technical / mounting
//! data and the parts database container.

use crate::{natural_cmp, Mesh3d, Model2d};
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
    ("Mounting", &["Mounting rail", "Cable duct", "Enclosure", "Busbar system", "Busbar holder"]),
    ("Cables", &["Power cable", "Control cable"]),
    ("Wiring material", &["Ferrule"]),
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


    pub(crate) fn short(self) -> &'static str {
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

    pub(crate) fn from_short(s: &str) -> Option<FnKind> {
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
    // Graphics
    /// Front view for mounting layouts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_2d: Option<Model2d>,
    /// 3D model for the panel layout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_3d: Option<Mesh3d>,
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
            model_2d: None,
            model_3d: None,
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
    /// Value of a `Name: value` line of the technical data (e.g. the cores of a cable).
    pub fn tech_value(&self, name: &str) -> Option<&str> {
        self.tech_data.lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.trim().eq_ignore_ascii_case(name).then_some(v.trim())
        })
    }

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
        PartsDb { parts: crate::default_catalog() }
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

    /// Adds / replaces parts (e.g. downloaded from a parts server). Returns (added, updated).
    pub fn merge(&mut self, parts: Vec<Part>) -> (usize, usize) {
        let (mut added, mut updated) = (0, 0);
        for part in parts {
            match self.parts.iter_mut().find(|p| p.number == part.number) {
                Some(existing) if *existing != part => {
                    *existing = part;
                    updated += 1;
                }
                Some(_) => {}
                None => {
                    self.parts.push(part);
                    added += 1;
                }
            }
        }
        self.sort();
        (added, updated)
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

#[cfg(test)]
mod tests;