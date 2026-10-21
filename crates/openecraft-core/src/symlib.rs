//! Project symbol library: user defined symbols (symbol editor). They are stored in
//! the project, shared as library files and placed like built-in symbols; placed
//! instances refer to them by key. [`register`] makes them known to
//! [`symbols::get`](crate::symbols::get).

use openecraft_i18n::{tr, tr_fmt};
use crate::geom::{dist_to_segment, p, P, GRID};
use crate::model::Project;
use crate::symbols::{self, Anchor, Category, FieldKind, Func, PinDef, Prim, SymbolDef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Keys of user symbols start with this prefix.
pub const PREFIX: &str = "user:";
/// Snap of graphics in the symbol editor (pins snap to the 4 mm grid).
pub const DRAW_SNAP: f32 = 0.5;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserPin {
    pub pos: P,
    /// Default connection point designation.
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserSymbol {
    pub key: String,
    pub name: String,
    /// Library (group in the symbol list), e.g. the company name.
    pub library: String,
    /// Identifying letter(s) for new device tags.
    pub letter: String,
    pub func: Func,
    pub default_label: String,
    pub auto_number: bool,
    pub show_tag: bool,
    pub show_pin_labels: bool,
    pub prims: Vec<Prim>,
    pub pins: Vec<UserPin>,
    /// Pins that are connected inside the symbol (terminals).
    pub bridges: Vec<(usize, usize)>,
}

impl Default for UserSymbol {
    fn default() -> Self {
        UserSymbol {
            key: String::new(),
            name: "New symbol".into(),
            library: "User".into(),
            letter: "A".into(),
            func: Func::Main { mirror: false },
            default_label: String::new(),
            auto_number: false,
            show_tag: true,
            show_pin_labels: true,
            prims: Vec::new(),
            pins: Vec::new(),
            bridges: Vec::new(),
        }
    }
}

/// File format for exchanging symbols between projects.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SymbolLibrary {
    pub symbols: Vec<UserSymbol>,
}

impl SymbolLibrary {
    /// A symbol library file, or a file with a single symbol. (Every object would parse
    /// as an empty library because of the defaults, so the key decides.)
    pub fn from_json(text: &str) -> Result<SymbolLibrary, String> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if value.get("symbols").is_some() {
            serde_json::from_value(value).map_err(|e| e.to_string())
        } else {
            serde_json::from_value::<UserSymbol>(value).map(|u| SymbolLibrary { symbols: vec![u] }).map_err(|e| e.to_string())
        }
    }
}

/// Symbol definition of a user symbol. Its texts are leaked to get the `'static`
/// lifetime of the built-in definitions, so it is only built when a symbol changed.
fn to_def(u: &UserSymbol) -> SymbolDef {
    let leak = |s: &str| -> &'static str { Box::leak(s.to_string().into_boxed_str()) };
    SymbolDef {
        key: leak(&u.key),
        name: leak(&u.name),
        category: Category::User,
        library: leak(&u.library),
        letter: leak(&u.letter),
        func: u.func,
        prims: u.prims.clone(),
        pins: u.pins.iter().map(|x| PinDef { pos: x.pos, label: leak(&x.label) }).collect(),
        bridges: u.bridges.iter().copied().filter(|(a, b)| *a < u.pins.len() && *b < u.pins.len()).collect(),
        default_label: leak(&u.default_label),
        auto_number: u.auto_number,
        show_tag: u.show_tag && u.func.has_tag(),
        show_pin_labels: u.show_pin_labels,
        bounds: symbols::bounds_of(&u.prims, u.pins.iter().map(|x| x.pos)),
    }
}

type Registry = RwLock<HashMap<String, (UserSymbol, &'static SymbolDef)>>;

fn registry() -> &'static Registry {
    static REG: OnceLock<Registry> = OnceLock::new();
    REG.get_or_init(Default::default)
}

