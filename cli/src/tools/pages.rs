//! Page tools: list, show, add, change and delete pages.

use super::common::*;
use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::model::*;
use openecraft_core::netlist::Netlist;
use openecraft_core::structure::{Aspect, Dt};
use openecraft_core::{dwg, dxf_import, frame, ops, page_templates, potentials};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "list_pages",
        help: tr_const!("Lists the pages of the project"),
        params: &[optional("work_area", Kind::Str, tr_const!("Only the pages of this work area (list_structure)"))],
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
            optional("kind", Kind::Str, tr_const!("schematic, overview, graphic, cover or a report: table_of_contents, device_list, bill_of_materials, terminal_diagram, terminal_lineup, terminal_strip_overview, terminal_connections, plug_diagram, pin_connections, connection_list, device_connections, plc_assignment, potential_list, panel_wiring, cable_overview, cable_diagram, cable_connections, cable_sizing, revision_overview, harness_data, duct_fill, panel_legend, climate, marking, hose_list, pct_loops, fluid_netlist, hose_configuration, tubing_list, fluid_device_groups, electro_fluid")),
            optional("template", Kind::Str, tr_const!("Page template (list_page_templates) instead of kind")),
            optional("frame", Kind::Str, tr_const!("Plot frame (list_frames); default: the project's")),
            optional("after", Kind::Str, tr_const!("Insert after this page (default: at the end)")),
            optional("name", Kind::Str, tr_const!("Page name (default: next number)")),
            optional("description", Kind::Str, tr_const!("Description")),
        ],
        needs_project: true,
        run: add_page,
    },
    Tool {
        name: "import_dxf",
        help: tr_const!("Imports a DXF drawing as graphics (lines, arcs, texts, blocks) on a new schematic page"),
        params: &[
            param("path", Kind::Str, tr_const!("DXF file")),
            optional("after", Kind::Str, tr_const!("Insert after this page (default: at the end)")),
            optional("description", Kind::Str, tr_const!("Description")),
            optional("fit", Kind::Bool, tr_const!("Scale drawings down that do not fit on the page (default: true)")),
        ],
        needs_project: true,
        run: import_drawing,
    },
    Tool {
        name: "import_dwg",
        help: tr_const!("Imports a DWG drawing (AutoCAD R12 … 2018) as graphics (lines, arcs, texts, blocks) on a new schematic page"),
        params: &[
            param("path", Kind::Str, tr_const!("DWG file")),
            optional("after", Kind::Str, tr_const!("Insert after this page (default: at the end)")),
            optional("description", Kind::Str, tr_const!("Description")),
            optional("fit", Kind::Bool, tr_const!("Scale drawings down that do not fit on the page (default: true)")),
        ],
        needs_project: true,
        run: import_drawing,
    },
    Tool {
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
];

fn list_pages(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
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

fn get_page(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let pg = &prj.pages[page_index(prj, a.req("page"))?];
    let (nl, open) = (Netlist::build(prj, true), Netlist::build(prj, false));
    let mut v = page_json(pg);
    v["symbols"] = pg.symbols.iter().map(symbol_json).collect();
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

/// `import_dxf` and `import_dwg`: the format is told apart by the content.
fn import_drawing(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("path");
    let bytes = std::fs::read(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let prj = s.project_mut()?;
    let after = match a.str("after") {
        Some(n) => page_index(prj, n)?,
        None => prj.pages.len() - 1,
    };
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let description = a.str("description").map(String::from).unwrap_or_else(|| file.rsplit_once('.').map(|x| x.0).unwrap_or(file).to_string());
    let fit = a.0.get("fit").and_then(Value::as_bool).unwrap_or(true);
    let r = dwg::import_drawing(prj, &bytes, &description, after, dxf_import::DxfOptions { fit })?;
    let mut v = page_json(&prj.pages[r.page]);
    v["lines"] = json!(r.lines);
    v["texts"] = json!(r.texts);
    v["scale"] = json!(r.scale);
    v["not_imported"] = json!(r.skipped);
    Ok(v)
}

fn add_page(s: &mut Session, a: &Args) -> Result<Value, String> {
    let report = match a.str("kind").map(|k| k.trim().to_ascii_lowercase()) {
        Some(k) if !matches!(k.as_str(), "schematic" | "overview" | "graphic" | "graphics" | "cover") => Some(report_id(s, &k)?),
        _ => None,
    };
    let prj = s.project_mut()?;
    let template = match a.str("template") {
        Some(name) => Some(page_templates::find(prj, name).ok_or_else(|| tr_fmt!("No page template named {}", name))?),
        None => None,
    };
    let kind = match (a.str("kind").map(|k| k.trim().to_ascii_lowercase()), &template) {
        (Some(k), _) => match k.as_str() {
            "schematic" => PageKind::Schematic,
            "overview" => PageKind::Overview,
            "graphic" | "graphics" => PageKind::Graphic,
            "cover" => PageKind::Cover,
            _ => PageKind::Report(report.unwrap_or(ReportKind::DeviceList.into())),
        },
        (None, Some(t)) => t.kind,
        (None, None) => return Err(tr("Give kind or template").into()),
    };
    if let Some(f) = a.str("frame") {
        if !frame::names(prj).iter().any(|n| n == f) {
            return Err(tr_fmt!("No plot frame named {}", f));
        }
    }
    let at = match a.str("after") {
        Some(after) => page_index(prj, after)? + 1,
        None => prj.pages.len(),
    };
    if let Some(name) = a.str("name") {
        if prj.pages.iter().any(|pg| pg.name == name) {
            return Err(tr_fmt!("A page named {} already exists", name));
        }
    }
    let i = match template {
        Some(mut t) => {
            t.kind = kind;
            page_templates::insert(prj, &t, at)
        }
        None => ops::add_page(prj, kind, at),
    };
    let pg = &mut prj.pages[i];
    if let Some(f) = a.str("frame") {
        pg.frame = f.into();
    }
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

fn delete_page(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = page_index(prj, a.req("page"))?;
    let pg = ops::delete_page(prj, i).map_err(tr)?;
    Ok(json!({ "deleted": pg.name }))
}

#[cfg(test)]
mod tests;
