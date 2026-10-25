//! Wrapper of the EPLAN Data Portal API (`https://dataportal.eplan.com/api`, JSON:API)
//! that answers in the types of the parts server API, so that the part handling reads
//! the EPLAN Data Portal like a parts server: server info, faceted search, parts by
//! number and the files of a part (commercial data as CSV, DXF drawings, pictures).
//! The portal needs the personal access token of an EPLAN account (sent as
//! `Authorization: Bearer PAT:…`) or the token of a browser session. Its API offers
//! no changes, so the connection is read-only.
//!
//! | Parts server | EPLAN Data Portal |
//! | --- | --- |
//! | `GET /info` | `GET /parts?page[limit]=1` (number of parts) |
//! | `POST /search` | `GET /parts?search=&filter[manufacturer]=&filter[catalog_eplan]=&sort=&page[offset]=&page[limit]=` |
//! | `POST /search` with numbers | `POST /parts`, part numbers separated by commas |
//! | `GET /parts/{number}` | `GET /parts/{number}` |
//! | `GET /files/{id}` | `GET /download/commercial_data/part/{id}`, `GET /download/dxf_data/part/{id}`, picture previews |
//!
//! Product groups are filtered by the EPLAN catalog UID: generic product group (two
//! digits), product group and subgroup (four digits each), e.g. `0100020003`.

use crate::endpoint::with_content_type;
use openecraft_i18n::{lang, tr, tr_fmt};
use openecraft_parts::api::{self, encode_component, PartQuery, ServerInfo};
use openecraft_parts::portal::{Attachment, AttachmentKind, Facet, SearchResult, Sort};
use openecraft_parts::productgroups::{self, Classification};
use openecraft_parts::{unix_from_date, Part};
use serde_json::{Map, Value};

pub const EPLAN_URL: &str = "https://dataportal.eplan.com/api";
/// [`ServerInfo::name`] of the portal.
pub const EPLAN_NAME: &str = "EPLAN Data Portal";
/// Parts per request if the query sets no limit.
const LIMIT: usize = 50;
/// The facets the portal can filter by.
const FACETS: [&str; 4] = ["top_group", "group", "subgroup", "manufacturer"];

/// Address of the portal's API and the user's token; builds the requests.
#[derive(Clone, Debug, PartialEq)]
pub struct EplanEndpoint {
    pub url: String,
    pub token: String,
}

impl Default for EplanEndpoint {
    fn default() -> Self {
        EplanEndpoint { url: EPLAN_URL.into(), token: String::new() }
    }
}

impl EplanEndpoint {
    fn url(&self, path: &str) -> String {
        api::url(&self.url, path)
    }

    /// A personal access token is sent as `Bearer PAT:<token>`; the token of a browser
    /// session (a JWT, `eyJ…`) or one written with `PAT:` as it is.
    fn authorized(&self, mut req: ehttp::Request) -> ehttp::Request {
        let token = self.token.trim();
        if !token.is_empty() {
            let value = if token.starts_with("PAT:") || token.starts_with("eyJ") { format!("Bearer {token}") } else { format!("Bearer PAT:{token}") };
            req.headers.insert("Authorization", value);
        }
        req
    }

    /// The first part: checks address and token, the answer counts the parts.
    pub fn info(&self) -> ehttp::Request {
        self.authorized(ehttp::Request::get(self.url(&format!("/parts?{}=1", encode_component("page[limit]")))))
    }

    /// Search with text, manufacturer, product groups, order and page of the query;
    /// a query with part numbers loads these parts.
    pub fn search(&self, query: &PartQuery) -> ehttp::Request {
        if !query.numbers.is_empty() {
            return self.parts(&query.numbers);
        }
        self.authorized(ehttp::Request::get(self.url(&format!("/parts{}", query_string(query)))))
    }

    /// POST /parts: the parts with these numbers.
    pub fn parts(&self, numbers: &[String]) -> ehttp::Request {
        let req = ehttp::Request::post(self.url("/parts"), numbers.join(",").into_bytes());
        self.authorized(with_content_type(req, "text/plain"))
    }

    /// GET /parts/{number}
    pub fn part(&self, number: &str) -> ehttp::Request {
        self.authorized(ehttp::Request::get(self.url(&format!("/parts/{}", encode_component(number)))))
    }

    /// Address of a file of a part ([`Attachment::id`]: a path of the API or an address).
    pub fn file_url(&self, id: &str) -> String {
        if id.contains("://") {
            id.to_string()
        } else {
            self.url(&format!("/{}", id.trim_start_matches('/')))
        }
    }

