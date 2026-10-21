//! Macros: reusable circuit parts. A window macro is a selection of a page, a page
//! macro a complete page. Placeholders `{NAME}` in tags, texts, function texts,
//! designations and connection points are filled from value sets when inserting.

use openecraft_i18n::tr_fmt;
use crate::model::*;
use crate::ops::{self, Clip};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MacroKind {
    #[default]
    Window,
    Page,
}

/// Named values for the placeholders of a macro (e.g. "Pump 1.5 kW").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ValueSet {
    pub name: String,
    pub values: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Macro {
    pub name: String,
    pub category: String,
    pub description: String,
    pub kind: MacroKind,
    pub clip: Clip,
    /// Page macros: description of the inserted page.
    pub page_description: String,
    pub value_sets: Vec<ValueSet>,
}

/// The macro library (stored with the application, like the parts database).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MacroLibrary {
    pub macros: Vec<Macro>,
}

impl MacroLibrary {
    /// Adds a macro or replaces the one with the same name.
    pub fn upsert(&mut self, m: Macro) {
        match self.macros.iter_mut().find(|x| x.name == m.name) {
            Some(x) => *x = m,
            None => self.macros.push(m),
        }
        self.macros.sort_by(|a, b| natural_cmp(&a.category, &b.category).then_with(|| natural_cmp(&a.name, &b.name)));
    }

    /// Adds the macros of another library (same names are replaced). Returns the count.
    pub fn merge(&mut self, other: MacroLibrary) -> usize {
        let n = other.macros.len();
        for m in other.macros {
            self.upsert(m);
        }
        n
    }
}

fn scan(s: &str, out: &mut BTreeSet<String>) {
    let mut rest = s;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) if end > 0 && after[..end].chars().all(|c| c.is_alphanumeric() || c == '_' || c == ' ' || c == '.') => {
                out.insert(after[..end].to_string());
                rest = &after[end + 1..];
            }
            _ => rest = after,
        }
    }
}

fn fill(s: &str, values: &BTreeMap<String, String>) -> String {
    let mut out = s.to_string();
    for (k, v) in values {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Every text of a clip that may hold placeholders.
fn texts_mut(clip: &mut Clip) -> Vec<&mut String> {
    let mut v: Vec<&mut String> = Vec::new();
    for s in &mut clip.symbols {
        v.push(&mut s.tag);
        v.push(&mut s.label);
        v.push(&mut s.function_text);
        v.extend(s.pins.iter_mut());
    }
    for w in &mut clip.wires {
        v.push(&mut w.color);
        v.push(&mut w.cross_section);
    }
    v.extend(clip.texts.iter_mut().map(|t| &mut t.text));
    v
}

impl Macro {
    /// Placeholders used in the macro, sorted.
    pub fn placeholders(&self) -> Vec<String> {
        let mut clip = self.clip.clone();
        let mut set = BTreeSet::new();
        for t in texts_mut(&mut clip) {
            scan(t, &mut set);
        }
        scan(&self.page_description, &mut set);
        set.into_iter().collect()
    }

    /// The macro's items with the placeholders filled in (unknown ones stay).
    pub fn resolve(&self, values: &BTreeMap<String, String>) -> Clip {
        let mut clip = self.clip.clone();
        for t in texts_mut(&mut clip) {
            *t = fill(t, values);
        }
        clip
    }

    /// Number of symbols / wires / texts and graphics.
    pub fn summary(&self) -> String {
        let c = &self.clip;
        tr_fmt!("{} symbols, {} wires, {} texts / graphics", c.symbols.len(), c.wires.len(), c.texts.len() + c.shapes.len())
    }
}

/// Window macro from the selected items of a page.
pub fn from_selection(page: &Page, sel: &BTreeSet<u64>, name: &str) -> Macro {
    Macro { name: name.into(), kind: MacroKind::Window, clip: ops::copy(page, sel), ..Default::default() }
}

/// Page macro from a complete page.
pub fn from_page(page: &Page, name: &str) -> Macro {
    let all = ops::all_ids(page).into_iter().collect();
    Macro {
        name: name.into(),
        kind: MacroKind::Page,
        clip: ops::copy(page, &all),
        page_description: page.description.clone(),
        description: page.description.clone(),
        ..Default::default()
    }
}

/// Inserts a page macro as a new schematic page after page `after`, in the structure
/// of the macro's page. Returns the index of the new page.
pub fn insert_page(prj: &mut Project, after: usize, m: &Macro, values: &BTreeMap<String, String>) -> usize {
    let clip = m.resolve(values);
    let name = (prj.pages.iter().filter_map(|p| p.name.parse::<u32>().ok()).max().unwrap_or(0) + 1).to_string();
    let id = prj.new_id();
    let mut page = Page::new(id, &name, &fill(&m.page_description, values), PageKind::Schematic);
    page.function = clip.function.clone();
    page.location = clip.location.clone();
    let at = (after + 1).min(prj.pages.len());
    prj.pages.insert(at, page);
    // Items keep their position on the page.
    let origin = clip.bounds().min;
    ops::paste(prj, at, &clip, origin);
    at
}

#[cfg(test)]
mod tests;
