//! Project management tools: project database, batch changes of project properties,
//! subprojects, model options, archives, version conversion and verification.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::archive::{self, ArchiveOptions};
use openecraft_core::model::Project;
use openecraft_core::projects::{self, Batch, Database, Entry};
use openecraft_core::issue::Severity;
use openecraft_core::{options, subproject, verify};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "index_projects",
        help: tr_const!("Reads the project files and archives of a folder (and its subfolders) into the project database"),
        params: &[
            param("folder", Kind::Str, tr_const!("Folder with project files")),
            optional("database", Kind::Str, tr_const!("Project database file (JSON) to update")),
            optional("verify", Kind::Bool, tr_const!("Also verify each project (slower)")),
        ],
        needs_project: false,
        run: index_projects,
    },
    Tool {
        name: "search_projects",
        help: tr_const!("Finds projects in the project database or a folder: terms separated by ';', each key=value, key~text or plain text"),
        params: &[
            optional("query", Kind::Str, tr_const!("Search terms, e.g. company=ACME; customer~plant; conveyor")),
            optional("database", Kind::Str, tr_const!("Project database file (JSON)")),
            optional("folder", Kind::Str, tr_const!("Folder with project files (instead of a database)")),
        ],
        needs_project: false,
        run: search_projects,
    },
    Tool {
        name: "batch_project_properties",
        help: tr_const!("Changes project properties in many project files at once (fields name, description, company, creator, date, revision or project properties)"),
        params: &[
            optional("files", Kind::Str, tr_const!("Project files, separated by ';'")),
            optional("folder", Kind::Str, tr_const!("All project files of a folder")),
            optional("query", Kind::Str, tr_const!("Only the projects that match these search terms")),
            optional("set", Kind::Object, tr_const!("Values to set (name: value; an empty value removes a property)")),
            optional("remove", Kind::Str, tr_const!("Properties to remove, separated by ','")),
            optional("replace", Kind::Str, tr_const!("Field or property whose value is searched ('*' for all)")),
            optional("find", Kind::Str, tr_const!("Text to replace")),
            optional("with", Kind::Str, tr_const!("Replacement text")),
            optional("rename", Kind::Object, tr_const!("Properties to rename (old name: new name)")),
            optional("dry_run", Kind::Bool, tr_const!("Only show the changes, write nothing")),
        ],
        needs_project: false,
        run: batch_project_properties,
    },
    Tool {
        name: "check_project_data",
        help: tr_const!("Verifies the project data (ids, references, options, subprojects) and runs the checks; optionally repairs what can be repaired"),
        params: &[optional("repair", Kind::Bool, tr_const!("Repair the repairable findings"))],
        needs_project: true,
        run: check_project_data,
    },
    Tool {
        name: "archive_project",
        help: tr_const!("Writes the project into an archive (.oecz: project, manifest, embedded files, optionally a PDF)"),
        params: &[
            param("path", Kind::Str, tr_const!("Archive file (.oecz)")),
            optional("comment", Kind::Str, tr_const!("Comment stored in the archive")),
            optional("pdf", Kind::Bool, tr_const!("Add a PDF of all pages")),
            optional("compact", Kind::Bool, tr_const!("Leave out stored baselines and wiring marks")),
        ],
        needs_project: true,
        run: archive_project,
    },
    Tool {
        name: "archive_info",
        help: tr_const!("Shows the manifest of a project archive"),
        params: &[param("path", Kind::Str, tr_const!("Archive file (.oecz)"))],
        needs_project: false,
        run: archive_info,
    },
    Tool {
        name: "convert_project",
        help: tr_const!("Converts a project file or archive of an older version to the current file format"),
        params: &[
            param("path", Kind::Str, tr_const!("Project file (.oec) or archive (.oecz)")),
            optional("output", Kind::Str, tr_const!("Project file to write (default: the file itself; archives: next to it as .oec)")),
        ],
        needs_project: false,
        run: convert_project,
    },
    Tool {
        name: "list_subprojects",
        help: tr_const!("Lists the subprojects taken out of the project, or the project it was taken from"),
        params: &[],
        needs_project: true,
        run: list_subprojects,
    },
    Tool {
        name: "extract_subproject",
        help: tr_const!("Takes pages out as a subproject file; they stay locked here until the subproject is inserted again"),
        params: &[
            param("name", Kind::Str, tr_const!("Subproject name")),
            param("path", Kind::Str, tr_const!("Subproject file to write (.oec)")),
            optional("pages", Kind::Str, tr_const!("Page names, separated by ','")),
            optional("structure", Kind::Str, tr_const!("Pages whose structure starts with this, e.g. =A1+CAB1")),
            optional("description", Kind::Str, tr_const!("Description")),
        ],
        needs_project: true,
        run: extract_subproject,
    },
    Tool {
        name: "insert_subproject",
        help: tr_const!("Inserts an edited subproject: its pages replace the pages taken out"),
        params: &[param("path", Kind::Str, tr_const!("Subproject file (.oec)"))],
        needs_project: true,
        run: insert_subproject,
    },
    Tool {
        name: "discard_subproject",
        help: tr_const!("Gives up a subproject: its pages are unlocked again"),
        params: &[param("name", Kind::Str, tr_const!("Subproject name"))],
        needs_project: true,
        run: discard_subproject,
    },
    Tool {
        name: "list_model_options",
        help: tr_const!("Lists the project options that show or hide model content"),
        params: &[],
        needs_project: true,
        run: list_model_options,
    },
    Tool {
        name: "define_model_option",
        help: tr_const!("Adds a project option or changes its description and condition"),
        params: &[
            param("name", Kind::Str, tr_const!("Option name")),
            optional("description", Kind::Str, tr_const!("Description")),
            optional("condition", Kind::Str, tr_const!("Configurator condition, e.g. HEATER=yes (empty: switched by hand)")),
        ],
        needs_project: true,
        run: define_model_option,
    },
    Tool {
        name: "remove_model_option",
        help: tr_const!("Removes a project option; its content stays in the project"),
        params: &[param("name", Kind::Str, tr_const!("Option name"))],
        needs_project: true,
        run: remove_model_option,
    },
    Tool {
        name: "assign_model_option",
        help: tr_const!("Makes pages or page items content of a project option"),
        params: &[
            param("name", Kind::Str, tr_const!("Option name")),
            optional("ids", Kind::Ids, tr_const!("Ids of page items")),
            optional("pages", Kind::Str, tr_const!("Page names, separated by ','")),
        ],
        needs_project: true,
        run: assign_model_option,
    },
    Tool {
        name: "unassign_model_option",
        help: tr_const!("Takes pages or page items out of their project option (hidden content is shown again)"),
        params: &[optional("ids", Kind::Ids, tr_const!("Ids of page items")), optional("pages", Kind::Str, tr_const!("Page names, separated by ','"))],
        needs_project: true,
        run: unassign_model_option,
    },
    Tool {
        name: "set_model_option",
        help: tr_const!("Activates (shows) or deactivates (hides) the content of a project option"),
        params: &[param("name", Kind::Str, tr_const!("Option name")), param("active", Kind::Bool, tr_const!("Show the content"))],
        needs_project: true,
        run: set_model_option,
    },
    Tool {
        name: "apply_model_options",
        help: tr_const!("Switches the project options with a condition by the configurator options"),
        params: &[],
        needs_project: true,
        run: apply_model_options,
    },
];

fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    openecraft_parts::date_from_unix(secs)
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path.display(), e))
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| tr_fmt!("Cannot write {}: {}", path.display(), e))
}

/// Project files of a folder and its subfolders.
fn project_files(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut dirs = vec![folder.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| tr_fmt!("Cannot read {}: {}", dir.display(), e))?;
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                dirs.push(path);
            } else if projects::is_project_file(&path.to_string_lossy()) {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn scan(folder: &str, verify: bool) -> Result<Vec<Entry>, String> {
    project_files(Path::new(folder))?.iter().map(|p| Ok(Entry::read(&p.display().to_string(), &read(p)?, verify))).collect()
}

fn index_projects(_: &mut Session, a: &Args) -> Result<Value, String> {
    let folder = a.req("folder");
    let found = scan(folder, a.flag("verify"))?;
    let mut db = match a.str("database") {
        Some(path) if Path::new(path).exists() => Database::from_json(&String::from_utf8_lossy(&read(Path::new(path))?))?,
        _ => Database::default(),
    };
    db.update_folder(&Path::new(folder).display().to_string(), found.clone());
    if let Some(path) = a.str("database") {
        write(Path::new(path), db.to_json().as_bytes())?;
    }
    Ok(json!({ "projects": found, "total": db.entries.len() }))
}

fn search_projects(_: &mut Session, a: &Args) -> Result<Value, String> {
    let db = match (a.str("database"), a.str("folder")) {
        (Some(path), _) => Database::from_json(&String::from_utf8_lossy(&read(Path::new(path))?))?,
        (None, Some(folder)) => Database { folders: vec![folder.into()], entries: scan(folder, false)? },
        (None, None) => return Err(tr("Pass a database or a folder").into()),
    };
    Ok(json!(db.search(a.str("query").unwrap_or_default())))
}

fn batch_project_properties(_: &mut Session, a: &Args) -> Result<Value, String> {
    let mut b = Batch::default();
    if let Some(Value::Object(set)) = a.value("set") {
        for (k, v) in set {
            let v = match v {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => other.to_string(),
            };
            b.set.insert(k.clone(), v);
        }
    }
    b.remove = a.str("remove").unwrap_or_default().split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect();
    if let Some(key) = a.str("replace") {
        b.replace.push((key.into(), a.str("find").unwrap_or_default().into(), a.str("with").unwrap_or_default().into()));
    }
    if let Some(Value::Object(rename)) = a.value("rename") {
        b.rename = rename.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string())).collect();
    }
    if b.is_empty() {
        return Err(tr("Nothing to change: pass set, remove, replace or rename").into());
    }
    let mut files: Vec<PathBuf> = a.str("files").unwrap_or_default().split(';').map(str::trim).filter(|s| !s.is_empty()).map(PathBuf::from).collect();
    if let Some(folder) = a.str("folder") {
        files.extend(project_files(Path::new(folder))?);
    }
    if files.is_empty() {
        return Err(tr("Pass files or a folder").into());
    }
    let (query, dry, date) = (a.str("query").unwrap_or_default(), a.flag("dry_run"), today());
    let mut out = Vec::new();
    for path in files {
        let bytes = read(&path)?;
        let name = path.display().to_string();
        if !query.is_empty() && !Entry::read(&name, &bytes, false).matches(query) {
            continue;
        }
        match b.apply_to_file(&bytes, &date) {
            Ok((changes, new)) => {
                let written = match new {
                    Some(new) if !dry => write(&path, &new).map(|_| true)?,
                    _ => false,
                };
                out.push(json!({ "file": name, "changes": changes, "written": written }));
            }
            Err(e) => out.push(json!({ "file": name, "error": e })),
        }
    }
    Ok(json!(out))
}

