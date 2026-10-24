//! Parts server tools: the functions of the parts manager and the data portal
//! (faceted search, catalogs, files, users, favorites, update check) without a window.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_i18n::{tr, tr_const, tr_fmt};
use openecraft_parts::api::{PartQuery, UserUpdate};
use openecraft_parts::portal::{self, catalog_id, AttachmentKind, Catalog, Facet, Lifecycle, Role, Sort};
use openecraft_parts::{Part, TopGroup};
use serde_json::{json, Value};

const NUMBER: super::Param = param("number", Kind::Str, tr_const!("Part number"));

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "parts_connect",
        help: tr_const!("Sets the parts server (URL and access token, or user and password) and shows its state"),
        params: &[
            optional("url", Kind::Str, tr_const!("Server URL, e.g. http://127.0.0.1:8090")),
            optional("token", Kind::Str, tr_const!("Access token for changes")),
            optional("user", Kind::Str, tr_const!("User name to sign in with")),
            optional("password", Kind::Str, tr_const!("Password of the user")),
        ],
        needs_project: false,
        run: connect,
    },
    Tool {
        name: "parts_search",
        help: tr_const!("Searches the parts on the parts server (data portal) by text, groups, catalog, lifecycle and technical data"),
        params: &[
            optional("query", Kind::Str, tr_const!("Text in number, type, manufacturer, description, groups or catalog; every word must occur")),
            optional("top_group", Kind::Str, tr_const!("Product top group: Electrical, Fluid, Mechanics or Process")),
            optional("group", Kind::Str, tr_const!("Product group")),
            optional("subgroup", Kind::Str, tr_const!("Product subgroup")),
            optional("manufacturer", Kind::Str, tr_const!("Manufacturer")),
            optional("catalog", Kind::Str, tr_const!("Catalog id")),
            optional("lifecycle", Kind::Str, tr_const!("Lifecycle: Active, PhaseOut or Discontinued")),
            optional("attributes", Kind::Object, tr_const!("Technical data that must match, e.g. {\"Voltage\": \"24 V DC\", \"Poles\": \"3\"}")),
            optional("sort", Kind::Str, tr_const!("Order: Number, Manufacturer, Description, Price or Modified")),
            optional("descending", Kind::Bool, tr_const!("Descending order")),
            optional("offset", Kind::Int, tr_const!("Number of hits to skip")),
            optional("limit", Kind::Int, tr_const!("Maximum number of hits (default 50)")),
            optional("full", Kind::Bool, tr_const!("Return all part data instead of a summary")),
            optional("facets", Kind::Bool, tr_const!("Also return the facets: values of groups, manufacturers, catalogs, lifecycle and technical data with their counts")),
        ],
        needs_project: false,
        run: search,
    },
    Tool {
        name: "parts_history",
        help: tr_const!("Shows the changes of a part on the parts server, or the part as of one change"),
        params: &[NUMBER, optional("revision", Kind::Int, tr_const!("Revision of the change; returns the part as it was"))],
        needs_project: false,
        run: history,
    },
    Tool {
        name: "parts_attach",
        help: tr_const!("Stores a file (datasheet, image, model, macro …) with a part on the parts server"),
        params: &[
            NUMBER,
            param("path", Kind::Str, tr_const!("File to upload")),
            optional("kind", Kind::Str, tr_const!("Datasheet, Image, Manual, Certificate, Drawing, Model3d, Macro or Other (default: by file name)")),
        ],
        needs_project: false,
        run: attach,
    },
    Tool {
        name: "parts_detach",
        help: tr_const!("Removes a file from a part on the parts server"),
        params: &[NUMBER, param("id", Kind::Str, tr_const!("File id (see the attachments of parts_get)"))],
        needs_project: false,
        run: detach,
    },
    Tool {
        name: "parts_file",
        help: tr_const!("Downloads a file of a part from the parts server"),
        params: &[param("id", Kind::Str, tr_const!("File id (see the attachments of parts_get)")), param("path", Kind::Str, tr_const!("Output file"))],
        needs_project: false,
        run: file,
    },
    Tool {
        name: "parts_catalogs",
        help: tr_const!("Lists the catalogs of the parts server with their number of parts"),
        params: &[],
        needs_project: false,
        run: catalogs,
    },
    Tool {
        name: "parts_catalog_save",
        help: tr_const!("Creates or changes a catalog on the parts server (id, name, manufacturer, version, description)"),
        params: &[param("catalog", Kind::Object, tr_const!("Catalog data; without id it is derived from the name"))],
        needs_project: false,
        run: catalog_save,
    },
    Tool {
        name: "parts_catalog_delete",
        help: tr_const!("Deletes a catalog without parts from the parts server"),
        params: &[param("id", Kind::Str, tr_const!("Catalog id"))],
        needs_project: false,
        run: catalog_delete,
    },
    Tool {
        name: "parts_favorites",
        help: tr_const!("Shows the favorites of the signed in user, optionally adding or removing a part"),
        params: &[optional("add", Kind::Str, tr_const!("Part number to add")), optional("remove", Kind::Str, tr_const!("Part number to remove"))],
        needs_project: false,
        run: favorites,
    },
    Tool {
        name: "parts_users",
        help: tr_const!("Lists the users of the parts server (administrators only)"),
        params: &[],
        needs_project: false,
        run: users,
    },
    Tool {
        name: "parts_user_save",
        help: tr_const!("Creates or changes a user of the parts server (administrators only)"),
        params: &[
            param("name", Kind::Str, tr_const!("User name")),
            param("role", Kind::Str, tr_const!("Reader, Manufacturer, Editor or Admin")),
            optional("manufacturer", Kind::Str, tr_const!("Manufacturer whose parts a manufacturer user maintains")),
            optional("password", Kind::Str, tr_const!("Password (needed for new users; empty keeps the current one)")),
        ],
        needs_project: false,
        run: user_save,
    },
    Tool {
        name: "parts_user_delete",
        help: tr_const!("Deletes a user of the parts server (administrators only)"),
        params: &[param("name", Kind::Str, tr_const!("User name"))],
        needs_project: false,
        run: user_delete,
    },
    Tool {
        name: "parts_check_updates",
        help: tr_const!("Compares the project parts with the parts server: newer versions, phase-out, discontinued and removed parts"),
        params: &[optional("apply", Kind::Bool, tr_const!("Replace the project parts by their newer server versions"))],
        needs_project: true,
        run: check_updates,
    },
    Tool {
        name: "parts_get",
        help: tr_const!("Shows all data of a part"),
        params: &[NUMBER],
        needs_project: false,
        run: get,
    },
    Tool {
        name: "parts_save",
        help: tr_const!("Creates or replaces a part on the parts server (same JSON as parts_get; missing fields are empty)"),
        params: &[param("part", Kind::Object, tr_const!("Part data"))],
        needs_project: false,
        run: save,
    },
    Tool {
        name: "parts_rename",
        help: tr_const!("Changes the number of a part on the parts server"),
        params: &[NUMBER, param("new_number", Kind::Str, tr_const!("New part number"))],
        needs_project: false,
        run: rename,
    },
    Tool {
        name: "parts_delete",
        help: tr_const!("Deletes a part from the parts server"),
        params: &[NUMBER],
        needs_project: false,
        run: delete,
    },
    Tool {
        name: "parts_import",
        help: tr_const!("Imports a CSV or JSON parts file into the parts server"),
        params: &[param("path", Kind::Str, tr_const!("CSV or JSON file"))],
        needs_project: false,
        run: import,
    },
    Tool {
        name: "parts_export",
        help: tr_const!("Exports all parts of the parts server as CSV"),
        params: &[optional("path", Kind::Str, tr_const!("Output file; without it the content is returned"))],
        needs_project: false,
        run: export,
    },
];

