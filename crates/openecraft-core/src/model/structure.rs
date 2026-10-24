//! Data of `crate::structure` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

/// Structure management of the project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Structures {
    /// Aspects the pages use; empty = function and location.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aspects: Vec<Aspect>,
    /// Defined identifiers with their descriptions.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<Entry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub work_areas: Vec<WorkArea>,
}

/// Aspect of a structure identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Aspect {
    /// `=` what the system does.
    Function,
    /// `++` the plant, building or area.
    Installation,
    /// `+` where it is mounted (enclosure, field).
    Location,
    /// `-` the higher-level product (assembly) the device belongs to.
    Product,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub aspect: Aspect,
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// A defined part of the project: only the pages and devices whose structure
/// matches the filter are shown.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkArea {
    pub name: String,
    /// Pattern per aspect: empty = any, `*` = any characters; without `*` the
    /// identifier and its sub-levels (`A1.2`, `U1-A3`) match.
    pub filter: Dt,
}

/// A device tag split into its aspects (stored without the prefix signs, except
/// the device name which keeps its `-`). Product levels are separated by `-`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Dt {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub function: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub installation: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub location: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub product: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
}
