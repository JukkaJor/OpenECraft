//! Editing operations on a page: placement defaults, move, rotate, delete, copy/paste.

use crate::geom::{p, P, R, GRID};
use crate::model::*;
use crate::symbols::{self, ContactKind, Func, SymbolDef};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// Context remembered between placements (e.g. the coil contacts belong to).
#[derive(Default, Clone)]
pub struct PlaceCtx {
    /// identifying letter -> last used main function tag
    pub last_main: HashMap<String, String>,
    pub last_strip: Option<String>,
    /// symbol key -> last PLC card
    pub last_plc: HashMap<String, String>,
}

impl PlaceCtx {
    pub fn remember(&mut self, s: &SymbolInst) {
        let Some(def) = symbols::get(&s.sym) else { return };
        if s.tag.is_empty() {
            return;
        }
        match def.func {
            Func::Main { .. } => {
                self.last_main.insert(def.letter.to_string(), s.tag.clone());
            }
            Func::Terminal => self.last_strip = Some(s.tag.clone()),
            Func::Plc => {
                self.last_plc.insert(s.sym.clone(), s.tag.clone());
            }
            _ => {}
        }
    }
}

fn parse_plc(addr: &str) -> Option<(char, u32, u32)> {
    let mut chars = addr.chars();
    let prefix = chars.next()?;
    let rest: String = chars.collect();
    let (byte, bit) = rest.split_once('.')?;
    Some((prefix, byte.parse().ok()?, bit.parse().ok()?))
}

fn symbols_with_tag<'a>(prj: &'a Project, tag: &'a str) -> impl Iterator<Item = &'a SymbolInst> + 'a {
    prj.pages.iter().flat_map(|pg| pg.symbols.iter()).filter(move |s| s.tag == tag)
}

/// Connection point designations for the next auxiliary contact of a device
/// (13/14, 21/22, 31/32/34 ...; first digit = ordinal number).
pub fn next_contact_pins(prj: &Project, tag: &str, kind: ContactKind, exclude: Option<u64>) -> Vec<String> {
    let mut used = BTreeSet::new();
    for s in symbols_with_tag(prj, tag) {
        if Some(s.id) == exclude {
            continue;
        }
        if let Some(def) = symbols::get(&s.sym) {
            if def.auto_number || matches!(def.func, Func::Main { .. }) {
                for pin in &s.pins {
                    if pin.len() == 2 {
                        if let Some(d) = pin.chars().next().and_then(|c| c.to_digit(10)) {
                            used.insert(d);
                        }
                    }
                }
            }
        }
    }
    let o = (1..=9).find(|o| !used.contains(o)).unwrap_or(9);
    match kind {
        ContactKind::No | ContactKind::MainNo => vec![format!("{o}3"), format!("{o}4")],
        ContactKind::Nc => vec![format!("{o}1"), format!("{o}2")],
        ContactKind::Changeover => vec![format!("{o}1"), format!("{o}2"), format!("{o}4")],
    }
}

