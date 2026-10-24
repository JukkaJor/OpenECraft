//! Potential definitions: kind, voltage, wire colour and cross-section of a named
//! potential (L1, N, PE, +24V …). Wires without their own colour / cross-section
//! take them from the potential of their net, otherwise from the project default.

use openecraft_i18n::{tr, tr_const, tr_fmt};
use crate::model::*;
use crate::netlist::{Net, Netlist};
pub use crate::model::potentials::{PotentialDef, PotentialKind, WireDefaults};

impl PotentialKind {
    pub const ALL: [PotentialKind; 7] = [
        PotentialKind::L,
        PotentialKind::N,
        PotentialKind::Pe,
        PotentialKind::DcPlus,
        PotentialKind::DcMinus,
        PotentialKind::Signal,
        PotentialKind::Other,
    ];

    pub fn title(self) -> &'static str {
        match self {
            PotentialKind::L => tr("Phase (L)"),
            PotentialKind::N => tr("Neutral (N)"),
            PotentialKind::Pe => tr("Protective earth (PE)"),
            PotentialKind::DcPlus => tr("DC +"),
            PotentialKind::DcMinus => tr("DC -"),
            PotentialKind::Signal => tr("Signal"),
            PotentialKind::Other => tr("Other"),
        }
    }
}

impl WireDefaults {
    pub fn is_default(&self) -> bool {
        *self == WireDefaults::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    /// IEC 60204-1 (machinery): power circuits black, DC control blue.
    Iec60204,
    /// IEC 60445 phase colours: L1 brown, L2 black, L3 grey.
    Iec60445,
}

impl Preset {
    pub const ALL: [Preset; 2] = [Preset::Iec60204, Preset::Iec60445];

    pub fn title(self) -> &'static str {
        match self {
            Preset::Iec60204 => tr("IEC 60204-1 (machinery: phases BK, DC control BU)"),
            Preset::Iec60445 => tr("IEC 60445 (phases BN, BK, GY)"),
        }
    }

    pub fn defs(self) -> Vec<PotentialDef> {
        let phases = match self {
            Preset::Iec60204 => ["BK", "BK", "BK"],
            Preset::Iec60445 => ["BN", "BK", "GY"],
        };
        let def = |name: &str, kind, voltage: &str, color: &str, cs: &str| PotentialDef {
            name: name.into(),
            kind,
            voltage: voltage.into(),
            color: color.into(),
            cross_section: cs.into(),
            description: String::new(),
        };
        vec![
            def("L1", PotentialKind::L, "400 V AC", phases[0], "2.5 mm²"),
            def("L2", PotentialKind::L, "400 V AC", phases[1], "2.5 mm²"),
            def("L3", PotentialKind::L, "400 V AC", phases[2], "2.5 mm²"),
            def("N", PotentialKind::N, "230 V AC", "BU", "2.5 mm²"),
            def("PE", PotentialKind::Pe, "", "GNYE", "2.5 mm²"),
            def("+24V", PotentialKind::DcPlus, "24 V DC", "BU", "0.75 mm²"),
            def("0V", PotentialKind::DcMinus, "24 V DC", "BU", "0.75 mm²"),
        ]
    }

    /// Default for wires without a defined potential (DC control circuits).
    pub fn wire_defaults(self) -> WireDefaults {
        WireDefaults { color: "BU".into(), cross_section: "0.75 mm²".into() }
    }
}

/// Wire colour codes of IEC 60757 with a display colour.
pub const COLORS: [(&str, &str, [u8; 3]); 15] = [
    ("BK", tr_const!("black"), [20, 20, 20]),
    ("BN", tr_const!("brown"), [120, 70, 30]),
    ("RD", tr_const!("red"), [210, 30, 30]),
    ("OG", tr_const!("orange"), [240, 130, 0]),
    ("YE", tr_const!("yellow"), [235, 205, 0]),
    ("GN", tr_const!("green"), [30, 150, 50]),
    ("BU", tr_const!("blue"), [30, 90, 210]),
    ("VT", tr_const!("violet"), [130, 50, 170]),
    ("GY", tr_const!("grey"), [130, 130, 130]),
    ("WH", tr_const!("white"), [205, 205, 205]),
    ("PK", tr_const!("pink"), [240, 140, 180]),
    ("GD", tr_const!("gold"), [200, 160, 40]),
    ("TQ", tr_const!("turquoise"), [40, 190, 190]),
    ("SR", tr_const!("silver"), [170, 170, 185]),
    ("GNYE", tr_const!("green-yellow"), [120, 180, 20]),
];

/// Colour code without separators and in upper case: `gn/ye` → `GNYE`.
pub fn normalize_color(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_uppercase()
}

/// Display colour of a colour code.
pub fn color_rgb(code: &str) -> Option<[u8; 3]> {
    let code = normalize_color(code);
    COLORS.iter().find(|c| c.0 == code).map(|c| c.2)
}

/// Display colours of a wire colour code: one, or two for two-colour codes such as
/// `GNYE` or `WHBU` (base colour first); empty for unknown codes.
pub fn color_parts(code: &str) -> Vec<[u8; 3]> {
    let code = normalize_color(code);
    if let Some(c) = color_rgb(&code).filter(|_| code != "GNYE") {
        return vec![c];
    }
    match (code.get(..2).and_then(color_rgb), code.get(2..).and_then(color_rgb)) {
        (Some(a), Some(b)) if code.len() == 4 => vec![a, b],
        _ => Vec::new(),
    }
}

