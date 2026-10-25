//! Autodesk Vault tools: signing in, browsing and searching folders and items,
//! downloading files, opening projects from the vault, putting the project, exports
//! and files into the vault (new files or new versions) and taking the item numbers,
//! revisions and lifecycle states of the vault's items over into the project parts.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use crate::vault::Origin;
use openecraft_api::builtin::integration::connector;
use openecraft_client::vault::{item_records, Entry, Target, Upload, ROOT};
use openecraft_core::analysis::Analysis;
use openecraft_core::archive;
use openecraft_core::integration;
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "vault_connect",
        help: tr_const!("Connects to Autodesk Vault (Vault Data API, Vault Professional 2025.2 or newer): signs in with a Vault account or uses an access token, and shows the connection"),
        params: &[
            optional("url", Kind::Str, tr_const!("Address of the Vault server, e.g. https://vault.example.com")),
            optional("vault", Kind::Str, tr_const!("Name of the knowledge vault (default Vault)")),
            optional("user", Kind::Str, tr_const!("Vault user name (signs in)")),
            optional("password", Kind::Str, tr_const!("Password of the Vault user")),
            optional("token", Kind::Str, tr_const!("Access token of a Vault session or an Autodesk account instead of user and password")),
            optional("vault_id", Kind::Str, tr_const!("Id of the knowledge vault (taken from the session if not given)")),
        ],
        needs_project: false,
        run: connect,
    },
    Tool {
        name: "vault_browse",
        help: tr_const!("Lists the folders and files of a Vault folder, or searches the file properties below it (or in the whole vault)"),
        params: &[
            optional("folder", Kind::Str, tr_const!("Folder id (default root)")),
            optional("query", Kind::Str, tr_const!("Text to search for")),
        ],
        needs_project: false,
        run: browse,
    },
    Tool {
        name: "vault_items",
        help: tr_const!("Searches the items of the vault (number, title, revision, lifecycle state, part number)"),
        params: &[param("query", Kind::Str, tr_const!("Text to search for"))],
        needs_project: false,
        run: items,
    },
    Tool {
        name: "vault_download",
        help: tr_const!("Downloads a file version from the vault"),
        params: &[param("version", Kind::Str, tr_const!("Id of the file version (version_id of vault_browse)")), param("path", Kind::Str, tr_const!("File to write"))],
        needs_project: false,
        run: download,
    },
    Tool {
        name: "vault_open",
        help: tr_const!("Opens a project file or archive from the vault; vault_upload without a target checks in a new version of it"),
        params: &[
            param("version", Kind::Str, tr_const!("Id of the file version (version_id of vault_browse)")),
            optional("path", Kind::Str, tr_const!("Also save the project in this local file")),
        ],
        needs_project: false,
        run: open,
    },
    Tool {
        name: "vault_upload",
        help: tr_const!("Puts the project, an export, a connector export or a local file into the vault; a file of the same name in the folder gets a new version (checked out and in), otherwise the file is added"),
        params: &[
            optional("folder", Kind::Str, tr_const!("Target folder id (root for the top folder); default the folder the project was opened from")),
            optional("file", Kind::Str, tr_const!("Id of a file (file_id of vault_browse) that gets a new version")),
            optional("name", Kind::Str, tr_const!("File name in the vault")),
            optional("path", Kind::Str, tr_const!("Local file to put into the vault")),
            optional("format", Kind::Str, tr_const!("Export name as shown by list_exports, e.g. pdf or step")),
            optional("page", Kind::Str, tr_const!("Page of the export")),
            optional("language", Kind::Str, tr_const!("Language of the export (a project language)")),
            optional("options", Kind::Str, tr_const!("Further parameters of the export as name=value, separated by commas")),
            optional("connector", Kind::Str, tr_const!("Connector export instead: erp, pdm, plm, ecad, mcad, procurement, wiring_machine or exchange")),
            optional("comment", Kind::Str, tr_const!("Check-in comment")),
            optional("keep_checked_out", Kind::Bool, tr_const!("Keep the file checked out after the check-in")),
        ],
        needs_project: false,
        run: upload,
    },
    Tool {
        name: "vault_sync_items",
        help: tr_const!("Finds the Vault items of the project parts (by item number or Part Number property) and lists the changes of item ID, revision and lifecycle state; with apply they are taken over"),
        params: &[
            optional("apply", Kind::Bool, tr_const!("Take the changes over (otherwise only a preview)")),
            optional("overwrite", Kind::Bool, tr_const!("Also take over values changed on both sides since the last export")),
            optional("date", Kind::Str, tr_const!("Date for the exchange log")),
        ],
        needs_project: true,
        run: sync_items,
    },
];