fn next_terminal(prj: &Project, strip: &str) -> String {
    let max = symbols_with_tag(prj, strip)
        .filter(|s| symbols::get(&s.sym).map(|d| d.func) == Some(Func::Terminal))
        .filter_map(|s| s.label.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    (max + 1).to_string()
}

fn next_plc_address(prj: &Project, card: &str, prefix: char) -> String {
    let max = symbols_with_tag(prj, card)
        .filter_map(|s| parse_plc(&s.label))
        .filter(|(c, _, _)| *c == prefix)
        .map(|(_, by, bi)| by * 8 + bi)
        .max();
    let n = max.map(|m| m + 1).unwrap_or(0);
    format!("{prefix}{}.{}", n / 8, n % 8)
}

/// Creates a symbol instance with sensible defaults (tag, numbering, ...).
pub fn new_symbol(prj: &mut Project, ctx: &PlaceCtx, def: &SymbolDef, pos: P, rot: u8, mirror: bool) -> SymbolInst {
    let mut s = SymbolInst {
        id: prj.new_id(),
        sym: def.key.into(),
        pos,
        rot,
        mirror,
        label: def.default_label.into(),
        pins: def.pins.iter().map(|x| x.label.to_string()).collect(),
        ..Default::default()
    };
    match def.func {
        Func::Main { .. } => s.tag = prj.next_tag(def.letter),
        Func::Aux(kind) => {
            s.tag = ctx.last_main.get(def.letter).cloned().unwrap_or_else(|| prj.next_tag(def.letter));
            if def.auto_number {
                s.pins = next_contact_pins(prj, &s.tag, kind, None);
            }
        }
        Func::Terminal => {
            s.tag = ctx.last_strip.clone().unwrap_or_else(|| "-X1".into());
            if def.default_label.chars().all(|c| c.is_ascii_digit()) {
                s.label = next_terminal(prj, &s.tag);
            }
        }
        Func::Plc => {
            s.tag = ctx.last_plc.get(def.key).cloned().unwrap_or_else(|| prj.next_tag(def.letter));
            let prefix = def.default_label.chars().next().unwrap_or('I');
            s.label = next_plc_address(prj, &s.tag, prefix);
        }
        Func::Potential | Func::Interruption => {}
    }
    s
}

pub fn text_bounds(t: &TextItem) -> R {
    let lines: Vec<&str> = t.text.lines().collect();
    let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1).max(1) as f32 * t.size * 0.62;
    let h = lines.len().max(1) as f32 * t.size * 1.4;
    R::from_points(p(t.pos.x, t.pos.y - t.size * 0.7), p(t.pos.x + w, t.pos.y - t.size * 0.7 + h))
}

/// Bounding box of any item on a page.
pub fn item_bounds(page: &Page, id: u64) -> Option<R> {
    if let Some(s) = page.symbols.iter().find(|s| s.id == id) {
        return symbols::get(&s.sym).map(|d| d.world_bounds(s.pos, s.rot, s.mirror));
    }
    if let Some(w) = page.wires.iter().find(|w| w.id == id) {
        return Some(R::from_points(w.a, w.b));
    }
    if let Some(t) = page.texts.iter().find(|t| t.id == id) {
        return Some(text_bounds(t));
    }
    page.shapes.iter().find(|s| s.id == id).map(|s| R::from_points(s.a, s.b))
}

pub fn all_ids(page: &Page) -> Vec<u64> {
    page.symbols
        .iter()
        .map(|s| s.id)
        .chain(page.wires.iter().map(|w| w.id))
        .chain(page.texts.iter().map(|t| t.id))
        .chain(page.shapes.iter().map(|s| s.id))
        .collect()
}

/// World positions of all connection points of the selected symbols and the end
/// points of the selected wires.
pub fn selection_points(page: &Page, sel: &BTreeSet<u64>) -> Vec<P> {
    let mut pts = Vec::new();
    for s in page.symbols.iter().filter(|s| sel.contains(&s.id)) {
        if let Some(def) = symbols::get(&s.sym) {
            for i in 0..def.pins.len() {
                pts.push(def.pin_world(i, s.pos, s.rot, s.mirror));
            }
        }
    }
    for w in page.wires.iter().filter(|w| sel.contains(&w.id)) {
        pts.push(w.a);
        pts.push(w.b);
    }
    pts
}

/// Wire ends (wire id, is end `a`) of unselected wires touching the selection, so they
/// can be stretched ("rubber band") while moving.
pub fn attached_wire_ends(page: &Page, sel: &BTreeSet<u64>) -> Vec<(u64, bool)> {
    let pts = selection_points(page, sel);
    let mut v = Vec::new();
    for w in page.wires.iter().filter(|w| !sel.contains(&w.id)) {
        if pts.iter().any(|q| q.same(w.a)) {
            v.push((w.id, true));
        }
        if pts.iter().any(|q| q.same(w.b)) {
            v.push((w.id, false));
        }
    }
    v
}

