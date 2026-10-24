//! Data of `crate::plugs` that the project file holds (see `docs/architecture.md`).


use crate::symbols::PinKind;
use serde::{Deserialize, Serialize};

/// Plug data beyond the placed pins.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PlugData {
    /// Pin designations in plug order; pins not listed follow sorted.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<String>,
}

/// A plug pin symbol as placed in the schematic.
#[derive(Clone, Debug, PartialEq)]
pub struct Pin {
    pub page: usize,
    pub sym: usize,
    pub id: u64,
    pub label: String,
    pub kind: PinKind,
}
