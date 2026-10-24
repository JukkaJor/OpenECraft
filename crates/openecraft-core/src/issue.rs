//! What a check found: an [`Issue`] with its [`Severity`], a message and, if it belongs
//! to one item, the page and item to show. Used by every module that reports problems.
//! The message of a built-in check is an English template with its arguments
//! ([`issue!`](crate::issue!)), translated where it is shown, so that the analysis
//! survives a language change and clients can tell issues apart by their key; plain
//! texts (plugins, generated texts) are kept as they are.

use openecraft_i18n::tr_args;
use std::fmt::Display;

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

/// The text of an issue.
#[derive(Clone, Debug, PartialEq)]
pub enum IssueText {
    /// A catalog entry with `{}` placeholders and the arguments that fill them.
    Template { key: &'static str, args: Vec<String> },
    /// A finished text.
    Plain(String),
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub severity: Severity,
    pub text: IssueText,
    pub target: Option<(usize, u64)>,
}

impl Issue {
    /// An issue with a translatable message (`key` is an English catalog entry, see the
    /// [`issue!`](crate::issue!) macro).
    pub fn new(severity: Severity, key: &'static str, args: Vec<String>, target: Option<(usize, u64)>) -> Issue {
        Issue { severity, text: IssueText::Template { key, args }, target }
    }

    /// An issue with a finished text.
    pub fn plain(severity: Severity, message: impl Into<String>, target: Option<(usize, u64)>) -> Issue {
        Issue { severity, text: IssueText::Plain(message.into()), target }
    }

    /// The message in the current language.
    pub fn message(&self) -> String {
        match &self.text {
            IssueText::Template { key, args } => {
                let args: Vec<&dyn Display> = args.iter().map(|a| a as &dyn Display).collect();
                tr_args(key, &args)
            }
            IssueText::Plain(text) => text.clone(),
        }
    }

    /// The English template of the message (stable across languages); `None` for a
    /// plain text.
    pub fn key(&self) -> Option<&'static str> {
        match &self.text {
            IssueText::Template { key, .. } => Some(key),
            IssueText::Plain(_) => None,
        }
    }

    /// The arguments of the template (empty for a plain text).
    pub fn args(&self) -> &[String] {
        match &self.text {
            IssueText::Template { args, .. } => args,
            IssueText::Plain(_) => &[],
        }
    }
}

/// `issue!(Severity::Error, target, "Cable {} defined {} times", tag, n)`: an issue whose
/// message is an English catalog entry with `{}` placeholders, filled with the
/// arguments (anything that implements `Display`) and translated where it is shown.
#[macro_export]
macro_rules! issue {
    ($severity:expr, $target:expr, $text:literal $(, $arg:expr)* $(,)?) => {
        $crate::issue::Issue::new($severity, $text, vec![$(($arg).to_string()),*], $target)
    };
}

#[cfg(test)]
mod tests;
