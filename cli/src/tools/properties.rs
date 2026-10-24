//! Property database tools: property definitions, values of all objects, function
//! definitions, placement schemes and property library files.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::properties::Library;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "export_property_library",
        help: tr_const!("Writes the project's property definitions, function definitions and schemes as a library file"),
        params: &[optional("path", Kind::Str, tr_const!("Output file; without it the content is returned"))],
        needs_project: true,
        run: export_library,
    },
    Tool {
        name: "import_property_library",
        help: tr_const!("Adds the definitions and schemes of a property library file to the project"),
        params: &[param("path", Kind::Str, tr_const!("Library file"))],
        needs_project: true,
        run: import_library,
    },
];

fn export_library(s: &mut Session, a: &Args) -> Result<Value, String> {
    let content = s.project()?.property_db.to_json();
    match a.str("path") {
        Some(path) => {
            std::fs::write(path, &content).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
            Ok(json!({ "path": path, "bytes": content.len() }))
        }
        None => serde_json::from_str(&content).map_err(|e| e.to_string()),
    }
}

fn import_library(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("path");
    let text = std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let lib = Library::from_json(&text)?;
    let n = s.project_mut()?.property_db.merge(lib);
    Ok(json!({ "path": path, "imported": n }))
}

#[cfg(test)]
mod tests;
