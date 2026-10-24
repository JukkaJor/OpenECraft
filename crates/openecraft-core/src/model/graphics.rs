//! Data of `crate::graphics` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    /// Items on a locked layer cannot be selected.
    pub locked: bool,
    /// Items are printed / exported.
    pub print: bool,
    /// Colour of items that have none of their own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
}
