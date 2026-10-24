//! Extension points: what a plugin adds in [`crate::Plugin::register`].

use crate::params::{Args, Param, Text};
use openecraft_core::model::Project;
use openecraft_core::reports::{self, Analysis, Issue};
use serde_json::Value;
use std::sync::Arc;

pub type CommandFn = dyn Fn(&mut Project, &Args) -> Result<Value, String> + Send + Sync;
pub type CheckFn = dyn Fn(&Project, &Analysis) -> Result<Vec<Issue>, String> + Send + Sync;
pub type ReportFn = dyn Fn(&Project, &Analysis) -> Result<Table, String> + Send + Sync;
pub type ExportFn = dyn Fn(&Project) -> Result<String, String> + Send + Sync;

/// A function on the project with typed parameters. Offered as command of the command
/// line, tool of the MCP server and entry of the editor's Plugins menu; changes of the
/// project become one undo step.
#[derive(Clone)]
pub struct Command {
    pub name: String,
    pub help: Text,
    pub params: Vec<Param>,
    pub run: Arc<CommandFn>,
}

/// Adds its issues to the project check.
#[derive(Clone)]
pub struct Check {
    pub name: String,
    pub title: Text,
    pub run: Arc<CheckFn>,
}

/// A table generated from the project (editor: Plugins › reports, command line: `report`).
#[derive(Clone)]
pub struct Report {
    pub name: String,
    pub title: Text,
    pub run: Arc<ReportFn>,
}

/// A text file generated from the project (editor: Plugins › export, command line: `export`).
#[derive(Clone)]
pub struct Exporter {
    pub name: String,
    pub title: Text,
    /// File extension without the dot.
    pub extension: String,
    pub run: Arc<ExportFn>,
}

/// Result of a report.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub headers: Vec<Text>,
    pub rows: Vec<Vec<String>>,
    /// Optional jump target (page index, item id) per row.
    pub targets: Vec<Option<(usize, u64)>>,
}

impl Table {
    pub fn new(headers: impl IntoIterator<Item = impl Into<Text>>) -> Table {
        Table { headers: headers.into_iter().map(Into::into).collect(), ..Default::default() }
    }

    pub fn push(&mut self, row: Vec<String>, target: Option<(usize, u64)>) {
        self.rows.push(row);
        self.targets.push(target);
    }

    /// The headers in the current language.
    pub fn header_texts(&self) -> Vec<&str> {
        self.headers.iter().map(crate::params::text).collect()
    }

    /// CSV like the built-in reports (`;` separated).
    pub fn to_csv(&self) -> String {
        let esc = |s: &str| if s.contains([';', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
        let line = |cells: Vec<&str>| cells.into_iter().map(esc).collect::<Vec<_>>().join(";") + "\n";
        let mut out = line(self.header_texts());
        for r in &self.rows {
            out.push_str(&line(r.iter().map(String::as_str).collect()));
        }
        out
    }
}

impl From<reports::Table> for Table {
    fn from(t: reports::Table) -> Table {
        Table { headers: t.headers.into_iter().map(Text::Borrowed).collect(), rows: t.rows, targets: t.targets }
    }
}

/// What one plugin adds. Names are snake_case and unique among all plugins (commands
/// also among the built-in commands of the front ends), see [`crate::Plugins::add`].
#[derive(Clone, Default)]
pub struct Registry {
    pub commands: Vec<Command>,
    pub checks: Vec<Check>,
    pub reports: Vec<Report>,
    pub exporters: Vec<Exporter>,
}

impl Registry {
    pub fn command(&mut self, name: impl Into<String>, help: impl Into<Text>, params: Vec<Param>, run: impl Fn(&mut Project, &Args) -> Result<Value, String> + Send + Sync + 'static) -> &mut Self {
        self.commands.push(Command { name: name.into(), help: help.into(), params, run: Arc::new(run) });
        self
    }

    pub fn check(&mut self, name: impl Into<String>, title: impl Into<Text>, run: impl Fn(&Project, &Analysis) -> Result<Vec<Issue>, String> + Send + Sync + 'static) -> &mut Self {
        self.checks.push(Check { name: name.into(), title: title.into(), run: Arc::new(run) });
        self
    }

    pub fn report(&mut self, name: impl Into<String>, title: impl Into<Text>, run: impl Fn(&Project, &Analysis) -> Result<Table, String> + Send + Sync + 'static) -> &mut Self {
        self.reports.push(Report { name: name.into(), title: title.into(), run: Arc::new(run) });
        self
    }

    pub fn exporter(&mut self, name: impl Into<String>, title: impl Into<Text>, extension: impl Into<String>, run: impl Fn(&Project) -> Result<String, String> + Send + Sync + 'static) -> &mut Self {
        self.exporters.push(Exporter { name: name.into(), title: title.into(), extension: extension.into(), run: Arc::new(run) });
        self
    }

    /// Names of everything registered, with the kind (for the uniqueness check).
    pub(crate) fn names(&self) -> impl Iterator<Item = (&'static str, &str)> {
        let c = self.commands.iter().map(|x| ("command", x.name.as_str()));
        let k = self.checks.iter().map(|x| ("check", x.name.as_str()));
        let r = self.reports.iter().map(|x| ("report", x.name.as_str()));
        let e = self.exporters.iter().map(|x| ("exporter", x.name.as_str()));
        c.chain(k).chain(r).chain(e)
    }
}

#[cfg(test)]
mod tests;