fn status(s: &Session) -> Value {
    let v = &s.vault;
    json!({
        "url": v.endpoint.url,
        "vault": v.vault_name,
        "vault_id": v.endpoint.vault,
        "user": v.user,
        "connected": v.connected().is_ok(),
        "origin": v.origin.as_ref().map(|o| json!({ "folder": o.folder, "name": o.name })),
    })
}

fn connect(s: &mut Session, a: &Args) -> Result<Value, String> {
    let v = &mut s.vault;
    if let Some(url) = a.str("url") {
        v.endpoint.url = url.trim().to_string();
    }
    if let Some(name) = a.str("vault") {
        v.vault_name = name.trim().to_string();
    }
    if let Some(id) = a.str("vault_id") {
        v.endpoint.vault = id.trim().to_string();
    }
    if v.endpoint.url.trim().is_empty() && (a.str("user").is_some() || a.str("token").is_some()) {
        return Err(tr_fmt!("Missing parameter: {}", "url"));
    }
    if let Some(user) = a.str("user") {
        let vault = if v.vault_name.is_empty() { "Vault".to_string() } else { v.vault_name.clone() };
        v.sign_in(&vault, user, a.str("password").unwrap_or_default())?;
    } else if let Some(token) = a.str("token") {
        v.endpoint.token = token.trim().to_string();
        if a.str("vault_id").is_none() {
            v.adopt_session()?;
        }
    }
    let mut out = status(s);
    // The command line starts a session per call: the token goes to OPENECRAFT_VAULT_TOKEN.
    if a.str("user").is_some() {
        out["token"] = s.vault.endpoint.token.clone().into();
    }
    if !s.vault.endpoint.url.trim().is_empty() {
        if let Ok(info) = s.vault.server_info() {
            out["server"] = json!({ "name": info.name, "version": info.product_version });
        }
    }
    Ok(out)
}

fn entry_json(e: &Entry) -> Value {
    match e {
        Entry::Folder(f) => json!({ "kind": "folder", "id": f.id, "name": f.name, "path": f.full_name }),
        Entry::File(f) => json!({
            "kind": "file",
            "version_id": f.id,
            "file_id": f.file_id(),
            "name": f.name,
            "version": f.version_number(),
            "revision": f.revision,
            "state": f.state,
            "checked_out": f.is_checked_out,
            "folder": f.parent_folder_id,
        }),
        Entry::Item(i) => item_json(i),
    }
}

fn item_json(i: &openecraft_client::vault::ItemVersion) -> Value {
    json!({
        "kind": "item",
        "id": i.id,
        "number": i.number,
        "title": i.title,
        "revision": i.revision,
        "state": i.state,
        "part_number": i.property(&["Part Number", "PartNumber"]),
    })
}

fn browse(s: &mut Session, a: &Args) -> Result<Value, String> {
    let folder = a.str("folder").map(str::trim).filter(|f| !f.is_empty());
    let page = s.vault.entries(folder, a.str("query"))?;
    Ok(json!({
        "folder": folder.unwrap_or(ROOT),
        "total": page.pagination.total_results.max(page.results.len()),
        "more": page.pagination.next_url.is_some(),
        "entries": page.results.iter().map(entry_json).collect::<Vec<_>>(),
    }))
}

fn items(s: &mut Session, a: &Args) -> Result<Value, String> {
    let page = s.vault.items(a.req("query"))?;
    Ok(json!({
        "total": page.pagination.total_results.max(page.results.len()),
        "items": page.results.iter().map(item_json).collect::<Vec<_>>(),
    }))
}

fn download(s: &mut Session, a: &Args) -> Result<Value, String> {
    let bytes = s.vault.download(a.req("version"))?;
    let path = a.req("path");
    std::fs::write(path, &bytes).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
    Ok(json!({ "path": path, "bytes": bytes.len() }))
}

fn open(s: &mut Session, a: &Args) -> Result<Value, String> {
    let id = a.req("version");
    let file = s.vault.file_version(id)?;
    let bytes = s.vault.download(id)?;
    let (prj, conv) = archive::read_project(&bytes)?;
    let folder = if file.parent_folder_id.is_empty() { ROOT.to_string() } else { file.parent_folder_id.clone() };
    s.replace(prj, None);
    s.vault.origin = Some(Origin { folder, name: file.name.clone() });
    if let Some(path) = a.str("path") {
        s.save(Some(path))?;
    }
    let prj = s.project()?;
    Ok(json!({
        "name": file.name,
        "version": file.version_number(),
        "revision": file.revision,
        "state": file.state,
        "project": prj.info.name,
        "pages": prj.pages.len(),
        "conversion": conv.message(),
    }))
}

