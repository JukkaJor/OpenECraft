//! Macro boxes (in the spirit of EPLAN's macro boxes and "update macros"): a window
//! macro placed on a page remembers its macro, variant, placeholder values and items.
//! When the macro changes in the library, the placed copies are updated (their items
//! replaced by the new content at the same place); a box edited on the page can be
//! saved back into the library.

use crate::geom::{p, R};
use crate::macros::{Macro, MacroLibrary};
use crate::model::{Page, Project};
use crate::ops::{self, Clip};
pub use crate::model::macro_box::MacroBox;
use std::collections::{BTreeMap, BTreeSet};

/// FNV-1a fingerprint of a macro's content (its clip as JSON).
pub fn fingerprint(clip: &Clip) -> u64 {
    let json = serde_json::to_string(clip).unwrap_or_default();
    json.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// The macro of a box from the library (its variant if it has one).
fn library_macro(lib: &MacroLibrary, b: &MacroBox) -> Option<Macro> {
    let m = lib.get(&b.name)?;
    if b.variant.is_empty() { Some(m.clone()) } else { m.with_variant(&b.variant) }
}

/// Places a window macro at `at` on a page and records the box; returns the ids of the
/// placed items.
pub fn place(prj: &mut Project, page_i: usize, m: &Macro, values: &BTreeMap<String, String>, at: crate::geom::P) -> BTreeSet<u64> {
    let ids = ops::paste(prj, page_i, &m.resolve(values), at);
    record(prj, page_i, m, values, &ids);
    ids
}

/// Records a box for items placed from a macro.
pub fn record(prj: &mut Project, page_i: usize, m: &Macro, values: &BTreeMap<String, String>, ids: &BTreeSet<u64>) -> u64 {
    let id = prj.new_id();
    let b = MacroBox { id, name: m.name.clone(), variant: m.variant.clone(), values: values.clone(), items: ids.iter().copied().collect(), source: fingerprint(&m.clip) };
    prj.pages[page_i].macro_boxes.push(b);
    id
}

/// Items of a box that still exist.
pub fn live_items(page: &Page, b: &MacroBox) -> BTreeSet<u64> {
    let all: BTreeSet<u64> = ops::all_ids(page).into_iter().collect();
    b.items.iter().copied().filter(|i| all.contains(i)).collect()
}

/// The area of a box on its page (its items' bounds); `None` once all items are gone.
pub fn bounds(prj: &Project, page_i: usize, b: &MacroBox) -> Option<R> {
    let symbols = prj.symbol_table();
    let page = &prj.pages[page_i];
    let mut r: Option<R> = None;
    for id in live_items(page, b) {
        if let Some(ib) = ops::item_bounds(&symbols, page, id) {
            r = Some(match r {
                Some(r) => R { min: p(r.min.x.min(ib.min.x), r.min.y.min(ib.min.y)), max: p(r.max.x.max(ib.max.x), r.max.y.max(ib.max.y)) },
                None => ib,
            });
        }
    }
    r
}

/// The boxes of the project: (page index, box).
pub fn all(prj: &Project) -> Vec<(usize, &MacroBox)> {
    prj.pages.iter().enumerate().flat_map(|(i, pg)| pg.macro_boxes.iter().map(move |b| (i, b))).collect()
}

/// State of a box against the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Current,
    /// The macro changed in the library.
    Outdated,
    /// The macro is not in the library.
    Missing,
}

pub fn state(lib: &MacroLibrary, b: &MacroBox) -> State {
    match library_macro(lib, b) {
        None => State::Missing,
        Some(m) if fingerprint(&m.clip) != b.source => State::Outdated,
        Some(_) => State::Current,
    }
}

/// What an update did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Updated {
    pub updated: usize,
    /// Boxes on released pages (not changed).
    pub locked: usize,
    pub missing: Vec<String>,
}

