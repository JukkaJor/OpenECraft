//! Offline numbering of device tags and pages (in the spirit of EPLAN's "Number
//! devices" and "Renumber pages"). Device tags are numbered per structure and
//! identifying letter in schematic order (page, column, top to bottom) after a
//! format such as `{letter}{n}` (`-K1`), `{letter}{page}{column}` (`-K34`) or the
//! NFPA style `{page}{column}{letter}` (`-34CR`); the device keeps its structure, and
//! everything that refers to its tag follows (see [`ops::rename_device`]). Pages are
//! numbered with a start value and step, optionally restarting per structure and
//! keeping sub-pages (`3.a`).

use crate::index::{Index, Loc};
use crate::model::Project;
use crate::ops;
use crate::structure::{self, Dt};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// Formats offered for device tags: (format, example).
pub const DEVICE_FORMATS: [(&str, &str); 4] = [("{letter}{n}", "-K1"), ("{letter}{page}.{n}", "-K3.1"), ("{letter}{page}{column}", "-K34"), ("{page}{column}{letter}", "-34K")];

/// Settings of the device numbering.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceNumbering {
    /// `{letter}`: identifying letter(s), `{n}`: counter, `{page}`: page name,
    /// `{column}` / `{row}`: where the device's main function stands.
    pub format: String,
    pub start: u32,
    pub step: u32,
    /// Only devices whose main function is on these pages (indices); empty = all.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<usize>,
    /// Only these identifying letters (e.g. `K`, `Q`); empty = all.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub letters: Vec<String>,
    /// Terminal strips, plugs and PLC cards keep their tags.
    pub skip_terminals: bool,
}

impl Default for DeviceNumbering {
    fn default() -> Self {
        DeviceNumbering { format: DEVICE_FORMATS[0].0.into(), start: 1, step: 1, pages: Vec::new(), letters: Vec::new(), skip_terminals: true }
    }
}

/// Splits the device name of a tag (`-K12`, `-1CR2`) into the identifying letters
/// (`K`, `CR`); None if it has none.
pub fn letters_of(name: &str) -> Option<String> {
    let s = name.trim_start_matches('-').trim_start_matches(|c: char| c.is_ascii_digit());
    let l: String = s.chars().take_while(|c| c.is_alphabetic()).collect();
    (!l.is_empty()).then_some(l)
}

fn fill(format: &str, letter: &str, n: Option<u32>, page: &str, column: &str, row: &str) -> String {
    let mut s = format.replace("{letter}", letter).replace("{page}", page).replace("{column}", column).replace("{row}", row);
    if let Some(n) = n {
        s = s.replace("{n}", &n.to_string());
    }
    s
}

/// Where a device is numbered from: its first main function, otherwise its first
/// other function (terminal, PLC point …), otherwise its first auxiliary function.
fn anchor(prj: &Project, idx: &Index, tag: &str) -> Option<Loc> {
    let d = idx.devices.get(tag)?;
    let first = |v: &[Loc]| v.iter().min_by(|a, b| key(prj, a).cmp(&key(prj, b))).copied();
    first(&d.mains).or_else(|| first(&d.others)).or_else(|| first(&d.auxs))
}

/// Schematic order: page, column, top to bottom, left to right.
type Order = (usize, usize, i64, i64);

fn key(prj: &Project, l: &Loc) -> Order {
    let frame = crate::frame::of_page(prj, l.page);
    (l.page, frame.column_index(l.pos), (l.pos.y * 10.0) as i64, (l.pos.x * 10.0) as i64)
}

