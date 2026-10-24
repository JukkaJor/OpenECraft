//! The commands of the command line and the MCP server. A tool has typed parameters
//! (from which the JSON schema and the command line parsing follow) and returns JSON.
//! Help texts are English (`tr_const!`); the command line shows them translated, MCP
//! clients get them as they are.

mod common;
mod config;
mod devices;
mod drawing;
mod electrical;
mod engineering;
mod field;
mod languages;
mod output;
mod pages;
mod panel;
mod panel_checks;
mod parts;
mod plc;
mod plugs;
mod potentials;
mod process;
mod reports;
mod project;
mod properties;
mod routing;
mod sheets;
mod singleline;
mod structure;
mod tables;
mod wiring;

use crate::session::Session;
use openecraft_i18n::{tr, tr_fmt};
use serde_json::{json, Map, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Str,
    Int,
    Num,
    Bool,
    /// List of item ids.
    Ids,
    /// List of `[x, y]` points in mm.
    Points,
    /// JSON object.
    Object,
}

pub struct Param {
    pub name: &'static str,
    pub kind: Kind,
    pub required: bool,
    pub help: &'static str,
}

pub type Run = fn(&mut Session, &Args) -> Result<Value, String>;

pub struct Tool {
    pub name: &'static str,
    pub help: &'static str,
    pub params: &'static [Param],
    /// Works on the open project (the command line opens `--project` first).
    pub needs_project: bool,
    pub run: Run,
}

pub const fn param(name: &'static str, kind: Kind, help: &'static str) -> Param {
    Param { name, kind, required: true, help }
}

pub const fn optional(name: &'static str, kind: Kind, help: &'static str) -> Param {
    Param { name, kind, required: false, help }
}

pub fn all() -> impl Iterator<Item = &'static Tool> {
    [project::TOOLS, pages::TOOLS, sheets::TOOLS, drawing::TOOLS, devices::TOOLS, plugs::TOOLS, plc::TOOLS, potentials::TOOLS, process::TOOLS, singleline::TOOLS, tables::TOOLS, engineering::TOOLS, electrical::TOOLS, panel::TOOLS, panel_checks::TOOLS, routing::TOOLS, field::TOOLS, wiring::TOOLS, languages::TOOLS, structure::TOOLS, properties::TOOLS, config::TOOLS, output::TOOLS, reports::TOOLS, parts::TOOLS].into_iter().flatten()
}

pub fn find(name: &str) -> Option<&'static Tool> {
    all().find(|t| t.name == name)
}

/// Checks the parameters against the tool and runs it. Changes of the project become
/// one undo step.
pub fn call(session: &mut Session, tool: &Tool, params: &Map<String, Value>) -> Result<Value, String> {
    for name in params.keys() {
        if !tool.params.iter().any(|p| p.name == name) {
            return Err(tr_fmt!("Unknown parameter: {}", name));
        }
    }
    for p in tool.params {
        match params.get(p.name) {
            None | Some(Value::Null) if p.required => return Err(tr_fmt!("Missing parameter: {}", p.name)),
            None | Some(Value::Null) => {}
            Some(v) if !fits(p.kind, v) => return Err(tr_fmt!("Parameter {} has the wrong type", p.name)),
            Some(_) => {}
        }
    }
    if tool.needs_project {
        session.project()?;
    }
    let result = (tool.run)(session, &Args(params));
    session.commit();
    result
}

fn fits(kind: Kind, v: &Value) -> bool {
    let point = |p: &Value| p.as_array().is_some_and(|a| a.len() == 2 && a.iter().all(Value::is_number));
    match kind {
        Kind::Str => v.is_string(),
        Kind::Int => v.is_i64() || v.is_u64(),
        Kind::Num => v.is_number(),
        Kind::Bool => v.is_boolean(),
        Kind::Ids => v.as_array().is_some_and(|a| a.iter().all(Value::is_u64)),
        Kind::Points => v.as_array().is_some_and(|a| a.iter().all(point)),
        Kind::Object => v.is_object(),
    }
}

/// JSON schema of the tool's parameters (MCP `inputSchema`).
pub fn schema(tool: &Tool) -> Value {
    let mut props = Map::new();
    for p in tool.params {
        let mut s = match p.kind {
            Kind::Str => json!({ "type": "string" }),
            Kind::Int => json!({ "type": "integer" }),
            Kind::Num => json!({ "type": "number" }),
            Kind::Bool => json!({ "type": "boolean" }),
            Kind::Ids => json!({ "type": "array", "items": { "type": "integer", "minimum": 0 } }),
            Kind::Points => json!({
                "type": "array",
                "items": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2 }
            }),
            Kind::Object => json!({ "type": "object" }),
        };
        s["description"] = p.help.into();
        props.insert(p.name.into(), s);
    }
    let required: Vec<&str> = tool.params.iter().filter(|p| p.required).map(|p| p.name).collect();
    json!({ "type": "object", "properties": props, "required": required, "additionalProperties": false })
}