pub fn translate(page: &mut Page, sel: &BTreeSet<u64>, attached: &[(u64, bool)], d: P) {
    for s in page.symbols.iter_mut().filter(|s| sel.contains(&s.id)) {
        s.pos = s.pos + d;
    }
    for w in page.wires.iter_mut() {
        if sel.contains(&w.id) {
            w.a = w.a + d;
            w.b = w.b + d;
        } else {
            for &(id, is_a) in attached {
                if id == w.id {
                    if is_a {
                        w.a = w.a + d;
                    } else {
                        w.b = w.b + d;
                    }
                }
            }
        }
    }
    for t in page.texts.iter_mut().filter(|t| sel.contains(&t.id)) {
        t.pos = t.pos + d;
    }
    for s in page.shapes.iter_mut().filter(|s| sel.contains(&s.id)) {
        s.a = s.a + d;
        s.b = s.b + d;
    }
}

/// Rotates the selection by 90° clockwise around its (grid snapped) centre.
pub fn rotate(page: &mut Page, sel: &BTreeSet<u64>) {
    let mut bounds = R::nothing();
    for &id in sel {
        if let Some(b) = item_bounds(page, id) {
            bounds.union(b);
        }
    }
    if !bounds.is_valid() {
        return;
    }
    // A single symbol rotates around its own origin.
    let single = page.symbols.iter().find(|s| sel.contains(&s.id)).filter(|_| sel.len() == 1);
    let c = single.map(|s| s.pos).unwrap_or_else(|| bounds.center().snap(GRID));
    let attached = attached_wire_ends(page, sel);
    let before = selection_points(page, sel);
    let rot = |q: P| c + (q - c).transform(1, false);
    for s in page.symbols.iter_mut().filter(|s| sel.contains(&s.id)) {
        s.pos = rot(s.pos);
        s.rot = (s.rot + 1) % 4;
    }
    for w in page.wires.iter_mut().filter(|w| sel.contains(&w.id)) {
        w.a = rot(w.a);
        w.b = rot(w.b);
    }
    for t in page.texts.iter_mut().filter(|t| sel.contains(&t.id)) {
        t.pos = rot(t.pos);
    }
    for s in page.shapes.iter_mut().filter(|s| sel.contains(&s.id)) {
        s.a = rot(s.a);
        s.b = rot(s.b);
    }
    // Attached wire ends follow their connection point.
    for (id, is_a) in attached {
        if let Some(w) = page.wires.iter_mut().find(|w| w.id == id) {
            let e = if is_a { &mut w.a } else { &mut w.b };
            if before.iter().any(|q| q.same(*e)) {
                *e = rot(*e);
            }
        }
    }
}

pub fn mirror(page: &mut Page, sel: &BTreeSet<u64>) {
    for s in page.symbols.iter_mut().filter(|s| sel.contains(&s.id)) {
        s.mirror = !s.mirror;
    }
}

pub fn delete(page: &mut Page, sel: &BTreeSet<u64>) {
    page.symbols.retain(|s| !sel.contains(&s.id));
    page.wires.retain(|w| !sel.contains(&w.id));
    page.texts.retain(|t| !sel.contains(&t.id));
    page.shapes.retain(|s| !sel.contains(&s.id));
}

