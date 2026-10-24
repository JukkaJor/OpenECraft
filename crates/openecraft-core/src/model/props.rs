//! Data of `crate::props` that the project file holds (see `docs/architecture.md`).

use super::*;
use serde::{Deserialize, Serialize};

/// A placement scheme: properties with their positions relative to [`reference`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Scheme {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub entries: Vec<PlacedProp>,
    #[serde(skip)]
    pub builtin: bool,
}