fn connect(s: &mut Session, a: &Args) -> Result<Value, String> {
    if let Some(url) = a.str("url") {
        s.server.endpoint.url = url.into();
    }
    if let Some(token) = a.str("token") {
        s.server.endpoint.token = token.into();
    }
    if let Some(user) = a.str("user") {
        s.server.login(user, a.str("password").unwrap_or(""))?;
    }
    let info = s.server.info()?;
    Ok(json!({ "url": s.server.endpoint.url, "info": info }))
}

/// A value of an enumeration by its name (case insensitive), e.g. `PhaseOut`.
fn choice<T: Copy + std::fmt::Debug>(all: &[T], name: &str, text: &str) -> Result<T, String> {
    all.iter()
        .copied()
        .find(|v| format!("{v:?}").eq_ignore_ascii_case(text.trim()))
        .ok_or_else(|| tr_fmt!("Unknown {}: {} (possible: {})", name, text, all.iter().map(|v| format!("{v:?}")).collect::<Vec<_>>().join(", ")))
}

fn optional_choice<T: Copy + std::fmt::Debug>(a: &Args, key: &str, all: &[T]) -> Result<Option<T>, String> {
    a.str(key).map(|t| choice(all, key, t)).transpose()
}

fn search(s: &mut Session, a: &Args) -> Result<Value, String> {
    let text = |k: &str| a.str(k).map(String::from);
    let attributes = match a.value("attributes") {
        Some(Value::Object(m)) => m.iter().map(|(k, v)| (k.clone(), v.as_str().map(String::from).unwrap_or_else(|| v.to_string()))).collect(),
        _ => Vec::new(),
    };
    let query = PartQuery {
        q: text("query"),
        top_group: optional_choice(a, "top_group", &TopGroup::ALL)?,
        group: text("group"),
        subgroup: text("subgroup"),
        manufacturer: text("manufacturer"),
        catalog: text("catalog"),
        lifecycle: optional_choice(a, "lifecycle", &Lifecycle::ALL)?,
        attributes,
        sort: optional_choice(a, "sort", &Sort::ALL)?.unwrap_or_default(),
        descending: a.flag("descending"),
        brief: !a.flag("full"),
        offset: a.int("offset").map(|o| o.max(0) as usize),
        limit: Some(a.int("limit").unwrap_or(50).max(0) as usize),
        ..Default::default()
    };
    let r = s.server.search(&query)?;
    let parts: Vec<Value> = if a.flag("full") {
        r.parts.iter().map(|p| serde_json::to_value(p).unwrap_or_default()).collect()
    } else {
        r.parts
            .iter()
            .map(|p| {
                let mut v = json!({
                    "number": p.number,
                    "manufacturer": p.manufacturer,
                    "type_number": p.type_number,
                    "description": p.description,
                    "group": p.group,
                });
                if !p.lifecycle.is_active() {
                    v["lifecycle"] = json!(p.lifecycle);
                    v["successor"] = json!(p.successor);
                }
                if !p.catalog.is_empty() {
                    v["catalog"] = json!(p.catalog);
                }
                v
            })
            .collect()
    };
    let mut out = json!({ "total": r.total, "offset": r.offset, "parts": parts });
    if a.flag("facets") {
        let facets = |list: &[Facet]| -> Value {
            list.iter().map(|f| (f.name.clone(), f.values.iter().map(|v| json!([v.value, v.count])).collect::<Value>())).collect::<serde_json::Map<_, _>>().into()
        };
        out["facets"] = facets(&r.facets);
        out["attributes"] = facets(&r.attributes);
    }
    Ok(out)
}

