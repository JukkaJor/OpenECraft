//! Fluid connections: hoses and pipes between the ports of fluid power and process
//! symbols. They form their own nets ([`crate::netlist::Net::fluid`]); each
//! point-to-point connection is a hose with a medium, a nominal size, a colour and a
//! length, listed in the hose list.

use openecraft_i18n::{tr, tr_fmt};
use crate::model::*;
use crate::netlist::{chained_pins, pin_designation, Net, Netlist, PinRef};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Medium {
    #[default]
    CompressedAir,
    Vacuum,
    HydraulicOil,
    Water,
    Coolant,
    Lubricant,
    Gas,
    Steam,
    /// Product of the process plant.
    Product,
}

impl Medium {
    pub const ALL: [Medium; 9] = [
        Medium::CompressedAir,
        Medium::Vacuum,
        Medium::HydraulicOil,
        Medium::Water,
        Medium::Coolant,
        Medium::Lubricant,
        Medium::Gas,
        Medium::Steam,
        Medium::Product,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Medium::CompressedAir => tr("Compressed air"),
            Medium::Vacuum => tr("Vacuum"),
            Medium::HydraulicOil => tr("Hydraulic oil"),
            Medium::Water => tr("Water"),
            Medium::Coolant => tr("Coolant"),
            Medium::Lubricant => tr("Lubricant"),
            Medium::Gas => tr("Gas"),
            Medium::Steam => tr("Steam"),
            Medium::Product => tr("Product"),
        }
    }

    /// Name in files and on the command line.
    pub fn code(self) -> &'static str {
        match self {
            Medium::CompressedAir => "compressed_air",
            Medium::Vacuum => "vacuum",
            Medium::HydraulicOil => "hydraulic_oil",
            Medium::Water => "water",
            Medium::Coolant => "coolant",
            Medium::Lubricant => "lubricant",
            Medium::Gas => "gas",
            Medium::Steam => "steam",
            Medium::Product => "product",
        }
    }

    pub fn from_code(code: &str) -> Option<Medium> {
        let c = code.trim().to_ascii_lowercase().replace([' ', '-'], "_");
        Medium::ALL.into_iter().find(|m| m.code() == c)
    }
}

/// Defaults of hoses that do not set their own values, and the hose numbering.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FluidSettings {
    pub medium: Medium,
    /// Nominal size, e.g. "8 mm" (outer diameter) or "DN25".
    pub size: String,
    pub color: String,
    /// Put in front of the counted hose numbers, e.g. "H".
    pub prefix: String,
}

impl Default for FluidSettings {
    fn default() -> Self {
        FluidSettings { medium: Medium::CompressedAir, size: "8 mm".into(), color: "BU".into(), prefix: "H".into() }
    }
}

impl FluidSettings {
    pub fn is_default(&self) -> bool {
        *self == FluidSettings::default()
    }
}

/// A hose (or pipe) between two ports.
#[derive(Clone, Debug, PartialEq)]
pub struct Hose {
    pub number: String,
    /// Fluid net (`F1` …).
    pub net: String,
    pub a: PinRef,
    pub b: PinRef,
    pub from: String,
    pub to: String,
    pub medium: Medium,
    pub size: String,
    pub color: String,
    /// Metres; 0 = not set.
    pub length: f32,
}

/// Medium, size, colour and length of a fluid net: the first value set on its wires,
/// otherwise the project's defaults.
pub fn props_of(prj: &Project, net: &Net) -> (Medium, String, String, f32) {
    let wires = crate::wirenum::wires_of(prj, net);
    let medium = wires.iter().find_map(|w| w.medium).unwrap_or(prj.fluid.medium);
    let pick = |f: &dyn Fn(&Wire) -> &String, default: &String| wires.iter().map(|w| f(w)).find(|v| !v.is_empty()).unwrap_or(default).clone();
    let size = pick(&|w| &w.cross_section, &prj.fluid.size);
    let color = pick(&|w| &w.color, &prj.fluid.color);
    let length = wires.iter().map(|w| w.length).find(|l| *l > 0.0).unwrap_or(0.0);
    (medium, size, color, length)
}

/// All hoses of the project in schematic order (`open`: netlist with the terminal
/// sides apart).
pub fn hoses(prj: &Project, open: &Netlist) -> Vec<Hose> {
    let mut out = Vec::new();
    for net in open.nets.iter().filter(|n| n.fluid && !n.mixed) {
        let pins = chained_pins(prj, net);
        let (medium, size, color, length) = props_of(prj, net);
        let number = crate::wirenum::number_of(prj, net);
        for pair in pins.windows(2) {
            out.push(Hose {
                number: number.clone(),
                net: net.name.clone(),
                a: pair[0],
                b: pair[1],
                from: pin_designation(prj, pair[0]),
                to: pin_designation(prj, pair[1]),
                medium,
                size: size.clone(),
                color: color.clone(),
                length,
            });
        }
    }
    out
}

/// Fluid problems: ports connected to electrical connection points, hoses without
/// length. (message, error, jump target)
pub fn problems(prj: &Project, open: &Netlist) -> Vec<Problem> {
    let mut v = Vec::new();
    for net in open.nets.iter().filter(|n| n.mixed) {
        let names: Vec<String> = net.pins.iter().map(|p| pin_designation(prj, *p)).collect();
        let target = net.wires.first().copied();
        v.push((tr_fmt!("Fluid port connected to an electrical connection point: {}", names.join(", ")), true, target));
    }
    for h in hoses(prj, open) {
        if h.length <= 0.0 {
            let s = &prj.pages[h.a.page].symbols[h.a.sym];
            v.push((tr_fmt!("Hose {} – {} without length", h.from, h.to), false, Some((h.a.page, s.id))));
        }
    }
    v
}

/// Hose length text for reports.
pub fn length_text(length: f32) -> String {
    if length > 0.0 { format!("{length:.2}") } else { "–".into() }
}

/// (message, error (else info), jump target)
pub type Problem = (String, bool, Option<(usize, u64)>);

#[cfg(test)]
mod tests;
