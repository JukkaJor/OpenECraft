//! Data of `crate::mounting` that the project file holds (see `docs/architecture.md`).

use serde::{Deserialize, Serialize};

/// Mark of a panel item in the mounting assistant, with the item's place and part at
/// that time (to find items moved or changed after they were mounted).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MountingMark {
    /// Id of the panel item.
    pub item: u64,
    pub mark: crate::model::wiring::Mark,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
    pub x: f32,
    pub y: f32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub part: String,
}
