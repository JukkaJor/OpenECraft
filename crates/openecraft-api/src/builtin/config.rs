//! Configurator commands: options, rules and generating pages from a macro library file.

use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::config::{ConfigOption, Rule};
use openecraft_core::model::Project;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct Configurator;

impl Plugin for Configurator {
    fn info(&self) -> Info {
        info("configurator", tr_const!("Configurator"), tr_const!("Options and rules of the configurator"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "get_configuration",
        help: tr_const!("Shows the options and rules of the configurator"),
        params: &[],
        run: get_configuration,
    },
    Spec {
        name: "set_option",
        help: tr_const!("Creates or changes a configurator option"),
        params: &[
            param("name", Kind::Str, tr_const!("Option name, e.g. STARTER")),
            optional("choices", Kind::Str, tr_const!("Allowed values separated by commas (empty: any value)")),
            optional("value", Kind::Str, tr_const!("Chosen value")),
        ],
        run: set_option,
    },
    Spec {
        name: "delete_option",
        help: tr_const!("Deletes a configurator option"),
        params: &[param("name", Kind::Str, tr_const!("Option name"))],
        run: delete_option,
    },
    Spec {
        name: "add_rule",
        help: tr_const!("Adds a configurator rule: insert a page macro when the condition holds"),
        params: &[
            param("macro", Kind::Str, tr_const!("Page macro name")),
            optional("variant", Kind::Str, tr_const!("Variant (empty: the first)")),
            optional("values", Kind::Str, tr_const!("Placeholder values KEY=value; KEY2=value (may use {OPTION})")),
            optional("condition", Kind::Str, tr_const!("OPTION=value, OPTION!=value, OPTION=a|b joined by & (empty: always)")),
        ],
        run: add_rule,
    },
    Spec {
        name: "delete_rule",
        help: tr_const!("Deletes a configurator rule"),
        params: &[param("number", Kind::Int, tr_const!("Rule number, from 1"))],
        run: delete_rule,
    },
];

fn configuration(prj: &Project) -> Result<Value, String> {
    let cfg = &prj.configuration;
    Ok(json!({
        "options": cfg.options.iter().map(|o| json!({ "name": o.name, "choices": o.choices, "value": o.value })).collect::<Vec<_>>(),
        "rules": cfg.rules.iter().enumerate().map(|(i, r)| json!({ "number": i + 1, "macro": r.macro_name, "variant": r.variant, "values": r.values, "condition": r.condition })).collect::<Vec<_>>(),
    }))
}

fn get_configuration(prj: &mut Project, _: &Args) -> Result<Value, String> {
    configuration(prj)
}

fn set_option(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let cfg = &mut prj.configuration;
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
    configuration(prj)
}

fn delete_option(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let cfg = &mut prj.configuration;
    let i = cfg.options.iter().position(|o| o.name == a.req("name")).ok_or_else(|| tr_fmt!("Unknown option {}", a.req("name")))?;
    cfg.options.remove(i);
    configuration(prj)
}

fn add_rule(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let rule = Rule {
        macro_name: a.req("macro").into(),
        variant: a.str("variant").unwrap_or("").into(),
        values: a.str("values").unwrap_or("").into(),
        condition: a.str("condition").unwrap_or("").into(),
    };
    prj.configuration.rules.push(rule);
    configuration(prj)
}

fn delete_rule(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let cfg = &mut prj.configuration;
    let n = a.int("number").unwrap_or(0);
    if n < 1 || n as usize > cfg.rules.len() {
        return Err(tr_fmt!("No rule {}", n));
    }
    cfg.rules.remove(n as usize - 1);
    configuration(prj)
}