/// File name from the project name, e.g. `Plant_1`.
fn project_stem(s: &Session) -> String {
    let name = s.project().map(|p| p.info.name.clone()).unwrap_or_default();
    let stem: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    if stem.is_empty() { "project".into() } else { stem }
}

/// Content and default file name of what `vault_upload` puts into the vault.
fn upload_content(s: &mut Session, a: &Args, name: Option<&str>) -> Result<(Vec<u8>, String), String> {
    if let Some(path) = a.str("path") {
        let bytes = std::fs::read(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
        return Ok((bytes, path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()));
    }
    let stem = project_stem(s);
    if let Some(format) = a.str("format") {
        let ext = s.plugins.exporter(format).map(|e| e.extension.clone()).ok_or_else(|| tr_fmt!("Unknown format: {}", format))?;
        let default = format!("{stem}.{ext}");
        let out = super::output::export_content(s, a, Some(name.unwrap_or(&default)))?;
        return Ok((out.into_bytes(), default));
    }
    if a.str("connector").is_some() {
        let prj = s.project_mut()?;
        let an = Analysis::with_builtin(prj);
        if a.req("connector") == "exchange" {
            return Ok((integration::exchange(prj, &an).into_bytes(), format!("{stem}_exchange.json")));
        }
        let c = connector(a)?;
        let text = integration::export(prj, &an, c, "");
        return Ok((text.into_bytes(), format!("{stem}_{}.{}", c.id, c.extension)));
    }
    let prj = s.project()?;
    let default = match &s.vault.origin {
        Some(o) => o.name.clone(),
        None => format!("{stem}.oec"),
    };
    Ok((prj.to_json().into_bytes(), default))
}

fn upload(s: &mut Session, a: &Args) -> Result<Value, String> {
    s.vault.connected()?;
    let name = a.str("name").map(str::trim).filter(|n| !n.is_empty());
    let project = a.str("path").is_none() && a.str("format").is_none() && a.str("connector").is_none();
    let (data, default) = upload_content(s, a, name)?;
    let target = match (a.str("file"), a.str("folder")) {
        (Some(file), _) => Target::File(file.trim().to_string()),
        (None, Some(folder)) => Target::Folder(folder.trim().to_string()),
        (None, None) => match &s.vault.origin {
            Some(o) => Target::Folder(o.folder.clone()),
            None => return Err(tr_fmt!("Missing parameter: {}", "folder")),
        },
    };
    // A new version of a given file keeps its name unless one is given.
    let name = match (&target, name) {
        (_, Some(n)) => n.to_string(),
        (Target::File(_), None) => String::new(),
        (Target::Folder(_), None) => default,
    };
    let mut job = Upload::new(s.vault.endpoint.clone(), target.clone(), &name, data, a.str("comment").unwrap_or_default());
    job.keep_checked_out = a.flag("keep_checked_out");
    let done = s.vault.upload(job)?;
    let v = &done.version;
    if project {
        let folder = match target {
            Target::Folder(f) => f,
            Target::File(_) => v.parent_folder_id.clone(),
        };
        s.vault.origin = Some(Origin { folder: if folder.is_empty() { ROOT.into() } else { folder }, name: v.name.clone() });
    }
    Ok(json!({
        "name": if v.name.is_empty() { name } else { v.name.clone() },
        "added": done.added,
        "file_id": v.file_id(),
        "version_id": v.id,
        "version": v.version_number(),
    }))
}

fn sync_items(s: &mut Session, a: &Args) -> Result<Value, String> {
    s.vault.connected()?;
    let numbers: Vec<String> = s.project()?.parts.iter().map(|p| p.number.clone()).collect();
    let found = s.vault.lookup(numbers)?;
    let text = item_records(&found);
    let c = integration::connector("plm").ok_or_else(|| tr_fmt!("Unknown connector: {}", "plm"))?;
    let prj = s.project_mut()?;
    let an = Analysis::with_builtin(prj);
    let preview = integration::import(prj, &an, c, &text, &BTreeMap::new())?;
    let applied = if a.flag("apply") { Some(integration::apply(prj, &an, c, &preview.changes, a.flag("overwrite"), a.str("date").unwrap_or_default())) } else { None };
    Ok(json!({
        "items": found.len(),
        "changes": preview.changes.iter().map(super::integration::change_json).collect::<Vec<_>>(),
        "conflicts": preview.conflicts(),
        "unmatched": preview.unmatched,
        "applied": applied,
        "message": if found.is_empty() { Some(tr("No Vault item matches a part of the project")) } else { None },
    }))
}

#[cfg(test)]
mod tests;
