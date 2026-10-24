//! Data of `crate::wiring` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

/// Mark of a wire, keyed by its two ends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WiringMark {
    pub from: String,
    pub to: String,
    pub mark: Mark,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// Properties of the wire when it was marked.
    pub color: String,
    pub cross_section: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub number: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Mark {
    #[default]
    Done,
    /// Could not be wired (wrong length, missing part …), see the note.
    Problem,
}