/// Removes zero length wires and merges exact duplicates.
pub fn cleanup_wires(page: &mut Page) {
    page.wires.retain(|w| !w.a.same(w.b));
    let mut seen = std::collections::HashSet::new();
    page.wires.retain(|w| {
        let (a, b) = (w.a.key(), w.b.key());
        let k = if a <= b { (a, b) } else { (b, a) };
        seen.insert(k)
    });
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Clip {
    pub magic: String,
    pub symbols: Vec<SymbolInst>,
    pub wires: Vec<Wire>,
    pub texts: Vec<TextItem>,
    pub shapes: Vec<Shape>,
}

const CLIP_MAGIC: &str = "openecraft-clipboard-v1";

pub fn copy(page: &Page, sel: &BTreeSet<u64>) -> Clip {
    Clip {
        magic: CLIP_MAGIC.into(),
        symbols: page.symbols.iter().filter(|s| sel.contains(&s.id)).cloned().collect(),
        wires: page.wires.iter().filter(|s| sel.contains(&s.id)).cloned().collect(),
        texts: page.texts.iter().filter(|s| sel.contains(&s.id)).cloned().collect(),
        shapes: page.shapes.iter().filter(|s| sel.contains(&s.id)).cloned().collect(),
    }
}

impl Clip {
    pub fn to_text(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_text(s: &str) -> Option<Clip> {
        let c: Clip = serde_json::from_str(s).ok()?;
        (c.magic == CLIP_MAGIC).then_some(c)
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty() && self.wires.is_empty() && self.texts.is_empty() && self.shapes.is_empty()
    }

    fn bounds(&self) -> R {
        let mut r = R::nothing();
        for s in &self.symbols {
            r.extend(s.pos);
        }
        for w in &self.wires {
            r.extend(w.a);
            r.extend(w.b);
        }
        for t in &self.texts {
            r.extend(t.pos);
        }
        for s in &self.shapes {
            r.extend(s.a);
            r.extend(s.b);
        }
        r
    }
}

/// Pastes the clip so that its top-left corner lands on `at`. Device tags of main
/// functions that already exist get new numbers (consistently for coil + contacts).
pub fn paste(prj: &mut Project, page_i: usize, clip: &Clip, at: P) -> BTreeSet<u64> {
    let b = clip.bounds();
    if !b.is_valid() {
        return BTreeSet::new();
    }
    let d = (at - b.min).snap(GRID);
    let existing: BTreeSet<String> = prj.all_tags().into_iter().collect();
    let mut rename: HashMap<String, String> = HashMap::new();
    let mut new_ids = BTreeSet::new();
    let mut symbols_out = Vec::new();
    for s in &clip.symbols {
        let mut s = s.clone();
        s.id = prj.new_id();
        s.pos = s.pos + d;
        if let Some(def) = symbols::get(&s.sym) {
            match def.func {
                Func::Main { .. } | Func::Aux(_) if existing.contains(&s.tag) => {
                    if let Some(n) = rename.get(&s.tag) {
                        s.tag = n.clone();
                    } else {
                        let n = prj.next_tag(def.letter);
                        // Reserve the tag by temporarily recording it.
                        rename.insert(s.tag.clone(), n.clone());
                        s.tag = n;
                    }
                }
                Func::Terminal if s.label.chars().all(|c| c.is_ascii_digit()) => {
                    s.label = next_terminal(prj, &s.tag);
                }
                _ => {}
            }
        }
        new_ids.insert(s.id);
        // Push immediately so the next `next_tag` / numbering sees it.
        prj.pages[page_i].symbols.push(s.clone());
        symbols_out.push(s.id);
    }
    for w in &clip.wires {
        let mut w = w.clone();
        w.id = prj.new_id();
        w.a = w.a + d;
        w.b = w.b + d;
        new_ids.insert(w.id);
        prj.pages[page_i].wires.push(w);
    }
    for t in &clip.texts {
        let mut t = t.clone();
        t.id = prj.new_id();
        t.pos = t.pos + d;
        new_ids.insert(t.id);
        prj.pages[page_i].texts.push(t);
    }
    for s in &clip.shapes {
        let mut s = s.clone();
        s.id = prj.new_id();
        s.a = s.a + d;
        s.b = s.b + d;
        new_ids.insert(s.id);
        prj.pages[page_i].shapes.push(s);
    }
    new_ids
}

/// Renames a device everywhere (all functions with that tag, device data).
pub fn rename_device(prj: &mut Project, old: &str, new: &str) {
    if old == new || old.is_empty() {
        return;
    }
    for pg in &mut prj.pages {
        for s in &mut pg.symbols {
            if s.tag == old {
                s.tag = new.to_string();
            }
        }
    }
    if let Some(d) = prj.devices.remove(old) {
        prj.devices.entry(new.to_string()).or_insert(d);
    }
}

#[cfg(test)]
mod tests;
