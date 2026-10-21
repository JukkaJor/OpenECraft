//! Project tools: files, pages, symbols, wires, devices, parts, checks, reports and
//! export. Positions are in mm on the A3 page (420 x 297, y downwards) and snap to
//! the 4 mm grid like in the editor.

use super::{optional, param, Args, Kind, Param, Tool};
use crate::session::Session;
use openecraft_core::geom::{p, P, GRID};
use openecraft_core::index::Index;
use openecraft_core::model::*;
use openecraft_core::ops::{self, PlaceCtx};
use openecraft_core::parts::{self, PartsDb};
use openecraft_core::reports::{Analysis, Severity};
use openecraft_core::symbols::{self, Func, SymbolDef};
use openecraft_core::{dxf, harness, pdf, plc, reports, sample, svg, wirenum};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const PAGE: Param = param("page", Kind::Str, tr_const!("Page name as shown by list_pages"));
const DISCARD: Param = optional("discard", Kind::Bool, tr_const!("Discard unsaved changes of the open project"));
const ROTATION: Param = optional("rotation", Kind::Int, tr_const!("Rotation in degrees clockwise: 0, 90, 180 or 270"));
const MIRROR: Param = optional("mirror", Kind::Bool, tr_const!("Mirror the symbol"));
const TAG: Param = optional("tag", Kind::Str, tr_const!("Device tag, e.g. -K1"));
const LABEL: Param = optional("label", Kind::Str, tr_const!("Label: terminal number, PLC address or potential name"));
const FUNCTION_TEXT: Param = optional("function_text", Kind::Str, tr_const!("Function text"));
const PINS: Param = optional("pins", Kind::Str, tr_const!("Connection point designations, comma separated, e.g. 13,14"));

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "new_project",
        help: tr_const!("Creates a new project (optionally the demo project) and saves it if a path is given"),
        params: &[
            optional("path", Kind::Str, tr_const!("File to save the project to (.oec)")),
            optional("name", Kind::Str, tr_const!("Project name")),
            optional("demo", Kind::Bool, tr_const!("Start from the demo project")),
            DISCARD,
        ],
        needs_project: false,
        run: new_project,
    },
    Tool {
        name: "open_project",
        help: tr_const!("Opens a project file"),
        params: &[param("path", Kind::Str, tr_const!("Project file (.oec)")), DISCARD],
        needs_project: false,
        run: open_project,
    },
    Tool {
        name: "save_project",
        help: tr_const!("Saves the project to its file or to a new path"),
        params: &[optional("path", Kind::Str, tr_const!("New file name (save as)"))],
        needs_project: true,
        run: save_project,
    },
    Tool {
        name: "project_info",
        help: tr_const!("Shows the project properties, file, number of pages and devices and the check result"),
        params: &[],
        needs_project: true,
        run: project_info,
    },
    Tool {
        name: "set_project_info",
        help: tr_const!("Changes project properties"),
        params: &[
            optional("name", Kind::Str, tr_const!("Project name")),
            optional("description", Kind::Str, tr_const!("Description")),
            optional("company", Kind::Str, tr_const!("Company")),
            optional("creator", Kind::Str, tr_const!("Creator")),
            optional("date", Kind::Str, tr_const!("Date")),
            optional("revision", Kind::Str, tr_const!("Revision")),
            optional("properties", Kind::Object, tr_const!("User defined properties (name: value; an empty value removes it)")),
        ],
        needs_project: true,
        run: set_project_info,
    },
    Tool {
        name: "list_pages",
        help: tr_const!("Lists the pages of the project"),
        params: &[],
        needs_project: true,
        run: list_pages,
    },
    Tool {
        name: "get_page",
        help: tr_const!("Shows the content of a page: symbols with their connection point positions, wires and texts"),
        params: &[PAGE],
        needs_project: true,
        run: get_page,
    },
    Tool {
        name: "add_page",
        help: tr_const!("Adds a page"),
        params: &[
            param("kind", Kind::Str, tr_const!("schematic, cover or a report: table_of_contents, device_list, bill_of_materials, terminal_diagram, connection_list, plc_assignment, potential_list, panel_wiring, cable_overview, cable_diagram, revision_overview, harness_data")),
            optional("after", Kind::Str, tr_const!("Insert after this page (default: at the end)")),
            optional("name", Kind::Str, tr_const!("Page name (default: next number)")),
            optional("description", Kind::Str, tr_const!("Description")),
        ],
        needs_project: true,
        run: add_page,
    },
    Tool {
        name: "set_page",
        help: tr_const!("Changes name, description or structure of a page"),
        params: &[
            PAGE,
            optional("name", Kind::Str, tr_const!("New page name")),
            optional("description", Kind::Str, tr_const!("Description")),
            optional("function", Kind::Str, tr_const!("Function aspect (=) of the page; the devices on the page move along")),
            optional("location", Kind::Str, tr_const!("Location aspect (+) of the page; the devices on the page move along")),
        ],
        needs_project: true,
        run: set_page,
    },
    Tool {
        name: "delete_page",
        help: tr_const!("Deletes a page"),
        params: &[PAGE],
        needs_project: true,
        run: delete_page,
    },
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
    Tool {
        name: "list_devices",
        help: tr_const!("Lists the devices with cross references and assigned parts"),
        params: &[],
        needs_project: true,
        run: list_devices,
    },
    Tool {
        name: "rename_device",
        help: tr_const!("Renames a device with all its functions"),
        params: &[
            param("old", Kind::Str, tr_const!("Current device tag")),
            param("new", Kind::Str, tr_const!("New device tag")),
        ],
        needs_project: true,
        run: rename_device,
    },
    Tool {
        name: "assign_part",
        help: tr_const!("Assigns a part to a device (from the project, the built-in catalog or the parts server)"),
        params: &[
            param("tag", Kind::Str, tr_const!("Device tag")),
            param("number", Kind::Str, tr_const!("Part number")),
            optional("count", Kind::Int, tr_const!("Quantity (default 1)")),
            optional("replace", Kind::Bool, tr_const!("Remove the parts assigned so far")),
        ],
        needs_project: true,
        run: assign_part,
    },
    Tool {
        name: "check",
        help: tr_const!("Checks the project (errors, warnings, hints)"),
        params: &[],
        needs_project: true,
        run: check,
    },
    Tool {
        name: "report",
        help: tr_const!("Generates a report as table"),
        params: &[
            param("kind", Kind::Str, tr_const!("table_of_contents, device_list, bill_of_materials, terminal_diagram, connection_list, plc_assignment, potential_list, panel_wiring, cable_overview, cable_diagram, revision_overview or harness_data")),
            optional("format", Kind::Str, tr_const!("json (default) or csv")),
        ],
        needs_project: true,
        run: report,
    },
    Tool {
        name: "renumber_wires",
        help: tr_const!("Numbers the wires by the numbering scheme of the project"),
        params: &[optional("overwrite_fixed", Kind::Bool, tr_const!("Also renumber wires with a fixed number"))],
        needs_project: true,
        run: renumber_wires,
    },
    Tool {
        name: "export",
        help: tr_const!("Exports the project: svg (one page), html, pdf, dxf, wire_list (CSV), plc_table (CSV) or kbl (harness XML)"),
        params: &[
            param("format", Kind::Str, tr_const!("svg, html, pdf, dxf, wire_list, plc_table or kbl")),
            optional("path", Kind::Str, tr_const!("Output file; without it the content is returned")),
            optional("page", Kind::Str, tr_const!("Page (svg: required, dxf: default all pages)")),
        ],
        needs_project: true,
        run: export,
    },
    Tool { name: "undo", help: tr_const!("Undoes the last change"), params: &[], needs_project: true, run: undo },
    Tool { name: "redo", help: tr_const!("Redoes the last undone change"), params: &[], needs_project: true, run: redo },
];

