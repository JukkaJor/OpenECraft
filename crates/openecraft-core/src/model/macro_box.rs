//! Data of `crate::macro_boxes` that the project file holds (see `docs/architecture.md`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A window macro placed on a page: which macro (and variant) with which placeholder
/// values, the items it placed, and the fingerprint of the macro content at that time
/// (to find boxes whose library macro changed since).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MacroBox {
    pub id: u64,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub variant: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, String>,
    /// Ids of the items the macro placed (items deleted since are left out).
    pub items: Vec<u64>,
    /// Fingerprint of the macro content when it was placed or updated.
    pub source: u64,
}
