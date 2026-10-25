//! Pre-planning (in the spirit of EPLAN Preplanning): the plant as a tree of segments
//! with the devices planned in them before the schematic exists, segment templates,
//! the power budget per segment, the state of every planned device against the
//! schematic, schematic pages generated from page macros, and the exchange as CSV /
//! Excel.

use crate::index::Index;
use crate::issue::{Issue, Severity};
use crate::macros::MacroLibrary;
use crate::model::Project;
use crate::structure::Dt;
pub use crate::model::preplanning::{PlannedDevice, Preplanning, Segment, SegmentTemplate};
use openecraft_i18n::tr;
use std::collections::{BTreeMap, HashSet};

/// Separator of segment paths in the exchange files.
pub const PATH_SEP: char = '/';

/// Adds a segment below `parent` (0: top level); returns its id.
pub fn add_segment(prj: &mut Project, parent: u64, name: &str) -> u64 {
    let id = prj.new_id();
    prj.preplanning.segments.push(Segment { id, parent, name: name.trim().to_string(), ..Default::default() });
    id
}

pub fn segment(prj: &Project, id: u64) -> Option<&Segment> {
    prj.preplanning.segments.iter().find(|s| s.id == id)
}

pub fn segment_mut(prj: &mut Project, id: u64) -> Option<&mut Segment> {
    prj.preplanning.segments.iter_mut().find(|s| s.id == id)
}

/// The segment and all segments below it.
pub fn subtree(prj: &Project, id: u64) -> Vec<u64> {
    let mut out = vec![id];
    let mut k = 0;
    while k < out.len() {
        let p = out[k];
        out.extend(prj.preplanning.segments.iter().filter(|s| s.parent == p && s.id != p).map(|s| s.id));
        k += 1;
    }
    out
}

/// Removes a segment with everything below it; returns the number of segments removed.
pub fn remove_segment(prj: &mut Project, id: u64) -> usize {
    let gone: HashSet<u64> = subtree(prj, id).into_iter().collect();
    let before = prj.preplanning.segments.len();
    prj.preplanning.segments.retain(|s| !gone.contains(&s.id));
    before - prj.preplanning.segments.len()
}

/// Moves a segment below another one (not below itself or its own children).
pub fn move_segment(prj: &mut Project, id: u64, parent: u64) -> bool {
    if subtree(prj, id).contains(&parent) {
        return false;
    }
    match segment_mut(prj, id) {
        Some(s) => {
            s.parent = parent;
            true
        }
        None => false,
    }
}

/// The segments in tree order with their depth.
pub fn tree(prj: &Project) -> Vec<(usize, &Segment)> {
    fn walk<'a>(all: &'a [Segment], parent: u64, depth: usize, out: &mut Vec<(usize, &'a Segment)>) {
        if depth > 32 {
            return;
        }
        let mut kids: Vec<&Segment> = all.iter().filter(|s| s.parent == parent && s.id != parent).collect();
        kids.sort_by(|a, b| crate::model::natural_cmp(&a.name, &b.name));
        for k in kids {
            out.push((depth, k));
            walk(all, k.id, depth + 1, out);
        }
    }
    let all = &prj.preplanning.segments;
    let mut out = Vec::new();
    let ids: HashSet<u64> = all.iter().map(|s| s.id).collect();
    // Segments whose parent is gone count as top level.
    let mut roots: Vec<u64> = all.iter().filter(|s| s.parent != 0 && !ids.contains(&s.parent)).map(|s| s.parent).collect();
    roots.sort_unstable();
    roots.dedup();
    walk(all, 0, 0, &mut out);
    for r in roots {
        walk(all, r, 0, &mut out);
    }
    out
}

/// The path of a segment from the top (`Plant/Area/Unit`).
pub fn path(prj: &Project, id: u64) -> String {
    let mut names = Vec::new();
    let mut at = id;
    for _ in 0..32 {
        let Some(s) = segment(prj, at) else { break };
        names.push(s.name.clone());
        at = s.parent;
    }
    names.reverse();
    names.join(&PATH_SEP.to_string())
}

/// Function and location of a segment, inherited from its parents where empty.
pub fn structure(prj: &Project, id: u64) -> (String, String) {
    let (mut function, mut location) = (String::new(), String::new());
    let mut at = id;
    for _ in 0..32 {
        let Some(s) = segment(prj, at) else { break };
        if function.is_empty() {
            function = s.function.clone();
        }
        if location.is_empty() {
            location = s.location.clone();
        }
        at = s.parent;
    }
    (function, location)
}

/// Complete device tag of a planned device (`=A1+L1-M1`).
pub fn full_tag(prj: &Project, seg: u64, d: &PlannedDevice) -> String {
    let (function, location) = structure(prj, seg);
    Dt { function, location, ..Default::default() }.complete(&d.tag)
}

