//! Data of `crate::rules` that the project file holds (see `docs/architecture.md`).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RuleSet {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Standard, customer or plant the rules come from.
    pub source: String,
    /// Customer rule sets: the project is verified against it.
    pub active: bool,
    pub limits: LimitOverrides,
    pub checks: Vec<Check>,
}

/// Limits a rule set sets; unset ones keep the value of the other rule sets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LimitOverrides {
    /// Highest voltage drop from the supply to a load, %.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voltage_drop: Option<f32>,
    /// Longest disconnection time of final circuits (up to 32 A), s.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disconnect_final: Option<f32>,
    /// Longest disconnection time of distribution circuits, s.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disconnect_distribution: Option<f32>,
    /// Smallest cross-section of power circuits (mains), mm².
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_cross_section_power: Option<f32>,
    /// Smallest cross-section of control circuits, mm².
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_cross_section_control: Option<f32>,
    /// Reserve to keep on protective devices, the supply and power supplies, %.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_reserve: Option<f32>,
    /// Highest unbalance of the phase currents, %.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_unbalance: Option<f32>,
    /// Protective devices must be selective up to the short-circuit current.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_selectivity: Option<bool>,
    /// Motor cables of frequency converters must be shielded at both ends.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shield_drive_cables: Option<bool>,
    /// Cables of analogue signals must be shielded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shield_analog_cables: Option<bool>,
    /// The supply needs a surge protective device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surge_protection: Option<bool>,
}

/// The limits in effect: the defaults (IEC 60204-1, IEC 60364-4-41) tightened by
/// the active rule sets.
#[derive(Clone, Debug, PartialEq)]
pub struct Limits {
    pub voltage_drop: f32,
    pub disconnect_final: f32,
    pub disconnect_distribution: f32,
    /// 0: no minimum.
    pub min_cross_section_power: f32,
    pub min_cross_section_control: f32,
    pub load_reserve: f32,
    /// 0: not checked.
    pub max_unbalance: f32,
    pub total_selectivity: bool,
    pub shield_drive_cables: bool,
    pub shield_analog_cables: bool,
    pub surge_protection: bool,
}

/// Every item of `scope` that meets all conditions of `when` must meet `require`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Check {
    pub scope: Scope,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<Condition>,
    pub require: Condition,
    pub level: Level,
    pub message: String,
}

/// What a check looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Scope {
    Project,
    Page,
    #[default]
    Device,
    /// Parts used in the project.
    Part,
    Cable,
    Connection,
    Potential,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Condition {
    pub field: String,
    pub op: Op,
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Op {
    #[default]
    NotEmpty,
    Empty,
    Equals,
    NotEquals,
    Contains,
    NotContains,
    StartsWith,
    /// One of a comma separated list.
    OneOf,
    NoneOf,
    AtLeast,
    AtMost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Level {
    Error,
    #[default]
    Warning,
    Info,
}
