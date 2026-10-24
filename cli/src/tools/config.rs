//! Configurator tools: options, rules and generating pages from a macro library file.

use super::{param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::config::{self};
use openecraft_core::macros::MacroLibrary;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "generate_pages",
        help: tr_const!("Replaces the pages generated before by the page macros of the rules whose condition holds"),
        params: &[param("macros", Kind::Str, tr_const!("Macro library file (JSON, exported by the editor)"))],
        needs_project: true,
        run: generate_pages,
    },
];

fn generate_pages(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("macros");
    let text = std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let lib = MacroLibrary::from_json(&text).map_err(|e| tr_fmt!("Not a macro file: {}", e))?;
    let g = config::generate(s.project_mut()?, &lib)?;
    Ok(json!({ "removed": g.removed, "pages": g.pages }))
}

#[cfg(test)]
mod tests;