/// Planned power of a segment with everything below it, kW.
pub fn power(prj: &Project, id: u64) -> f32 {
    subtree(prj, id).into_iter().filter_map(|s| segment(prj, s)).flat_map(|s| s.devices.iter()).map(|d| d.power).sum()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    /// Not in the schematic yet.
    Planned,
    /// In the schematic.
    Engineered,
    /// In the schematic with another part than planned.
    PartDiffers,
}

impl State {
    pub fn title(self) -> &'static str {
        match self {
            State::Planned => tr("Planned"),
            State::Engineered => tr("Engineered"),
            State::PartDiffers => tr("Part differs"),
        }
    }
}

pub fn state(prj: &Project, idx: &Index, tag: &str, d: &PlannedDevice) -> State {
    if !idx.devices.contains_key(tag) {
        return State::Planned;
    }
    let parts: Vec<&str> = prj.devices.get(tag).map(|dev| dev.parts.iter().map(|r| r.number.as_str()).collect()).unwrap_or_default();
    if !d.part.is_empty() && !parts.is_empty() && !parts.contains(&d.part.as_str()) {
        State::PartDiffers
    } else {
        State::Engineered
    }
}

/// One row of the pre-planning: segment, planned device, complete tag and state.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub segment: u64,
    pub path: String,
    pub device: PlannedDevice,
    pub tag: String,
    pub state: State,
}

pub fn rows(prj: &Project, idx: &Index) -> Vec<Row> {
    let mut out = Vec::new();
    for (_, s) in tree(prj) {
        let path = path(prj, s.id);
        for d in &s.devices {
            let tag = full_tag(prj, s.id, d);
            out.push(Row { segment: s.id, path: path.clone(), state: state(prj, idx, &tag, d), tag, device: d.clone() });
        }
    }
    out
}

/// Adds the devices of a template to a segment (devices with a tag the segment already
/// has are left out); returns the number added.
pub fn apply_template(prj: &mut Project, seg: u64, template: &str) -> Result<usize, String> {
    let t = prj.preplanning.templates.iter().find(|t| t.name == template).cloned().ok_or_else(|| openecraft_i18n::tr_fmt!("No segment template named {}", template))?;
    let s = segment_mut(prj, seg).ok_or_else(|| tr("Unknown segment").to_string())?;
    let mut n = 0;
    for d in t.devices {
        if !s.devices.iter().any(|x| x.tag == d.tag) {
            s.devices.push(d);
            n += 1;
        }
    }
    Ok(n)
}

/// Saves the devices of a segment as a template (replacing one of the same name).
pub fn save_template(prj: &mut Project, seg: u64, name: &str) -> Result<(), String> {
    let s = segment(prj, seg).ok_or_else(|| tr("Unknown segment").to_string())?;
    let t = SegmentTemplate { name: name.trim().to_string(), description: s.description.clone(), devices: s.devices.clone() };
    prj.preplanning.templates.retain(|x| x.name != t.name);
    prj.preplanning.templates.push(t);
    Ok(())
}

