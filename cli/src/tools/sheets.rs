//! Sheet tools: plot frames (paper format, orientation, columns / rows), page
//! templates and document pages with embedded files.

use super::common::*;
use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::frame::{self, Labels, Paper, PlotFrame};
use openecraft_core::{document, page_templates};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "list_frames",
        help: tr_const!("Lists the plot frames (paper format, orientation, columns and rows) and the project default"),
        params: &[],
        needs_project: true,
        run: list_frames,
    },
    Tool {
        name: "set_frame",
        help: tr_const!("Creates or changes a plot frame of the project (built-in frames are copied under a new name)"),
        params: &[
            param("name", Kind::Str, tr_const!("Name of the frame")),
            optional("based_on", Kind::Str, tr_const!("Frame to start from when the frame is new (default: A3 landscape)")),
            optional("paper", Kind::Str, tr_const!("A0, A1, A2, A3, A4, Letter, Legal, Tabloid or custom")),
            optional("landscape", Kind::Bool, tr_const!("Landscape (true) or portrait (false)")),
            optional("width", Kind::Num, tr_const!("Paper width in mm (custom paper)")),
            optional("height", Kind::Num, tr_const!("Paper height in mm (custom paper)")),
            optional("margin", Kind::Num, tr_const!("Distance of the border from the paper edge in mm")),
            optional("columns", Kind::Int, tr_const!("Number of columns")),
            optional("rows", Kind::Int, tr_const!("Number of rows")),
            optional("column_labels", Kind::Str, tr_const!("numbers or letters")),
            optional("row_labels", Kind::Str, tr_const!("numbers or letters")),
            optional("strips", Kind::Bool, tr_const!("Column and row strips")),
        ],
        needs_project: true,
        run: set_frame,
    },
    Tool {
        name: "delete_frame",
        help: tr_const!("Deletes a plot frame of the project that no page uses"),
        params: &[param("name", Kind::Str, tr_const!("Name of the frame"))],
        needs_project: true,
        run: delete_frame,
    },
    Tool {
        name: "set_default_frame",
        help: tr_const!("Sets the plot frame of all pages that do not choose one"),
        params: &[param("name", Kind::Str, tr_const!("Name of the frame"))],
        needs_project: true,
        run: set_default_frame,
    },
    Tool {
        name: "list_page_templates",
        help: tr_const!("Lists the page templates (built-in and the project's own)"),
        params: &[],
        needs_project: true,
        run: list_page_templates,
    },
    Tool {
        name: "save_page_template",
        help: tr_const!("Saves a page (type, plot frame, title block, structure, description, content) as a page template"),
        params: &[PAGE, param("name", Kind::Str, tr_const!("Name of the template"))],
        needs_project: true,
        run: save_page_template,
    },
    Tool {
        name: "delete_page_template",
        help: tr_const!("Deletes a page template of the project"),
        params: &[param("name", Kind::Str, tr_const!("Name of the template"))],
        needs_project: true,
        run: delete_page_template,
    },
    Tool {
        name: "add_document_page",
        help: tr_const!("Embeds a file (PDF, Office document, image …) as a document page; the PDF export attaches it to the page"),
        params: &[
            param("path", Kind::Str, tr_const!("File to embed")),
            optional("after", Kind::Str, tr_const!("Insert after this page (default: at the end)")),
            optional("description", Kind::Str, tr_const!("Description")),
        ],
        needs_project: true,
        run: add_document_page,
    },
    Tool {
        name: "export_document",
        help: tr_const!("Writes the embedded file of a document page"),
        params: &[PAGE, param("path", Kind::Str, tr_const!("Output file"))],
        needs_project: true,
        run: export_document,
    },
];

fn frame_json(f: &PlotFrame) -> Value {
    let (w, h) = f.size();
    json!({
        "name": f.name,
        "builtin": frame::is_builtin(&f.name),
        "paper": f.paper.name(),
        "landscape": w >= h,
        "width": w,
        "height": h,
        "margin": f.margin,
        "columns": f.columns,
        "rows": f.rows,
        "column_labels": snake(f.column_labels),
        "row_labels": snake(f.row_labels),
        "strips": f.strips,
    })
}

fn labels(s: &str) -> Result<Labels, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "numbers" => Ok(Labels::Numbers),
        "letters" => Ok(Labels::Letters),
        _ => Err(tr_fmt!("Unknown labels: {}", s)),
    }
}

fn list_frames(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let frames: Vec<Value> = frame::names(prj).iter().map(|n| frame_json(&frame::find(prj, n))).collect();
    Ok(json!({ "default": frame::project_default(prj).name, "frames": frames }))
}

