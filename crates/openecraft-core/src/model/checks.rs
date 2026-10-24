//! Settings of the project check stored with the project: checks switched off,
//! messages whose level is changed, and messages accepted as checked (in the spirit
//! of EPLAN's check schemes and message management).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Level a message is shown with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Level {
    Error,
    Warning,
    Info,
    /// Not shown at all.
    Off,
}

/// A message accepted as checked: hidden from the open messages as long as it stays
/// the same (same check, message, arguments and item).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Accepted {
    /// Fingerprint of the message (`Issue::fingerprint`).
    pub id: String,
    /// The message as it was shown when it was accepted.
    pub message: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub user: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CheckSettings {
    /// Names of the checks that do not run.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub disabled: Vec<String>,
    /// Message code (`Issue::code`) → level it is shown with.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub levels: BTreeMap<String, Level>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub accepted: Vec<Accepted>,
}

impl CheckSettings {
    pub fn is_default(&self) -> bool {
        self.disabled.is_empty() && self.levels.is_empty() && self.accepted.is_empty()
    }
}
