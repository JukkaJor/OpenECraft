//! Project language tools: source and target languages of the project texts and the
//! translation dictionary. Output per language: `export` with `language`.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::translation::{self, normalize_code};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Map, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "list_project_languages",
        help: tr_const!("Lists the source and target languages of the project texts with the number of untranslated lines"),
        params: &[],
        needs_project: true,
        run: list_languages,
    },
    Tool {
        name: "set_project_languages",
        help: tr_const!("Sets the source language of the project texts and the target languages (ISO 639-1 codes)"),
        params: &[
            optional("source", Kind::Str, tr_const!("Language the project texts are written in, e.g. en")),
            optional("targets", Kind::Str, tr_const!("Target languages separated by blanks, e.g. de fr")),
        ],
        needs_project: true,
        run: set_languages,
    },
    Tool {
        name: "list_dictionary",
        help: tr_const!("Lists the translation dictionary of the project texts (source line and translations)"),
        params: &[
            optional("missing", Kind::Str, tr_const!("Only lines without a translation into this language")),
            optional("query", Kind::Str, tr_const!("Text in the line or a translation")),
        ],
        needs_project: true,
        run: list_dictionary,
    },
    Tool {
        name: "set_translation",
        help: tr_const!("Sets the translation of a line of the project texts (empty removes it)"),
        params: &[
            param("text", Kind::Str, tr_const!("Line in the source language")),
            param("language", Kind::Str, tr_const!("Target language code")),
            param("translation", Kind::Str, tr_const!("Translated line")),
        ],
        needs_project: true,
        run: set_translation,
    },
    Tool {
        name: "collect_texts",
        help: tr_const!("Adds all lines of the project texts to the dictionary, optionally removing entries no longer used"),
        params: &[optional("remove_unused", Kind::Bool, tr_const!("Also remove entries the project no longer uses"))],
        needs_project: true,
        run: collect_texts,
    },
    Tool {
        name: "import_dictionary",
        help: tr_const!("Reads translations from CSV: first column the source line, one column per language code"),
        params: &[
            optional("path", Kind::Str, tr_const!("CSV file")),
            optional("csv", Kind::Str, tr_const!("CSV content instead of a file")),
        ],
        needs_project: true,
        run: import_dictionary,
    },
];

fn code(raw: &str) -> Result<String, String> {
    normalize_code(raw).ok_or_else(|| tr_fmt!("Not a language code: {}", raw))
}

fn list(s: &Session) -> Result<Value, String> {
    let prj = s.project()?;
    let targets: Vec<Value> = prj
        .languages
        .targets
        .iter()
        .map(|l| json!({ "code": l, "untranslated": translation::missing(prj, l).len() }))
        .collect();
    Ok(json!({ "source": prj.languages.source, "targets": targets, "dictionary_entries": prj.languages.dictionary.len() }))
}

fn list_languages(s: &mut Session, _: &Args) -> Result<Value, String> {
    list(s)
}

fn set_languages(s: &mut Session, a: &Args) -> Result<Value, String> {
    let source = a.str("source").map(code).transpose()?;
    let targets = a.str("targets").map(|t| t.split_whitespace().map(code).collect::<Result<Vec<_>, _>>()).transpose()?;
    let prj = s.project_mut()?;
    if let Some(src) = source {
        prj.languages.source = src;
    }
    if let Some(mut t) = targets {
        t.dedup();
        prj.languages.targets = t;
    }
    let src = prj.languages.source.clone();
    prj.languages.targets.retain(|l| *l != src);
    list(s)
}

fn list_dictionary(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
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

fn set_translation(s: &mut Session, a: &Args) -> Result<Value, String> {
    let lang = code(a.req("language"))?;
    let prj = s.project_mut()?;
    if lang == prj.languages.source {
        return Err(tr_fmt!("{} is the source language", lang));
    }
    if !prj.languages.targets.contains(&lang) {
        prj.languages.targets.push(lang.clone());
    }
    translation::set(prj, a.req("text"), &lang, a.req("translation"));
    list(s)
}

fn collect_texts(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let added = translation::collect(prj);
    let removed = if a.flag("remove_unused") { translation::remove_unused(prj) } else { 0 };
    let mut v = list(s)?;
    v["added"] = added.into();
    v["removed"] = removed.into();
    Ok(v)
}

fn import_dictionary(s: &mut Session, a: &Args) -> Result<Value, String> {
    let text = match (a.str("csv"), a.str("path")) {
        (Some(csv), _) => csv.to_string(),
        (None, Some(path)) => std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?,
        (None, None) => return Err(tr_fmt!("Missing parameter: {}", "path")),
    };
    let n = translation::from_csv(s.project_mut()?, &text)?;
    let mut v = list(s)?;
    v["imported"] = n.into();
    Ok(v)
}

#[cfg(test)]
mod tests;
