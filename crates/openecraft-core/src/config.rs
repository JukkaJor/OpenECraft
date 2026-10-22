//! Project configurator: options (e.g. `STARTER` = DOL / star-delta, `MOTORS` = 2)
//! and generation rules that insert page macros (in a variant, with placeholder
//! values) when their condition holds. Generating replaces the pages generated before,
//! so the project follows the options; option values fill `{OPTION}` placeholders.

use openecraft_i18n::{tr, tr_fmt};
use crate::macros::{MacroKind, MacroLibrary};
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// An option with its choices and the chosen value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ConfigOption {
    pub name: String,
    /// Allowed values; empty: any value.
    pub choices: Vec<String>,
    pub value: String,
}

/// Inserts a page macro when the condition holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Rule {
    #[serde(rename = "macro")]
    pub macro_name: String,
    /// Variant of the macro; empty: its first variant.
    pub variant: String,
    /// Placeholder values `KEY=value; KEY2=value` (may use `{OPTION}`).
    pub values: String,
    /// `OPTION=value`, `OPTION!=value`, `OPTION=a|b`, joined by `&`; empty: always.
    pub condition: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Configuration {
    pub options: Vec<ConfigOption>,
    pub rules: Vec<Rule>,
}

impl Configuration {
    pub fn is_empty(&self) -> bool {
        self.options.is_empty() && self.rules.is_empty()
    }

    pub fn option(&self, name: &str) -> Option<&ConfigOption> {
        self.options.iter().find(|o| o.name == name)
    }

    /// Option values by name.
    pub fn values(&self) -> BTreeMap<String, String> {
        self.options.iter().map(|o| (o.name.clone(), o.value.clone())).collect()
    }

    /// Whether a condition holds for the chosen option values.
    pub fn holds(&self, condition: &str) -> Result<bool, String> {
        for clause in condition.split('&').map(str::trim).filter(|c| !c.is_empty()) {
            let (name, values, negated) = match clause.split_once("!=") {
                Some((n, v)) => (n, v, true),
                None => match clause.split_once('=') {
                    Some((n, v)) => (n, v, false),
                    None => return Err(tr_fmt!("Condition '{}' needs OPTION=value", clause)),
                },
            };
            let Some(o) = self.option(name.trim()) else { return Err(tr_fmt!("Unknown option {}", name.trim())) };
            let any = values.split('|').any(|v| v.trim() == o.value.trim());
            if any == negated {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// `KEY=value; KEY2=value` → map (keys and values trimmed).
pub fn parse_values(text: &str) -> BTreeMap<String, String> {
    text.split(';')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

fn fill(text: &str, values: &BTreeMap<String, String>) -> String {
    let mut out = text.to_string();
    for (k, v) in values {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Result of a generation.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Generated {
    pub removed: usize,
    /// Names of the inserted pages.
    pub pages: Vec<String>,
}

/// Problems of the rules (unknown macro, variant or option, window macros): one
/// message per rule, numbered from 1.
pub fn problems(cfg: &Configuration, lib: &MacroLibrary) -> Vec<String> {
    let mut v = Vec::new();
    for (i, r) in cfg.rules.iter().enumerate() {
        let n = i + 1;
        match lib.get(&r.macro_name) {
            None => v.push(tr_fmt!("Rule {}: no macro {}", n, r.macro_name)),
            Some(m) if m.kind != MacroKind::Page => v.push(tr_fmt!("Rule {}: {} is no page macro", n, r.macro_name)),
            Some(m) if m.with_variant(&r.variant).is_none() => v.push(tr_fmt!("Rule {}: {} has no variant {}", n, r.macro_name, r.variant)),
            Some(_) => {}
        }
        if let Err(e) = cfg.holds(&r.condition) {
            v.push(tr_fmt!("Rule {}: {}", n, e));
        }
    }
    for o in cfg.options.iter().filter(|o| !o.choices.is_empty() && !o.choices.contains(&o.value)) {
        v.push(tr_fmt!("Option {}: {} is not one of its choices", o.name, o.value));
    }
    v
}

/// Removes the pages generated before and inserts the page macros of the rules whose
/// condition holds, at the end of the project. Nothing changes if a rule has a
/// problem or a generated page is released.
pub fn generate(prj: &mut Project, lib: &MacroLibrary) -> Result<Generated, String> {
    let cfg = prj.configuration.clone();
    if let Some(p) = problems(&cfg, lib).into_iter().next() {
        return Err(p);
    }
    if let Some(pg) = prj.pages.iter().find(|p| p.generated && p.locked) {
        return Err(tr_fmt!("Generated page {} is released", pg.name));
    }
    let before = prj.pages.len();
    prj.pages.retain(|p| !p.generated);
    let mut out = Generated { removed: before - prj.pages.len(), pages: Vec::new() };
    if prj.pages.is_empty() {
        let id = prj.new_id();
        prj.pages.push(Page::new(id, "1", "", PageKind::Schematic));
    }
    let options = cfg.values();
    for r in &cfg.rules {
        if !cfg.holds(&r.condition)? {
            continue;
        }
        let m = lib.get(&r.macro_name).and_then(|m| m.with_variant(&r.variant)).ok_or_else(|| tr("Unknown macro").to_string())?;
        let mut values = options.clone();
        for (k, v) in parse_values(&r.values) {
            values.insert(k, fill(&v, &options));
        }
        let at = crate::macros::insert_page(prj, prj.pages.len() - 1, &m, &values);
        prj.pages[at].generated = true;
        out.pages.push(prj.pages[at].name.clone());
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
