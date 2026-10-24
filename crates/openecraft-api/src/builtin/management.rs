//! Project management commands: project database, batch changes of project properties,
//! subprojects, model options, archives, version conversion and verification.

use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::issue::Severity;
use openecraft_core::model::Project;
use openecraft_core::{options, subproject, verify};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct Management;

impl Plugin for Management {
    fn info(&self) -> Info {
        info("management", tr_const!("Project management"), tr_const!("Subprojects, model options and the project data check"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "check_project_data",
        help: tr_const!("Verifies the project data (ids, references, options, subprojects) and runs the checks; optionally repairs what can be repaired"),
        params: &[optional("repair", Kind::Bool, tr_const!("Repair the repairable findings"))],
        run: check_project_data,
    },
    Spec {
        name: "list_subprojects",
        help: tr_const!("Lists the subprojects taken out of the project, or the project it was taken from"),
        params: &[],
        run: list_subprojects,
    },
    Spec {
        name: "discard_subproject",
        help: tr_const!("Gives up a subproject: its pages are unlocked again"),
        params: &[param("name", Kind::Str, tr_const!("Subproject name"))],
        run: discard_subproject,
    },
    Spec {
        name: "list_model_options",
        help: tr_const!("Lists the project options that show or hide model content"),
        params: &[],
        run: list_model_options,
    },
    Spec {
        name: "define_model_option",
        help: tr_const!("Adds a project option or changes its description and condition"),
        params: &[
            param("name", Kind::Str, tr_const!("Option name")),
            optional("description", Kind::Str, tr_const!("Description")),
            optional("condition", Kind::Str, tr_const!("Configurator condition, e.g. HEATER=yes (empty: switched by hand)")),
        ],
        run: define_model_option,
    },
    Spec {
        name: "remove_model_option",
        help: tr_const!("Removes a project option; its content stays in the project"),
        params: &[param("name", Kind::Str, tr_const!("Option name"))],
        run: remove_model_option,
    },
    Spec {
        name: "assign_model_option",
        help: tr_const!("Makes pages or page items content of a project option"),
        params: &[
            param("name", Kind::Str, tr_const!("Option name")),
            optional("ids", Kind::Ids, tr_const!("Ids of page items")),
            optional("pages", Kind::Str, tr_const!("Page names, separated by ','")),
        ],
        run: assign_model_option,
    },
    Spec {
        name: "unassign_model_option",
        help: tr_const!("Takes pages or page items out of their project option (hidden content is shown again)"),
        params: &[optional("ids", Kind::Ids, tr_const!("Ids of page items")), optional("pages", Kind::Str, tr_const!("Page names, separated by ','"))],
        run: unassign_model_option,
    },
    Spec {
        name: "set_model_option",
        help: tr_const!("Activates (shows) or deactivates (hides) the content of a project option"),
        params: &[param("name", Kind::Str, tr_const!("Option name")), param("active", Kind::Bool, tr_const!("Show the content"))],
        run: set_model_option,
    },
    Spec {
        name: "apply_model_options",
        help: tr_const!("Switches the project options with a condition by the configurator options"),
        params: &[],
        run: apply_model_options,
    },
];

fn severity(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn check_project_data(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let repaired = if a.flag("repair") { verify::repair(prj) } else { Vec::new() };
    let v = verify::verify(prj);
    let findings: Vec<Value> = v
        .findings
        .iter()
        .map(|f| json!({ "severity": severity(f.severity), "message": f.message, "page": f.target.map(|t| prj.pages[t.0].name.clone()), "id": f.target.map(|t| t.1).filter(|id| *id != 0), "repairable": f.repairable }))
        .collect();
    Ok(json!({ "passed": v.passed(), "findings": findings, "check_errors": v.check_errors, "check_warnings": v.check_warnings, "repaired": repaired }))
}

fn list_subprojects(prj: &mut Project, _: &Args) -> Result<Value, String> {
    let page_names = |ids: &[u64]| -> Vec<String> { ids.iter().filter_map(|id| prj.pages.iter().find(|p| p.id == *id)).map(|p| p.name.clone()).collect() };
    let children: Vec<Value> = prj
        .subprojects
        .children
        .iter()
        .map(|c| json!({ "name": c.name, "description": c.description, "pages": page_names(&c.pages), "out": c.out, "exported": c.exported, "inserted": c.inserted }))
        .collect();
    let parent = prj.subprojects.parent.as_ref().map(|p| json!({ "project": p.project, "name": p.name, "exported": p.exported }));
    Ok(json!({ "subprojects": children, "parent": parent }))
}

pub fn page_list(a: &Args) -> Vec<String> {
    a.str("pages").unwrap_or_default().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

fn discard_subproject(prj: &mut Project, a: &Args) -> Result<Value, String> {
    subproject::discard(prj, a.req("name"))?;
    list_subprojects(prj, a)
}

fn options_json(prj: &Project) -> Value {
    json!(prj
        .model_options
        .options
        .iter()
        .map(|o| json!({ "name": o.name, "description": o.description, "condition": o.condition, "active": o.active, "members": o.members.len(), "pages": options::page_count(prj, o) }))
        .collect::<Vec<_>>())
}

fn list_model_options(prj: &mut Project, _: &Args) -> Result<Value, String> {
    Ok(options_json(prj))
}

fn define_model_option(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let old = prj.model_options.get(a.req("name"));
    let description = a.str("description").map(String::from).or_else(|| old.map(|o| o.description.clone())).unwrap_or_default();
    let condition = a.str("condition").map(String::from).or_else(|| old.map(|o| o.condition.clone())).unwrap_or_default();
    options::define(prj, a.req("name"), &description, &condition)?;
    Ok(options_json(prj))
}

fn remove_model_option(prj: &mut Project, a: &Args) -> Result<Value, String> {
    options::remove(prj, a.req("name"))?;
    Ok(options_json(prj))
}

/// Item ids and the ids of the named pages.
fn targets(prj: &Project, a: &Args) -> Result<Vec<u64>, String> {
    let mut ids = a.ids("ids");
    for n in page_list(a) {
        ids.push(prj.pages.iter().find(|p| p.name == n).map(|p| p.id).ok_or_else(|| tr_fmt!("No page {}", n))?);
    }
    if ids.is_empty() {
        return Err(tr("Pass ids or pages").into());
    }
    Ok(ids)
}

fn assign_model_option(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let ids = targets(prj, a)?;
    let n = options::assign(prj, a.req("name"), &ids)?;
    Ok(json!({ "assigned": n, "options": options_json(prj) }))
}

fn unassign_model_option(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let mut ids = a.ids("ids");
    // Pages of inactive options are not in the page list: look them up in the parked content.
    for n in page_list(a) {
        let parked = prj.model_options.options.iter().flat_map(|o| &o.parked).find_map(|p| match &p.item {
            options::Item::Page(pg) if pg.name == n => Some(pg.id),
            _ => None,
        });
        ids.push(prj.pages.iter().find(|p| p.name == n).map(|p| p.id).or(parked).ok_or_else(|| tr_fmt!("No page {}", n))?);
    }
    if ids.is_empty() {
        return Err(tr("Pass ids or pages").into());
    }
    let n = options::unassign(prj, &ids);
    Ok(json!({ "unassigned": n, "options": options_json(prj) }))
}

fn set_model_option(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let moved = options::set_active(prj, a.req("name"), a.flag("active"))?;
    Ok(json!({ "moved": moved, "options": options_json(prj) }))
}

fn apply_model_options(prj: &mut Project, _: &Args) -> Result<Value, String> {
    let switched = options::apply_conditions(prj)?;
    let switched: Vec<Value> = switched.into_iter().map(|(name, active)| json!({ "name": name, "active": active })).collect();
    Ok(json!({ "switched": switched, "options": options_json(prj) }))
}
