//! HTTP API of the parts server, shared by the server and its clients.
//!
//! | Method | Path | |
//! | --- | --- | --- |
//! | GET | `/api/v1/info` | [`ServerInfo`] |
//! | GET | `/api/v1/parts?q=&group=&manufacturer=&offset=&limit=` | [`PartList`] |
//! | GET | `/api/v1/parts/{number}` | [`Part`] |
//! | PUT | `/api/v1/parts/{number}` | create / replace a part (token) |
//! | DELETE | `/api/v1/parts/{number}` | delete a part (token) |
//! | POST | `/api/v1/import` | CSV or JSON body, upserts parts (token) → [`ImportResult`] |
//! | GET | `/api/v1/export.csv` | all parts as CSV |
//!
//! Write requests need `Authorization: Bearer <token>`. Errors are returned as
//! [`ApiError`] JSON.

use crate::{natural_cmp, Part};
use serde::{Deserialize, Serialize};

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
    /// False if the server has no write token configured (read-only).
    pub writable: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PartList {
    pub revision: u64,
    /// Number of matching parts (before offset / limit).
    pub total: usize,
    pub offset: usize,
    pub parts: Vec<Part>,
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

/// Query parameters of `GET /api/v1/parts`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PartQuery {
    /// Free text (number, type, manufacturer, description, groups).
    pub q: Option<String>,
    /// Product group (exact, case insensitive).
    pub group: Option<String>,
    /// Manufacturer (exact, case insensitive).
    pub manufacturer: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl PartQuery {
    /// Filters, sorts (by part number) and pages `parts`. Returns the number of
    /// matches before paging and the requested page.
    pub fn apply(&self, parts: &[Part]) -> (usize, Vec<Part>) {
        let eq = |a: &str, b: &Option<String>| b.as_deref().map(|b| a.eq_ignore_ascii_case(b)).unwrap_or(true);
        let mut hits: Vec<&Part> = parts
            .iter()
            .filter(|p| p.matches_text(self.q.as_deref().unwrap_or("")))
            .filter(|p| eq(&p.group, &self.group) && eq(&p.manufacturer, &self.manufacturer))
            .collect();
        hits.sort_by(|a, b| natural_cmp(&a.number, &b.number));
        let total = hits.len();
        let page = hits
            .into_iter()
            .skip(self.offset.unwrap_or(0))
            .take(self.limit.unwrap_or(usize::MAX))
            .cloned()
            .collect();
        (total, page)
    }

    /// `?q=…&group=…` (empty if no parameter is set).
    pub fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        for (k, v) in [("q", &self.q), ("group", &self.group), ("manufacturer", &self.manufacturer)] {
            if let Some(v) = v {
                params.push(format!("{k}={}", encode_component(v)));
            }
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
