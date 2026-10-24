//! Data of `crate::safety` that the project file holds (see `docs/architecture.md`).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SafetyFunction {
    pub name: String,
    pub description: String,
    /// Required performance level (PLr).
    pub required: Pl,
    pub subsystems: Vec<Subsystem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Subsystem {
    pub name: String,
    /// Device tags of a channel.
    pub devices: Vec<String>,
    pub category: Category,
    /// Average diagnostic coverage, %.
    pub dc: f32,
    /// Operations per year (MTTFd from B10d).
    pub operations: f32,
    /// MTTFd of a channel in years; 0: from the devices.
    pub mttfd: f32,
    /// Measures against common cause failure (at least 65 points, annex F).
    pub ccf: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum Category {
    B,
    One,
    Two,
    #[default]
    Three,
    Four,
}

/// Performance level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum Pl {
    A,
    B,
    C,
    #[default]
    D,
    E,
}