/// Parameters from the command line: `--json` first, then `--name value` pairs
/// converted to the parameter's type.
pub fn from_cli(tool: &Tool, json: Option<&str>, pairs: &[(String, Option<String>)]) -> Result<Map<String, Value>, String> {
    let mut out = match json {
        Some(text) => match serde_json::from_str(text) {
            Ok(Value::Object(map)) => map,
            _ => return Err(tr("--json needs a JSON object").to_string()),
        },
        None => Map::new(),
    };
    for (name, raw) in pairs {
        let p = tool.params.iter().find(|p| p.name == name).ok_or_else(|| tr_fmt!("Unknown parameter: {}", name))?;
        let value = match (p.kind, raw) {
            (Kind::Bool, None) => Value::Bool(true),
            (_, None) => return Err(tr_fmt!("Missing value for --{}", name)),
            (kind, Some(raw)) => cli_value(kind, raw).ok_or_else(|| tr_fmt!("Parameter {} has the wrong type", name))?,
        };
        out.insert(name.clone(), value);
    }
    Ok(out)
}

fn cli_value(kind: Kind, raw: &str) -> Option<Value> {
    match kind {
        Kind::Str => Some(Value::String(raw.into())),
        Kind::Int => raw.trim().parse::<i64>().ok().map(Value::from),
        Kind::Num => raw.trim().parse::<f64>().ok().map(Value::from),
        Kind::Bool => match raw.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" => Some(Value::Bool(true)),
            "false" | "no" | "0" => Some(Value::Bool(false)),
            _ => None,
        },
        // `1,2,3` as well as `[1,2,3]`.
        Kind::Ids if !raw.trim_start().starts_with('[') => {
            raw.split(',').map(|s| s.trim().parse::<u64>().ok().map(Value::from)).collect::<Option<Vec<_>>>().map(Value::Array)
        }
        // `x,y x,y …` as well as `[[x,y],…]`.
        Kind::Points if !raw.trim_start().starts_with('[') => raw
            .split_whitespace()
            .map(|pt| {
                let (x, y) = pt.split_once(',')?;
                Some(json!([x.trim().parse::<f64>().ok()?, y.trim().parse::<f64>().ok()?]))
            })
            .collect::<Option<Vec<_>>>()
            .map(Value::Array),
        _ => serde_json::from_str(raw).ok(),
    }
}

/// Command line help of a tool.
pub fn help(tool: &Tool) -> String {
    let mut out = format!("openecraft-cli {}{}\n\n{}\n", tool.name, if tool.needs_project { " --project <file>" } else { "" }, tr(tool.help));
    if !tool.params.is_empty() {
        out.push('\n');
        for p in tool.params {
            let value = match p.kind {
                Kind::Str => "<text>",
                Kind::Int | Kind::Num => "<number>",
                Kind::Bool => "[true|false]",
                Kind::Ids => "<id,id,…>",
                Kind::Points => "<x,y x,y …>",
                Kind::Object => "<json>",
            };
            let arg = format!("--{} {value}", p.name);
            let mark = if p.required { String::new() } else { format!("{} ", tr("(optional)")) };
            out.push_str(&format!("  {arg:<28} {mark}{}\n", tr(p.help)));
        }
    }
    out
}

/// Typed access to checked parameters.
pub struct Args<'a>(pub &'a Map<String, Value>);

impl Args<'_> {
    pub fn str(&self, name: &str) -> Option<&str> {
        self.0.get(name).and_then(Value::as_str)
    }

    /// A parameter the tool declared as required.
    pub fn req(&self, name: &str) -> &str {
        self.str(name).unwrap_or_default()
    }

    pub fn num(&self, name: &str) -> Option<f32> {
        self.0.get(name).and_then(Value::as_f64).map(|v| v as f32)
    }

    pub fn int(&self, name: &str) -> Option<i64> {
        self.0.get(name).and_then(Value::as_i64)
    }

    pub fn flag(&self, name: &str) -> bool {
        self.0.get(name).and_then(Value::as_bool).unwrap_or(false)
    }

    pub fn ids(&self, name: &str) -> Vec<u64> {
        self.0.get(name).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_u64).collect()).unwrap_or_default()
    }

    pub fn points(&self, name: &str) -> Vec<[f32; 2]> {
        let pt = |v: &Value| Some([v.get(0)?.as_f64()? as f32, v.get(1)?.as_f64()? as f32]);
        self.0.get(name).and_then(Value::as_array).map(|a| a.iter().filter_map(pt).collect()).unwrap_or_default()
    }

    pub fn value(&self, name: &str) -> Option<&Value> {
        self.0.get(name)
    }
}

#[cfg(test)]
mod tests;
