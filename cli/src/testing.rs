//! Test helpers: an in-memory parts server and sessions / files for the tool tests.

use crate::parts_server::{Server, Transport};
use crate::session::Session;
use openecraft_client::{ehttp, Endpoint};
use openecraft_parts::api::{ApiError, ImportResult, PartQuery, ServerInfo, API_PREFIX};
use openecraft_parts::{import, to_csv, Part};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub const URL: &str = "http://parts.test";
pub const TOKEN: &str = "secret";

/// Parts server with the routes of `openecraft-parts-server`, holding its parts in memory.
#[derive(Clone, Default)]
pub struct FakeServer {
    pub parts: Rc<RefCell<Vec<Part>>>,
}

fn response(status: u16, bytes: Vec<u8>) -> ehttp::Response {
    ehttp::Response {
        url: String::new(),
        ok: (200..300).contains(&status),
        status,
        status_text: String::new(),
        headers: Default::default(),
        bytes,
    }
}

fn json(value: &impl serde::Serialize) -> ehttp::Response {
    response(200, serde_json::to_vec(value).unwrap())
}

fn error(status: u16, text: &str) -> ehttp::Response {
    response(status, serde_json::to_vec(&ApiError { error: text.into() }).unwrap())
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap()
}

impl Transport for FakeServer {
    fn fetch(&self, req: &ehttp::Request) -> Result<ehttp::Response, String> {
        let rest = req.url.strip_prefix(URL).ok_or("host not found")?;
        let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
        let path = path.strip_prefix(API_PREFIX).ok_or("no API path")?;
        let authorized = req.headers.get("Authorization") == Some(format!("Bearer {TOKEN}").as_str());
        let write = !matches!(req.method, ehttp::Method::GET);
        if write && !authorized {
            return Ok(error(401, "invalid token"));
        }
        let mut parts = self.parts.borrow_mut();
        Ok(match (path, &req.method) {
            ("/info", _) => json(&ServerInfo { name: "fake".into(), parts: parts.len(), writable: true, ..Default::default() }),
            ("/parts", _) => {
                let mut q = PartQuery::default();
                for (k, v) in query.split('&').filter_map(|kv| kv.split_once('=')) {
                    match k {
                        "q" => q.q = Some(decode(v)),
                        "group" => q.group = Some(decode(v)),
                        "manufacturer" => q.manufacturer = Some(decode(v)),
                        "offset" => q.offset = v.parse().ok(),
                        "limit" => q.limit = v.parse().ok(),
                        _ => {}
                    }
                }
                let (total, page) = q.apply(&parts);
                json(&openecraft_parts::api::PartList { total, offset: q.offset.unwrap_or(0), parts: page, revision: 1 })
            }
            ("/export.csv", _) => response(200, to_csv(&parts).into_bytes()),
            ("/import", _) => match import(std::str::from_utf8(&req.body).unwrap()) {
                Ok(new) => {
                    let imported = new.len();
                    for p in new {
                        parts.retain(|x| x.number != p.number);
                        parts.push(p);
                    }
                    json(&ImportResult { imported, revision: 2 })
                }
                Err(e) => error(400, &e),
            },
            (p, method) => {
                let number = decode(p.strip_prefix("/parts/").ok_or("unknown path")?);
                let at = parts.iter().position(|x| x.number == number);
                match (method, at) {
                    (ehttp::Method::GET, Some(i)) => json(&parts[i]),
                    // Like axum's JSON extractor: the first content type counts.
                    (ehttp::Method::PUT, _) if req.headers.get("Content-Type") != Some("application/json") => {
                        error(415, "expected application/json")
                    }
                    (ehttp::Method::PUT, _) => {
                        let part: Part = serde_json::from_slice(&req.body).map_err(|e| e.to_string())?;
                        parts.retain(|x| x.number != number);
                        parts.push(part);
                        response(200, Vec::new())
                    }
                    (ehttp::Method::DELETE, Some(i)) => {
                        parts.remove(i);
                        response(204, Vec::new())
                    }
                    _ => error(404, "part not found"),
                }
            }
        })
    }
}

pub fn part(number: &str, group: &str) -> Part {
    Part { number: number.into(), manufacturer: "ACME".into(), group: group.into(), ..Default::default() }
}

/// Session connected to `server` (with the write token).
pub fn session_with(server: &FakeServer) -> Session {
    Session::new(Server { endpoint: Endpoint { url: URL.into(), token: TOKEN.into() }, transport: Box::new(server.clone()) })
}

pub fn session() -> Session {
    session_with(&FakeServer::default())
}

/// Empty directory for the files of one test.
pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("openecraft-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs a tool with JSON arguments.
pub fn run(s: &mut Session, tool: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    crate::tools::call(s, crate::tools::find(tool).unwrap(), args.as_object().unwrap())
}

/// Session with a new, empty project.
pub fn new_session() -> Session {
    let mut s = session();
    run(&mut s, "new_project", serde_json::json!({ "name": "Test" })).unwrap();
    s
}
