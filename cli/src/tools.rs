//! The tools of the command line and the MCP server that need the session or the
//! machine (files, parts server, undo); the operations on the project are commands
//! of the built-in plugins (`openecraft_api::builtin`). A tool has typed parameters
//! (from which the JSON schema and the command line parsing follow) and returns JSON.
//! Help texts are English (`tr_const!`); the command line shows them translated, MCP
//! clients get them as they are. The commands of plugins (`openecraft_api`) follow the
//! built-in tools.

mod config;
mod devices;
mod integration;
mod languages;
mod management;
mod output;
mod pages;
mod panel;
mod parts;
mod plc;
mod plugins;
mod reports;
mod project;
mod properties;
mod sheets;
mod structure;
mod tables;
#[cfg(test)]
mod workflows;

use crate::session::Session;
use openecraft_api::builtin::common;
use openecraft_api::params;
pub use openecraft_api::params::{optional, param, Args, Kind, Param};
use openecraft_api::Command;
use openecraft_i18n::tr;
use serde_json::{Map, Value};

pub type Run = fn(&mut Session, &Args) -> Result<Value, String>;

pub struct Tool {
    pub name: &'static str,
    pub help: &'static str,
    pub params: &'static [Param],
    /// Works on the open project (the command line opens `--project` first).
    pub needs_project: bool,
    pub run: Run,
}

pub fn all() -> impl Iterator<Item = &'static Tool> {
    [project::TOOLS, management::TOOLS, pages::TOOLS, sheets::TOOLS, devices::TOOLS, plc::TOOLS, tables::TOOLS, panel::TOOLS, integration::TOOLS, languages::TOOLS, structure::TOOLS, properties::TOOLS, config::TOOLS, output::TOOLS, reports::TOOLS, parts::TOOLS, plugins::TOOLS].into_iter().flatten()
}

pub fn find(name: &str) -> Option<&'static Tool> {
    all().find(|t| t.name == name)
}

/// A built-in tool or a command of a plugin: both are commands of the command line and
/// tools of the MCP server.
#[derive(Clone)]
pub enum Entry {
    Tool(&'static Tool),
    Plugin(Command),
}

impl From<&'static Tool> for Entry {
    fn from(t: &'static Tool) -> Entry {
        Entry::Tool(t)
    }
}

impl Entry {
    pub fn name(&self) -> &str {
        match self {
            Entry::Tool(t) => t.name,
            Entry::Plugin(c) => &c.name,
        }
    }

    /// Help text in English (MCP clients).
    pub fn help(&self) -> &str {
        match self {
            Entry::Tool(t) => t.help,
            Entry::Plugin(c) => &c.help,
        }
    }

    /// Help text in the current language.
    pub fn help_text(&self) -> &str {
        match self {
            Entry::Tool(t) => tr(t.help),
            Entry::Plugin(c) => params::text(&c.help),
        }
    }

    pub fn params(&self) -> &[Param] {
        match self {
            Entry::Tool(t) => t.params,
            Entry::Plugin(c) => &c.params,
        }
    }

    pub fn needs_project(&self) -> bool {
        match self {
            Entry::Tool(t) => t.needs_project,
            Entry::Plugin(_) => true,
        }
    }
}

/// The built-in tools followed by the commands of the enabled plugins.
pub fn entries(session: &Session) -> Vec<Entry> {
    all().map(Entry::Tool).chain(session.plugins.commands().cloned().map(Entry::Plugin)).collect()
}

pub fn lookup(session: &Session, name: &str) -> Option<Entry> {
    find(name).map(Entry::Tool).or_else(|| session.plugins.command(name).cloned().map(Entry::Plugin))
}

/// Checks the parameters against the tool and runs it. Changes of the project become
/// one undo step.
pub fn call(session: &mut Session, entry: impl Into<Entry>, values: &Map<String, Value>) -> Result<Value, String> {
    let entry = entry.into();
    params::check(entry.params(), values)?;
    if entry.needs_project() {
        session.project()?;
    }
    let result = match &entry {
        Entry::Tool(t) => (t.run)(session, &Args(values)),
        Entry::Plugin(c) => (c.run)(session.project_mut()?, &Args(values)),
    };
    session.commit();
    result
}

/// JSON schema of the tool's parameters (MCP `inputSchema`).
pub fn schema(entry: impl Into<Entry>) -> Value {
    params::schema(entry.into().params())
}

/// Parameters from the command line: `--json` first, then `--name value` pairs
/// converted to the parameter's type.
pub fn from_cli(entry: impl Into<Entry>, json: Option<&str>, pairs: &[(String, Option<String>)]) -> Result<Map<String, Value>, String> {
    params::from_cli(entry.into().params(), json, pairs)
}

/// Command line help of a tool.
pub fn help(entry: impl Into<Entry>) -> String {
    let entry = entry.into();
    let mut out = format!("openecraft-cli {}{}\n\n{}\n", entry.name(), if entry.needs_project() { " --project <file>" } else { "" }, entry.help_text());
    if !entry.params().is_empty() {
        out.push('\n');
        for p in entry.params() {
            let arg = format!("--{} {}", p.name, p.kind.hint());
            let mark = if p.required { String::new() } else { format!("{} ", tr("(optional)")) };
            out.push_str(&format!("  {arg:<28} {mark}{}\n", params::text(&p.help)));
        }
    }
    out
}

#[cfg(test)]
mod tests;