fn history(s: &mut Session, a: &Args) -> Result<Value, String> {
    let number = a.req("number");
    match a.int("revision") {
        Some(rev) => {
            let part = s.server.history_part(number, rev.max(0) as u64)?.ok_or_else(|| tr_fmt!("Part {} has no version {}", number, rev))?;
            Ok(serde_json::to_value(part).unwrap_or_default())
        }
        None => Ok(serde_json::to_value(s.server.history(number)?).unwrap_or_default()),
    }
}

fn attach(s: &mut Session, a: &Args) -> Result<Value, String> {
    let (number, path) = (a.req("number"), a.req("path"));
    let kind = optional_choice(a, "kind", &AttachmentKind::ALL)?;
    let data = std::fs::read(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.into());
    let part = s.server.attach(number, &name, kind, data)?;
    Ok(json!({ "number": part.number, "attachments": part.attachments }))
}

fn detach(s: &mut Session, a: &Args) -> Result<Value, String> {
    let part = s.server.detach(a.req("number"), a.req("id"))?;
    Ok(json!({ "number": part.number, "attachments": part.attachments }))
}

fn file(s: &mut Session, a: &Args) -> Result<Value, String> {
    let (id, path) = (a.req("id"), a.req("path"));
    let data = s.server.file(id)?;
    std::fs::write(path, &data).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
    Ok(json!({ "path": path, "bytes": data.len() }))
}

fn catalogs(s: &mut Session, _: &Args) -> Result<Value, String> {
    Ok(serde_json::to_value(s.server.catalogs()?).unwrap_or_default())
}

fn catalog_save(s: &mut Session, a: &Args) -> Result<Value, String> {
    let mut c: Catalog = serde_json::from_value(a.value("catalog").cloned().unwrap_or_default()).map_err(|e| tr_fmt!("Not a valid catalog: {}", e))?;
    if c.id.trim().is_empty() {
        c.id = catalog_id(&c.name);
    }
    if c.id.is_empty() {
        return Err(tr("The catalog needs a name").to_string());
    }
    Ok(serde_json::to_value(s.server.save_catalog(&c)?).unwrap_or_default())
}