fn severity(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn check_project_data(s: &mut Session, a: &Args) -> Result<Value, String> {
    let repaired = if a.flag("repair") {
        let done = verify::repair(s.project_mut()?);
        s.commit();
        done
    } else {
        Vec::new()
    };
    let prj = s.project()?;
    let v = verify::verify(prj);
    let findings: Vec<Value> = v
        .findings
        .iter()
        .map(|f| json!({ "severity": severity(f.severity), "message": f.message, "page": f.target.map(|t| prj.pages[t.0].name.clone()), "id": f.target.map(|t| t.1).filter(|id| *id != 0), "repairable": f.repairable }))
        .collect();
    Ok(json!({ "passed": v.passed(), "findings": findings, "check_errors": v.check_errors, "check_warnings": v.check_warnings, "repaired": repaired }))
}

fn archive_project(s: &mut Session, a: &Args) -> Result<Value, String> {
    let opts = ArchiveOptions { comment: a.str("comment").unwrap_or_default().into(), date: today(), pdf: a.flag("pdf"), compact: a.flag("compact") };
    let bytes = archive::archive(s.project()?, &opts);
    let path = Path::new(a.req("path"));
    write(path, &bytes)?;
    let m = archive::manifest(&bytes)?;
    Ok(json!({ "path": path.display().to_string(), "bytes": bytes.len(), "files": m.files }))
}

fn archive_info(_: &mut Session, a: &Args) -> Result<Value, String> {
    let m = archive::manifest(&read(Path::new(a.req("path")))?)?;
    Ok(serde_json::to_value(m).unwrap_or_default())
}

fn convert_project(_: &mut Session, a: &Args) -> Result<Value, String> {
    let path = Path::new(a.req("path"));
    let bytes = read(path)?;
    let (prj, conv) = archive::read_project(&bytes)?;
    let output = match a.str("output") {
        Some(o) => PathBuf::from(o),
        None if archive::is_archive(&bytes) => path.with_extension("oec"),
        None => path.to_path_buf(),
    };
    write(&output, prj.to_json().as_bytes())?;
    Ok(json!({ "from": conv.from, "to": prj.format, "steps": conv.steps, "newer": conv.newer(), "output": output.display().to_string() }))
}

fn list_subprojects(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
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

fn page_list(a: &Args) -> Vec<String> {
    a.str("pages").unwrap_or_default().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

fn extract_subproject(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let pages = subproject::select_pages(prj, &page_list(a), a.str("structure").unwrap_or_default())?;
    let mut edited = prj.clone();
    let sub = subproject::extract(&mut edited, a.req("name"), a.str("description").unwrap_or_default(), &pages, &today())?;
    let path = Path::new(a.req("path"));
    write(path, sub.to_json().as_bytes())?;
    *s.project_mut()? = edited;
    s.commit();
    Ok(json!({ "path": path.display().to_string(), "pages": sub.pages.iter().map(|p| p.name.clone()).collect::<Vec<_>>(), "devices": sub.devices.len() }))
}

fn insert_subproject(s: &mut Session, a: &Args) -> Result<Value, String> {
    let (sub, _) = archive::read_project(&read(Path::new(a.req("path")))?)?;
    let r = subproject::insert(s.project_mut()?, &sub, &today())?;
    s.commit();
    Ok(json!({ "name": r.name, "pages_removed": r.pages_removed, "pages_inserted": r.pages_inserted, "devices": r.devices, "parts": r.parts, "renumbered": r.renumbered }))
}

fn discard_subproject(s: &mut Session, a: &Args) -> Result<Value, String> {
    subproject::discard(s.project_mut()?, a.req("name"))?;
    s.commit();
    list_subprojects(s, a)
}

fn options_json(prj: &Project) -> Value {
    json!(prj
        .model_options
        .options
        .iter()
        .map(|o| json!({ "name": o.name, "description": o.description, "condition": o.condition, "active": o.active, "members": o.members.len(), "pages": options::page_count(prj, o) }))
        .collect::<Vec<_>>())
}

fn list_model_options(s: &mut Session, _: &Args) -> Result<Value, String> {
    Ok(options_json(s.project()?))
}

fn define_model_option(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let old = prj.model_options.get(a.req("name"));
    let description = a.str("description").map(String::from).or_else(|| old.map(|o| o.description.clone())).unwrap_or_default();
    let condition = a.str("condition").map(String::from).or_else(|| old.map(|o| o.condition.clone())).unwrap_or_default();
    options::define(prj, a.req("name"), &description, &condition)?;
    s.commit();
    Ok(options_json(s.project()?))
}

fn remove_model_option(s: &mut Session, a: &Args) -> Result<Value, String> {
    options::remove(s.project_mut()?, a.req("name"))?;
    s.commit();
    Ok(options_json(s.project()?))
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

fn assign_model_option(s: &mut Session, a: &Args) -> Result<Value, String> {
    let ids = targets(s.project()?, a)?;
    let n = options::assign(s.project_mut()?, a.req("name"), &ids)?;
    s.commit();
    Ok(json!({ "assigned": n, "options": options_json(s.project()?) }))
}

fn unassign_model_option(s: &mut Session, a: &Args) -> Result<Value, String> {
    let mut ids = a.ids("ids");
    let prj = s.project()?;
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
    let n = options::unassign(s.project_mut()?, &ids);
    s.commit();
    Ok(json!({ "unassigned": n, "options": options_json(s.project()?) }))
}

fn set_model_option(s: &mut Session, a: &Args) -> Result<Value, String> {
    let moved = options::set_active(s.project_mut()?, a.req("name"), a.flag("active"))?;
    s.commit();
    Ok(json!({ "moved": moved, "options": options_json(s.project()?) }))
}

fn apply_model_options(s: &mut Session, _: &Args) -> Result<Value, String> {
    let switched = options::apply_conditions(s.project_mut()?)?;
    s.commit();
    let switched: Vec<Value> = switched.into_iter().map(|(name, active)| json!({ "name": name, "active": active })).collect();
    Ok(json!({ "switched": switched, "options": options_json(s.project()?) }))
}

#[cfg(test)]
mod tests;