    /// The file; the token is only sent to the portal itself.
    pub fn file(&self, id: &str) -> ehttp::Request {
        let url = self.file_url(id);
        let own = url.starts_with(self.url.trim().trim_end_matches('/'));
        let req = ehttp::Request::get(url);
        if own {
            self.authorized(req)
        } else {
            req
        }
    }

    /// True if the file can only be loaded with the token (not in a web browser).
    pub fn needs_token(&self, id: &str) -> bool {
        !id.contains("://")
    }
}

/// Query parameters of `GET /parts`. Catalogs, lifecycle and technical data are not
/// known to the portal.
pub fn query_string(query: &PartQuery) -> String {
    let mut params: Vec<(&str, String)> = Vec::new();
    if let Some(q) = query.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        params.push(("search", q.to_string()));
    }
    if let Some(m) = query.manufacturer.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
        params.push(("filter[manufacturer]", m.to_string()));
    }
    if let Some(uid) = catalog_uid(query) {
        params.push(("filter[catalog_eplan]", uid));
    }
    let sort = match query.sort {
        Sort::Manufacturer => "manufacturer.short_name",
        _ => "part_number",
    };
    params.push(("sort", format!("{}{sort}", if query.descending { "-" } else { "" })));
    params.push(("page[offset]", query.offset.unwrap_or(0).to_string()));
    params.push(("page[limit]", query.limit.unwrap_or(LIMIT).to_string()));
    let params: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", encode_component(k), encode_component(v))).collect();
    format!("?{}", params.join("&"))
}

/// EPLAN catalog UID of the product groups of the query (the generic product group
/// defaults to electrical engineering).
pub fn catalog_uid(query: &PartQuery) -> Option<String> {
    let group = query.group.as_deref().and_then(productgroups::group_id).filter(|g| *g != 0);
    let sub = query.subgroup.as_deref().and_then(productgroups::subgroup_id).filter(|s| *s != 0);
    if query.top_group.is_none() && group.is_none() {
        return None;
    }
    let top = query.top_group.unwrap_or_default().eplan_id();
    Some(match (group, sub) {
        (Some(g), Some(s)) => format!("{top:02}{g:04}{s:04}"),
        (Some(g), None) => format!("{top:02}{g:04}"),
        _ => format!("{top:02}"),
    })
}

/// Message of a failed request: access denied, the JSON:API `errors`, otherwise the
/// HTTP status.
pub fn error_text(status: u16, status_text: &str, body: &[u8]) -> String {
    if status == 401 || status == 403 {
        return tr("access denied – check the personal access token").into();
    }
    let doc: Value = serde_json::from_slice(body).unwrap_or_default();
    let messages: Vec<String> = doc["errors"]
        .as_array()
        .map(|errors| errors.iter().map(|e| first(e, &["detail", "title"])).filter(|m| !m.is_empty()).collect())
        .unwrap_or_default();
    if messages.is_empty() {
        api::error_text(status, status_text, body)
    } else {
        messages.join("; ")
    }
}

fn answer(result: ehttp::Result<ehttp::Response>) -> Result<ehttp::Response, String> {
    match result {
        Err(e) => Err(tr_fmt!("connection failed: {}", e)),
        Ok(resp) if resp.ok => Ok(resp),
        Ok(resp) => Err(error_text(resp.status, &resp.status_text, &resp.bytes)),
    }
}

fn document(result: ehttp::Result<ehttp::Response>) -> Result<Value, String> {
    serde_json::from_slice(&answer(result)?.bytes).map_err(|e| tr_fmt!("unexpected answer: {}", e))
}

/// Answer of [`EplanEndpoint::info`]: the portal as a read-only parts server.
pub fn decode_info(result: ehttp::Result<ehttp::Response>) -> Result<ServerInfo, String> {
    let doc = document(result)?;
    let shown = doc["data"].as_array().map_or(0, Vec::len);
    Ok(ServerInfo { name: EPLAN_NAME.into(), version: "API".into(), parts: total(&doc, shown), storage: "eplan".into(), ..Default::default() })
}

/// Answer of [`EplanEndpoint::search`] for `query`. The facets count the parts of the
/// answer (the portal does not count them).
pub fn decode_search(query: &PartQuery, result: ehttp::Result<ehttp::Response>) -> Result<SearchResult, String> {
    let doc = document(result)?;
    let mut parts = parts(&doc);
    let (total, offset) = if query.numbers.is_empty() {
        let offset = query.offset.unwrap_or(0);
        (total(&doc, offset + parts.len()), offset)
    } else {
        parts.retain(|p| query.numbers.contains(&p.number));
        (parts.len(), 0)
    };
    Ok(SearchResult { revision: 0, total, offset, facets: facets(query, &parts), attributes: Vec::new(), parts })
}