fn catalog_delete(s: &mut Session, a: &Args) -> Result<Value, String> {
    let id = a.req("id");
    s.server.delete_catalog(id)?;
    Ok(json!({ "deleted": id }))
}

fn favorites(s: &mut Session, a: &Args) -> Result<Value, String> {
    let mut list = s.server.favorites()?;
    let before = list.clone();
    if let Some(n) = a.str("add").filter(|n| !list.iter().any(|f| f == n)) {
        list.push(n.into());
    }
    if let Some(n) = a.str("remove") {
        list.retain(|f| f != n);
    }
    if list != before {
        list = s.server.set_favorites(&list)?;
    }
    Ok(json!(list))
}

fn users(s: &mut Session, _: &Args) -> Result<Value, String> {
    Ok(serde_json::to_value(s.server.users()?).unwrap_or_default())
}

fn user_save(s: &mut Session, a: &Args) -> Result<Value, String> {
    let update = UserUpdate {
        role: choice(&Role::ALL, "role", a.req("role"))?,
        manufacturer: a.str("manufacturer").unwrap_or("").into(),
        password: a.str("password").unwrap_or("").into(),
    };
    Ok(serde_json::to_value(s.server.save_user(a.req("name"), &update)?).unwrap_or_default())
}

fn user_delete(s: &mut Session, a: &Args) -> Result<Value, String> {
    let name = a.req("name");
    s.server.delete_user(name)?;
    Ok(json!({ "deleted": name }))
}

fn check_updates(s: &mut Session, a: &Args) -> Result<Value, String> {
    let numbers: Vec<String> = s.project()?.parts.iter().map(|p| p.number.clone()).collect();
    if numbers.is_empty() {
        return Ok(json!({ "updates": [], "applied": 0 }));
    }
    let remote = s.server.search(&PartQuery { numbers, ..Default::default() })?.parts;
    let updates = portal::check_updates(&s.project()?.parts, &remote);
    let applied = if a.flag("apply") { portal::apply_updates(&mut s.project_mut()?.parts, &remote, &updates) } else { 0 };
    Ok(json!({ "updates": updates, "applied": applied }))
}

fn existing(s: &Session, number: &str) -> Result<Part, String> {
    s.server.get(number)?.ok_or_else(|| tr_fmt!("Unknown part: {}", number))
}

fn get(s: &mut Session, a: &Args) -> Result<Value, String> {
    Ok(serde_json::to_value(existing(s, a.req("number"))?).unwrap_or_default())
}

fn save(s: &mut Session, a: &Args) -> Result<Value, String> {
    let part: Part = serde_json::from_value(a.value("part").cloned().unwrap_or_default()).map_err(|e| tr_fmt!("Not a valid part: {}", e))?;
    if part.number.trim().is_empty() {
        return Err(tr("The part number must not be empty").to_string());
    }
    s.server.put(&part)?;
    Ok(json!({ "saved": part.number }))
}

fn rename(s: &mut Session, a: &Args) -> Result<Value, String> {
    let (number, new) = (a.req("number"), a.req("new_number").trim());
    let mut part = existing(s, number)?;
    if new.is_empty() {
        return Err(tr("The part number must not be empty").to_string());
    }
    if new != number && s.server.get(new)?.is_some() {
        return Err(tr("Part number already exists").to_string());
    }
    part.number = new.into();
    s.server.put(&part)?;
    if new != number {
        s.server.delete(number)?;
    }
    Ok(json!({ "old": number, "new": new }))
}

fn delete(s: &mut Session, a: &Args) -> Result<Value, String> {
    let number = a.req("number");
    existing(s, number)?;
    s.server.delete(number)?;
    Ok(json!({ "deleted": number }))
}

fn import(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("path");
    let content = std::fs::read(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let result = s.server.import(content)?;
    Ok(json!({ "imported": result.imported, "revision": result.revision }))
}

fn export(s: &mut Session, a: &Args) -> Result<Value, String> {
    let csv = s.server.export()?;
    match a.str("path") {
        Some(path) => {
            std::fs::write(path, &csv).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
            Ok(json!({ "path": path, "bytes": csv.len() }))
        }
        None => Ok(Value::String(csv)),
    }
}

#[cfg(test)]
mod tests;