fn set_frame(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let name = a.req("name").trim();
    if name.is_empty() {
        return Err(tr("The name must not be empty").into());
    }
    if frame::is_builtin(name) {
        return Err(tr_fmt!("{} is a built-in frame; give the frame a new name", name));
    }
    let mut f = match prj.forms.frames.iter().find(|f| f.name == name) {
        Some(f) => f.clone(),
        None => frame::find(prj, a.str("based_on").unwrap_or(frame::DEFAULT)),
    };
    f.name = name.into();
    if let Some(paper) = a.str("paper") {
        f.paper = Paper::from_name(paper).ok_or_else(|| tr_fmt!("Unknown paper format: {}", paper))?;
    }
    if let Some(l) = a.value("landscape").and_then(Value::as_bool) {
        f.landscape = l;
    }
    if a.num("width").is_some() || a.num("height").is_some() {
        let (w, h) = f.size();
        f.custom = [a.num("width").unwrap_or(w), a.num("height").unwrap_or(h)];
        f.paper = Paper::Custom;
    }
    if let Some(m) = a.num("margin") {
        f.margin = m.clamp(0.0, 50.0);
    }
    if let Some(c) = a.int("columns") {
        f.columns = c.clamp(1, 60) as usize;
    }
    if let Some(r) = a.int("rows") {
        f.rows = r.clamp(1, 40) as usize;
    }
    if let Some(l) = a.str("column_labels") {
        f.column_labels = labels(l)?;
    }
    if let Some(l) = a.str("row_labels") {
        f.row_labels = labels(l)?;
    }
    if let Some(st) = a.value("strips").and_then(Value::as_bool) {
        f.strips = st;
    }
    let v = frame_json(&f);
    frame::set(prj, f);
    Ok(v)
}

fn delete_frame(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let name = a.req("name");
    let Some(i) = prj.forms.frames.iter().position(|f| f.name == name) else {
        return Err(tr_fmt!("No plot frame named {}", name));
    };
    if prj.pages.iter().any(|p| p.frame == name) || prj.forms.default_frame == name {
        return Err(tr_fmt!("The plot frame {} is in use", name));
    }
    prj.forms.frames.remove(i);
    Ok(json!({ "deleted": name }))
}

fn set_default_frame(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let name = a.req("name");
    if !frame::names(prj).iter().any(|n| n == name) {
        return Err(tr_fmt!("No plot frame named {}", name));
    }
    prj.forms.default_frame = name.into();
    Ok(frame_json(&frame::project_default(prj)))
}

fn list_page_templates(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let list = page_templates::all(prj).into_iter().map(|t| {
        json!({
            "name": t.name,
            "builtin": !prj.page_templates.iter().any(|x| x.name == t.name),
            "kind": kind_name(t.kind),
            "frame": t.frame,
            "title_block": t.title_block,
            "description": t.description,
            "structure": t.structure.structure(),
            "items": t.content.symbols.len() + t.content.wires.len() + t.content.texts.len() + t.content.shapes.len(),
        })
    });
    Ok(Value::Array(list.collect()))
}

fn save_page_template(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let i = page_index(prj, a.req("page"))?;
    let name = a.req("name").trim();
    if name.is_empty() {
        return Err(tr("The name must not be empty").into());
    }
    let t = page_templates::from_page(prj, i, name);
    let items = t.content.symbols.len() + t.content.wires.len() + t.content.texts.len() + t.content.shapes.len();
    page_templates::save(prj, t);
    Ok(json!({ "name": name, "items": items }))
}

fn delete_page_template(s: &mut Session, a: &Args) -> Result<Value, String> {
    let name = a.req("name");
    if !page_templates::remove(s.project_mut()?, name) {
        return Err(tr_fmt!("No page template named {}", name));
    }
    Ok(json!({ "deleted": name }))
}

fn add_document_page(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("path");
    let bytes = std::fs::read(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let prj = s.project_mut()?;
    let after = match a.str("after") {
        Some(n) => page_index(prj, n)?,
        None => prj.pages.len() - 1,
    };
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let i = document::add_page(prj, after, file, &bytes);
    if let Some(d) = a.str("description") {
        prj.pages[i].description = d.into();
    }
    let mut v = page_json(&prj.pages[i]);
    v["bytes"] = json!(bytes.len());
    Ok(v)
}

fn export_document(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let pg = &prj.pages[page_index(prj, a.req("page"))?];
    let doc = pg.document.as_ref().ok_or_else(|| tr_fmt!("Page {} has no embedded document", pg.name))?;
    let bytes = doc.bytes()?;
    let path = a.req("path");
    std::fs::write(path, &bytes).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
    Ok(json!({ "path": path, "name": doc.name, "bytes": bytes.len() }))
}

#[cfg(test)]
mod tests;
