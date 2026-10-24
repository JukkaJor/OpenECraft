//! Data of `crate::document` that the project file holds (see `docs/architecture.md`).


use openecraft_parts::Shared;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Document {
    /// File name, e.g. `datasheet.pdf`.
    pub name: String,
    /// File content, base64 encoded (shared: project snapshots compare and clone it
    /// by pointer).
    pub data: Shared<String>,
}
