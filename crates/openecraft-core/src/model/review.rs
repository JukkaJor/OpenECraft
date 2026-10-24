//! Data of `crate::review` that the project file holds (see `docs/architecture.md`).


use crate::geom::P;
use super::is_zero;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Review {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Comment>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub markups: Vec<Markup>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub approvals: Vec<Approval>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requests: Vec<ChangeRequest>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Comment {
    pub id: u64,
    pub rev: u64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub deleted: bool,
    /// Page id; 0 = the whole project.
    pub page: u64,
    /// Position of the marker on the page.
    pub pos: P,
    pub author: String,
    pub date: String,
    pub text: String,
    pub status: Status,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub replies: Vec<Reply>,
    /// Change request the comment belongs to (0 = none).
    #[serde(skip_serializing_if = "is_zero")]
    pub request: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reply {
    pub author: String,
    pub date: String,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Open,
    InProgress,
    Resolved,
    Rejected,
}

/// Decision of a user on a revision submitted for review.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Approval {
    pub id: u64,
    pub rev: u64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub deleted: bool,
    /// Revision name.
    pub revision: String,
    /// Review round of the revision (approvals of earlier rounds are void).
    pub round: u32,
    pub user: String,
    pub date: String,
    pub approved: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Markup {
    pub id: u64,
    pub rev: u64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub deleted: bool,
    pub page: u64,
    pub kind: MarkupKind,
    pub pen: Pen,
    pub a: P,
    pub b: P,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    pub author: String,
    pub date: String,
    /// Taken into the drawing: drawn faded.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub done: bool,
}

/// Red: something to change; green: changed as built / done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pen {
    #[default]
    Red,
    Green,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkupKind {
    #[default]
    Line,
    Rect,
    /// Revision cloud around the rectangle `a`..`b`.
    Cloud,
    /// Text at `a`.
    Text,
}

/// A change request (ECR) from inside or outside the project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChangeRequest {
    pub id: u64,
    pub rev: u64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub deleted: bool,
    /// Number, e.g. "CR-001" or the number of the external system.
    pub number: String,
    pub title: String,
    pub description: String,
    pub requester: String,
    /// System or organisation the request comes from.
    pub origin: String,
    pub date: String,
    pub priority: Priority,
    pub status: RequestStatus,
    /// Affected pages (ids).
    pub pages: Vec<u64>,
    /// Affected devices (tags).
    pub devices: Vec<String>,
    /// Revision that implements the request.
    pub revision: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    #[default]
    New,
    Accepted,
    InWork,
    Implemented,
    Rejected,
    Closed,
}
