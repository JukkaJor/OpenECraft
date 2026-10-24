//! What a check found: an [`Issue`] with its [`Severity`] and, if it belongs to one
//! item, the page and item to show. Used by every module that reports problems.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    /// Machine readable name ("error", "warning", "info").
    pub fn key(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
    pub target: Option<(usize, u64)>,
}
