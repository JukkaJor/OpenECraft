//! Page commands: list, show, add, change and delete pages.

use crate::builtin::common::*;
use crate::params::{optional, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::model::*;
use openecraft_core::netlist::Netlist;
use openecraft_core::structure::{Aspect, Dt};
use openecraft_core::numbering::{self, PageNumbering};
use openecraft_core::{frame, ops, potentials};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct Pages;

impl Plugin for Pages {
    fn info(&self) -> Info {
        info("pages", tr_const!("Pages"), tr_const!("Lists, changes and deletes pages"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "list_pages",
        help: tr_const!("Lists the pages of the project"),
        params: &[optional("work_area", Kind::Str, tr_const!("Only the pages of this work area (list_structure)"))],
        run: list_pages,
    },
    Spec {
        name: "get_page",
        help: tr_const!("Shows the content of a page: symbols with their connection point positions, wires and texts"),
        params: &[PAGE],
        run: get_page,
    },
    Spec {
        name: "set_page",
        help: tr_const!("Changes name, description or structure of a page"),
        params: &[
            PAGE,
            optional("name", Kind::Str, tr_const!("New page name")),
            optional("description", Kind::Str, tr_const!("Description")),
            optional("frame", Kind::Str, tr_const!("Plot frame (list_frames); empty = the project default")),
            optional("function", Kind::Str, tr_const!("Function aspect (=) of the page; the devices on the page move along")),
            optional("installation", Kind::Str, tr_const!("Installation site aspect (++) of the page; the devices on the page move along")),
            optional("location", Kind::Str, tr_const!("Location aspect (+) of the page; the devices on the page move along")),
            optional("product", Kind::Str, tr_const!("Higher-level product aspect (-) of the page; the devices on the page move along")),
        ],
        run: set_page,
    },
    Spec {
        name: "delete_page",
        help: tr_const!("Deletes a page"),
        params: &[PAGE],
        run: delete_page,
    },
    Spec {
        name: "set_cross_references",
        help: tr_const!("Sets how cross references are shown (only the given settings) and shows the settings"),
        params: &[
            optional("prefix", Kind::Str, tr_const!("In front of the page name (default /)")),
            optional("separator", Kind::Str, tr_const!("Between page and column (default .)")),
            optional("row", Kind::Bool, tr_const!("Show the row after the column")),
            optional("structure", Kind::Str, tr_const!("Structure of the target page: different (default), always or never")),
            optional("interruptions", Kind::Str, tr_const!("Interruption points refer to: all (default), star or pairs")),
            optional("contact_image", Kind::Bool, tr_const!("Draw the contact image below coils (default true)")),
        ],
        run: set_cross_references,
    },
    Spec {
        name: "renumber_pages",
        help: tr_const!("Numbers the pages with a start value and step; released pages keep their name"),
        params: &[
            optional("start", Kind::Int, tr_const!("First page number (default 1)")),
            optional("step", Kind::Int, tr_const!("Counting step (default 1)")),
            optional("per_structure", Kind::Bool, tr_const!("Start again on every page whose structure differs from the page before")),
            optional("subpages", Kind::Bool, tr_const!("Pages with a suffix (3.a) stay sub-pages of the page before (default true)")),
            optional("pages", Kind::Str, tr_const!("Only these pages (names separated by commas)")),
        ],
        run: renumber_pages,
    },
];

fn list_pages(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let area = match a.str("work_area") {
        Some(name) => Some(prj.structures.work_area(name).ok_or_else(|| tr_fmt!("No work area named {}", name))?),
        None => None,
    };
    let pages = prj.pages.iter().filter(|pg| area.is_none_or(|w| w.shows_page(pg))).map(|pg| {
        let mut v = page_json(pg);
        v["symbols"] = pg.symbols.len().into();
        v["wires"] = pg.wires.len().into();
        v["texts"] = pg.texts.len().into();
        v
    });
    Ok(Value::Array(pages.collect()))
}

fn get_page(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let pg = &prj.pages[page_index(prj, a.req("page"))?];
    let (nl, open) = (Netlist::build(prj, true), Netlist::build(prj, false));
    let mut v = page_json(pg);
    let symbols = prj.symbol_table();
    v["symbols"] = pg.symbols.iter().map(|s| symbol_json(&symbols, s)).collect();
    v["wires"] = pg
        .wires
        .iter()
        .map(|w| {
            let auto = potentials::props_of_wire(prj, &nl, &open, w.id);
            json!({
                "id": w.id,
                "from": xy(w.a),
                "to": xy(w.b),
                "number": w.number,
                "color": w.color,
                "cross_section": w.cross_section,
                "cable": w.cable,
                "core": w.core,
                "potential": auto.potential,
                "resulting_color": auto.color,
                "resulting_cross_section": auto.cross_section,
            })
        })
        .collect();
    v["texts"] = pg.texts.iter().map(|t| json!({ "id": t.id, "x": t.pos.x, "y": t.pos.y, "text": t.text, "size": t.size })).collect();
    v["shapes"] = serde_json::to_value(&pg.shapes).unwrap_or_default();
    Ok(v)
}

fn set_page(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let i = unlocked(prj, a.req("page"))?;
    if let Some(name) = a.str("name") {
        if prj.pages.iter().enumerate().any(|(j, pg)| j != i && pg.name == name) {
            return Err(tr_fmt!("A page named {} already exists", name));
        }
        prj.pages[i].name = name.into();
    }
    if let Some(d) = a.str("description") {
        prj.pages[i].description = d.into();
    }
    if let Some(f) = a.str("frame") {
        if !f.is_empty() && !frame::names(prj).iter().any(|n| n == f) {
            return Err(tr_fmt!("No plot frame named {}", f));
        }
        prj.pages[i].frame = f.into();
    }
    if Aspect::ALL.iter().any(|asp| a.str(asp.key()).is_some()) {
        let mut dt = Dt::of_page(&prj.pages[i]);
        for asp in Aspect::ALL {
            if let Some(v) = a.str(asp.key()) {
                *dt.get_mut(asp) = v.to_string();
            }
        }
        ops::set_page_structure(prj, i, &dt, true);
    }
    Ok(page_json(&prj.pages[i]))
}

fn set_cross_references(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let x = &mut prj.xref;
    if let Some(v) = a.str("prefix") {
        x.prefix = v.to_string();
    }
    if let Some(v) = a.str("separator") {
        x.separator = v.to_string();
    }
    if let Some(v) = a.value("row").and_then(Value::as_bool) {
        x.row = v;
    }
    if let Some(v) = a.value("contact_image").and_then(Value::as_bool) {
        x.hide_contact_image = !v;
    }
    if let Some(v) = a.str("structure") {
        x.structure = match v.trim().to_ascii_lowercase().as_str() {
            "different" => XrefStructure::Different,
            "always" => XrefStructure::Always,
            "never" => XrefStructure::Never,
            other => return Err(tr_fmt!("Unknown setting: {}", other)),
        };
    }
    if let Some(v) = a.str("interruptions") {
        x.interruptions = match v.trim().to_ascii_lowercase().as_str() {
            "all" => InterruptionMode::All,
            "star" => InterruptionMode::Star,
            "pairs" => InterruptionMode::Pairs,
            other => return Err(tr_fmt!("Unknown setting: {}", other)),
        };
    }
    let structure = match x.structure {
        XrefStructure::Different => "different",
        XrefStructure::Always => "always",
        XrefStructure::Never => "never",
    };
    let interruptions = match x.interruptions {
        InterruptionMode::All => "all",
        InterruptionMode::Star => "star",
        InterruptionMode::Pairs => "pairs",
    };
    Ok(json!({ "prefix": x.prefix, "separator": x.separator, "row": x.row, "structure": structure, "interruptions": interruptions, "contact_image": !x.hide_contact_image }))
}

fn renumber_pages(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let defaults = PageNumbering::default();
    let opts = PageNumbering {
        start: a.int("start").map_or(defaults.start, |v| v.max(0) as u32),
        step: a.int("step").map_or(defaults.step, |v| v.max(1) as u32),
        per_structure: a.flag("per_structure"),
        subpages: a.value("subpages").and_then(Value::as_bool).unwrap_or(defaults.subpages),
        pages: crate::builtin::devices::page_list(prj, a.str("pages"))?,
    };
    let n = numbering::renumber_pages(prj, &opts);
    Ok(json!({ "renamed": n, "pages": prj.pages.iter().map(|pg| pg.name.clone()).collect::<Vec<_>>() }))
}

fn delete_page(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let i = page_index(prj, a.req("page"))?;
    let pg = ops::delete_page(prj, i).map_err(tr)?;
    Ok(json!({ "deleted": pg.name }))
}