/// Replaces the items of every outdated box by the macro's new content (placeholders
/// filled with the box's values) at the top left corner of its old area.
pub fn update(prj: &mut Project, lib: &MacroLibrary) -> Updated {
    let mut res = Updated::default();
    let todo: Vec<(usize, u64)> = all(prj).into_iter().filter(|(_, b)| state(lib, b) != State::Current).map(|(i, b)| (i, b.id)).collect();
    for (pi, id) in todo {
        let Some(b) = prj.pages[pi].macro_boxes.iter().find(|b| b.id == id).cloned() else { continue };
        let Some(m) = library_macro(lib, &b) else {
            if !res.missing.contains(&b.name) {
                res.missing.push(b.name.clone());
            }
            continue;
        };
        if !prj.pages[pi].is_editable() {
            res.locked += 1;
            continue;
        }
        let clip = m.resolve(&b.values);
        let old = live_items(&prj.pages[pi], &b);
        // Measured like paste places a clip (insertion points, not drawn extents), or
        // every update would move the content.
        let placed = ops::copy(&prj.pages[pi], &old).bounds();
        let at = if placed.is_valid() { placed.min } else { clip.bounds().min };
        // Device tags of the old symbols (renamed when placed) by symbol and place.
        let old_tags: Vec<(String, crate::geom::P, String)> =
            prj.pages[pi].symbols.iter().filter(|s| old.contains(&s.id) && !s.tag.is_empty()).map(|s| (s.sym.clone(), s.pos - at, s.tag.clone())).collect();
        ops::delete(&mut prj.pages[pi], &old);
        let ids = ops::paste(prj, pi, &clip, at);
        for s in prj.pages[pi].symbols.iter_mut().filter(|s| ids.contains(&s.id) && !s.tag.is_empty()) {
            let rel = s.pos - at;
            if let Some((_, _, tag)) = old_tags.iter().find(|(sym, q, _)| *sym == s.sym && (q.x - rel.x).abs() < 1.0 && (q.y - rel.y).abs() < 1.0) {
                s.tag = tag.clone();
            }
        }
        if let Some(x) = prj.pages[pi].macro_boxes.iter_mut().find(|x| x.id == id) {
            x.items = ids.into_iter().collect();
            x.source = fingerprint(&m.clip);
        }
        res.updated += 1;
    }
    res
}

/// Saves the items of a box back into the library as the box's macro (its variant if
/// it has one); placeholder values of the box are not turned back into placeholders.
pub fn save_to_library(prj: &mut Project, page_i: usize, id: u64, lib: &mut MacroLibrary) -> Result<(), String> {
    let b = prj.pages[page_i].macro_boxes.iter().find(|b| b.id == id).cloned().ok_or_else(|| openecraft_i18n::tr("Unknown macro box").to_string())?;
    let items = live_items(&prj.pages[page_i], &b);
    if items.is_empty() {
        return Err(openecraft_i18n::tr("The macro box has no items left").into());
    }
    let mut m = crate::macros::from_selection(&prj.pages[page_i], &items, &b.name);
    if let Some(old) = lib.get(&b.name) {
        m.category = old.category.clone();
        m.description = old.description.clone();
        m.value_sets = old.value_sets.clone();
    }
    let source = fingerprint(&m.clip);
    if b.variant.is_empty() {
        lib.upsert(m);
    } else {
        lib.upsert_variant(m, &b.variant);
    }
    if let Some(x) = prj.pages[page_i].macro_boxes.iter_mut().find(|x| x.id == id) {
        x.source = source;
    }
    Ok(())
}

/// Removes a box; its items stay on the page.
pub fn remove(prj: &mut Project, page_i: usize, id: u64) -> bool {
    let before = prj.pages[page_i].macro_boxes.len();
    prj.pages[page_i].macro_boxes.retain(|b| b.id != id);
    prj.pages[page_i].macro_boxes.len() < before
}

/// Removes boxes whose items are all gone; returns how many.
pub fn remove_empty(prj: &mut Project) -> usize {
    let mut n = 0;
    for pg in &mut prj.pages {
        let all: BTreeSet<u64> = ops::all_ids(pg).into_iter().collect();
        let before = pg.macro_boxes.len();
        pg.macro_boxes.retain(|b| b.items.iter().any(|i| all.contains(i)));
        n += before - pg.macro_boxes.len();
    }
    n
}

#[cfg(test)]
mod tests;
