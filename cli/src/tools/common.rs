//! Helpers shared by the project tools: pages by name, JSON of pages and symbols.

use super::{param, Kind, Param};
use openecraft_core::geom::P;
use openecraft_core::model::*;
use openecraft_core::{structure, symbols};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub const PAGE: Param = param("page", Kind::Str, tr_const!("Page name as shown by list_pages"));

/// `DeviceList` → `device_list`.
pub fn snake(debug: impl std::fmt::Debug) -> String {
    let mut out = String::new();
    for (i, c) in format!("{debug:?}").chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

pub fn report_kind(name: &str) -> Result<ReportKind, String> {
    let name = name.trim().to_ascii_lowercase().replace('-', "_");
    ReportKind::ALL.into_iter().find(|k| snake(k) == name).ok_or_else(|| tr_fmt!("Unknown report: {}", name))
}

pub fn kind_name(kind: PageKind) -> String {
    match kind {
        PageKind::Schematic => "schematic".into(),
        PageKind::Cover => "cover".into(),
        PageKind::Report(r) => snake(r),
        PageKind::Mounting(_) => "mounting".into(),
    }
}

pub fn page_index(prj: &Project, name: &str) -> Result<usize, String> {
    prj.pages.iter().position(|pg| pg.name == name).ok_or_else(|| tr_fmt!("No page named {}", name))
}

/// Page whose properties may be changed (not released).
pub fn unlocked(prj: &Project, name: &str) -> Result<usize, String> {
    let i = page_index(prj, name)?;
    if prj.pages[i].locked {
        return Err(tr_fmt!("Page {} is locked", name));
    }
    Ok(i)
}

/// Page whose content may be changed: like in the editor only unreleased schematic
/// pages, the others are generated.
pub fn editable(prj: &Project, name: &str) -> Result<usize, String> {
    let i = unlocked(prj, name)?;
    if !prj.pages[i].is_editable() {
        return Err(tr_fmt!("Page {} is generated and cannot be edited", name));
    }
    Ok(i)
}

pub fn xy(q: P) -> Value {
    json!([q.x, q.y])
}

/// Connection point designations and page positions of a placed symbol. Points
/// without designation (terminals) are numbered 1, 2, …
pub fn pins_of(s: &SymbolInst) -> Vec<(String, P)> {
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

pub fn symbol_json(s: &SymbolInst) -> Value {
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

pub fn page_json(pg: &Page) -> Value {
    json!({
        "name": pg.name,
        "description": pg.description,
        "kind": kind_name(pg.kind),
        "structure": structure::page_structure(pg),
        "function": pg.function,
        "installation": pg.installation,
        "location": pg.location,
        "product": pg.product,
        "locked": pg.locked,
    })
}
