//! Data of `crate::marking` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

/// Marking settings of a project: wire marker text and the marker material per kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub wire_text: WireText,
    pub device_material: String,
    /// Legend plates of door devices.
    pub door_material: String,
    pub terminal_material: String,
    pub wire_material: String,
    pub cable_material: String,
}

/// What a wire marker shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireText {
    /// The wire number, the same on both ends.
    #[default]
    Number,
    /// At each end the connection point at the other end (destination marking).
    Destination,
}
