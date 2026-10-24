//! HTTP API of the parts server (data portal), shared by the server and its clients.
//!
//! | Method | Path | |
//! | --- | --- | --- |
//! | GET | `/api/v1/info` | [`ServerInfo`] (with the caller's [`User`]) |
//! | GET | `/api/v1/parts?q=&group=&manufacturer=&catalog=&offset=&limit=` | [`PartList`] |
//! | POST | `/api/v1/search` | [`PartQuery`] JSON → [`SearchResult`] with facets |
//! | GET | `/api/v1/parts/{number}` | [`Part`] |
//! | PUT | `/api/v1/parts/{number}` | create / replace a part (editor) |
//! | DELETE | `/api/v1/parts/{number}` | delete a part (editor) |
//! | GET | `/api/v1/parts/{number}/history` | [`HistoryEntry`] list, newest first |
//! | GET | `/api/v1/parts/{number}/history/{revision}` | the part as of that change |
//! | POST | `/api/v1/parts/{number}/attachments?name=&kind=` | file body → [`Part`] (editor) |
//! | DELETE | `/api/v1/parts/{number}/attachments/{id}` | → [`Part`] (editor) |
//! | GET | `/api/v1/files/{id}` | file of an attachment |
//! | POST | `/api/v1/import` | CSV, JSON or BMEcat body, upserts parts (editor) → [`ImportResult`] |
//! | GET | `/api/v1/export.csv` | all parts as CSV |
//! | GET | `/api/v1/catalogs` | [`Catalog`] list |
//! | PUT / DELETE | `/api/v1/catalogs/{id}` | create, change, delete a catalog (editor) |
//! | POST | `/api/v1/login` | [`Login`] → [`Session`] |
//! | POST | `/api/v1/logout` | ends the session of the token |
//! | GET / PUT | `/api/v1/favorites` | part numbers of the caller (signed in) |
//! | GET | `/api/v1/users` | [`User`] list (administrator) |
//! | PUT / DELETE | `/api/v1/users/{name}` | [`UserUpdate`] (administrator) |
//! | GET | `/api/v1/texts?lang=de` | translations of the web portal |
//!
//! Requests of a user carry `Authorization: Bearer <token>`: the token of a
//! [`Session`] or the server's administrator token (`--token`). Errors are returned
//! as [`ApiError`] JSON.

use crate::portal::{Lifecycle, Role, Sort, User};
use crate::TopGroup;
use serde::{Deserialize, Serialize};

pub use crate::portal::{Catalog, HistoryEntry, SearchResult};

pub const API_PREFIX: &str = "/api/v1";
pub const DEFAULT_PORT: u16 = 8090;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub parts: usize,
    /// Incremented on every change of the database.
    pub revision: u64,
    /// False if nobody can change the database (no administrator token, no users).
    pub writable: bool,
    pub catalogs: usize,
    /// Storage backend (`sqlite`, `json`).
    pub storage: String,
    /// The user of the request's token (`None`: anonymous).
    pub user: Option<User>,
}

impl ServerInfo {
    /// True if the caller may change parts of this manufacturer.
    pub fn may_edit(&self, manufacturer: &str) -> bool {
        self.user.as_ref().is_some_and(|u| u.may_edit(manufacturer))
    }

