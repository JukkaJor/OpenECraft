//! Project language commands: source and target languages of the project texts and the
//! translation dictionary. Output per language: `export` with `language`.

use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::model::Project;
use openecraft_core::translation::{self, normalize_code};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Map, Value};

pub struct Languages;

impl Plugin for Languages {
    fn info(&self) -> Info {
        info("languages", tr_const!("Languages"), tr_const!("Project languages, dictionary and translations"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "list_project_languages",
        help: tr_const!("Lists the source and target languages of the project texts with the number of untranslated lines"),
        params: &[],
        run: list_languages,
    },
    Spec {
        name: "set_project_languages",
        help: tr_const!("Sets the source language of the project texts and the target languages (ISO 639-1 codes)"),
        params: &[
            optional("source", Kind::Str, tr_const!("Language the project texts are written in, e.g. en")),
            optional("targets", Kind::Str, tr_const!("Target languages separated by blanks, e.g. de fr")),
        ],
        run: set_languages,
    },
    Spec {
        name: "list_dictionary",
        help: tr_const!("Lists the translation dictionary of the project texts (source line and translations)"),
        params: &[
            optional("missing", Kind::Str, tr_const!("Only lines without a translation into this language")),
            optional("query", Kind::Str, tr_const!("Text in the line or a translation")),
        ],
        run: list_dictionary,
    },
    Spec {
        name: "set_translation",
        help: tr_const!("Sets the translation of a line of the project texts (empty removes it)"),
        params: &[
            param("text", Kind::Str, tr_const!("Line in the source language")),
            param("language", Kind::Str, tr_const!("Target language code")),
            param("translation", Kind::Str, tr_const!("Translated line")),
        ],
        run: set_translation,
    },
    Spec {
        name: "collect_texts",
        help: tr_const!("Adds all lines of the project texts to the dictionary, optionally removing entries no longer used"),
        params: &[optional("remove_unused", Kind::Bool, tr_const!("Also remove entries the project no longer uses"))],
        run: collect_texts,
    },
];

fn code(raw: &str) -> Result<String, String> {
    normalize_code(raw).ok_or_else(|| tr_fmt!("Not a language code: {}", raw))
}

pub fn list(prj: &Project) -> Result<Value, String> {
    let targets: Vec<Value> = prj
        .languages
        .targets
        .iter()
        .map(|l| json!({ "code": l, "untranslated": translation::missing(prj, l).len() }))
        .collect();
    Ok(json!({ "source": prj.languages.source, "targets": targets, "dictionary_entries": prj.languages.dictionary.len() }))
}

fn list_languages(prj: &mut Project, _: &Args) -> Result<Value, String> {
    list(prj)
}

fn set_languages(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let source = a.str("source").map(code).transpose()?;
    let targets = a.str("targets").map(|t| t.split_whitespace().map(code).collect::<Result<Vec<_>, _>>()).transpose()?;
    if let Some(src) = source {
        prj.languages.source = src;
    }
    if let Some(mut t) = targets {
        t.dedup();
        prj.languages.targets = t;
    }
    let src = prj.languages.source.clone();
    prj.languages.targets.retain(|l| *l != src);
    list(prj)
}

fn list_dictionary(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let missing = a.str("missing").map(code).transpose()?;
    let query = a.str("query").unwrap_or_default().to_lowercase();
    let entries: Vec<Value> = prj
        .languages
        .dictionary
        .iter()
        .filter(|(line, t)| query.is_empty() || line.to_lowercase().contains(&query) || t.values().any(|v| v.to_lowercase().contains(&query)))
        .filter(|(line, _)| missing.as_ref().is_none_or(|l| prj.languages.lookup(line, l).is_none()))
        .map(|(line, t)| {
            let translations: Map<String, Value> = t.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect();
            json!({ "text": line, "translations": translations })
        })
        .collect();
    Ok(Value::Array(entries))
}

fn set_translation(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let lang = code(a.req("language"))?;
    if lang == prj.languages.source {
        return Err(tr_fmt!("{} is the source language", lang));
    }
    if !prj.languages.targets.contains(&lang) {
        prj.languages.targets.push(lang.clone());
    }
    translation::set(prj, a.req("text"), &lang, a.req("translation"));
    list(prj)
}

fn collect_texts(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let added = translation::collect(prj);
    let removed = if a.flag("remove_unused") { translation::remove_unused(prj) } else { 0 };
    let mut v = list(prj)?;
    v["added"] = added.into();
    v["removed"] = removed.into();
    Ok(v)
}

#[cfg(test)]
mod tests;
