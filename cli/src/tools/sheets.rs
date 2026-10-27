//! Sheet tools: plot frames (paper format, orientation, columns / rows), page
//! templates and document pages with embedded files.

use super::common::*;
use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::document;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
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

fn add_document_page(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("path");
    let bytes = std::fs::read(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, openecraft_i18n::io_error(&e)))?;
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
    std::fs::write(path, &bytes).map_err(|e| tr_fmt!("Cannot write {}: {}", path, openecraft_i18n::io_error(&e)))?;
    Ok(json!({ "path": path, "name": doc.name, "bytes": bytes.len() }))
}

#[cfg(test)]
mod tests;
