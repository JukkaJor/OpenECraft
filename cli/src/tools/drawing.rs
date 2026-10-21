//! Drawing tools: symbols, wires, texts; moving and deleting items. Positions are in
//! mm on the A3 page (420 x 297, y downwards) and snap to the 4 mm grid like in the
//! editor.

use super::common::*;
use super::{optional, param, Args, Kind, Param, Tool};
use crate::session::Session;
use openecraft_core::geom::{p, P, GRID};
use openecraft_core::model::*;
use openecraft_core::ops::{self, PlaceCtx};
use openecraft_core::symbols::{self, Func, SymbolDef};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const ROTATION: Param = optional("rotation", Kind::Int, tr_const!("Rotation in degrees clockwise: 0, 90, 180 or 270"));
const MIRROR: Param = optional("mirror", Kind::Bool, tr_const!("Mirror the symbol"));
const TAG: Param = optional("tag", Kind::Str, tr_const!("Device tag, e.g. -K1"));
const LABEL: Param = optional("label", Kind::Str, tr_const!("Label: terminal number, PLC address or potential name"));
const FUNCTION_TEXT: Param = optional("function_text", Kind::Str, tr_const!("Function text"));
const PINS: Param = optional("pins", Kind::Str, tr_const!("Connection point designations, comma separated, e.g. 13,14"));

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "list_symbols",
        help: tr_const!("Lists the symbols that can be placed, with their connection points relative to the insertion point"),
        params: &[
            optional("query", Kind::Str, tr_const!("Text in the key or name")),
            optional("category", Kind::Str, tr_const!("Category, e.g. coils, contacts, terminals, plc")),
        ],
        needs_project: false,
        run: list_symbols,
    },
    Tool {
        name: "place_symbol",
        help: tr_const!("Places a symbol on a page. Without a tag the next free one is used; auxiliary contacts get the next free contact numbers of their device"),
        params: &[
            PAGE,
            param("symbol", Kind::Str, tr_const!("Symbol key from list_symbols")),
            param("x", Kind::Num, tr_const!("X position in mm")),
            param("y", Kind::Num, tr_const!("Y position in mm (downwards)")),
            ROTATION,
            MIRROR,
            TAG,
            LABEL,
            FUNCTION_TEXT,
            PINS,
        ],
        needs_project: true,
        run: place_symbol,
    },
    Tool {
        name: "update_symbol",
        help: tr_const!("Changes a placed symbol; when it moves, connected wire ends follow"),
        params: &[
            PAGE,
            param("id", Kind::Int, tr_const!("Item id")),
            optional("x", Kind::Num, tr_const!("X position in mm")),
            optional("y", Kind::Num, tr_const!("Y position in mm (downwards)")),
            ROTATION,
            MIRROR,
            TAG,
            LABEL,
            FUNCTION_TEXT,
            PINS,
        ],
        needs_project: true,
        run: update_symbol,
    },
    Tool {
        name: "add_wire",
        help: tr_const!("Draws a wire through two or more points"),
        params: &[PAGE, param("points", Kind::Points, tr_const!("Points in mm: [[x, y], …]"))],
        needs_project: true,
        run: add_wire,
    },
    Tool {
        name: "connect",
        help: tr_const!("Draws a wire between two connection points, e.g. -K1:A1 to -S1:14 (#id:pin for ambiguous ones, the name for potentials, 1, 2… for points without designation)"),
        params: &[
            PAGE,
            param("from", Kind::Str, tr_const!("Start: device tag and connection point, e.g. -K1:A1")),
            param("to", Kind::Str, tr_const!("End: device tag and connection point, e.g. -S1:14")),
            optional("via", Kind::Points, tr_const!("Points in between: [[x, y], …]")),
        ],
        needs_project: true,
        run: connect,
    },
    Tool {
        name: "update_wire",
        help: tr_const!("Changes the properties of a wire"),
        params: &[
            PAGE,
            param("id", Kind::Int, tr_const!("Item id")),
            optional("number", Kind::Str, tr_const!("Wire number (kept when renumbering)")),
            optional("color", Kind::Str, tr_const!("Color, e.g. BU")),
            optional("cross_section", Kind::Str, tr_const!("Cross section, e.g. 1.5")),
            optional("cable", Kind::Str, tr_const!("Cable tag")),
            optional("core", Kind::Str, tr_const!("Cable core")),
        ],
        needs_project: true,
        run: update_wire,
    },
    Tool {
        name: "add_text",
        help: tr_const!("Adds a text to a page"),
        params: &[
            PAGE,
            param("x", Kind::Num, tr_const!("X position in mm")),
            param("y", Kind::Num, tr_const!("Y position in mm (downwards)")),
            param("text", Kind::Str, tr_const!("Text")),
            optional("size", Kind::Num, tr_const!("Text height in mm (default 3.5)")),
        ],
        needs_project: true,
        run: add_text,
    },
    Tool {
        name: "move_items",
        help: tr_const!("Moves symbols, wires and texts; connected wire ends follow"),
        params: &[
            PAGE,
            param("ids", Kind::Ids, tr_const!("Item ids")),
            param("dx", Kind::Num, tr_const!("Distance in X (mm)")),
            param("dy", Kind::Num, tr_const!("Distance in Y (mm)")),
        ],
        needs_project: true,
        run: move_items,
    },
    Tool {
        name: "delete_items",
        help: tr_const!("Deletes symbols, wires, texts and shapes"),
        params: &[PAGE, param("ids", Kind::Ids, tr_const!("Item ids"))],
        needs_project: true,
        run: delete_items,
    },
];

