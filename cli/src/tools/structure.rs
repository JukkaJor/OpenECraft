//! Structure tools (IEC 81346): structure management with the defined identifiers,
//! project-wide renaming, templates and work areas.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::structure::{self, Aspect, Dt, Structures, WorkArea};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};

const ASPECT: super::Param = param("aspect", Kind::Str, tr_const!("function (=), installation (++), location (+) or product (-)"));

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "list_structure",
        help: tr_const!("Lists the structure management: aspects of the pages, defined and used identifiers, work areas"),
        params: &[],
        needs_project: true,
        run: list_structure,
    },
    Tool {
        name: "define_structure",
        help: tr_const!("Defines a structure identifier or changes its description"),
        params: &[ASPECT, param("id", Kind::Str, tr_const!("Identifier, e.g. A1 or CAB1")), optional("description", Kind::Str, tr_const!("Description"))],
        needs_project: true,
        run: define_structure,
    },
    Tool {
        name: "remove_structure",
        help: tr_const!("Removes the definition of a structure identifier (pages and devices keep it)"),
        params: &[ASPECT, param("id", Kind::Str, tr_const!("Identifier"))],
        needs_project: true,
        run: remove_structure,
    },
    Tool {
        name: "rename_structure",
        help: tr_const!("Renames a structure identifier in the whole project: pages, devices, cables, definition and work areas"),
        params: &[ASPECT, param("id", Kind::Str, tr_const!("Identifier")), param("new_id", Kind::Str, tr_const!("New identifier"))],
        needs_project: true,
        run: rename_structure,
    },
    Tool {
        name: "tidy_structure",
        help: tr_const!("Defines all identifiers in use and / or removes the definitions nobody uses"),
        params: &[
            optional("define_used", Kind::Bool, tr_const!("Define the identifiers in use (default: true)")),
            optional("remove_unused", Kind::Bool, tr_const!("Remove unused definitions (default: false)")),
        ],
        needs_project: true,
        run: tidy_structure,
    },
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
    Tool {
        name: "set_structure_aspects",
        help: tr_const!("Sets the aspects the pages use"),
        params: &[param("aspects", Kind::Str, tr_const!("Aspects separated by commas, e.g. function,location"))],
        needs_project: true,
        run: set_aspects,
    },
    Tool {
        name: "set_work_area",
        help: tr_const!("Creates or changes a work area: only the pages and devices of its structure are shown"),
        params: &[
            param("name", Kind::Str, tr_const!("Name of the work area")),
            optional("function", Kind::Str, tr_const!("Function filter (= A1, A*; empty = any)")),
            optional("installation", Kind::Str, tr_const!("Installation site filter (++)")),
            optional("location", Kind::Str, tr_const!("Location filter (+)")),
            optional("product", Kind::Str, tr_const!("Product filter (-)")),
        ],
        needs_project: true,
        run: set_work_area,
    },
    Tool {
        name: "delete_work_area",
        help: tr_const!("Deletes a work area"),
        params: &[param("name", Kind::Str, tr_const!("Name of the work area"))],
        needs_project: true,
        run: delete_work_area,
    },
];

fn aspect(a: &Args) -> Result<Aspect, String> {
    let s = a.req("aspect");
    Aspect::from_key(s).ok_or_else(|| tr_fmt!("Unknown aspect: {}", s))
}

fn work_area_json(w: &WorkArea) -> Value {
    json!({ "name": w.name, "filter": w.filter.structure() })
}

fn list_structure(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let rows: Vec<Value> = structure::overview(prj)
        .into_iter()
        .map(|u| {
            json!({
                "aspect": u.aspect.key(),
                "id": u.id,
                "identifier": u.aspect.format(&u.id),
                "description": u.description,
                "defined": u.defined,
                "pages": u.pages,
                "devices": u.devices,
            })
        })
        .collect();
    let aspects: Vec<&str> = prj.structures.page_aspects().into_iter().map(Aspect::key).collect();
    Ok(json!({
        "aspects": aspects,
        "identifiers": rows,
        "work_areas": prj.structures.work_areas.iter().map(work_area_json).collect::<Vec<_>>(),
    }))
}

