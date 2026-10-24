//! Data of `crate::revision` that the project file holds (see `docs/architecture.md`).


use super::Page;
use super::is_zero;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Who approves revisions before they are released.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Workflow {
    /// User names of the approvers; empty = revisions are released directly.
    pub approvers: Vec<String>,
    /// Approvals needed; 0 = all approvers.
    pub required: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Revision {
    /// Revision index, e.g. "A", "B" or "01".
    pub name: String,
    pub date: String,
    pub author: String,
    pub description: String,
    pub pages: Snapshot,
    #[serde(skip_serializing_if = "is_released")]
    pub state: State,
    /// Review round: counts the submissions; approvals of earlier rounds are void.
    #[serde(skip_serializing_if = "is_zero")]
    pub round: u32,
}

pub(crate) fn is_released(s: &State) -> bool {
    *s == State::Released
}

/// Pages of a revision. Shared, so that undo snapshots of the project stay cheap.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Snapshot(pub Arc<Vec<Page>>);

/// Stored state of a revision; approved / rejected follow from the approvals
/// ([`status`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Draft,
    InReview,
    #[default]
    Released,
}
