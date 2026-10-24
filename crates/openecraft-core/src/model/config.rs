//! Data of `crate::config` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Configuration {
    pub options: Vec<ConfigOption>,
    pub rules: Vec<Rule>,
}

/// Inserts a page macro when the condition holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Rule {
    #[serde(rename = "macro")]
    pub macro_name: String,
    /// Variant of the macro; empty: its first variant.
    pub variant: String,
    /// Placeholder values `KEY=value; KEY2=value` (may use `{OPTION}`).
    pub values: String,
    /// `OPTION=value`, `OPTION!=value`, `OPTION=a|b`, joined by `&`; empty: always.
    pub condition: String,
}

/// An option with its choices and the chosen value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ConfigOption {
    pub name: String,
    /// Allowed values; empty: any value.
    pub choices: Vec<String>,
    pub value: String,
}