fn define_structure(s: &mut Session, a: &Args) -> Result<Value, String> {
    let aspect = aspect(a)?;
    let id = structure::clean_aspect(a.req("id"));
    if id.is_empty() {
        return Err(tr("The identifier must not be empty").into());
    }
    let prj = s.project_mut()?;
    let description = a.str("description").map(String::from).or_else(|| prj.structures.entry(aspect, &id).map(|e| e.description.clone())).unwrap_or_default();
    prj.structures.define(aspect, &id, &description);
    Ok(json!({ "identifier": aspect.format(&id), "description": description }))
}

fn remove_structure(s: &mut Session, a: &Args) -> Result<Value, String> {
    let aspect = aspect(a)?;
    let id = structure::clean_aspect(a.req("id"));
    if !s.project_mut()?.structures.remove(aspect, &id) {
        return Err(tr_fmt!("{} is not defined", aspect.format(&id)));
    }
    Ok(json!({ "removed": aspect.format(&id) }))
}

fn rename_structure(s: &mut Session, a: &Args) -> Result<Value, String> {
    let aspect = aspect(a)?;
    let n = structure::rename(s.project_mut()?, aspect, a.req("id"), a.req("new_id"))?;
    Ok(json!({ "renamed": n, "identifier": aspect.format(&structure::clean_aspect(a.req("new_id"))) }))
}

fn tidy_structure(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let defined = if a.value("define_used").and_then(Value::as_bool).unwrap_or(true) { structure::define_used(prj) } else { 0 };
    let removed = if a.flag("remove_unused") { structure::remove_unused(prj) } else { 0 };
    Ok(json!({ "defined": defined, "removed": removed }))
}

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
            let text = std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
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
            std::fs::write(path, &content).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
            Ok(json!({ "path": path, "bytes": content.len() }))
        }
        None => serde_json::from_str(&content).map_err(|e| e.to_string()),
    }
}

fn set_aspects(s: &mut Session, a: &Args) -> Result<Value, String> {
    let mut aspects = Vec::new();
    for key in a.req("aspects").split(',').map(str::trim).filter(|k| !k.is_empty()) {
        let aspect = Aspect::from_key(key).ok_or_else(|| tr_fmt!("Unknown aspect: {}", key))?;
        if !aspects.contains(&aspect) {
            aspects.push(aspect);
        }
    }
    let prj = s.project_mut()?;
    prj.structures.aspects = aspects;
    let aspects: Vec<&str> = prj.structures.page_aspects().into_iter().map(Aspect::key).collect();
    Ok(json!({ "aspects": aspects }))
}

fn set_work_area(s: &mut Session, a: &Args) -> Result<Value, String> {
    let name = a.req("name").trim();
    if name.is_empty() {
        return Err(tr("The name must not be empty").into());
    }
    let prj = s.project_mut()?;
    let mut area = prj.structures.work_area(name).cloned().unwrap_or(WorkArea { name: name.into(), filter: Dt::default() });
    for aspect in Aspect::ALL {
        if let Some(v) = a.str(aspect.key()) {
            *area.filter.get_mut(aspect) = structure::clean_aspect(v);
        }
    }
    let v = work_area_json(&area);
    prj.structures.set_work_area(area);
    Ok(v)
}

fn delete_work_area(s: &mut Session, a: &Args) -> Result<Value, String> {
    let name = a.req("name");
    let areas = &mut s.project_mut()?.structures.work_areas;
    let n = areas.len();
    areas.retain(|w| w.name != name);
    if areas.len() == n {
        return Err(tr_fmt!("No work area named {}", name));
    }
    Ok(json!({ "deleted": name }))
}

#[cfg(test)]
mod tests;