// ---------------------------------------------------------------------- helpers

/// `DeviceList` → `device_list`.
fn snake(debug: impl std::fmt::Debug) -> String {
    let mut out = String::new();
    for (i, c) in format!("{debug:?}").chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

fn report_kind(name: &str) -> Result<ReportKind, String> {
    let name = name.trim().to_ascii_lowercase().replace('-', "_");
    ReportKind::ALL.into_iter().find(|k| snake(k) == name).ok_or_else(|| tr_fmt!("Unknown report: {}", name))
}

fn kind_name(kind: PageKind) -> String {
    match kind {
        PageKind::Schematic => "schematic".into(),
        PageKind::Cover => "cover".into(),
        PageKind::Report(r) => snake(r),
        PageKind::Mounting(_) => "mounting".into(),
    }
}

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

fn severity_name(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn page_index(prj: &Project, name: &str) -> Result<usize, String> {
    prj.pages.iter().position(|pg| pg.name == name).ok_or_else(|| tr_fmt!("No page named {}", name))
}

/// Page that may be changed.
fn editable(prj: &Project, name: &str) -> Result<usize, String> {
    let i = page_index(prj, name)?;
    if prj.pages[i].locked {
        return Err(tr_fmt!("Page {} is locked", name));
    }
    Ok(i)
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

fn xy(q: P) -> Value {
    json!([q.x, q.y])
}

fn pin_list(text: &str) -> Vec<String> {
    text.split(',').map(|s| s.trim().to_string()).collect()
}

fn ensure_unsaved_ok(s: &Session, a: &Args) -> Result<(), String> {
    if s.unsaved() && !a.flag("discard") {
        return Err(tr("The open project has unsaved changes: save it first or pass discard").to_string());
    }
    Ok(())
}

/// Connection point designations and page positions of a placed symbol. Points
/// without designation (terminals) are numbered 1, 2, …
fn pins_of(s: &SymbolInst) -> Vec<(String, P)> {
    let Some(def) = symbols::get(&s.sym) else { return Vec::new() };
    def.pins
        .iter()
        .enumerate()
        .map(|(i, pd)| {
            let label = s.pins.get(i).map(String::as_str).unwrap_or(pd.label);
            let label = if label.is_empty() { (i + 1).to_string() } else { label.to_string() };
            (label, def.pin_world(i, s.pos, s.rot, s.mirror))
        })
        .collect()
}

fn symbol_json(s: &SymbolInst) -> Value {
    let pins: Vec<Value> = pins_of(s).into_iter().map(|(label, q)| json!({ "pin": label, "x": q.x, "y": q.y })).collect();
    json!({
        "id": s.id,
        "symbol": s.sym,
        "tag": s.tag,
        "label": s.label,
        "function_text": s.function_text,
        "x": s.pos.x,
        "y": s.pos.y,
        "rotation": s.rot as u32 * 90,
        "mirror": s.mirror,
        "pins": pins,
    })
}

fn page_json(pg: &Page) -> Value {
    json!({
        "name": pg.name,
        "description": pg.description,
        "kind": kind_name(pg.kind),
        "function": pg.function,
        "location": pg.location,
        "locked": pg.locked,
    })
}

fn summary(s: &Session) -> Result<Value, String> {
    let prj = s.project()?;
    let an = Analysis::build(prj);
    let count = |sev| an.issues.iter().filter(|i| i.severity == sev).count();
    let i = &prj.info;
    Ok(json!({
        "name": i.name,
        "description": i.description,
        "company": i.company,
        "creator": i.creator,
        "date": i.date,
        "revision": i.revision,
        "properties": i.properties,
        "file": s.path.as_ref().map(|p| p.display().to_string()),
        "unsaved": s.unsaved(),
        "pages": prj.pages.len(),
        "devices": Index::build(prj).devices.len(),
        "errors": count(Severity::Error),
        "warnings": count(Severity::Warning),
        "can_undo": s.can_undo(),
        "can_redo": s.can_redo(),
    }))
}

// ---------------------------------------------------------------------- files

fn new_project(s: &mut Session, a: &Args) -> Result<Value, String> {
    ensure_unsaved_ok(s, a)?;
    let mut prj = if a.flag("demo") { sample::demo_project() } else { Project::default() };
    if let Some(name) = a.str("name") {
        prj.info.name = name.into();
    }
    s.replace(prj, None);
    if let Some(path) = a.str("path") {
        s.save(Some(path))?;
    }
    summary(s)
}

fn open_project(s: &mut Session, a: &Args) -> Result<Value, String> {
    ensure_unsaved_ok(s, a)?;
    s.open(a.req("path"))?;
    summary(s)
}

fn save_project(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = s.save(a.str("path"))?;
    Ok(json!({ "path": path.display().to_string() }))
}

fn project_info(s: &mut Session, _: &Args) -> Result<Value, String> {
    summary(s)
}

fn set_project_info(s: &mut Session, a: &Args) -> Result<Value, String> {
    let info = &mut s.project_mut()?.info;
    for (name, field) in [
        ("name", &mut info.name),
        ("description", &mut info.description),
        ("company", &mut info.company),
        ("creator", &mut info.creator),
        ("date", &mut info.date),
        ("revision", &mut info.revision),
    ] {
        if let Some(v) = a.str(name) {
            *field = v.into();
        }
    }
    if let Some(Value::Object(props)) = a.value("properties") {
        for (k, v) in props {
            match v.as_str().unwrap_or_default() {
                "" => info.properties.remove(k),
                v => info.properties.insert(k.clone(), v.into()),
            };
        }
    }
    summary(s)
}

// ---------------------------------------------------------------------- pages

fn list_pages(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let pages = prj.pages.iter().map(|pg| {
        let mut v = page_json(pg);
        v["symbols"] = pg.symbols.len().into();
        v["wires"] = pg.wires.len().into();
        v["texts"] = pg.texts.len().into();
        v
    });
    Ok(Value::Array(pages.collect()))
}

fn get_page(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let pg = &prj.pages[page_index(prj, a.req("page"))?];
    let mut v = page_json(pg);
    v["symbols"] = pg.symbols.iter().map(symbol_json).collect();
    v["wires"] = pg
        .wires
        .iter()
        .map(|w| {
            json!({
                "id": w.id,
                "from": xy(w.a),
                "to": xy(w.b),
                "number": w.number,
                "color": w.color,
                "cross_section": w.cross_section,
                "cable": w.cable,
                "core": w.core,
            })
        })
        .collect();
    v["texts"] = pg.texts.iter().map(|t| json!({ "id": t.id, "x": t.pos.x, "y": t.pos.y, "text": t.text, "size": t.size })).collect();
    v["shapes"] = serde_json::to_value(&pg.shapes).unwrap_or_default();
    Ok(v)
}

fn add_page(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let kind = match a.req("kind").trim().to_ascii_lowercase().as_str() {
        "schematic" => PageKind::Schematic,
        "cover" => PageKind::Cover,
        other => PageKind::Report(report_kind(other)?),
    };
    let at = match a.str("after") {
        Some(after) => page_index(prj, after)? + 1,
        None => prj.pages.len(),
    };
    if let Some(name) = a.str("name") {
        if prj.pages.iter().any(|pg| pg.name == name) {
            return Err(tr_fmt!("A page named {} already exists", name));
        }
    }
    let i = ops::add_page(prj, kind, at);
    let pg = &mut prj.pages[i];
    if let Some(name) = a.str("name") {
        pg.name = name.into();
    }
    if let Some(d) = a.str("description") {
        pg.description = d.into();
    }
    Ok(page_json(pg))
}

fn set_page(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = editable(prj, a.req("page"))?;
    if let Some(name) = a.str("name") {
        if prj.pages.iter().enumerate().any(|(j, pg)| j != i && pg.name == name) {
            return Err(tr_fmt!("A page named {} already exists", name));
        }
        prj.pages[i].name = name.into();
    }
    if let Some(d) = a.str("description") {
        prj.pages[i].description = d.into();
    }
    if a.str("function").is_some() || a.str("location").is_some() {
        let function = a.str("function").map(String::from).unwrap_or_else(|| prj.pages[i].function.clone());
        let location = a.str("location").map(String::from).unwrap_or_else(|| prj.pages[i].location.clone());
        ops::set_page_structure(prj, i, &function, &location, true);
    }
    Ok(page_json(&prj.pages[i]))
}

fn delete_page(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = editable(prj, a.req("page"))?;
    if prj.pages.len() == 1 {
        return Err(tr("The last page cannot be deleted").to_string());
    }
    let pg = prj.pages.remove(i);
    Ok(json!({ "deleted": pg.name }))
}

// ---------------------------------------------------------------------- symbols and wires

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

fn add_wire(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = editable(prj, a.req("page"))?;
    let pts: Vec<P> = a.points("points").into_iter().map(|[x, y]| snapped(x, y)).collect();
    if pts.len() < 2 {
        return Err(tr("A wire needs at least two points").to_string());
    }
    Ok(json!({ "ids": push_path(prj, i, &pts) }))
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

/// The ids of `a` that exist on the page.
fn selection(pg: &Page, a: &Args) -> Result<BTreeSet<u64>, String> {
    let all: BTreeSet<u64> = ops::all_ids(pg).into_iter().collect();
    let sel: BTreeSet<u64> = a.ids("ids").into_iter().collect();
    match sel.difference(&all).next() {
        Some(id) => Err(tr_fmt!("No item with id {}", id)),
        None => Ok(sel),
    }
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

// ---------------------------------------------------------------------- devices and parts

fn list_devices(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let idx = Index::build(prj);
    let refs = |locs: &[openecraft_core::index::Loc]| locs.iter().map(|l| l.reference(prj)).collect::<Vec<_>>();
    let list = idx.sorted_tags().into_iter().map(|tag| {
        let e = &idx.devices[tag];
        let data = prj.devices.get(tag);
        let parts: Vec<Value> = data.map(|d| d.parts.iter().map(|r| json!({ "number": r.number, "count": r.count })).collect()).unwrap_or_default();
        json!({
            "tag": tag,
            "description": data.map(|d| d.description.as_str()).unwrap_or_default(),
            "parts": parts,
            "main": refs(&e.mains),
            "auxiliary": refs(&e.auxs),
            "other": refs(&e.others),
            "overview": refs(&e.overviews),
        })
    });
    Ok(Value::Array(list.collect()))
}

fn rename_device(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let (old, new) = (a.req("old"), a.req("new").trim());
    let tags = prj.all_tags();
    if !tags.iter().any(|t| t == old) {
        return Err(tr_fmt!("No device {}", old));
    }
    if new.is_empty() || tags.iter().any(|t| t == new) {
        return Err(tr_fmt!("Device {} already exists", new));
    }
    ops::rename_device(prj, old, new);
    Ok(json!({ "old": old, "new": new }))
}

fn assign_part(s: &mut Session, a: &Args) -> Result<Value, String> {
    let (tag, number) = (a.req("tag"), a.req("number"));
    let prj = s.project()?;
    if !prj.all_tags().iter().any(|t| t == tag) {
        return Err(tr_fmt!("No device {}", tag));
    }
    let mut db = PartsDb::default();
    let known = parts::lookup(prj, Some(&db), number).cloned();
    let part = match known {
        Some(p) => p,
        None => s.server.get(number).map_err(|e| tr_fmt!("Unknown part {} (parts server: {})", number, e))?.ok_or_else(|| tr_fmt!("Unknown part: {}", number))?,
    };
    db.upsert(part);
    let prj = s.project_mut()?;
    parts::ensure_in_project(prj, &db, number);
    let dev = prj.devices.entry(tag.to_string()).or_default();
    if a.flag("replace") {
        dev.parts.clear();
    }
    let count = a.int("count").unwrap_or(1).max(1) as u32;
    match dev.parts.iter_mut().find(|r| r.number == number) {
        Some(r) => r.count += count,
        None => dev.parts.push(parts::PartRef { number: number.into(), count }),
    }
    let assigned: Vec<Value> = dev.parts.iter().map(|r| json!({ "number": r.number, "count": r.count })).collect();
    let designated = parts::apply_designations(prj, Some(&db), tag);
    Ok(json!({ "tag": tag, "parts": assigned, "designations_applied": designated }))
}

// ---------------------------------------------------------------------- checks, reports, export

fn check(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let an = Analysis::build(prj);
    let issues: Vec<Value> = an
        .issues
        .iter()
        .map(|i| {
            json!({
                "severity": severity_name(i.severity),
                "message": i.message,
                "page": i.target.map(|(pg, _)| prj.pages[pg].name.clone()),
                "id": i.target.map(|(_, id)| id).filter(|id| *id != 0),
            })
        })
        .collect();
    let count = |sev| an.issues.iter().filter(|i| i.severity == sev).count();
    Ok(json!({ "errors": count(Severity::Error), "warnings": count(Severity::Warning), "issues": issues }))
}

fn report(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let kind = report_kind(a.req("kind"))?;
    let an = Analysis::build(prj);
    let table = an.reports.get(&kind).cloned().unwrap_or_default();
    match a.str("format").unwrap_or("json") {
        "csv" => Ok(Value::String(table.to_csv())),
        "json" => Ok(json!({ "report": snake(kind), "title": kind.title(), "headers": table.headers, "rows": table.rows })),
        other => Err(tr_fmt!("Unknown format: {}", other)),
    }
}

fn renumber_wires(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let an = Analysis::build(prj);
    let n = wirenum::renumber(prj, &an.open, &an.netlist, a.flag("overwrite_fixed"));
    Ok(json!({ "renumbered": n }))
}

fn export(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let page = a.str("page").map(|n| page_index(prj, n)).transpose()?;
    let content = match a.req("format") {
        "svg" => svg::page_svg(prj, page.ok_or_else(|| tr("This format needs a page").to_string())?),
        "html" => svg::print_document(prj),
        "pdf" => pdf::document(prj),
        "dxf" => {
            let pages: Vec<usize> = page.map(|p| vec![p]).unwrap_or_else(|| (0..prj.pages.len()).collect());
            dxf::pages_dxf(prj, &pages)
        }
        "wire_list" => reports::production_wire_list(prj, &Analysis::build(prj)).to_csv(),
        "plc_table" => plc::symbol_table(prj, &Analysis::build(prj).netlist),
        "kbl" => harness::kbl(prj, &Analysis::build(prj)),
        other => return Err(tr_fmt!("Unknown format: {}", other)),
    };
    match a.str("path") {
        Some(path) => {
            std::fs::write(path, &content).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
            Ok(json!({ "path": path, "bytes": content.len() }))
        }
        None => Ok(Value::String(content)),
    }
}

fn undo(s: &mut Session, _: &Args) -> Result<Value, String> {
    let done = s.undo()?;
    Ok(json!({ "undone": done, "can_undo": s.can_undo(), "can_redo": s.can_redo() }))
}

fn redo(s: &mut Session, _: &Args) -> Result<Value, String> {
    let done = s.redo()?;
    Ok(json!({ "redone": done, "can_undo": s.can_undo(), "can_redo": s.can_redo() }))
}

#[cfg(test)]
mod tests;
