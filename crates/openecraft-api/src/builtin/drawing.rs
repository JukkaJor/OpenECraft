//! Drawing commands: symbols, wires, texts; moving and deleting items. Positions are in
//! mm on the A3 page (420 x 297, y downwards) and snap to the 4 mm grid like in the
//! editor.

use crate::builtin::common::*;
use crate::params::{optional, param, Args, Kind, Param};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::fluid::Medium;
use openecraft_core::geom::{p, P, GRID};
use openecraft_core::model::*;
use openecraft_core::ops::{self, PlaceCtx};
use openecraft_core::symbols::{self, Anchor, Func, SymbolRef};
use openecraft_core::symlib::Symbols;
use openecraft_core::arrange::{self, Align};
use openecraft_core::{cabledef, conndef, routing};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub struct Drawing;

impl Plugin for Drawing {
    fn info(&self) -> Info {
        info("drawing", tr_const!("Drawing"), tr_const!("Symbols, wires, texts, cable and connection definitions on schematic pages"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "list_symbols",
        help: tr_const!("Lists the symbols that can be placed, with their connection points relative to the insertion point"),
        params: &[
            optional("query", Kind::Str, tr_const!("Text in the key or name")),
            optional("category", Kind::Str, tr_const!("Category, e.g. coils, contacts, terminals, plc")),
        ],
        run: list_symbols,
    },
    Spec {
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
        run: place_symbol,
    },
    Spec {
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
        run: update_symbol,
    },
    Spec {
        name: "add_wire",
        help: tr_const!("Draws a wire through two or more points"),
        params: &[PAGE, param("points", Kind::Points, tr_const!("Points in mm: [[x, y], …]"))],
        run: add_wire,
    },
    Spec {
        name: "connect",
        help: tr_const!("Draws a wire between two connection points, e.g. -K1:A1 to -S1:14 (#id:pin for ambiguous ones, the name for potentials, 1, 2… for points without designation)"),
        params: &[
            PAGE,
            param("from", Kind::Str, tr_const!("Start: device tag and connection point, e.g. -K1:A1")),
            param("to", Kind::Str, tr_const!("End: device tag and connection point, e.g. -S1:14")),
            optional("via", Kind::Points, tr_const!("Points in between: [[x, y], …]")),
        ],
        run: connect,
    },
    Spec {
        name: "update_wire",
        help: tr_const!("Changes the properties of a wire"),
        params: &[
            PAGE,
            param("id", Kind::Int, tr_const!("Item id")),
            optional("number", Kind::Str, tr_const!("Wire number (kept when renumbering)")),
            optional("color", Kind::Str, tr_const!("Wire colour, e.g. BU (empty = from the potential definition)")),
            optional("cross_section", Kind::Str, tr_const!("Cross section, e.g. 1.5 mm² (empty = from the potential definition)")),
            optional("cable", Kind::Str, tr_const!("Cable tag")),
            optional("core", Kind::Str, tr_const!("Cable core")),
            optional("medium", Kind::Str, tr_const!("Hoses / pipes: medium (compressed_air, vacuum, hydraulic_oil, water, coolant, lubricant, gas, steam, product; empty = project default)")),
            optional("length", Kind::Num, tr_const!("Hoses / pipes: length in m")),
        ],
        run: update_wire,
    },
    Spec {
        name: "add_text",
        help: tr_const!("Adds a text to a page"),
        params: &[
            PAGE,
            param("x", Kind::Num, tr_const!("X position in mm")),
            param("y", Kind::Num, tr_const!("Y position in mm (downwards)")),
            param("text", Kind::Str, tr_const!("Text; placeholders such as {page} or {prop:Name} are filled in when drawn")),
            optional("size", Kind::Num, tr_const!("Text height in mm (default 3.5)")),
            optional("angle", Kind::Num, tr_const!("Rotation in degrees, counter-clockwise (90 reads from bottom to top)")),
            optional("align", Kind::Str, tr_const!("Horizontal alignment: left, center or right")),
            optional("valign", Kind::Str, tr_const!("Which line the position is on: top (first), middle or bottom (last)")),
            optional("width", Kind::Num, tr_const!("Wrap width in mm (0: no wrapping)")),
            optional("bold", Kind::Bool, tr_const!("Bold")),
            optional("italic", Kind::Bool, tr_const!("Italic")),
            optional("underline", Kind::Bool, tr_const!("Underline")),
            optional("frame", Kind::Bool, tr_const!("Border around the text")),
            optional("background", Kind::Bool, tr_const!("Opaque background in the paper colour")),
            optional("link", Kind::Str, tr_const!("Hyperlink (URL)")),
            optional("path_function", Kind::Bool, tr_const!("Path function text: function text of the functions in its column that have none")),
        ],
        run: add_text,
    },
    Spec {
        name: "update_text",
        help: tr_const!("Changes a text (only the given properties)"),
        params: &[
            PAGE,
            param("id", Kind::Int, tr_const!("Id of the text")),
            optional("text", Kind::Str, tr_const!("Text; placeholders such as {page} or {prop:Name} are filled in when drawn")),
            optional("x", Kind::Num, tr_const!("X position in mm")),
            optional("y", Kind::Num, tr_const!("Y position in mm (downwards)")),
            optional("size", Kind::Num, tr_const!("Text height in mm (default 3.5)")),
            optional("angle", Kind::Num, tr_const!("Rotation in degrees, counter-clockwise (90 reads from bottom to top)")),
            optional("align", Kind::Str, tr_const!("Horizontal alignment: left, center or right")),
            optional("valign", Kind::Str, tr_const!("Which line the position is on: top (first), middle or bottom (last)")),
            optional("width", Kind::Num, tr_const!("Wrap width in mm (0: no wrapping)")),
            optional("bold", Kind::Bool, tr_const!("Bold")),
            optional("italic", Kind::Bool, tr_const!("Italic")),
            optional("underline", Kind::Bool, tr_const!("Underline")),
            optional("frame", Kind::Bool, tr_const!("Border around the text")),
            optional("background", Kind::Bool, tr_const!("Opaque background in the paper colour")),
            optional("link", Kind::Str, tr_const!("Hyperlink (URL)")),
            optional("path_function", Kind::Bool, tr_const!("Path function text: function text of the functions in its column that have none")),
        ],
        run: update_text,
    },
    Spec {
        name: "move_items",
        help: tr_const!("Moves symbols, wires and texts; connected wire ends follow"),
        params: &[
            PAGE,
            param("ids", Kind::Ids, tr_const!("Item ids")),
            param("dx", Kind::Num, tr_const!("Distance in X (mm)")),
            param("dy", Kind::Num, tr_const!("Distance in Y (mm)")),
        ],
        run: move_items,
    },
    Spec {
        name: "align_items",
        help: tr_const!("Aligns symbols, texts and graphics to a common edge or centre; connected wire ends follow"),
        params: &[
            PAGE,
            param("ids", Kind::Ids, tr_const!("Item ids")),
            param("align", Kind::Str, tr_const!("left, center, right, top, middle or bottom")),
        ],
        run: align_items,
    },
    Spec {
        name: "distribute_items",
        help: tr_const!("Distributes symbols, texts and graphics evenly between the first and the last one"),
        params: &[PAGE, param("ids", Kind::Ids, tr_const!("Item ids")), optional("vertical", Kind::Bool, tr_const!("Distribute vertically (default horizontally)"))],
        run: distribute_items,
    },
    Spec {
        name: "copy_items",
        help: tr_const!("Copies items several times at a fixed distance; devices get new tags, terminals new numbers"),
        params: &[
            PAGE,
            param("ids", Kind::Ids, tr_const!("Item ids")),
            optional("count", Kind::Int, tr_const!("Number of copies (default 1)")),
            param("dx", Kind::Num, tr_const!("Distance in X (mm)")),
            param("dy", Kind::Num, tr_const!("Distance in Y (mm)")),
        ],
        run: copy_items,
    },
    Spec {
        name: "delete_items",
        help: tr_const!("Deletes symbols, wires, texts and shapes"),
        params: &[PAGE, param("ids", Kind::Ids, tr_const!("Item ids"))],
        run: delete_items,
    },
    Spec {
        name: "route",
        help: tr_const!("Connects two connection points with wires routed around symbols and other wires"),
        params: &[
            PAGE,
            param("from", Kind::Str, tr_const!("Connection point TAG:PIN, e.g. -K1:A1")),
            param("to", Kind::Str, tr_const!("Connection point TAG:PIN, e.g. -K1:A1")),
        ],
        run: route,
    },
    Spec {
        name: "autoconnect",
        help: tr_const!("Draws wires between free connection points of symbols that face each other"),
        params: &[PAGE, optional("ids", Kind::Ids, tr_const!("Symbol ids (default: all symbols of the page)"))],
        run: autoconnect,
    },
    Spec {
        name: "add_cable_definition",
        help: tr_const!("Draws a cable definition line; the wires it crosses become cores of the cable (a new cable when none is given)"),
        params: &[
            PAGE,
            param("points", Kind::Points, tr_const!("Start and end point of the line")),
            optional("cable", Kind::Str, tr_const!("Cable tag, e.g. -W1")),
        ],
        run: add_cable_definition,
    },
    Spec {
        name: "add_connection_definition",
        help: tr_const!("Places a connection definition point on a wire; the connection takes its number, colour and cross-section"),
        params: &[
            PAGE,
            param("x", Kind::Num, tr_const!("X position in mm")),
            param("y", Kind::Num, tr_const!("Y position in mm (downwards)")),
            optional("number", Kind::Str, tr_const!("Wire number")),
            optional("color", Kind::Str, tr_const!("Wire colour, e.g. BU")),
            optional("cross_section", Kind::Str, tr_const!("Cross-section, e.g. 1.5 mm²")),
        ],
        run: add_connection_definition,
    },
];

const ROTATION: Param = optional("rotation", Kind::Int, tr_const!("Rotation in degrees clockwise: 0, 90, 180 or 270"));

const MIRROR: Param = optional("mirror", Kind::Bool, tr_const!("Mirror the symbol"));

const TAG: Param = optional("tag", Kind::Str, tr_const!("Device tag, e.g. -K1"));

const LABEL: Param = optional("label", Kind::Str, tr_const!("Label: terminal number, PLC address or potential name"));

const FUNCTION_TEXT: Param = optional("function_text", Kind::Str, tr_const!("Function text"));

const PINS: Param = optional("pins", Kind::Str, tr_const!("Connection point designations, comma separated, e.g. 13,14"));

fn func_name(f: Func) -> &'static str {
    match f {
        Func::Main { .. } => "main",
        Func::Aux(_) => "auxiliary",
        Func::Terminal => "terminal",
        Func::Plc => "plc",
        Func::Plug(_) => "plug",
        Func::Potential => "potential",
        Func::Interruption => "interruption",
        Func::Overview => "overview",
    }
}

fn symbol_def(prj: &Project, key: &str) -> Result<SymbolRef, String> {
    prj.symbol(key).ok_or_else(|| tr_fmt!("Unknown symbol: {}", key))
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
fn pin_position(symbols: &Symbols, pg: &Page, spec: &str) -> Result<P, String> {
    let (dev, pin) = spec.rsplit_once(':').ok_or_else(|| tr_fmt!("{} is not a connection point (TAG:PIN)", spec))?;
    let id = dev.strip_prefix('#').and_then(|n| n.parse::<u64>().ok());
    let mut found: Vec<P> = Vec::new();
    for s in &pg.symbols {
        let named = |f: Func| matches!(f, Func::Potential | Func::Interruption) && s.label == dev;
        let hit = match id {
            Some(id) => s.id == id,
            None => s.tag == dev || symbols.get(&s.sym).is_some_and(|d| named(d.func)),
        };
        if hit {
            for (label, q) in pins_of(symbols, s) {
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

fn list_symbols(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let query = a.str("query").unwrap_or_default().to_lowercase();
    let category = a.str("category").map(|c| c.trim().to_ascii_lowercase());
    let user: Vec<SymbolRef> = prj.symbols.iter().filter_map(|u| prj.symbol(&u.key)).collect();
    let list = symbols::library()
        .iter()
        .map(SymbolRef::Builtin)
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

fn place_symbol(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let symbols = prj.symbol_table();
    let i = editable(prj, a.req("page"))?;
    let def = symbol_def(prj, a.req("symbol"))?;
    // A given tag decides the device the new function belongs to, like the last
    // placed main function in the editor.
    let mut ctx = PlaceCtx::default();
    if let Some(tag) = a.str("tag") {
        match def.func {
            Func::Aux(_) | Func::Overview => {
                ctx.last_main.insert(def.letter.into(), tag.into());
            }
            Func::Terminal => ctx.last_strip = Some(tag.into()),
            Func::Plug(_) => ctx.last_plug = Some(tag.into()),
            Func::Plc => {
                ctx.last_plc.insert(def.key.into(), tag.into());
            }
            _ => {}
        }
    }
    let pos = snapped(a.num("x").unwrap_or_default(), a.num("y").unwrap_or_default());
    let mut sym = ops::new_symbol(prj, i, &ctx, &def, pos, rotation(a)?.unwrap_or(0), a.flag("mirror"));
    if let Some(tag) = a.str("tag").filter(|_| def.func.has_tag()) {
        sym.tag = tag.into();
    }
    apply_symbol_params(&mut sym, a)?;
    let out = symbol_json(&symbols, &sym);
    prj.pages[i].symbols.push(sym);
    Ok(out)
}

fn update_symbol(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let symbols = prj.symbol_table();
    let i = editable(prj, a.req("page"))?;
    let id = a.int("id").unwrap_or_default() as u64;
    let page = &mut prj.pages[i];
    let old = page.symbols.iter().find(|x| x.id == id).ok_or_else(|| tr_fmt!("No symbol with id {}", id))?.pos;
    let to = snapped(a.num("x").unwrap_or(old.x), a.num("y").unwrap_or(old.y));
    if !to.same(old) {
        let sel = BTreeSet::from([id]);
        let attached = ops::attached_wire_ends(&symbols, page, &sel);
        ops::translate(page, &sel, &attached, to - old);
    }
    let sym = page.symbols.iter_mut().find(|x| x.id == id).ok_or_else(|| tr_fmt!("No symbol with id {}", id))?;
    if let Some(tag) = a.str("tag") {
        sym.tag = tag.into();
    }
    apply_symbol_params(sym, a)?;
    Ok(symbol_json(&symbols, sym))
}

fn add_wire(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let i = editable(prj, a.req("page"))?;
    let pts: Vec<P> = a.points("points").into_iter().map(|[x, y]| snapped(x, y)).collect();
    if pts.len() < 2 {
        return Err(tr("A wire needs at least two points").to_string());
    }
    Ok(json!({ "ids": push_path(prj, i, &pts) }))
}

fn connect(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let symbols = prj.symbol_table();
    let i = editable(prj, a.req("page"))?;
    let from = pin_position(&symbols, &prj.pages[i], a.req("from"))?;
    let to = pin_position(&symbols, &prj.pages[i], a.req("to"))?;
    let mut pts = vec![from];
    pts.extend(a.points("via").into_iter().map(|[x, y]| snapped(x, y)));
    pts.push(to);
    let path = orthogonal(&pts);
    let ids = push_path(prj, i, &path);
    Ok(json!({ "ids": ids, "points": path.into_iter().map(xy).collect::<Vec<_>>() }))
}

fn update_wire(prj: &mut Project, a: &Args) -> Result<Value, String> {
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
    if let Some(m) = a.str("medium") {
        w.medium = if m.trim().is_empty() { None } else { Some(Medium::from_code(m).ok_or_else(|| tr_fmt!("Unknown medium: {}", m))?) };
    }
    if let Some(l) = a.num("length") {
        w.length = l.max(0.0);
    }
    let medium = w.medium.map(|m| m.code()).unwrap_or_default();
    Ok(json!({ "id": w.id, "number": w.number, "color": w.color, "cross_section": w.cross_section, "cable": w.cable, "core": w.core, "medium": medium, "length": w.length }))
}

/// Takes the style parameters of `add_text` / `update_text` that are given.
fn text_style_args(t: &mut TextItem, a: &Args) -> Result<(), String> {
    if let Some(s) = a.num("size") {
        t.size = s.max(0.5);
    }
    if let Some(v) = a.num("angle") {
        t.angle = v.rem_euclid(360.0);
    }
    if let Some(v) = a.str("align") {
        t.align = match v.trim().to_ascii_lowercase().as_str() {
            "left" => Anchor::Left,
            "center" | "centre" => Anchor::Center,
            "right" => Anchor::Right,
            _ => return Err(tr_fmt!("Unknown alignment: {}", v)),
        };
    }
    if let Some(v) = a.str("valign") {
        t.valign = match v.trim().to_ascii_lowercase().as_str() {
            "top" => VAnchor::Top,
            "middle" => VAnchor::Middle,
            "bottom" => VAnchor::Bottom,
            _ => return Err(tr_fmt!("Unknown vertical anchor: {}", v)),
        };
    }
    if let Some(w) = a.num("width") {
        t.width = w.max(0.0);
    }
    for (name, field) in [
        ("bold", &mut t.bold),
        ("italic", &mut t.italic),
        ("underline", &mut t.underline),
        ("frame", &mut t.frame),
        ("background", &mut t.background),
        ("path_function", &mut t.path_function),
    ] {
        if let Some(v) = a.value(name).and_then(Value::as_bool) {
            *field = v;
        }
    }
    if let Some(l) = a.str("link") {
        t.link = l.trim().to_string();
    }
    Ok(())
}

fn text_json(t: &TextItem) -> Value {
    let align = match t.align {
        Anchor::Left => "left",
        Anchor::Center => "center",
        Anchor::Right => "right",
    };
    let valign = match t.valign {
        VAnchor::Top => "top",
        VAnchor::Middle => "middle",
        VAnchor::Bottom => "bottom",
    };
    json!({
        "id": t.id, "x": t.pos.x, "y": t.pos.y, "text": t.text, "size": t.size, "angle": t.angle, "align": align, "valign": valign,
        "width": t.width, "bold": t.bold, "italic": t.italic, "underline": t.underline, "frame": t.frame, "background": t.background,
        "link": t.link, "path_function": t.path_function
    })
}

fn add_text(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let i = editable(prj, a.req("page"))?;
    let id = prj.new_id();
    let pos = snapped(a.num("x").unwrap_or_default(), a.num("y").unwrap_or_default());
    let mut t = TextItem { id, pos, text: a.req("text").into(), size: 3.5, ..Default::default() };
    text_style_args(&mut t, a)?;
    let out = text_json(&t);
    prj.pages[i].texts.push(t);
    Ok(out)
}

fn update_text(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let i = editable(prj, a.req("page"))?;
    let id = a.int("id").unwrap_or_default() as u64;
    let t = prj.pages[i].texts.iter_mut().find(|t| t.id == id).ok_or_else(|| tr_fmt!("No text with id {}", id))?;
    if let Some(s) = a.str("text") {
        t.text = s.into();
    }
    if let (Some(x), Some(y)) = (a.num("x"), a.num("y")) {
        t.pos = snapped(x, y);
    }
    text_style_args(t, a)?;
    Ok(text_json(t))
}

fn move_items(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let symbols = prj.symbol_table();
    let i = editable(prj, a.req("page"))?;
    let page = &mut prj.pages[i];
    let sel = selection(page, a)?;
    let d = snapped(a.num("dx").unwrap_or_default(), a.num("dy").unwrap_or_default());
    let attached = ops::attached_wire_ends(&symbols, page, &sel);
    ops::translate(page, &sel, &attached, d);
    Ok(json!({ "moved": sel.len(), "dx": d.x, "dy": d.y }))
}

fn align_items(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let symbols = prj.symbol_table();
    let i = editable(prj, a.req("page"))?;
    let how = Align::from_code(a.req("align")).ok_or_else(|| tr_fmt!("Unknown alignment: {}", a.req("align")))?;
    let sel = selection(&prj.pages[i], a)?;
    let n = arrange::align(&symbols, &mut prj.pages[i], &sel, how);
    Ok(json!({ "moved": n }))
}

fn distribute_items(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let symbols = prj.symbol_table();
    let i = editable(prj, a.req("page"))?;
    let sel = selection(&prj.pages[i], a)?;
    let n = arrange::distribute(&symbols, &mut prj.pages[i], &sel, !a.flag("vertical"));
    Ok(json!({ "moved": n }))
}

fn copy_items(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let i = editable(prj, a.req("page"))?;
    let sel = selection(&prj.pages[i], a)?;
    let count = a.int("count").unwrap_or(1).clamp(1, 100) as usize;
    let d = p(a.num("dx").unwrap_or_default(), a.num("dy").unwrap_or_default());
    let ids = arrange::multiple_copy(prj, i, &sel, count, d);
    Ok(json!({ "ids": ids }))
}

fn delete_items(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let i = editable(prj, a.req("page"))?;
    let sel = selection(&prj.pages[i], a)?;
    ops::delete(&mut prj.pages[i], &sel);
    Ok(json!({ "deleted": sel.len() }))
}

fn route(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let symbols = prj.symbol_table();
    let i = editable(prj, a.req("page"))?;
    let from = pin_position(&symbols, &prj.pages[i], a.req("from"))?;
    let to = pin_position(&symbols, &prj.pages[i], a.req("to"))?;
    let area = openecraft_core::frame::of_page(prj, i).area();
    let path = routing::route(&prj.symbol_table(), &prj.pages[i], area, from, to);
    let ids = push_path(prj, i, &path);
    Ok(json!({ "ids": ids, "points": path.into_iter().map(xy).collect::<Vec<_>>() }))
}

fn autoconnect(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let i = editable(prj, a.req("page"))?;
    let ids: BTreeSet<u64> = match a.ids("ids") {
        v if v.is_empty() => prj.pages[i].symbols.iter().map(|s| s.id).collect(),
        v => v.into_iter().collect(),
    };
    let before = prj.pages[i].wires.len();
    routing::apply_autoconnect(prj, i, &ids);
    let added: Vec<u64> = prj.pages[i].wires[before..].iter().map(|w| w.id).collect();
    Ok(json!({ "ids": added }))
}

fn add_cable_definition(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let i = editable(prj, a.req("page"))?;
    let pts = a.points("points");
    let [[x0, y0], [x1, y1]] = pts.as_slice() else {
        return Err(tr("A cable definition needs a start and an end point").to_string());
    };
    let cable = a.str("cable").unwrap_or_default();
    if !cable.is_empty() && prj.cable(cable).is_none() {
        return Err(tr_fmt!("No cable {}", cable));
    }
    let id = cabledef::add(prj, i, p(*x0, *y0), p(*x1, *y1), cable);
    let def = prj.pages[i].cable_defs.iter().find(|c| c.id == id).cloned().unwrap_or_default();
    let cores: Vec<Value> = prj.pages[i].wires.iter().filter(|w| w.cable == def.cable).map(|w| json!({ "wire": w.id, "core": w.core })).collect();
    Ok(json!({ "id": id, "cable": def.cable, "cores": cores }))
}

fn add_connection_definition(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let i = editable(prj, a.req("page"))?;
    let pos = p(a.num("x").unwrap_or_default(), a.num("y").unwrap_or_default());
    if conndef::wire_at(&prj.pages[i], pos).is_none() {
        return Err(tr("Click on a wire to place the connection definition point").to_string());
    }
    let open = openecraft_core::netlist::Netlist::build(prj, false);
    let mut cd = conndef::adopt(prj, &open, i, pos);
    for (field, name) in [(&mut cd.number, "number"), (&mut cd.color, "color"), (&mut cd.cross_section, "cross_section")] {
        if let Some(v) = a.str(name) {
            *field = v.to_string();
        }
    }
    cd.id = prj.new_id();
    let id = cd.id;
    prj.pages[i].conn_defs.push(cd);
    let changed = conndef::apply(prj, &open, i, id);
    Ok(json!({ "id": id, "wires_changed": changed }))
}

#[cfg(test)]
mod tests;