/// Generates a schematic page per planned device of the segment (and below) that has a
/// page macro and is not in the schematic yet. The macro's placeholders get `TAG`,
/// `DESCRIPTION`, `POWER`, `VOLTAGE`, `PART` and `SEGMENT`; the pages get the
/// segment's function and location. Returns the names of the new pages and the macros
/// not found.
pub fn generate(prj: &mut Project, seg: u64, lib: &MacroLibrary) -> (Vec<String>, Vec<String>) {
    let idx = Index::build(prj);
    let mut todo = Vec::new();
    for s in subtree(prj, seg) {
        let Some(segm) = segment(prj, s) else { continue };
        for d in segm.devices.iter().filter(|d| !d.macro_name.is_empty()) {
            let tag = full_tag(prj, s, d);
            if !idx.devices.contains_key(&tag) {
                todo.push((s, d.clone(), tag));
            }
        }
    }
    let (mut pages, mut missing) = (Vec::new(), Vec::new());
    for (s, d, tag) in todo {
        let Some(m) = lib.get(&d.macro_name).cloned() else {
            if !missing.contains(&d.macro_name) {
                missing.push(d.macro_name.clone());
            }
            continue;
        };
        let values: BTreeMap<String, String> = [
            ("TAG", Dt::parse(&tag).name.clone()),
            ("DESCRIPTION", d.description.clone()),
            ("POWER", if d.power > 0.0 { format!("{} kW", d.power) } else { String::new() }),
            ("VOLTAGE", d.voltage.clone()),
            ("PART", d.part.clone()),
            ("SEGMENT", path(prj, s)),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        let at = crate::macros::insert_page(prj, prj.pages.len().saturating_sub(1), &m, &values);
        let (function, location) = structure(prj, s);
        let pg = &mut prj.pages[at];
        if !function.is_empty() {
            pg.function = function;
        }
        if !location.is_empty() {
            pg.location = location;
        }
        if pg.description.trim().is_empty() {
            pg.description = if d.description.is_empty() { tag.clone() } else { d.description.clone() };
        }
        pages.push(pg.name.clone());
    }
    (pages, missing)
}

const HEADER: [&str; 8] = ["Segment", "Tag", "Kind", "Description", "Power (kW)", "Voltage", "Part", "Macro"];

/// The planned devices as CSV (`;`, header [`HEADER`]); segments without devices get a
/// row without a tag.
pub fn csv(prj: &Project) -> String {
    let cell = |s: &str| if s.contains([';', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
    let mut out = HEADER.join(";");
    out.push('\n');
    for row in table_rows(prj) {
        out.push_str(&row.iter().map(|c| cell(c)).collect::<Vec<_>>().join(";"));
        out.push('\n');
    }
    out
}

/// The exchange rows (header first) for Excel.
pub fn sheet(prj: &Project) -> Vec<Vec<String>> {
    let mut v = vec![HEADER.iter().map(|h| h.to_string()).collect()];
    v.extend(table_rows(prj));
    v
}

fn table_rows(prj: &Project) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for (_, s) in tree(prj) {
        let path = path(prj, s.id);
        if s.devices.is_empty() {
            out.push(vec![path.clone(), String::new(), String::new(), s.description.clone(), String::new(), String::new(), String::new(), String::new()]);
        }
        for d in &s.devices {
            let power = if d.power > 0.0 { d.power.to_string() } else { String::new() };
            out.push(vec![path.clone(), d.tag.clone(), d.kind.clone(), d.description.clone(), power, d.voltage.clone(), d.part.clone(), d.macro_name.clone()]);
        }
    }
    out
}

/// Imports rows (header first, columns found by their names as in [`HEADER`]):
/// segments are created along the paths, devices added or updated by tag. Returns
/// (segments created, devices added, devices updated).
pub fn import_rows(prj: &mut Project, rows: &[Vec<String>]) -> Result<(usize, usize, usize), String> {
    let Some((header, data)) = rows.split_first() else { return Err(tr("The file is empty").into()) };
    let col = |name: &str| header.iter().position(|h| h.trim().eq_ignore_ascii_case(name));
    let seg_col = col("Segment").ok_or_else(|| tr("Column Segment is missing").to_string())?;
    let cols: Vec<Option<usize>> = HEADER.iter().map(|h| col(h)).collect();
    let (mut created, mut added, mut updated) = (0, 0, 0);
    for r in data {
        let get = |i: usize| cols[i].and_then(|c| r.get(c)).map(|s| s.trim().to_string()).unwrap_or_default();
        let path = r.get(seg_col).map(|s| s.trim()).unwrap_or_default();
        if path.is_empty() {
            continue;
        }
        let mut parent = 0;
        for name in path.split(PATH_SEP).map(str::trim).filter(|n| !n.is_empty()) {
            parent = match prj.preplanning.segments.iter().find(|s| s.parent == parent && s.name == name) {
                Some(s) => s.id,
                None => {
                    created += 1;
                    add_segment(prj, parent, name)
                }
            };
        }
        let tag = get(1);
        let Some(s) = segment_mut(prj, parent) else { continue };
        if tag.is_empty() {
            if !get(3).is_empty() {
                s.description = get(3);
            }
            continue;
        }
        let d = PlannedDevice { tag: tag.clone(), kind: get(2), description: get(3), power: get(4).replace(',', ".").parse().unwrap_or(0.0), voltage: get(5), part: get(6), macro_name: get(7) };
        match s.devices.iter_mut().find(|x| x.tag == tag) {
            Some(x) => {
                if *x != d {
                    *x = d;
                    updated += 1;
                }
            }
            None => {
                s.devices.push(d);
                added += 1;
            }
        }
    }
    Ok((created, added, updated))
}

/// Check: planned devices with another part in the schematic, tags planned twice.
pub fn issues(prj: &Project, idx: &Index) -> Vec<Issue> {
    let mut out = Vec::new();
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for row in rows(prj, idx) {
        if row.state == State::PartDiffers {
            out.push(crate::issue!(Severity::Warning, None, "{}: planned with part {}, the schematic uses another part", row.tag, row.device.part));
        }
        if let Some(first) = seen.insert(row.tag.clone(), row.path.clone()) {
            out.push(crate::issue!(Severity::Error, None, "{} is planned twice ({} and {})", row.tag, first, row.path));
        }
    }
    let open = rows(prj, idx).iter().filter(|r| r.state == State::Planned).count();
    if open > 0 {
        out.push(crate::issue!(Severity::Info, None, "{} planned devices are not in the schematic yet", open));
    }
    out
}

#[cfg(test)]
mod tests;
