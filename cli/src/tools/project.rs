//! Project tools: files, project properties, undo / redo.

use super::{optional, param, Args, Kind, Param, Tool};
use crate::session::Session;
use openecraft_core::index::Index;
use openecraft_core::model::*;
use openecraft_core::reports::{Analysis, Severity};
use openecraft_core::sample;
use openecraft_i18n::{tr, tr_const};
use serde_json::{json, Value};

const DISCARD: Param = optional("discard", Kind::Bool, tr_const!("Discard unsaved changes of the open project"));

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "new_project",
        help: tr_const!("Creates a new project (optionally the demo project) and saves it if a path is given"),
        params: &[
            optional("path", Kind::Str, tr_const!("File to save the project to (.oec)")),
            optional("name", Kind::Str, tr_const!("Project name")),
            optional("demo", Kind::Bool, tr_const!("Start from the demo project")),
            DISCARD,
        ],
        needs_project: false,
        run: new_project,
    },
    Tool {
        name: "open_project",
        help: tr_const!("Opens a project file"),
        params: &[param("path", Kind::Str, tr_const!("Project file (.oec)")), DISCARD],
        needs_project: false,
        run: open_project,
    },
    Tool {
        name: "save_project",
        help: tr_const!("Saves the project to its file or to a new path"),
        params: &[optional("path", Kind::Str, tr_const!("New file name (save as)"))],
        needs_project: true,
        run: save_project,
    },
    Tool {
        name: "project_info",
        help: tr_const!("Shows the project properties, file, number of pages and devices and the check result"),
        params: &[],
        needs_project: true,
        run: project_info,
    },
    Tool {
        name: "set_project_info",
        help: tr_const!("Changes project properties"),
        params: &[
            optional("name", Kind::Str, tr_const!("Project name")),
            optional("description", Kind::Str, tr_const!("Description")),
            optional("company", Kind::Str, tr_const!("Company")),
            optional("creator", Kind::Str, tr_const!("Creator")),
            optional("date", Kind::Str, tr_const!("Date")),
            optional("revision", Kind::Str, tr_const!("Revision")),
            optional("properties", Kind::Object, tr_const!("User defined properties (name: value; an empty value removes it)")),
        ],
        needs_project: true,
        run: set_project_info,
    },
    Tool { name: "undo", help: tr_const!("Undoes the last change"), params: &[], needs_project: true, run: undo },
    Tool { name: "redo", help: tr_const!("Redoes the last undone change"), params: &[], needs_project: true, run: redo },
];

fn ensure_unsaved_ok(s: &Session, a: &Args) -> Result<(), String> {
    if s.unsaved() && !a.flag("discard") {
        return Err(tr("The open project has unsaved changes: save it first or pass discard").to_string());
    }
    Ok(())
}

fn summary(s: &Session) -> Result<Value, String> {
    let prj = s.project()?;
    let an = Analysis::build(prj);
    let count = |sev| an.issues.iter().filter(|i| i.severity == sev).count();
    let i = &prj.info;
    Ok(json!({
        "name": i.name,
        "description": i.description,
        "company": i.company,
        "creator": i.creator,
        "date": i.date,
        "revision": i.revision,
        "properties": i.properties,
        "file": s.path.as_ref().map(|p| p.display().to_string()),
        "unsaved": s.unsaved(),
        "pages": prj.pages.len(),
        "devices": Index::build(prj).devices.len(),
        "errors": count(Severity::Error),
        "warnings": count(Severity::Warning),
        "can_undo": s.can_undo(),
        "can_redo": s.can_redo(),
    }))
}

fn new_project(s: &mut Session, a: &Args) -> Result<Value, String> {
    ensure_unsaved_ok(s, a)?;
    let mut prj = if a.flag("demo") { sample::demo_project() } else { Project::default() };
    if let Some(name) = a.str("name") {
        prj.info.name = name.into();
    }
    s.replace(prj, None);
    if let Some(path) = a.str("path") {
        s.save(Some(path))?;
    }
    summary(s)
}

fn open_project(s: &mut Session, a: &Args) -> Result<Value, String> {
    ensure_unsaved_ok(s, a)?;
    s.open(a.req("path"))?;
    summary(s)
}

fn save_project(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = s.save(a.str("path"))?;
    Ok(json!({ "path": path.display().to_string() }))
}

fn project_info(s: &mut Session, _: &Args) -> Result<Value, String> {
    summary(s)
}

fn set_project_info(s: &mut Session, a: &Args) -> Result<Value, String> {
    let info = &mut s.project_mut()?.info;
    for (name, field) in [
        ("name", &mut info.name),
        ("description", &mut info.description),
        ("company", &mut info.company),
        ("creator", &mut info.creator),
        ("date", &mut info.date),
        ("revision", &mut info.revision),
    ] {
        if let Some(v) = a.str(name) {
            *field = v.into();
        }
    }
    if let Some(Value::Object(props)) = a.value("properties") {
        for (k, v) in props {
            match v.as_str().unwrap_or_default() {
                "" => info.properties.remove(k),
                v => info.properties.insert(k.clone(), v.into()),
            };
        }
    }
    summary(s)
}

fn undo(s: &mut Session, _: &Args) -> Result<Value, String> {
    let done = s.undo()?;
    Ok(json!({ "undone": done, "can_undo": s.can_undo(), "can_redo": s.can_redo() }))
}

fn redo(s: &mut Session, _: &Args) -> Result<Value, String> {
    let done = s.redo()?;
    Ok(json!({ "redone": done, "can_undo": s.can_undo(), "can_redo": s.can_redo() }))
}

#[cfg(test)]
mod tests;
