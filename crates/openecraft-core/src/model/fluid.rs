//! Data of `crate::fluid` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default)]
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

/// Configuration of a hose / tube / pipe, stored on its wires (all wires of the
/// connection carry the same).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HoseConfig {
    pub line: LineType,
    /// Part number of the hose / tube / pipe (sold by the metre).
    pub part: String,
    /// End fittings at the start and the end of the hose.
    pub fitting_a: String,
    pub fitting_b: String,
    /// Maximum operating pressure in bar; 0 = not set.
    pub pressure: f32,
    /// Smallest bend radius in mm; 0 = not set.
    pub bend_radius: f32,
}

/// Hose (flexible, with fittings), tube (cut to length, push-in or compression
/// fittings) or pipe (rigid piping).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum LineType {
    #[default]
    Hose,
    Tube,
    Pipe,
}


/// Defaults of hoses that do not set their own values, the hose numbering, device
/// groups and the links to the electrical devices.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FluidSettings {
    pub medium: Medium,
    /// Nominal size, e.g. "8 mm" (outer diameter) or "DN25".
    pub size: String,
    pub color: String,
    /// Put in front of the counted hose numbers, e.g. "H".
    pub prefix: String,
    /// Operating pressure of the system in bar; 0 = not set (ratings are not checked).
    pub pressure: f32,
    /// Added to each end of a tube for its cut length (insertion depth), mm.
    pub allowance: f32,
    /// Valve terminals, service units, manifolds.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<super::electrofluid::DeviceGroup>,
    /// Fluid device tag → electrical devices that actuate or monitor it (solenoid
    /// coils, position and pressure switches).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub links: BTreeMap<String, Vec<String>>,
}
