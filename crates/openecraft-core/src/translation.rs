//! Project texts in several languages (in the spirit of EPLAN's multi-language
//! texts): the texts of the project (function texts, page descriptions, free texts,
//! device and cable descriptions …) are written in the source language; a
//! translation dictionary of the project maps each line to the target languages.
//! Documentation is put out per language: [`with_language`] translates the project
//! texts and generates the reports, title blocks and check messages in that
//! language, too, if the UI knows it.

use openecraft_i18n::{tr, Lang};
use crate::model::Project;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Languages {
    /// Language the project texts are written in (ISO 639-1 code).
    pub source: String,
    /// Further languages of the documentation.
    pub targets: Vec<String>,
    /// Source line → language code → translation.
    pub dictionary: BTreeMap<String, BTreeMap<String, String>>,
}

impl Default for Languages {
    fn default() -> Self {
        Languages { source: "en".into(), targets: Vec::new(), dictionary: BTreeMap::new() }
    }
}

impl Languages {
    pub fn is_default(&self) -> bool {
        *self == Languages::default()
    }

    /// Source and target languages.
    pub fn all(&self) -> Vec<String> {
        std::iter::once(self.source.clone()).chain(self.targets.iter().cloned()).collect()
    }

    pub fn lookup(&self, line: &str, lang: &str) -> Option<&str> {
        self.dictionary.get(line.trim())?.get(lang).map(String::as_str).filter(|t| !t.trim().is_empty())
    }

