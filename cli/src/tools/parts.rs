//! Parts server tools: the functions of the parts manager without a window.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_i18n::{tr, tr_const, tr_fmt};
use openecraft_parts::api::PartQuery;
use openecraft_parts::Part;
use serde_json::{json, Value};

const NUMBER: super::Param = param("number", Kind::Str, tr_const!("Part number"));

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "parts_connect",
        help: tr_const!("Sets the parts server (URL and access token) and shows its state"),
        params: &[
            optional("url", Kind::Str, tr_const!("Server URL, e.g. http://127.0.0.1:8090")),
            optional("token", Kind::Str, tr_const!("Access token for changes")),
        ],
        needs_project: false,
        run: connect,
    },
    Tool {
        name: "parts_search",
        help: tr_const!("Searches the parts on the parts server"),
        params: &[
            optional("query", Kind::Str, tr_const!("Text in number, type, manufacturer, description or groups")),
            optional("group", Kind::Str, tr_const!("Product group")),
            optional("manufacturer", Kind::Str, tr_const!("Manufacturer")),
            optional("offset", Kind::Int, tr_const!("Number of hits to skip")),
            optional("limit", Kind::Int, tr_const!("Maximum number of hits (default 50)")),
            optional("full", Kind::Bool, tr_const!("Return all part data instead of a summary")),
        ],
        needs_project: false,
        run: search,
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
        s.server.url = url.into();
    }
    if let Some(token) = a.str("token") {
        s.server.token = token.into();
    }
    let info = s.server.info()?;
    Ok(json!({ "url": s.server.url, "info": info }))
}

fn search(s: &mut Session, a: &Args) -> Result<Value, String> {
    let query = PartQuery {
        q: a.str("query").map(String::from),
        group: a.str("group").map(String::from),
        manufacturer: a.str("manufacturer").map(String::from),
        offset: a.int("offset").map(|o| o.max(0) as usize),
        limit: Some(a.int("limit").unwrap_or(50).max(0) as usize),
    };
    let list = s.server.list(&query)?;
    let parts: Vec<Value> = if a.flag("full") {
        list.parts.iter().map(|p| serde_json::to_value(p).unwrap_or_default()).collect()
    } else {
        list.parts
            .iter()
            .map(|p| {
                json!({
                    "number": p.number,
                    "manufacturer": p.manufacturer,
                    "type_number": p.type_number,
                    "description": p.description,
                    "group": p.group,
                })
            })
            .collect()
    };
    Ok(json!({ "total": list.total, "offset": list.offset, "parts": parts }))
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