    /// True if the caller may change any parts at all.
    pub fn can_edit(&self) -> bool {
        self.user.as_ref().is_some_and(|u| u.role >= Role::Manufacturer)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PartList {
    pub revision: u64,
    /// Number of matching parts (before offset / limit).
    pub total: usize,
    pub offset: usize,
    pub parts: Vec<crate::Part>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImportResult {
    pub imported: usize,
    pub revision: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiError {
    pub error: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Login {
    pub name: String,
    pub password: String,
}

/// A signed in user and the token for its requests.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub token: String,
    pub user: User,
}

/// Creates or changes a user. An empty password keeps the current one (a new user
/// needs one).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserUpdate {
    pub role: Role,
    pub manufacturer: String,
    pub password: String,
}

/// Query parameters of `GET /api/v1/parts`, body of `POST /api/v1/search`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PartQuery {
    /// Free text (number, type, manufacturer, description, groups, catalog); every
    /// word must occur.
    pub q: Option<String>,
    pub top_group: Option<TopGroup>,
    /// Product group (exact, case insensitive).
    pub group: Option<String>,
    pub subgroup: Option<String>,
    /// Manufacturer (exact, case insensitive).
    pub manufacturer: Option<String>,
    /// Catalog id (exact, case insensitive).
    pub catalog: Option<String>,
    pub lifecycle: Option<Lifecycle>,
    /// Technical attributes (name, value) that must match (search only).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<(String, String)>,
    /// Only these part numbers (search only; update checks).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub numbers: Vec<String>,
    /// Only parts changed after this server revision.
    pub changed_since: Option<u64>,
    pub sort: Sort,
    pub descending: bool,
    /// Leave out the models of the parts (lists).
    pub brief: bool,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl PartQuery {
    /// `?q=…&group=…` (empty if no parameter is set). Attributes and numbers are only
    /// sent with `POST /api/v1/search`.
    pub fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        for (k, v) in [("q", &self.q), ("group", &self.group), ("subgroup", &self.subgroup), ("manufacturer", &self.manufacturer), ("catalog", &self.catalog)] {
            if let Some(v) = v {
                params.push(format!("{k}={}", encode_component(v)));
            }
        }
        if let Some(t) = self.top_group {
            params.push(format!("top_group={t:?}"));
        }
        if let Some(l) = self.lifecycle {
            params.push(format!("lifecycle={l:?}"));
        }
        if let Some(r) = self.changed_since {
            params.push(format!("changed_since={r}"));
        }
        if self.sort != Sort::Number {
            params.push(format!("sort={:?}", self.sort));
        }
        if self.descending {
            params.push("descending=true".into());
        }
        if self.brief {
            params.push("brief=true".into());
        }
        if let Some(o) = self.offset {
            params.push(format!("offset={o}"));
        }
        if let Some(l) = self.limit {
            params.push(format!("limit={l}"));
        }
        if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        }
    }
}

/// Readable message of a failed request: the server's [`ApiError`], otherwise the
/// HTTP status.
pub fn error_text(status: u16, status_text: &str, body: &[u8]) -> String {
    serde_json::from_slice::<ApiError>(body).map(|e| e.error).unwrap_or_else(|_| format!("{status} {status_text}"))
}

/// Percent-encodes everything except unreserved URL characters (RFC 3986).
pub fn encode_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Path of a single part, e.g. `/api/v1/parts/GEN.CT-9A-24DC`.
pub fn part_path(number: &str) -> String {
    format!("{API_PREFIX}/parts/{}", encode_component(number))
}

/// Path of an attachment's file.
pub fn file_path(id: &str) -> String {
    format!("{API_PREFIX}/files/{}", encode_component(id))
}

pub fn catalog_path(id: &str) -> String {
    format!("{API_PREFIX}/catalogs/{}", encode_component(id))
}

pub fn user_path(name: &str) -> String {
    format!("{API_PREFIX}/users/{}", encode_component(name))
}

/// Joins a server base URL (with or without trailing slash) and an API path.
pub fn url(base: &str, path: &str) -> String {
    format!("{}{path}", base.trim().trim_end_matches('/'))
}

/// The token of an `Authorization: Bearer <token>` header value.
pub fn bearer_token(header: &str) -> Option<&str> {
    header.strip_prefix("Bearer ").map(str::trim)
}

/// Compares a presented token with the expected one in constant time.
pub fn token_matches(presented: &str, expected: &str) -> bool {
    presented.len() == expected.len() && presented.bytes().zip(expected.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests;