/// Makes the user symbols known to the symbol lookup. Unchanged symbols are kept;
/// symbols removed from the project stay known so that open drawings still render.
pub fn register(list: &[UserSymbol]) {
    let changed: Vec<&UserSymbol> = {
        let Ok(reg) = registry().read() else { return };
        list.iter().filter(|u| reg.get(&u.key).map(|(old, _)| old != *u).unwrap_or(true)).collect()
    };
    if changed.is_empty() {
        return;
    }
    let Ok(mut reg) = registry().write() else { return };
    for u in changed {
        // Definitions live as long as the program (`&'static`), like the built-in
        // ones; only changed symbols are rebuilt.
        let def: &'static SymbolDef = Box::leak(Box::new(to_def(u)));
        reg.insert(u.key.clone(), (u.clone(), def));
    }
}

pub fn lookup(key: &str) -> Option<&'static SymbolDef> {
    if !key.starts_with(PREFIX) {
        return None;
    }
    registry().read().ok()?.get(key).map(|(_, d)| *d)
}

/// A new unique key for a symbol name, e.g. `user:motor_starter_2`.
pub fn new_key(existing: &[UserSymbol], name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    let base = format!("{PREFIX}{}", if slug.is_empty() { "symbol" } else { &slug });
    let mut key = base.clone();
    let mut n = 2;
    while existing.iter().any(|u| u.key == key) {
        key = format!("{base}_{n}");
        n += 1;
    }
    key
}

/// Editable copy of a built-in (or other) symbol.
pub fn from_def(def: &SymbolDef, existing: &[UserSymbol]) -> UserSymbol {
    let name = format!("{} (copy)", def.name);
    UserSymbol {
        key: new_key(existing, &name),
        name,
        library: "User".into(),
        letter: def.letter.into(),
        func: def.func,
        default_label: def.default_label.into(),
        auto_number: def.auto_number,
        show_tag: def.show_tag,
        show_pin_labels: def.show_pin_labels,
        prims: def.prims.clone(),
        pins: def.pins.iter().map(|x| UserPin { pos: x.pos, label: x.label.into() }).collect(),
        bridges: def.bridges.clone(),
    }
}

/// Number of placed instances of a symbol.
pub fn usage(prj: &Project, key: &str) -> usize {
    prj.pages.iter().flat_map(|pg| pg.symbols.iter()).filter(|s| s.sym == key).count()
}

/// Adds or replaces symbols of a library file; returns (added, replaced).
pub fn merge(list: &mut Vec<UserSymbol>, lib: SymbolLibrary) -> (usize, usize) {
    let (mut added, mut replaced) = (0, 0);
    for mut u in lib.symbols {
        if !u.key.starts_with(PREFIX) {
            u.key = new_key(list, &u.name);
        }
        match list.iter_mut().find(|x| x.key == u.key) {
            Some(x) => {
                *x = u;
                replaced += 1;
            }
            None => {
                list.push(u);
                added += 1;
            }
        }
    }
    (added, replaced)
}

/// Problems that make a symbol unusable or awkward to connect.
pub fn problems(u: &UserSymbol) -> Vec<String> {
    let mut v = Vec::new();
    if u.name.trim().is_empty() {
        v.push(tr("The symbol has no name.").into());
    }
    if u.pins.is_empty() && !matches!(u.func, Func::Overview) {
        v.push(tr("The symbol has no connection points.").into());
    }
    for (i, a) in u.pins.iter().enumerate() {
        if a.pos.x % GRID != 0.0 || a.pos.y % GRID != 0.0 {
            v.push(tr_fmt!("Connection point {} is not on the {} mm grid.", i + 1, GRID));
        }
        if u.pins[..i].iter().any(|b| b.pos.same(a.pos)) {
            v.push(tr_fmt!("Connection point {} lies on another one.", i + 1));
        }
    }
    if u.func.has_tag() && u.letter.trim().is_empty() {
        v.push(tr("No identifying letter for the device tag.").into());
    }
    v
}

