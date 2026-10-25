//! Data of `crate::electrical` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

/// Electrical design data of a project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Electrical {
    pub supply: Supply,
    pub conditions: Conditions,
    /// Diversity factor of the loads; 0: rated diversity factor of IEC 61439-1,
    /// table 101, by the number of circuits.
    pub diversity: f32,
    /// Safety functions (ISO 13849-1).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub safety: Vec<super::safety::SafetyFunction>,
    /// Ids of the built-in rule libraries the project is verified against.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub standards: Vec<String>,
    /// Customer rule sets.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rule_sets: Vec<super::rules::RuleSet>,
    /// Selectivity and back-up protection of device pairs from the manufacturers'
    /// tables; they replace the estimate from the time-current curves.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub coordination: Vec<Coordination>,
}

/// Coordination of two protective devices as tested by the manufacturer (IEC 60947-2,
/// annex A), by part number.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Coordination {
    pub upstream: String,
    pub downstream: String,
    /// Selective up to this short-circuit current, kA; 0: not given.
    pub selectivity: f32,
    /// The upstream device protects the downstream one up to this short-circuit
    /// current (back-up protection), kA; 0: not given.
    pub backup: f32,
}

/// Transformer feeding the supply, with the network upstream (IEC 60909-0).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Transformer {
    /// Rated apparent power, kVA.
    pub power: f32,
    /// Rated short-circuit voltage, %.
    pub uk: f32,
    /// Load losses at rated current, W; 0: from typical values of the rated power.
    pub losses: f32,
    /// Short-circuit power of the network on the high-voltage side, MVA; 0: infinite.
    pub network: f32,
}

/// Installation conditions of the conductors: correct the current-carrying capacity
/// of IEC 60204-1 table 6 (copper, PVC, 40 °C, one circuit).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Conditions {
    /// Ambient temperature in °C.
    pub ambient: f32,
    /// Loaded circuits bunched together (IEC 60364-5-52, table B.52.17).
    pub grouped: u32,
    pub material: Material,
    pub insulation: Insulation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Insulation {
    /// PVC, 70 °C.
    #[default]
    Pvc,
    /// XLPE / EPR, 90 °C.
    Xlpe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Material {
    #[default]
    Copper,
    Aluminium,
}

/// The supply at the incoming terminals of the machine / assembly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Supply {
    /// Line-to-line voltage in V.
    pub voltage: f32,
    pub frequency: f32,
    pub earthing: Earthing,
    /// Highest three-phase short-circuit current at the supply point in kA.
    pub ik_max: f32,
    /// Lowest single-phase fault current (line to PE) at the supply point in kA;
    /// 0: from the highest short-circuit current.
    pub ik_min: f32,
    /// Rated current of the supply in A; 0: not limited.
    pub rated_current: f32,
    /// R/X ratio of the supply impedance behind `ik_max`.
    pub rx: f32,
    /// Transformer feeding the supply: its impedances replace `ik_max` and `ik_min`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transformer: Option<Transformer>,
    /// Asynchronous motors fed directly from the mains add to the short-circuit
    /// current (IEC 60909-0, 3.8).
    pub motors: bool,
    /// IT system with a distributed neutral: the second fault is checked at the
    /// line-to-earth voltage.
    pub neutral: bool,
}

/// Earthing system of the supply (IEC 60364-1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Earthing {
    #[default]
    TnS,
    TnC,
    TnCS,
    Tt,
    It,
}
