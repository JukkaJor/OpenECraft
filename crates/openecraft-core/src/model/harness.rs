//! Data of `crate::harness` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Harness {
    pub id: u64,
    /// Complete device tag, e.g. `=A1-WH1`.
    pub tag: String,
    pub description: String,
    /// Length in metres of wires that are not routed in the panel layout (field
    /// wiring); 0 = unknown.
    pub length: f32,
    /// Part number of the finished harness (bill of materials).
    pub part: String,
    /// Connections of the harness: both ends as connection point designations
    /// (`-S1:13`), in [`key`] order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<[String; 2]>,
}