/// The new tags of the devices the settings cover: (old, new), only changed ones, in
/// schematic order. Devices with a function on a released page are left out.
pub fn plan_devices(prj: &Project, opts: &DeviceNumbering) -> Vec<(String, String)> {
    let idx = Index::build(prj);
    let locked: HashSet<&str> = prj.pages.iter().filter(|pg| pg.locked).flat_map(|pg| pg.symbols.iter().map(|s| s.tag.as_str())).collect();
    let terminal = |tag: &str| {
        prj.strips.contains_key(tag)
            || prj.plugs.contains_key(tag)
            || prj.plc_cards.contains_key(tag)
            || idx.devices.get(tag).is_some_and(|d| d.mains.is_empty() && !d.others.is_empty())
    };
    let mut todo: Vec<(Order, String, Loc)> = Vec::new();
    let mut keep: HashSet<String> = HashSet::new();
    for tag in idx.devices.keys() {
        let dt = Dt::parse(tag);
        let covered = anchor(prj, &idx, tag).filter(|l| {
            letters_of(&dt.name).is_some_and(|l| opts.letters.is_empty() || opts.letters.iter().any(|x| x.eq_ignore_ascii_case(&l)))
                && (opts.pages.is_empty() || opts.pages.contains(&l.page))
                && !locked.contains(tag.as_str())
                && !(opts.skip_terminals && terminal(tag))
        });
        match covered {
            Some(l) => todo.push((key(prj, &l), tag.clone(), l)),
            None => {
                keep.insert(tag.clone());
            }
        }
    }
    // Cables and harnesses share the tag space.
    keep.extend(prj.cables.iter().map(|c| c.tag.clone()));
    keep.extend(prj.harnesses.iter().map(|h| h.tag.clone()));
    todo.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let has_n = opts.format.contains("{n}");
    let step = opts.step.max(1);
    let mut counters: BTreeMap<String, u32> = BTreeMap::new();
    let mut used = keep;
    let mut out = Vec::new();
    for (_, tag, loc) in todo {
        let dt = Dt::parse(&tag);
        let letter = letters_of(&dt.name).unwrap_or_default();
        let frame = crate::frame::of_page(prj, loc.page);
        let (page, column, row) = (prj.pages[loc.page].name.as_str(), frame.column_of(loc.pos), frame.row_of(loc.pos));
        let base = fill(&opts.format, &letter, None, page, &column, &row);
        let group = format!("{}{base}", dt.structure());
        let make = |dt: &Dt, name: String| Dt { name: format!("-{name}"), ..dt.clone() }.tag();
        let new = if has_n {
            let c = counters.entry(group).or_insert(opts.start);
            loop {
                let t = make(&dt, fill(&base, &letter, Some(*c), page, &column, &row));
                *c += step;
                if !used.contains(&t) {
                    break t;
                }
            }
        } else {
            let plain = make(&dt, base.clone());
            if used.contains(&plain) {
                (2..).map(|k| make(&dt, format!("{base}.{k}"))).find(|t| !used.contains(t)).unwrap_or(plain)
            } else {
                plain
            }
        };
        used.insert(new.clone());
        if new != tag {
            out.push((tag, new));
        }
    }
    out
}

/// Renumbers the devices (see [`plan_devices`]); returns the renamed ones.
pub fn renumber_devices(prj: &mut Project, opts: &DeviceNumbering) -> Vec<(String, String)> {
    let plan = plan_devices(prj, opts);
    // Through temporary tags, so that swapped tags do not merge two devices.
    for (i, (old, _)) in plan.iter().enumerate() {
        ops::rename_device(prj, old, &format!("\u{1}{i}"));
    }
    for (i, (_, new)) in plan.iter().enumerate() {
        ops::rename_device(prj, &format!("\u{1}{i}"), new);
    }
    plan
}

/// Settings of the page numbering.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PageNumbering {
    pub start: u32,
    pub step: u32,
    /// Counting starts again on every page whose structure differs from the page
    /// before.
    pub per_structure: bool,
    /// Pages with a suffix (`3.a`, `3.1`) stay sub-pages of the page before.
    pub subpages: bool,
    /// Only these pages (indices); empty = all.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<usize>,
}

impl Default for PageNumbering {
    fn default() -> Self {
        PageNumbering { start: 1, step: 1, per_structure: false, subpages: true, pages: Vec::new() }
    }
}

/// New page names: (page index, new name) of the pages that change. Released pages
/// keep their name.
pub fn plan_pages(prj: &Project, opts: &PageNumbering) -> Vec<(usize, String)> {
    let step = opts.step.max(1);
    let mut n: Option<u32> = None;
    let mut structure_before: Option<String> = None;
    let mut out = Vec::new();
    for (i, pg) in prj.pages.iter().enumerate() {
        if !opts.pages.is_empty() && !opts.pages.contains(&i) {
            continue;
        }
        let s = structure::page_structure(pg);
        if opts.per_structure && structure_before.as_ref().is_some_and(|b| *b != s) {
            n = None;
        }
        structure_before = Some(s);
        let suffix = pg.name.split_once('.').map(|(_, x)| x).filter(|x| !x.is_empty());
        let name = match (opts.subpages, suffix, n) {
            (true, Some(sfx), Some(cur)) => format!("{cur}.{sfx}"),
            _ => {
                let next = n.map_or(opts.start, |c| c + step);
                n = Some(next);
                match (opts.subpages, suffix) {
                    (true, Some(sfx)) => format!("{next}.{sfx}"),
                    _ => next.to_string(),
                }
            }
        };
        if name != pg.name && !pg.locked {
            out.push((i, name));
        }
    }
    out
}

/// Renumbers the pages (see [`plan_pages`]); returns how many were renamed.
pub fn renumber_pages(prj: &mut Project, opts: &PageNumbering) -> usize {
    let plan = plan_pages(prj, opts);
    for (i, name) in &plan {
        prj.pages[*i].name = name.clone();
    }
    plan.len()
}

#[cfg(test)]
mod tests;
