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
    /// Name of the check that found it (set by the host).
    pub check: String,
    /// Accepted as checked (project check settings).
    pub accepted: bool,
}

/// FNV-1a hash of a text (stable across versions and platforms).
fn hash(text: &str) -> u32 {
    text.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193))
}

impl Issue {
    /// An issue with a translatable message (`key` is an English catalog entry, see the
    /// [`issue!`](crate::issue!) macro).
    pub fn new(severity: Severity, key: &'static str, args: Vec<String>, target: Option<(usize, u64)>) -> Issue {
        Issue { severity, text: IssueText::Template { key, args }, target, check: String::new(), accepted: false }
    }

    /// An issue with a finished text.
    pub fn plain(severity: Severity, message: impl Into<String>, target: Option<(usize, u64)>) -> Issue {
        Issue { severity, text: IssueText::Plain(message.into()), target, check: String::new(), accepted: false }
    }

    /// Code of the kind of message, e.g. `wires.3f9a1c`: the check and a hash of the
    /// English template, the same in every language and for every occurrence. Plain
    /// texts have the check's name as code.
    pub fn code(&self) -> String {
        let check = if self.check.is_empty() { "project" } else { self.check.as_str() };
        match &self.text {
            IssueText::Template { key, .. } => format!("{check}.{:06x}", hash(key) & 0x00ff_ffff),
            IssueText::Plain(_) => check.to_string(),
        }
    }

    /// Identity of this occurrence: code, arguments (or the text) and the item, used
    /// to accept a message until it changes.
    pub fn fingerprint(&self) -> String {
        let what = match &self.text {
            IssueText::Template { args, .. } => args.join("|"),
            IssueText::Plain(text) => text.clone(),
        };
        let item = self.target.map(|t| t.1).unwrap_or_default();
        format!("{}:{:08x}:{item}", self.code(), hash(&what))
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
