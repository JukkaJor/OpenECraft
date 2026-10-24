//! Data of `crate::translation` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Languages {
    /// Language the project texts are written in (ISO 639-1 code).
    pub source: String,
    /// Further languages of the documentation.
    pub targets: Vec<String>,
    /// Source line → language code → translation.
    pub dictionary: BTreeMap<String, BTreeMap<String, String>>,
}
