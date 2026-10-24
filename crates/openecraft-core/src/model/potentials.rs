//! Data of `crate::potentials` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

/// Colour and cross-section of wires whose net has no defined potential.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WireDefaults {
    pub color: String,
    pub cross_section: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PotentialDef {
    /// Potential name as on the potential connection points, e.g. `L1`.
    pub name: String,
    pub kind: PotentialKind,
    /// e.g. "400 V AC".
    pub voltage: String,
    /// Wire colour (IEC 60757 code, e.g. "BK").
    pub color: String,
    /// e.g. "2.5 mm²".
    pub cross_section: String,
    pub description: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PotentialKind {
    /// Phase conductor.
    L,
    N,
    Pe,
    DcPlus,
    DcMinus,
    Signal,
    #[default]
    Other,
}