/// Item of a symbol under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Prim(usize),
    Pin(usize),
}

/// Distance from `q` to a graphic primitive.
pub fn prim_distance(prim: &Prim, q: P) -> f32 {
    match prim {
        Prim::Line { a, b, .. } => dist_to_segment(q, *a, *b),
        Prim::Poly { pts, closed, fill } => {
            let mut d = pts.windows(2).map(|s| dist_to_segment(q, s[0], s[1])).fold(f32::INFINITY, f32::min);
            if *closed && pts.len() > 2 {
                d = d.min(dist_to_segment(q, pts[pts.len() - 1], pts[0]));
                if *fill && inside(pts, q) {
                    d = 0.0;
                }
            }
            d
        }
        Prim::Circle { c, r, fill } => {
            let d = (q.dist(*c) - r).abs();
            if *fill && q.dist(*c) <= *r { 0.0 } else { d }
        }
        Prim::Text { pos, text, size, anchor } => text_distance(*pos, text, *size, *anchor, q),
        Prim::Field { pos, size, anchor, kind } => {
            let sample = if *kind == FieldKind::Tag { "-K1" } else { "1" };
            text_distance(*pos, sample, *size, *anchor, q)
        }
    }
}

fn text_distance(pos: P, text: &str, size: f32, anchor: Anchor, q: P) -> f32 {
    let w = text.chars().count().max(1) as f32 * size * 0.62;
    let x0 = match anchor {
        Anchor::Left => pos.x,
        Anchor::Center => pos.x - w / 2.0,
        Anchor::Right => pos.x - w,
    };
    let dx = (x0 - q.x).max(q.x - (x0 + w)).max(0.0);
    let dy = ((q.y - pos.y).abs() - size * 0.6).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

fn inside(pts: &[P], q: P) -> bool {
    let mut c = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[j]);
        if (a.y > q.y) != (b.y > q.y) && q.x < (b.x - a.x) * (q.y - a.y) / (b.y - a.y) + a.x {
            c = !c;
        }
        j = i;
    }
    c
}

/// Pins first (they are small), then the nearest graphic within `tol`.
pub fn hit(u: &UserSymbol, q: P, tol: f32) -> Option<Hit> {
    if let Some(i) = u.pins.iter().position(|x| x.pos.dist(q) <= tol.max(1.0)) {
        return Some(Hit::Pin(i));
    }
    u.prims
        .iter()
        .enumerate()
        .map(|(i, prim)| (i, prim_distance(prim, q)))
        .filter(|(_, d)| *d <= tol)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| Hit::Prim(i))
}

/// Moves a primitive by `d`.
pub fn translate(prim: &mut Prim, d: P) {
    match prim {
        Prim::Line { a, b, .. } => {
            *a = *a + d;
            *b = *b + d;
        }
        Prim::Poly { pts, .. } => pts.iter_mut().for_each(|q| *q = *q + d),
        Prim::Circle { c, .. } => *c = *c + d,
        Prim::Text { pos, .. } | Prim::Field { pos, .. } => *pos = *pos + d,
    }
}

/// Removes a pin and fixes the bridges that refer to pins after it.
pub fn remove_pin(u: &mut UserSymbol, i: usize) {
    if i >= u.pins.len() {
        return;
    }
    u.pins.remove(i);
    u.bridges.retain(|(a, b)| *a != i && *b != i);
    for (a, b) in &mut u.bridges {
        if *a > i {
            *a -= 1;
        }
        if *b > i {
            *b -= 1;
        }
    }
}

/// Next free numeric pin designation ("1", "2", …).
pub fn next_pin_label(u: &UserSymbol) -> String {
    let max = u.pins.iter().filter_map(|x| x.label.parse::<u32>().ok()).max().unwrap_or(0);
    (max + 1).to_string()
}

pub fn snap(q: P, step: f32) -> P {
    p((q.x / step).round() * step, (q.y / step).round() * step)
}

#[cfg(test)]
mod tests;
