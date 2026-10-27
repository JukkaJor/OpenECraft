//! Structure tools (IEC 81346): structure management with the defined identifiers,
//! project-wide renaming, templates and work areas.

use super::{optional, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::structure::{self, Aspect, Structures};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "list_structure_templates",
        help: tr_const!("Lists the built-in structure templates"),
        params: &[],
        needs_project: false,
        run: list_templates,
    },
    Tool {
        name: "apply_structure_template",
        help: tr_const!("Applies a structure template: the aspects of the pages and its identifiers"),
        params: &[
            optional("template", Kind::Str, tr_const!("Name of a built-in template (list_structure_templates)")),
            optional("path", Kind::Str, tr_const!("Template file written by export_structure_template")),
        ],
        needs_project: true,
        run: apply_template,
    },
    Tool {
        name: "export_structure_template",
        help: tr_const!("Writes the structure management (aspects, identifiers, work areas) as a template file"),
        params: &[optional("path", Kind::Str, tr_const!("Output file; without it the content is returned"))],
        needs_project: true,
        run: export_template,
    },
];

fn list_templates(_: &mut Session, _: &Args) -> Result<Value, String> {
    let list = structure::TEMPLATES.iter().map(|t| {
        json!({
            "name": t.name,
            "aspects": t.aspects.iter().map(|a| a.key()).collect::<Vec<_>>(),
            "identifiers": t.entries.iter().map(|(a, id, _)| a.format(id)).collect::<Vec<_>>(),
        })
    });
    Ok(Value::Array(list.collect()))
}

fn apply_template(s: &mut Session, a: &Args) -> Result<Value, String> {
    let added = match (a.str("template"), a.str("path")) {
        (Some(name), None) => {
            let t = structure::template(name).ok_or_else(|| tr_fmt!("Unknown structure template: {}", name))?;
            structure::apply_template(&mut s.project_mut()?.structures, t)
        }
        (None, Some(path)) => {
            let text = std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, openecraft_i18n::io_error(&e)))?;
            let file = Structures::from_json(&text)?;
            s.project_mut()?.structures.merge(&file)
        }
        _ => return Err(tr("Give either template or path").into()),
    };
    let prj = s.project()?;
    let aspects: Vec<&str> = prj.structures.page_aspects().into_iter().map(Aspect::key).collect();
    Ok(json!({ "added": added, "aspects": aspects }))
}

fn export_template(s: &mut Session, a: &Args) -> Result<Value, String> {
    let content = s.project()?.structures.to_json();
    match a.str("path") {
        Some(path) => {
            std::fs::write(path, &content).map_err(|e| tr_fmt!("Cannot write {}: {}", path, openecraft_i18n::io_error(&e)))?;
            Ok(json!({ "path": path, "bytes": content.len() }))
        }
        None => serde_json::from_str(&content).map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
mod tests;
