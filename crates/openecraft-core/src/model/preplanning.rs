//! Data of `crate::preplanning` that the project file holds (see `docs/architecture.md`).

use serde::{Deserialize, Serialize};

/// Pre-planning of a plant before the schematic: a tree of segments (plant, area,
/// unit, …) with the devices planned in them, and segment templates.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preplanning {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<Segment>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<SegmentTemplate>,
}

impl Preplanning {
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() && self.templates.is_empty()
    }
}

/// A planning segment: its place in the tree, its structure and planned devices.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Segment {
    pub id: u64,
    /// Parent segment (0 = top level).
    pub parent: u64,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Function (`=` without the sign) of the devices; empty: the parent's.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub function: String,
    /// Location (`+` without the sign); empty: the parent's.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub location: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<PlannedDevice>,
}

/// A device planned in a segment.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlannedDevice {
    /// Device tag within the segment's structure (`-M1`).
    pub tag: String,
    /// What it is: motor, valve, sensor, …
    #[serde(skip_serializing_if = "String::is_empty")]
    pub kind: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Planned power, kW (consumption budget).
    #[serde(skip_serializing_if = "is_zero")]
    pub power: f32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub voltage: String,
    /// Planned part number.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub part: String,
    /// Page macro that generates its schematic.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub macro_name: String,
}

fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

/// A reusable set of planned devices.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SegmentTemplate {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub devices: Vec<PlannedDevice>,
}
