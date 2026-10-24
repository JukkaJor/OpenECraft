//! Data of `crate::compare` that the project file holds (see `docs/architecture.md`).


use super::Project;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A named state of the whole project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Baseline {
    pub name: String,
    pub date: String,
    pub author: String,
    pub description: String,
    pub project: Stored,
}

/// Stored project. Shared, so that undo snapshots stay cheap.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Stored(pub Arc<Project>);