/// Answer of [`EplanEndpoint::part`]; `None` if the portal does not know the part.
pub fn decode_part(result: ehttp::Result<ehttp::Response>) -> Result<Option<Part>, String> {
    if result.as_ref().is_ok_and(|r| r.status == 404) {
        return Ok(None);
    }
    Ok(parts(&document(result)?).into_iter().next())
}

/// Answer of [`EplanEndpoint::file`].
pub fn decode_file(result: ehttp::Result<ehttp::Response>) -> Result<Vec<u8>, String> {
    answer(result).map(|r| r.bytes)
}

fn number(v: &Value) -> Option<usize> {
    v.as_u64().and_then(|n| usize::try_from(n).ok()).or_else(|| v.as_str()?.trim().parse().ok())
}

/// Number of matching parts from `meta`; without it `fallback`, one more if there is
/// a link to the next page.
fn total(doc: &Value, fallback: usize) -> usize {
    let meta = &doc["meta"];
    [&meta["total"], &meta["total_count"], &meta["totalCount"], &meta["count"], &meta["page"]["total"], &meta["pagination"]["total"]]
        .into_iter()
        .find_map(number)
        .unwrap_or_else(|| fallback + usize::from(!doc["links"]["next"].is_null()))
}

fn facets(query: &PartQuery, parts: &[Part]) -> Vec<Facet> {
    let filters = PartQuery {
        top_group: query.top_group,
        group: query.group.clone(),
        subgroup: query.subgroup.clone(),
        manufacturer: query.manufacturer.clone(),
        ..Default::default()
    };
    filters.search(parts, 0).facets.into_iter().filter(|f| FACETS.contains(&f.name.as_str())).collect()
}

/// Text of a value: strings, numbers, the first text of a list, a multilingual text.
fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(list) => list.iter().map(text).find(|t| !t.is_empty()).unwrap_or_default(),
        Value::Object(map) => localized(map),
        _ => String::new(),
    }
}

/// A multilingual text (`{"de_DE": "…", "en_US": "…"}`) in the user's language,
/// otherwise in English, otherwise the first one.
fn localized(map: &Map<String, Value>) -> String {
    let find = |code: &str| {
        map.iter()
            .filter(|(k, _)| k.get(..2).is_some_and(|p| p.eq_ignore_ascii_case(code)) && (k.len() == 2 || matches!(k.as_bytes()[2], b'_' | b'-')))
            .map(|(_, v)| text(v))
            .find(|t| !t.is_empty())
    };
    find(lang().code()).or_else(|| find("en")).or_else(|| map.values().filter(|v| !v.is_object()).map(text).find(|t| !t.is_empty())).unwrap_or_default()
}

/// The first non-empty text of these keys of an object; the text of anything else.
fn first(v: &Value, keys: &[&str]) -> String {
    match v {
        Value::Object(map) => keys.iter().filter_map(|k| map.get(*k)).map(text).find(|t| !t.is_empty()).unwrap_or_default(),
        _ => text(v),
    }
}

/// A resource with its attributes and `id` in one object; a resource identifier
/// (`{"type", "id"}`) is looked up in `included`.
fn flat(res: &Value, included: &[Value]) -> Value {
    let res = if res.get("attributes").is_none() && res.get("type").is_some() {
        included.iter().find(|i| i["type"] == res["type"] && i["id"] == res["id"]).unwrap_or(res)
    } else {
        res
    };
    match res.get("attributes") {
        Some(Value::Object(attributes)) => {
            let mut out = attributes.clone();
            if let Some(id) = res.get("id") {
                out.entry("id").or_insert_with(|| id.clone());
            }
            Value::Object(out)
        }
        _ => res.clone(),
    }
}

fn resolve(v: &Value, included: &[Value]) -> Value {
    match v {
        Value::Array(list) => Value::Array(list.iter().map(|x| flat(x, included)).collect()),
        Value::Object(_) => flat(v, included),
        _ => v.clone(),
    }
}

/// An attribute of a resource, or one of its relationships.
fn field(res: &Value, included: &[Value], name: &str) -> Value {
    let attribute = &res["attributes"][name];
    if attribute.is_null() {
        resolve(&res["relationships"][name]["data"], included)
    } else {
        resolve(attribute, included)
    }
}

/// The parts of a JSON:API document (`data` is one resource or a list).
pub fn parts(doc: &Value) -> Vec<Part> {
    let included = doc["included"].as_array().map(Vec::as_slice).unwrap_or_default();
    let data: Vec<&Value> = match &doc["data"] {
        Value::Array(list) => list.iter().collect(),
        v @ Value::Object(_) => vec![v],
        _ => Vec::new(),
    };
    data.into_iter().filter(|r| r["type"].as_str().is_none_or(|t| t == "parts")).map(|r| part(r, included)).filter(|p| !p.number.is_empty()).collect()
}

