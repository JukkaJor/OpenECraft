//! Data of `crate::subproject` that the project file holds (see `docs/architecture.md`).


use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Subprojects {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<Parent>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Child>,
}

/// The project a subproject was taken from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Parent {
    /// Name of the main project.
    pub project: String,
    /// Name of the subproject in the main project.
    pub name: String,
    pub pages: Vec<u64>,
    pub exported: String,
}

/// A subproject taken out of this project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Child {
    pub name: String,
    pub description: String,
    /// Ids of the pages taken out.
    pub pages: Vec<u64>,
    pub exported: String,
    /// Date of the last insertion; empty: not inserted yet.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub inserted: String,
    /// Being edited in the subproject: its pages are locked here.
    pub out: bool,
    /// Pages that were locked before the subproject was taken out.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub locked_before: Vec<u64>,
    /// First id reserved for the subproject when it was taken out (0: files of older
    /// versions); a subproject file of an earlier extraction counts below it.
    #[serde(skip_serializing_if = "is_zero")]
    pub first_id: u64,
}

fn is_zero(v: &u64) -> bool {
    *v == 0
}