fn func_name(f: Func) -> &'static str {
    match f {
        Func::Main { .. } => "main",
        Func::Aux(_) => "auxiliary",
        Func::Terminal => "terminal",
        Func::Plc => "plc",
        Func::Potential => "potential",
        Func::Interruption => "interruption",
        Func::Overview => "overview",
    }
}

fn symbol_def(key: &str) -> Result<&'static SymbolDef, String> {
    symbols::get(key).ok_or_else(|| tr_fmt!("Unknown symbol: {}", key))
}

fn rotation(a: &Args) -> Result<Option<u8>, String> {
    match a.int("rotation") {
        None => Ok(None),
        Some(deg) if deg % 90 == 0 => Ok(Some((deg / 90).rem_euclid(4) as u8)),
        Some(_) => Err(tr("The rotation must be 0, 90, 180 or 270").to_string()),
    }
}

fn snapped(x: f32, y: f32) -> P {
    p(x, y).snap(GRID)
}

fn pin_list(text: &str) -> Vec<String> {
    text.split(',').map(|s| s.trim().to_string()).collect()
}

/// Applies the optional symbol parameters shared by `place_symbol` and `update_symbol`.
fn apply_symbol_params(s: &mut SymbolInst, a: &Args) -> Result<(), String> {
    if let Some(rot) = rotation(a)? {
        s.rot = rot;
    }
    if let Some(m) = a.value("mirror").and_then(Value::as_bool) {
        s.mirror = m;
    }
    if let Some(l) = a.str("label") {
        s.label = l.into();
    }
    if let Some(t) = a.str("function_text") {
        s.function_text = t.into();
    }
    if let Some(pins) = a.str("pins") {
        s.pins = pin_list(pins);
    }
    Ok(())
}

fn push_path(prj: &mut Project, page: usize, pts: &[P]) -> Vec<u64> {
    let mut ids = Vec::new();
    for seg in pts.windows(2).filter(|s| !s[0].same(s[1])) {
        let id = prj.new_id();
        prj.pages[page].wires.push(Wire { id, a: seg[0], b: seg[1], ..Default::default() });
        ids.push(id);
    }
    ops::cleanup_wires(&mut prj.pages[page]);
    ids
}