/// EPLAN number and name of a product group: `level` 0 generic product group, 1 product
/// group, 2 subgroup. The id may be the catalog UID of all three levels.
fn group(v: &Value, level: usize) -> (u16, String) {
    let name = first(v, &["name", "designation", "description"]);
    let id = first(v, &["id", "uid"]);
    let digits = !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit());
    let segment = [0..2, 2..6, 6..10][level].clone();
    let by_id = match id.len() {
        10 if digits => id[segment].parse().ok(),
        _ if digits => id.parse().ok(),
        _ => None,
    };
    let by_name = || match level {
        0 => productgroups::TOP_GROUPS.iter().find(|(_, n)| n.eq_ignore_ascii_case(&name)).map(|(i, _)| *i),
        1 => productgroups::group_id(&name),
        _ => productgroups::subgroup_id(&name),
    };
    (by_id.filter(|n| *n != 0).or_else(by_name).unwrap_or(0), name)
}

/// Lower case names of the data types of a part (`commercial_data`, `dxf_data`, …).
fn data_types(v: &Value) -> Vec<String> {
    let list = match v {
        Value::Array(list) => list.clone(),
        Value::Null => Vec::new(),
        other => vec![other.clone()],
    };
    list.iter().map(|t| first(t, &["name", "id", "type"]).to_lowercase().replace([' ', '-'], "_")).filter(|t| !t.is_empty()).collect()
}

/// A part number as part of a file name.
fn file_stem(number: &str) -> String {
    number.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' }).collect()
}

/// A part resource of the portal as a part of the parts server.
pub fn part(res: &Value, included: &[Value]) -> Part {
    let attr = |name: &str| text(&field(res, included, name));
    let id = text(&res["id"]);
    let number = Some(attr("part_number")).filter(|n| !n.is_empty()).unwrap_or_else(|| id.clone());
    let designation = attr("designation");
    let description = attr("description");
    let mut part = Part {
        type_number: attr("type_number"),
        order_number: attr("order_number"),
        manufacturer: first(&field(res, included, "manufacturer"), &["short_name", "name", "long_name", "id"]),
        note: if designation.is_empty() || description == designation { String::new() } else { description.clone() },
        description: if designation.is_empty() { description } else { designation },
        ..Default::default()
    };
    let [top, grp, sub] = [("product_top_group", 0), ("product_group", 1), ("product_sub_group", 2)].map(|(name, level)| group(&field(res, included, name), level));
    productgroups::apply(&mut part, Classification { top_group: top.0, group: grp.0, subgroup: sub.0 });
    // Groups the tables do not know keep their name.
    for (target, (_, name)) in [(&mut part.group, grp), (&mut part.subgroup, sub)] {
        if target.is_empty() && !name.bytes().all(|b| b.is_ascii_digit()) {
            *target = name;
        }
    }
    if let Value::Array(list) = field(res, included, "accessories") {
        part.accessories = list.iter().map(|a| first(a, &["part_number", "id"])).filter(|n| !n.is_empty()).collect();
    }
    part.modified = unix_from_date(&attr("update_date")).unwrap_or(0);
    part.revision = part.modified;
    if !id.is_empty() {
        let (pid, stem) = (encode_component(&id), file_stem(&number));
        let types = data_types(&field(res, included, "data_types"));
        let has = |t: &str| types.iter().any(|x| x == t);
        if types.is_empty() || has("commercial_data") {
            part.attachments.push(Attachment { id: format!("download/commercial_data/part/{pid}"), name: format!("{stem}_commercial_data.csv"), kind: AttachmentKind::Other, size: 0 });
        }
        if has("dxf_data") {
            part.attachments.push(Attachment { id: format!("download/dxf_data/part/{pid}"), name: format!("{stem}_dxf.zip"), kind: AttachmentKind::Drawing, size: 0 });
        }
    }
    let picture = field(res, included, "picture_file");
    let preview = first(&picture["preview"], &["512", "96"]);
    if preview.contains("://") {
        let path = first(&picture, &["id"]);
        let name = path.rsplit(['/', '\\']).next().filter(|n| n.contains('.')).map_or_else(|| format!("{}.png", file_stem(&number)), str::to_string);
        part.attachments.push(Attachment { id: preview, name, kind: AttachmentKind::Image, size: 0 });
    }
    part.number = number;
    part
}

#[cfg(test)]
mod tests;