    /// A text in `lang`: every line with a translation is replaced, the others stay.
    pub fn translate(&self, text: &str, lang: &str) -> String {
        if lang == self.source || !text.chars().any(char::is_alphabetic) {
            return text.to_string();
        }
        text.split('\n')
            .map(|line| match self.lookup(line, lang) {
                Some(t) => {
                    let indent = &line[..line.len() - line.trim_start().len()];
                    format!("{indent}{t}")
                }
                None => line.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Normalized language code: lower case, `de_DE` → `de-de`; None if it is no code.
pub fn normalize_code(code: &str) -> Option<String> {
    let c = code.trim().to_ascii_lowercase().replace('_', "-");
    let mut parts = c.split('-');
    let primary = parts.next()?;
    let ok = (2..=3).contains(&primary.len()) && primary.chars().all(|ch| ch.is_ascii_lowercase()) && parts.all(|p| (2..=8).contains(&p.len()) && p.chars().all(|ch| ch.is_ascii_alphanumeric()));
    ok.then_some(c)
}

/// Name of a language for menus: the native name of the UI languages, else the code.
pub fn language_name(code: &str) -> String {
    match Lang::from_code(code) {
        Some(l) if l.code() == code => format!("{} ({code})", l.native_name()),
        _ => code.to_string(),
    }
}

/// Calls `f` for every translatable text of the project.
fn visit(prj: &mut Project, f: &mut dyn FnMut(&mut String)) {
    f(&mut prj.info.description);
    for pg in &mut prj.pages {
        f(&mut pg.description);
        pg.symbols.iter_mut().for_each(|s| f(&mut s.function_text));
        pg.texts.iter_mut().for_each(|t| f(&mut t.text));
    }
    prj.devices.values_mut().for_each(|d| f(&mut d.description));
    prj.cables.iter_mut().for_each(|c| f(&mut c.description));
    prj.plc_cards.values_mut().for_each(|c| f(&mut c.description));
    prj.harnesses.iter_mut().for_each(|h| f(&mut h.description));
    prj.potentials.iter_mut().for_each(|p| f(&mut p.description));
    prj.pct_loops.iter_mut().for_each(|l| f(&mut l.description));
    prj.revisions.iter_mut().for_each(|r| f(&mut r.description));
}

/// The lines of the project texts that are translated (with letters), sorted.
pub fn source_lines(prj: &Project) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    visit(&mut prj.clone(), &mut |t| {
        out.extend(t.split('\n').map(str::trim).filter(|l| l.chars().any(char::is_alphabetic)).map(String::from));
    });
    out
}

/// The project with its texts in `lang`.
pub fn translated(prj: &Project, lang: &str) -> Project {
    let mut out = prj.clone();
    if lang != prj.languages.source {
        let langs = prj.languages.clone();
        visit(&mut out, &mut |t| *t = langs.translate(t, lang));
    }
    out
}

/// Generates output in `lang`: project texts from the dictionary, generated texts
/// (reports, title blocks) in that language if the UI has it, otherwise in English.
pub fn with_language<R>(prj: &Project, lang: &str, f: impl FnOnce(&Project) -> R) -> R {
    let t = translated(prj, lang);
    match Lang::from_code(lang) {
        Some(l) => openecraft_i18n::with_lang(l, || f(&t)),
        None => openecraft_i18n::with_lang(Lang::En, || f(&t)),
    }
}

/// Source lines without a translation into `lang`.
pub fn missing(prj: &Project, lang: &str) -> Vec<String> {
    if lang == prj.languages.source {
        return Vec::new();
    }
    source_lines(prj).into_iter().filter(|l| prj.languages.lookup(l, lang).is_none()).collect()
}

/// Adds the lines of the project texts to the dictionary. Returns how many are new.
pub fn collect(prj: &mut Project) -> usize {
    let mut n = 0;
    for line in source_lines(prj) {
        if let std::collections::btree_map::Entry::Vacant(e) = prj.languages.dictionary.entry(line) {
            e.insert(BTreeMap::new());
            n += 1;
        }
    }
    n
}

/// Removes dictionary entries the project no longer uses. Returns how many.
pub fn remove_unused(prj: &mut Project) -> usize {
    let lines = source_lines(prj);
    let before = prj.languages.dictionary.len();
    prj.languages.dictionary.retain(|k, _| lines.contains(k));
    before - prj.languages.dictionary.len()
}

/// Sets (or with an empty text removes) the translation of a source line.
pub fn set(prj: &mut Project, line: &str, lang: &str, translation: &str) {
    let entry = prj.languages.dictionary.entry(line.trim().to_string()).or_default();
    if translation.trim().is_empty() {
        entry.remove(lang);
    } else {
        entry.insert(lang.to_string(), translation.trim().to_string());
    }
}

/// Dictionary as CSV for translators: one column per language, source first.
pub fn to_csv(prj: &Project) -> String {
    let langs = &prj.languages;
    let esc = |s: &str| if s.contains([';', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
    let mut out = langs.all().join(";");
    out.push('\n');
    for (line, tr) in &langs.dictionary {
        let mut row = vec![esc(line)];
        row.extend(langs.targets.iter().map(|l| esc(tr.get(l).map(String::as_str).unwrap_or(""))));
        out.push_str(&row.join(";"));
        out.push('\n');
    }
    out
}

fn csv_fields(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ';' | '\t' if !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// Reads a dictionary CSV (first column the source text, one column per language
/// code); non-empty translations are taken over, new languages are added as targets.
/// Returns the number of translations read.
pub fn from_csv(prj: &mut Project, text: &str) -> Result<usize, String> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header = csv_fields(lines.next().ok_or_else(|| tr("The file is empty").to_string())?);
    let mut codes = Vec::new();
    for h in header.iter().skip(1) {
        codes.push(normalize_code(h.trim_start_matches('\u{feff}')).ok_or_else(|| openecraft_i18n::tr_fmt!("Not a language code: {}", h))?);
    }
    for c in &codes {
        if *c != prj.languages.source && !prj.languages.targets.contains(c) {
            prj.languages.targets.push(c.clone());
        }
    }
    let mut n = 0;
    for line in lines {
        let f = csv_fields(line);
        let Some(source) = f.first().map(|s| s.trim()).filter(|s| !s.is_empty()) else { continue };
        for (code, value) in codes.iter().zip(f.iter().skip(1)) {
            if !value.trim().is_empty() && *code != prj.languages.source {
                set(prj, source, code, value);
                n += 1;
            }
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests;
