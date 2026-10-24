//! External plugins: programs in any language, one folder each with a `plugin.json`
//!
//! ```json
//! { "id": "my_plugin", "name": "My plugin", "version": "1.0", "description": "…",
//!   "api": 1, "run": ["python3", "main.py"] }
//! ```
//!
//! For every request the program is started in its folder, gets one JSON object on
//! stdin and answers with one JSON object on stdout (protocol: `docs/plugins.md`).
//! `{"error": "…"}` is an error. Not available in the browser (no processes).

use crate::params::{Param, Text};
use crate::{Export, Registry, Table};
use crate::{Info, Plugin, API_VERSION};
use openecraft_core::model::Project;
use openecraft_core::issue::{Issue, Severity};
use openecraft_i18n::tr_fmt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const MANIFEST: &str = "plugin.json";

/// Sends a request to the plugin and returns its answer.
pub type Transport = Arc<dyn Fn(&Value) -> Result<Value, String> + Send + Sync>;

#[derive(Deserialize)]
struct Manifest {
    #[serde(flatten)]
    info: Info,
    #[serde(default = "api_version")]
    api: u32,
    run: Vec<String>,
}

fn api_version() -> u32 {
    API_VERSION
}

/// Answer to `describe`: what the plugin offers.
#[derive(Default, Deserialize)]
#[serde(default)]
struct Description {
    commands: Vec<CommandDesc>,
    checks: Vec<Named>,
    reports: Vec<Named>,
    exporters: Vec<ExporterDesc>,
}

#[derive(Deserialize)]
struct CommandDesc {
    name: String,
    #[serde(default)]
    help: String,
    #[serde(default)]
    params: Vec<Param>,
}

#[derive(Deserialize)]
struct Named {
    name: String,
    #[serde(default)]
    title: String,
}

#[derive(Deserialize)]
struct ExporterDesc {
    name: String,
    #[serde(default)]
    title: String,
    extension: String,
    #[serde(default)]
    params: Vec<Param>,
}

pub struct External {
    info: Info,
    transport: Transport,
}

impl External {
    pub fn new(info: Info, transport: Transport) -> External {
        External { info, transport }
    }

    /// The plugin in folder `dir`.
    pub fn open(dir: &Path) -> Result<External, String> {
        let path = dir.join(MANIFEST);
        let text = std::fs::read_to_string(&path).map_err(|e| tr_fmt!("Cannot read {}: {}", path.display(), e))?;
        let m: Manifest = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if m.api > API_VERSION {
            return Err(tr_fmt!("Plugin {} needs a newer OpenECraft (API {})", m.info.id, m.api));
        }
        if m.run.is_empty() {
            return Err(tr_fmt!("{}: the program to run is missing", path.display()));
        }
        Ok(External::new(m.info, process(dir.to_path_buf(), m.run)))
    }
}

/// Runs the program once per request.
fn process(dir: PathBuf, run: Vec<String>) -> Transport {
    Arc::new(move |request: &Value| {
        // A program given by path is found in the plugin folder.
        let program = if run[0].contains(['/', '\\']) && dir.join(&run[0]).exists() { dir.join(&run[0]) } else { PathBuf::from(&run[0]) };
        let mut child = std::process::Command::new(&program)
            .args(&run[1..])
            .current_dir(&dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| tr_fmt!("Cannot start {}: {}", program.display(), e))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(request.to_string().as_bytes()).map_err(|e| e.to_string())?;
        }
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(tr_fmt!("The plugin program failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
        serde_json::from_slice(&out.stdout).map_err(|e| tr_fmt!("Invalid answer of the plugin: {}", e))
    })
}

/// Sends a request; `{"error": …}` becomes an error.
fn ask(transport: &Transport, mut request: Value) -> Result<Value, String> {
    request["api"] = API_VERSION.into();
    let answer = transport(&request)?;
    match answer.get("error") {
        Some(e) => Err(e.as_str().map(String::from).unwrap_or_else(|| e.to_string())),
        None => Ok(answer),
    }
}

fn project_value(prj: &Project) -> Value {
    serde_json::to_value(prj).unwrap_or_default()
}

fn decode<T: for<'a> Deserialize<'a>>(v: Value) -> Result<T, String> {
    serde_json::from_value(v).map_err(|e| tr_fmt!("Invalid answer of the plugin: {}", e))
}

fn text(s: String, name: &str) -> Text {
    Text::Owned(if s.is_empty() { name.to_string() } else { s })
}

#[derive(Deserialize)]
struct IssueDesc {
    #[serde(default)]
    severity: String,
    message: String,
    page: Option<usize>,
    item: Option<u64>,
}

fn issue(d: IssueDesc) -> Issue {
    let severity = match d.severity.as_str() {
        "error" => Severity::Error,
        "info" => Severity::Info,
        _ => Severity::Warning,
    };
    Issue::plain(severity, d.message, d.page.zip(d.item))
}

impl Plugin for External {
    fn info(&self) -> Info {
        self.info.clone()
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        let desc: Description = decode(ask(&self.transport, json!({ "method": "describe" }))?)?;
        for c in desc.commands {
            let (t, name) = (self.transport.clone(), c.name.clone());
            reg.command(c.name.clone(), text(c.help, &c.name), c.params, move |prj, args| {
                let mut answer = ask(&t, json!({ "method": "command", "name": name, "params": args.0, "project": project_value(prj) }))?;
                if let Some(changed) = answer.get_mut("project").map(Value::take) {
                    *prj = Project::from_json(&changed.to_string())?;
                }
                Ok(answer.get_mut("result").map(Value::take).unwrap_or(Value::Null))
            });
        }
        for c in desc.checks {
            let (t, name) = (self.transport.clone(), c.name.clone());
            reg.check(c.name.clone(), text(c.title, &c.name), move |prj, _| {
                let mut answer = ask(&t, json!({ "method": "check", "name": name, "project": project_value(prj) }))?;
                let found: Vec<IssueDesc> = decode(answer.get_mut("issues").map(Value::take).unwrap_or(json!([])))?;
                Ok(found.into_iter().map(issue).collect())
            });
        }
        for r in desc.reports {
            let (t, name) = (self.transport.clone(), r.name.clone());
            reg.report(r.name.clone(), text(r.title, &r.name), move |prj, _| {
                #[derive(Deserialize)]
                struct Answer {
                    headers: Vec<String>,
                    #[serde(default)]
                    rows: Vec<Vec<String>>,
                }
                let a: Answer = decode(ask(&t, json!({ "method": "report", "name": name, "project": project_value(prj) }))?)?;
                let mut table = Table::new(a.headers);
                for row in a.rows {
                    table.push(row, None);
                }
                Ok(table)
            });
        }
        for e in desc.exporters {
            let (t, name) = (self.transport.clone(), e.name.clone());
            reg.exporter_with(e.name.clone(), text(e.title, &e.name), e.extension, false, e.params, move |prj, _, args| {
                let answer = ask(&t, json!({ "method": "export", "name": name, "params": args.0, "project": project_value(prj) }))?;
                answer.get("content").and_then(Value::as_str).map(|s| Export::Text(s.to_string())).ok_or_else(|| tr_fmt!("Invalid answer of the plugin: {}", "content"))
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