/// Position of a connection point `TAG:PIN` / `#ID:PIN` / `POTENTIAL:PIN` on a page.
fn pin_position(pg: &Page, spec: &str) -> Result<P, String> {
    let (dev, pin) = spec.rsplit_once(':').ok_or_else(|| tr_fmt!("{} is not a connection point (TAG:PIN)", spec))?;
    let id = dev.strip_prefix('#').and_then(|n| n.parse::<u64>().ok());
    let mut found: Vec<P> = Vec::new();
    for s in &pg.symbols {
        let named = |f: Func| matches!(f, Func::Potential | Func::Interruption) && s.label == dev;
        let hit = match id {
            Some(id) => s.id == id,
            None => s.tag == dev || symbols::get(&s.sym).is_some_and(|d| named(d.func)),
        };
        if hit {
            for (label, q) in pins_of(s) {
                if label == pin && !found.iter().any(|f| f.same(q)) {
                    found.push(q);
                }
            }
        }
    }
    match found.as_slice() {
        [q] => Ok(*q),
        [] => Err(tr_fmt!("No connection point {} on this page", spec)),
        _ => Err(tr_fmt!("{} is ambiguous: use #id:pin", spec)),
    }
}

/// Orthogonal path through the points: vertical first, then horizontal.
fn orthogonal(pts: &[P]) -> Vec<P> {
    let mut out = vec![pts[0]];
    for &q in &pts[1..] {
        let last = *out.last().unwrap();
        if last.x != q.x && last.y != q.y {
            out.push(p(last.x, q.y));
        }
        out.push(q);
    }
    out
}

/// The ids of `a` that exist on the page.
fn selection(pg: &Page, a: &Args) -> Result<BTreeSet<u64>, String> {
    let all: BTreeSet<u64> = ops::all_ids(pg).into_iter().collect();
    let sel: BTreeSet<u64> = a.ids("ids").into_iter().collect();
    match sel.difference(&all).next() {
        Some(id) => Err(tr_fmt!("No item with id {}", id)),
        None => Ok(sel),
    }
}

fn list_symbols(s: &mut Session, a: &Args) -> Result<Value, String> {
    let query = a.str("query").unwrap_or_default().to_lowercase();
    let category = a.str("category").map(|c| c.trim().to_ascii_lowercase());
    let user: Vec<&SymbolDef> = s.project().map(|prj| prj.symbols.iter().filter_map(|u| symbols::get(&u.key)).collect()).unwrap_or_default();
    let list = symbols::library()
        .iter()
        .chain(user)
        .filter(|d| query.is_empty() || d.key.to_lowercase().contains(&query) || d.name.to_lowercase().contains(&query))
        .filter(|d| category.as_ref().is_none_or(|c| snake(d.category) == *c))
        .map(|d| {
            let pins: Vec<Value> = d.pins.iter().map(|pd| json!({ "pin": pd.label, "x": pd.pos.x, "y": pd.pos.y })).collect();
            json!({
                "key": d.key,
                "name": d.name,
                "category": snake(d.category),
                "function": func_name(d.func),
                "letter": d.letter,
                "default_label": d.default_label,
                "pins": pins,
            })
        });
    Ok(Value::Array(list.collect()))
}

fn place_symbol(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = editable(prj, a.req("page"))?;
    let def = symbol_def(a.req("symbol"))?;
    // A given tag decides the device the new function belongs to, like the last
    // placed main function in the editor.
    let mut ctx = PlaceCtx::default();
    if let Some(tag) = a.str("tag") {
        match def.func {
            Func::Aux(_) | Func::Overview => {
                ctx.last_main.insert(def.letter.into(), tag.into());
            }
            Func::Terminal => ctx.last_strip = Some(tag.into()),
            Func::Plc => {
                ctx.last_plc.insert(def.key.into(), tag.into());
            }
            _ => {}
        }
    }
    let pos = snapped(a.num("x").unwrap_or_default(), a.num("y").unwrap_or_default());
    let mut sym = ops::new_symbol(prj, i, &ctx, def, pos, rotation(a)?.unwrap_or(0), a.flag("mirror"));
    if let Some(tag) = a.str("tag").filter(|_| def.func.has_tag()) {
        sym.tag = tag.into();
    }
    apply_symbol_params(&mut sym, a)?;
    let out = symbol_json(&sym);
    prj.pages[i].symbols.push(sym);
    Ok(out)
}