/// Wire colour code per wire id, for drawing wires in their colour.
pub fn wire_colors(prj: &Project, nl: &Netlist, open: &Netlist) -> std::collections::HashMap<u64, String> {
    let mut v = std::collections::HashMap::new();
    for net in open.nets.iter().filter(|n| !n.wires.is_empty()) {
        let color = props_of(prj, nl, net).color;
        if !color.is_empty() {
            for (_, id) in &net.wires {
                v.insert(*id, color.clone());
            }
        }
    }
    v
}

/// Where a wire property comes from.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Source {
    #[default]
    None,
    /// Set on a wire of the connection.
    Manual,
    /// From the definition of this potential.
    Potential(String),
    /// From the project default.
    Default,
}

/// Colour and cross-section a connection is made with.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct WireProps {
    pub color: String,
    pub color_source: Source,
    pub cross_section: String,
    pub cross_section_source: Source,
    /// Potential of the connection (from the netlist with bridged terminals).
    pub potential: String,
}

impl WireProps {
    /// Hint for an empty field of the wire properties: `auto: BU (potential N)`.
    pub fn hint(value: &str, source: &Source) -> String {
        match source {
            Source::Potential(p) => tr_fmt!("auto: {} (potential {})", value, p),
            Source::Default => tr_fmt!("auto: {} (project default)", value),
            Source::Manual => tr_fmt!("{} (set on another wire of this connection)", value),
            Source::None => String::new(),
        }
    }
}

impl Project {
    pub fn potential(&self, name: &str) -> Option<&PotentialDef> {
        self.potentials.iter().find(|p| p.name == name)
    }

    /// Starts the potential definitions of a new project.
    pub fn apply_preset(&mut self, preset: Preset) {
        for d in preset.defs() {
            match self.potentials.iter_mut().find(|p| p.name == d.name) {
                Some(p) => *p = d,
                None => self.potentials.push(d),
            }
        }
        self.wire_defaults = preset.wire_defaults();
    }

    /// A new, empty project with the default potential definitions.
    pub fn new_default() -> Project {
        let mut prj = Project::default();
        prj.apply_preset(Preset::Iec60204);
        prj
    }
}

/// Name of the potential of a connection (`open` net) in the bridged netlist.
pub fn potential_of(nl: &Netlist, net: &Net) -> String {
    net.wires.iter().find_map(|(_, id)| nl.net_of_wire(*id)).filter(|n| n.named).map(|n| n.name.clone()).unwrap_or_default()
}

/// Colour and cross-section of a connection: set on one of its wires, else from the
/// definition of its potential, else the project default. Cable cores take only
/// what is set on them (the cable defines the rest); short circuits (`A / B`) take
/// no potential.
pub fn props_of(prj: &Project, nl: &Netlist, net: &Net) -> WireProps {
    let wires = crate::wirenum::wires_of(prj, net);
    let potential = potential_of(nl, net);
    let mut props = WireProps { potential: potential.clone(), ..Default::default() };
    if let Some(w) = wires.iter().find(|w| !w.color.is_empty()) {
        props.color = w.color.clone();
        props.color_source = Source::Manual;
    }
    if let Some(w) = wires.iter().find(|w| !w.cross_section.is_empty()) {
        props.cross_section = w.cross_section.clone();
        props.cross_section_source = Source::Manual;
    }
    if wires.iter().any(|w| !w.cable.is_empty()) {
        return props;
    }
    let def = prj.potential(&potential);
    if props.color.is_empty() {
        if let Some(d) = def.filter(|d| !d.color.is_empty()) {
            props.color = d.color.clone();
            props.color_source = Source::Potential(d.name.clone());
        } else if def.is_none() && !prj.wire_defaults.color.is_empty() && !is_pe_name(&potential) {
            props.color = prj.wire_defaults.color.clone();
            props.color_source = Source::Default;
        }
    }
    if props.cross_section.is_empty() {
        if let Some(d) = def.filter(|d| !d.cross_section.is_empty()) {
            props.cross_section = d.cross_section.clone();
            props.cross_section_source = Source::Potential(d.name.clone());
        } else if def.is_none() && !prj.wire_defaults.cross_section.is_empty() {
            props.cross_section = prj.wire_defaults.cross_section.clone();
            props.cross_section_source = Source::Default;
        }
    }
    props
}

/// Wire properties of a single wire (by id), see [`props_of`].
pub fn props_of_wire(prj: &Project, nl: &Netlist, open: &Netlist, wire: u64) -> WireProps {
    open.net_of_wire(wire).map(|net| props_of(prj, nl, net)).unwrap_or_default()
}

fn is_pe_name(name: &str) -> bool {
    matches!(name, "PE" | "PEN")
}

/// The potential is a protective conductor: defined as PE, or named PE / PEN.
pub fn is_protective(prj: &Project, name: &str) -> bool {
    match prj.potential(name) {
        Some(d) => d.kind == PotentialKind::Pe,
        None => is_pe_name(name),
    }
}

#[cfg(test)]
mod tests;
