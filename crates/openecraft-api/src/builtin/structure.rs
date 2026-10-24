//! Structure commands (IEC 81346): structure management with the defined identifiers,
//! project-wide renaming, templates and work areas.

use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::model::Project;
use openecraft_core::structure::{self, Aspect, Dt, WorkArea};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct Structure;

impl Plugin for Structure {
    fn info(&self) -> Info {
        info("structure", tr_const!("Structure"), tr_const!("Structure identifiers, aspects and work areas"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "list_structure",
        help: tr_const!("Lists the structure management: aspects of the pages, defined and used identifiers, work areas"),
        params: &[],
        run: list_structure,
    },
    Spec {
        name: "define_structure",
        help: tr_const!("Defines a structure identifier or changes its description"),
        params: &[ASPECT, param("id", Kind::Str, tr_const!("Identifier, e.g. A1 or CAB1")), optional("description", Kind::Str, tr_const!("Description"))],
        run: define_structure,
    },
    Spec {
        name: "remove_structure",
        help: tr_const!("Removes the definition of a structure identifier (pages and devices keep it)"),
        params: &[ASPECT, param("id", Kind::Str, tr_const!("Identifier"))],
        run: remove_structure,
    },
    Spec {
        name: "rename_structure",
        help: tr_const!("Renames a structure identifier in the whole project: pages, devices, cables, definition and work areas"),
        params: &[ASPECT, param("id", Kind::Str, tr_const!("Identifier")), param("new_id", Kind::Str, tr_const!("New identifier"))],
        run: rename_structure,
    },
    Spec {
        name: "tidy_structure",
        help: tr_const!("Defines all identifiers in use and / or removes the definitions nobody uses"),
        params: &[
            optional("define_used", Kind::Bool, tr_const!("Define the identifiers in use (default: true)")),
            optional("remove_unused", Kind::Bool, tr_const!("Remove unused definitions (default: false)")),
        ],
        run: tidy_structure,
    },
    Spec {
        name: "set_structure_aspects",
        help: tr_const!("Sets the aspects the pages use"),
        params: &[param("aspects", Kind::Str, tr_const!("Aspects separated by commas, e.g. function,location"))],
        run: set_aspects,
    },
    Spec {
        name: "set_work_area",
        help: tr_const!("Creates or changes a work area: only the pages and devices of its structure are shown"),
        params: &[
            param("name", Kind::Str, tr_const!("Name of the work area")),
            optional("function", Kind::Str, tr_const!("Function filter (= A1, A*; empty = any)")),
            optional("installation", Kind::Str, tr_const!("Installation site filter (++)")),
            optional("location", Kind::Str, tr_const!("Location filter (+)")),
            optional("product", Kind::Str, tr_const!("Product filter (-)")),
        ],
        run: set_work_area,
    },
    Spec {
        name: "delete_work_area",
        help: tr_const!("Deletes a work area"),
        params: &[param("name", Kind::Str, tr_const!("Name of the work area"))],
        run: delete_work_area,
    },
];

const ASPECT: super::Param = param("aspect", Kind::Str, tr_const!("function (=), installation (++), location (+) or product (-)"));

fn aspect(a: &Args) -> Result<Aspect, String> {
    let s = a.req("aspect");
    Aspect::from_key(s).ok_or_else(|| tr_fmt!("Unknown aspect: {}", s))
}

fn work_area_json(w: &WorkArea) -> Value {
    json!({ "name": w.name, "filter": w.filter.structure() })
}

fn list_structure(prj: &mut Project, _: &Args) -> Result<Value, String> {
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

fn define_structure(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let aspect = aspect(a)?;
    let id = structure::clean_aspect(a.req("id"));
    if id.is_empty() {
        return Err(tr("The identifier must not be empty").into());
    }
    let description = a.str("description").map(String::from).or_else(|| prj.structures.entry(aspect, &id).map(|e| e.description.clone())).unwrap_or_default();
    prj.structures.define(aspect, &id, &description);
    Ok(json!({ "identifier": aspect.format(&id), "description": description }))
}

fn remove_structure(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let aspect = aspect(a)?;
    let id = structure::clean_aspect(a.req("id"));
    if !prj.structures.remove(aspect, &id) {
        return Err(tr_fmt!("{} is not defined", aspect.format(&id)));
    }
    Ok(json!({ "removed": aspect.format(&id) }))
}

fn rename_structure(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let aspect = aspect(a)?;
    let n = structure::rename(prj, aspect, a.req("id"), a.req("new_id"))?;
    Ok(json!({ "renamed": n, "identifier": aspect.format(&structure::clean_aspect(a.req("new_id"))) }))
}

fn tidy_structure(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let defined = if a.value("define_used").and_then(Value::as_bool).unwrap_or(true) { structure::define_used(prj) } else { 0 };
    let removed = if a.flag("remove_unused") { structure::remove_unused(prj) } else { 0 };
    Ok(json!({ "defined": defined, "removed": removed }))
}

fn set_aspects(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let mut aspects = Vec::new();
    for key in a.req("aspects").split(',').map(str::trim).filter(|k| !k.is_empty()) {
        let aspect = Aspect::from_key(key).ok_or_else(|| tr_fmt!("Unknown aspect: {}", key))?;
        if !aspects.contains(&aspect) {
            aspects.push(aspect);
        }
    }
    prj.structures.aspects = aspects;
    let aspects: Vec<&str> = prj.structures.page_aspects().into_iter().map(Aspect::key).collect();
    Ok(json!({ "aspects": aspects }))
}

fn set_work_area(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let name = a.req("name").trim();
    if name.is_empty() {
        return Err(tr("The name must not be empty").into());
    }
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

fn delete_work_area(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let name = a.req("name");
    let areas = &mut prj.structures.work_areas;
    let n = areas.len();
    areas.retain(|w| w.name != name);
    if areas.len() == n {
        return Err(tr_fmt!("No work area named {}", name));
    }
    Ok(json!({ "deleted": name }))
}
