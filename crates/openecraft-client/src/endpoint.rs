use openecraft_i18n::{tr, tr_fmt};
use openecraft_parts::api::{self, PartQuery, API_PREFIX, DEFAULT_PORT};
use openecraft_parts::Part;
use serde::de::DeserializeOwned;

/// Server address and access token; builds the requests of the API. Changes carry
/// the token as `Authorization: Bearer …`.
#[derive(Clone, Debug, PartialEq)]
pub struct Endpoint {
    pub url: String,
    pub token: String,
}

impl Default for Endpoint {
    fn default() -> Self {
        Endpoint { url: format!("http://127.0.0.1:{DEFAULT_PORT}"), token: String::new() }
    }
}

impl Endpoint {
    fn url(&self, path: &str) -> String {
        api::url(&self.url, path)
    }

    fn authorized(&self, mut req: ehttp::Request) -> ehttp::Request {
        if !self.token.trim().is_empty() {
            req.headers.insert("Authorization", format!("Bearer {}", self.token.trim()));
        }
        req
    }

    /// GET /api/v1/info
    pub fn info(&self) -> ehttp::Request {
        ehttp::Request::get(self.url(&format!("{API_PREFIX}/info")))
    }

    /// GET /api/v1/parts?…
    pub fn list(&self, query: &PartQuery) -> ehttp::Request {
        ehttp::Request::get(self.url(&format!("{API_PREFIX}/parts{}", query.to_query_string())))
    }

    /// GET /api/v1/parts/{number}
    pub fn part(&self, number: &str) -> ehttp::Request {
        ehttp::Request::get(self.url(&api::part_path(number)))
    }

    /// PUT /api/v1/parts/{number}: creates or replaces a part.
    pub fn save(&self, part: &Part) -> ehttp::Request {
        let body = serde_json::to_vec(part).unwrap_or_default();
        self.authorized(with_content_type(ehttp::Request::put(self.url(&api::part_path(&part.number)), body), "application/json"))
    }

    /// DELETE /api/v1/parts/{number}
    pub fn delete(&self, number: &str) -> ehttp::Request {
        self.authorized(ehttp::Request::delete(&self.url(&api::part_path(number))))
    }

    /// POST /api/v1/import with CSV or JSON parts.
    pub fn import(&self, body: Vec<u8>, content_type: &str) -> ehttp::Request {
        self.authorized(with_content_type(ehttp::Request::post(self.url(&format!("{API_PREFIX}/import")), body), content_type))
    }

    /// GET /api/v1/export.csv
    pub fn export(&self) -> ehttp::Request {
        ehttp::Request::get(self.url(&format!("{API_PREFIX}/export.csv")))
    }
}

/// Sets the content type. `Headers::insert` would add a second one next to the
/// `text/plain` ehttp sets for requests with a body, and the server uses the first.
fn with_content_type(mut req: ehttp::Request, content_type: &str) -> ehttp::Request {
    req.headers.headers.retain(|(k, _)| !k.eq_ignore_ascii_case("content-type"));
    req.headers.insert("Content-Type", content_type);
    req
}

/// Successful response or a readable error (server message, HTTP status, connection error).
pub fn check(result: ehttp::Result<ehttp::Response>) -> Result<ehttp::Response, String> {
    match result {
        Err(e) => Err(tr_fmt!("connection failed: {}", e)),
        Ok(resp) if resp.ok => Ok(resp),
        Ok(resp) => Err(api::error_text(resp.status, &resp.status_text, &resp.bytes)),
    }
}

/// Turns an HTTP result into the expected JSON type or a readable error.
pub fn decode<T: DeserializeOwned>(result: ehttp::Result<ehttp::Response>) -> Result<T, String> {
    let resp = check(result)?;
    serde_json::from_slice(&resp.bytes).map_err(|e| tr_fmt!("unexpected answer: {}", e))
}

/// Like [`decode`], but `None` if the server does not know the resource (404).
pub fn decode_found<T: DeserializeOwned>(result: ehttp::Result<ehttp::Response>) -> Result<Option<T>, String> {
    match result {
        Ok(resp) if resp.status == 404 => Ok(None),
        result => decode(result).map(Some),
    }
}

/// The answer as text (CSV export).
pub fn text(result: ehttp::Result<ehttp::Response>) -> Result<String, String> {
    String::from_utf8(check(result)?.bytes).map_err(|_| tr("The server sent no text").to_string())
}

#[cfg(test)]
mod tests;
