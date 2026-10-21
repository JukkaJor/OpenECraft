//! Blocking client for the parts server (`openecraft-server`). The HTTP transport is
//! a trait so that the tools can be tested against an in-memory server.

use crate::args::Options;
use openecraft_i18n::{tr, tr_fmt};
use openecraft_parts::api::{self, ImportResult, PartList, PartQuery, ServerInfo, API_PREFIX, DEFAULT_PORT};
use openecraft_parts::Part;
use serde::de::DeserializeOwned;

pub trait Transport {
    fn fetch(&self, request: &ehttp::Request) -> Result<ehttp::Response, String>;
}

pub struct Http;

impl Transport for Http {
    fn fetch(&self, request: &ehttp::Request) -> Result<ehttp::Response, String> {
        ehttp::fetch_blocking(request)
    }
}

/// Sets the content type. `Headers::insert` would add a second one next to the
/// `text/plain` ehttp sets for requests with a body, and the server uses the first.
fn with_content_type(mut req: ehttp::Request, content_type: &str) -> ehttp::Request {
    req.headers.headers.retain(|(k, _)| !k.eq_ignore_ascii_case("content-type"));
    req.headers.insert("Content-Type", content_type);
    req
}

pub struct Server {
    pub url: String,
    pub token: String,
    pub transport: Box<dyn Transport>,
}

impl Server {
    /// Server from `--server` / `--token`, otherwise from the environment
    /// (`OPENECRAFT_SERVER`, `OPENECRAFT_TOKEN`), otherwise the local default.
    pub fn from_options(o: &Options) -> Server {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        Server {
            url: o.server.clone().or_else(|| env("OPENECRAFT_SERVER")).unwrap_or_else(|| format!("http://127.0.0.1:{DEFAULT_PORT}")),
            token: o.token.clone().or_else(|| env("OPENECRAFT_TOKEN")).unwrap_or_default(),
            transport: Box::new(Http),
        }
    }

    fn endpoint(&self, path: &str) -> String {
        api::url(&self.url, path)
    }

    fn fetch(&self, mut req: ehttp::Request) -> Result<ehttp::Response, String> {
        if !self.token.trim().is_empty() {
            req.headers.insert("Authorization", format!("Bearer {}", self.token.trim()));
        }
        self.transport.fetch(&req).map_err(|e| tr_fmt!("connection failed: {}", e))
    }

    fn send(&self, req: ehttp::Request) -> Result<ehttp::Response, String> {
        let resp = self.fetch(req)?;
        if resp.ok {
            Ok(resp)
        } else {
            Err(api::error_text(resp.status, &resp.status_text, &resp.bytes))
        }
    }

    fn decode<T: DeserializeOwned>(resp: ehttp::Response) -> Result<T, String> {
        serde_json::from_slice(&resp.bytes).map_err(|e| tr_fmt!("unexpected answer: {}", e))
    }

    pub fn info(&self) -> Result<ServerInfo, String> {
        Self::decode(self.send(ehttp::Request::get(self.endpoint(&format!("{API_PREFIX}/info"))))?)
    }

    pub fn list(&self, query: &PartQuery) -> Result<PartList, String> {
        Self::decode(self.send(ehttp::Request::get(self.endpoint(&format!("{API_PREFIX}/parts{}", query.to_query_string()))))?)
    }

    /// The part, `None` if the server does not know it.
    pub fn get(&self, number: &str) -> Result<Option<Part>, String> {
        let resp = self.fetch(ehttp::Request::get(self.endpoint(&api::part_path(number))))?;
        match resp.status {
            404 => Ok(None),
            _ if resp.ok => Self::decode(resp).map(Some),
            _ => Err(api::error_text(resp.status, &resp.status_text, &resp.bytes)),
        }
    }

    pub fn put(&self, part: &Part) -> Result<(), String> {
        let body = serde_json::to_vec(part).unwrap_or_default();
        self.send(with_content_type(ehttp::Request::put(self.endpoint(&api::part_path(&part.number)), body), "application/json")).map(|_| ())
    }

    pub fn delete(&self, number: &str) -> Result<(), String> {
        self.send(ehttp::Request::delete(&self.endpoint(&api::part_path(number)))).map(|_| ())
    }

    /// Uploads the content of a CSV or JSON parts file.
    pub fn import(&self, content: Vec<u8>) -> Result<ImportResult, String> {
        Self::decode(self.send(ehttp::Request::post(self.endpoint(&format!("{API_PREFIX}/import")), content))?)
    }

    /// All parts as CSV.
    pub fn export(&self) -> Result<String, String> {
        let resp = self.send(ehttp::Request::get(self.endpoint(&format!("{API_PREFIX}/export.csv"))))?;
        String::from_utf8(resp.bytes).map_err(|_| tr("The server sent no text").to_string())
    }
}