fn update_symbol(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = editable(prj, a.req("page"))?;
    let id = a.int("id").unwrap_or_default() as u64;
    let page = &mut prj.pages[i];
    let old = page.symbols.iter().find(|x| x.id == id).ok_or_else(|| tr_fmt!("No symbol with id {}", id))?.pos;
    let to = snapped(a.num("x").unwrap_or(old.x), a.num("y").unwrap_or(old.y));
    if !to.same(old) {
        let sel = BTreeSet::from([id]);
        let attached = ops::attached_wire_ends(page, &sel);
        ops::translate(page, &sel, &attached, to - old);
    }
    let sym = page.symbols.iter_mut().find(|x| x.id == id).ok_or_else(|| tr_fmt!("No symbol with id {}", id))?;
    if let Some(tag) = a.str("tag") {
        sym.tag = tag.into();
    }
    apply_symbol_params(sym, a)?;
    Ok(symbol_json(sym))
}

fn add_wire(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = editable(prj, a.req("page"))?;
    let pts: Vec<P> = a.points("points").into_iter().map(|[x, y]| snapped(x, y)).collect();
    if pts.len() < 2 {
        return Err(tr("A wire needs at least two points").to_string());
    }
    Ok(json!({ "ids": push_path(prj, i, &pts) }))
}

fn connect(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = editable(prj, a.req("page"))?;
    let from = pin_position(&prj.pages[i], a.req("from"))?;
    let to = pin_position(&prj.pages[i], a.req("to"))?;
    let mut pts = vec![from];
    pts.extend(a.points("via").into_iter().map(|[x, y]| snapped(x, y)));
    pts.push(to);
    let path = orthogonal(&pts);
    let ids = push_path(prj, i, &path);
    Ok(json!({ "ids": ids, "points": path.into_iter().map(xy).collect::<Vec<_>>() }))
}

fn update_wire(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = editable(prj, a.req("page"))?;
    let id = a.int("id").unwrap_or_default() as u64;
    let w = prj.pages[i].wires.iter_mut().find(|w| w.id == id).ok_or_else(|| tr_fmt!("No wire with id {}", id))?;
    if let Some(n) = a.str("number") {
        w.number = n.into();
        w.number_fixed = !n.is_empty();
    }
    for (name, field) in [("color", &mut w.color), ("cross_section", &mut w.cross_section), ("cable", &mut w.cable), ("core", &mut w.core)] {
        if let Some(v) = a.str(name) {
            *field = v.into();
        }
    }
    Ok(json!({ "id": w.id, "number": w.number, "color": w.color, "cross_section": w.cross_section, "cable": w.cable, "core": w.core }))
}

fn add_text(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = editable(prj, a.req("page"))?;
    let id = prj.new_id();
    let pos = snapped(a.num("x").unwrap_or_default(), a.num("y").unwrap_or_default());
    let size = a.num("size").unwrap_or(3.5);
    prj.pages[i].texts.push(TextItem { id, pos, text: a.req("text").into(), size });
    Ok(json!({ "id": id }))
}

fn move_items(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = editable(prj, a.req("page"))?;
    let page = &mut prj.pages[i];
    let sel = selection(page, a)?;
    let d = snapped(a.num("dx").unwrap_or_default(), a.num("dy").unwrap_or_default());
    let attached = ops::attached_wire_ends(page, &sel);
    ops::translate(page, &sel, &attached, d);
    Ok(json!({ "moved": sel.len(), "dx": d.x, "dy": d.y }))
}

fn delete_items(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = editable(prj, a.req("page"))?;
    let sel = selection(&prj.pages[i], a)?;
    ops::delete(&mut prj.pages[i], &sel);
    Ok(json!({ "deleted": sel.len() }))
}

#[cfg(test)]
mod tests;
