//! Configurator tools: options, rules and generating pages from a macro library file.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::config::{self, ConfigOption, Rule};
use openecraft_core::macros::MacroLibrary;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "get_configuration",
        help: tr_const!("Shows the options and rules of the configurator"),
        params: &[],
        needs_project: true,
        run: get_configuration,
    },
    Tool {
        name: "set_option",
        help: tr_const!("Creates or changes a configurator option"),
        params: &[
            param("name", Kind::Str, tr_const!("Option name, e.g. STARTER")),
            optional("choices", Kind::Str, tr_const!("Allowed values separated by commas (empty: any value)")),
            optional("value", Kind::Str, tr_const!("Chosen value")),
        ],
        needs_project: true,
        run: set_option,
    },
    Tool {
        name: "delete_option",
        help: tr_const!("Deletes a configurator option"),
        params: &[param("name", Kind::Str, tr_const!("Option name"))],
        needs_project: true,
        run: delete_option,
    },
    Tool {
        name: "add_rule",
        help: tr_const!("Adds a configurator rule: insert a page macro when the condition holds"),
        params: &[
            param("macro", Kind::Str, tr_const!("Page macro name")),
            optional("variant", Kind::Str, tr_const!("Variant (empty: the first)")),
            optional("values", Kind::Str, tr_const!("Placeholder values KEY=value; KEY2=value (may use {OPTION})")),
            optional("condition", Kind::Str, tr_const!("OPTION=value, OPTION!=value, OPTION=a|b joined by & (empty: always)")),
        ],
        needs_project: true,
        run: add_rule,
    },
    Tool {
        name: "delete_rule",
        help: tr_const!("Deletes a configurator rule"),
        params: &[param("number", Kind::Int, tr_const!("Rule number, from 1"))],
        needs_project: true,
        run: delete_rule,
    },
    Tool {
        name: "generate_pages",
        help: tr_const!("Replaces the pages generated before by the page macros of the rules whose condition holds"),
        params: &[param("macros", Kind::Str, tr_const!("Macro library file (JSON, exported by the editor)"))],
        needs_project: true,
        run: generate_pages,
    },
];

fn configuration(s: &Session) -> Result<Value, String> {
    let cfg = &s.project()?.configuration;
    Ok(json!({
        "options": cfg.options.iter().map(|o| json!({ "name": o.name, "choices": o.choices, "value": o.value })).collect::<Vec<_>>(),
        "rules": cfg.rules.iter().enumerate().map(|(i, r)| json!({ "number": i + 1, "macro": r.macro_name, "variant": r.variant, "values": r.values, "condition": r.condition })).collect::<Vec<_>>(),
    }))
}

fn get_configuration(s: &mut Session, _: &Args) -> Result<Value, String> {
    configuration(s)
}

fn set_option(s: &mut Session, a: &Args) -> Result<Value, String> {
    let cfg = &mut s.project_mut()?.configuration;
    let name = a.req("name").trim().to_string();
    if name.is_empty() {
        return Err(tr_fmt!("Missing parameter: {}", "name"));
    }
    let mut o = cfg.options.iter().find(|o| o.name == name).cloned().unwrap_or(ConfigOption { name, ..Default::default() });
    if let Some(c) = a.str("choices") {
        o.choices = c.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
    }
    if let Some(v) = a.str("value") {
        o.value = v.trim().into();
    }
    if !o.choices.is_empty() && !o.choices.contains(&o.value) {
        return Err(tr_fmt!("Option {}: {} is not one of its choices", o.name, o.value));
    }
    match cfg.options.iter_mut().find(|x| x.name == o.name) {
        Some(x) => *x = o,
        None => cfg.options.push(o),
    }
    configuration(s)
}

fn delete_option(s: &mut Session, a: &Args) -> Result<Value, String> {
    let cfg = &mut s.project_mut()?.configuration;
    let i = cfg.options.iter().position(|o| o.name == a.req("name")).ok_or_else(|| tr_fmt!("Unknown option {}", a.req("name")))?;
    cfg.options.remove(i);
    configuration(s)
}

fn add_rule(s: &mut Session, a: &Args) -> Result<Value, String> {
    let rule = Rule {
        macro_name: a.req("macro").into(),
        variant: a.str("variant").unwrap_or("").into(),
        values: a.str("values").unwrap_or("").into(),
        condition: a.str("condition").unwrap_or("").into(),
    };
    s.project_mut()?.configuration.rules.push(rule);
    configuration(s)
}

fn delete_rule(s: &mut Session, a: &Args) -> Result<Value, String> {
    let cfg = &mut s.project_mut()?.configuration;
    let n = a.int("number").unwrap_or(0);
    if n < 1 || n as usize > cfg.rules.len() {
        return Err(tr_fmt!("No rule {}", n));
    }
    cfg.rules.remove(n as usize - 1);
    configuration(s)
}

fn generate_pages(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("macros");
    let text = std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let lib = MacroLibrary::from_json(&text).map_err(|e| tr_fmt!("Not a macro file: {}", e))?;
    let g = config::generate(s.project_mut()?, &lib)?;
    Ok(json!({ "removed": g.removed, "pages": g.pages }))
}

#[cfg(test)]
mod tests;
