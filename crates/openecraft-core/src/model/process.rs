//! Data of `crate::process` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

/// A planned PCT loop.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PctLoop {
    /// Loop number, e.g. "101".
    pub name: String,
    pub description: String,
    /// Measuring range or set point, e.g. "0 … 120 °C".
    pub range: String,
    /// Device tags of the loop besides its instruments (e.g. the transmitter).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<String>,
}
