//! Test helpers: an in-memory parts server and sessions / files for the tool tests.

use crate::parts_server::{Server, Transport};
use crate::session::Session;
use openecraft_client::{ehttp, Endpoint, PortalKind};
use openecraft_parts::access::{Policy, Settings};
use openecraft_parts::api::{ApiError, ImportResult, Login, PartQuery, ServerInfo, Session as ApiSession, UserUpdate, API_PREFIX};
use openecraft_parts::portal::{Attachment, AttachmentKind, Catalog, ChangeKind, HistoryEntry, PartsRight, Role, User};
use openecraft_parts::{import, to_csv, Part};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub const URL: &str = "http://parts.test";
pub const TOKEN: &str = "secret";

/// Parts server with the routes of `openecraft-parts-server`, holding its data in
/// memory. The user `ann` (password `pw`) signs in with the token [`TOKEN`].
#[derive(Clone, Default)]
pub struct FakeServer {
    pub parts: Rc<RefCell<Vec<Part>>>,
    pub catalogs: Rc<RefCell<Vec<Catalog>>>,
    pub users: Rc<RefCell<Vec<User>>>,
    pub favorites: Rc<RefCell<Vec<String>>>,
    pub files: Rc<RefCell<Vec<FakeFile>>>,
    pub policy: Rc<RefCell<Policy<PartsRight>>>,
}

/// Attachment file: id, name and content.
pub type FakeFile = (String, String, Vec<u8>);

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
        let write = !matches!(req.method, ehttp::Method::GET) && path != "/login" && path != "/search";
        if write && !authorized {
            return Ok(error(401, "invalid token"));
        }
        if path == "/login" {
            let login: Login = serde_json::from_slice(&req.body).map_err(|e| e.to_string())?;
            return Ok(if login.name == "ann" && login.password == "pw" {
                json(&ApiSession { token: TOKEN.into(), user: User { name: "ann".into(), role: Role::Editor, manufacturer: String::new(), ..Default::default() }, ..Default::default() })
            } else {
                error(401, "wrong user name or password")
            });
        }
        if let Some(rest) = path.strip_prefix("/parts/").filter(|r| r.contains('/')) {
            return Ok(self.part_route(rest, query, req));
        }
        let segments: Vec<String> = path.split('/').skip(1).map(decode).collect();
        match (segments.iter().map(String::as_str).collect::<Vec<_>>().as_slice(), &req.method) {
            (["search"], _) => {
                let q: PartQuery = serde_json::from_slice(&req.body).map_err(|e| e.to_string())?;
                return Ok(json(&q.search(&self.parts.borrow(), 1)));
            }
            (["catalogs"], _) => return Ok(json(&*self.catalogs.borrow())),
            (["catalogs", id], ehttp::Method::PUT) => {
                let c: Catalog = serde_json::from_slice(&req.body).map_err(|e| e.to_string())?;
                self.catalogs.borrow_mut().retain(|x| x.id != *id);
                self.catalogs.borrow_mut().push(c.clone());
                return Ok(json(&c));
            }
            (["catalogs", id], ehttp::Method::DELETE) => {
                self.catalogs.borrow_mut().retain(|x| x.id != *id);
                return Ok(response(204, Vec::new()));
            }
            (["favorites"], ehttp::Method::GET) if !authorized => return Ok(error(401, "invalid token")),
            (["favorites"], ehttp::Method::GET) => return Ok(json(&*self.favorites.borrow())),
            (["favorites"], _) => {
                *self.favorites.borrow_mut() = serde_json::from_slice(&req.body).map_err(|e| e.to_string())?;
                return Ok(json(&*self.favorites.borrow()));
            }
            (["users"], _) => return Ok(json(&*self.users.borrow())),
            (["users", name], ehttp::Method::PUT) => {
                let u: UserUpdate = serde_json::from_slice(&req.body).map_err(|e| e.to_string())?;
                let user = User { name: name.to_string(), role: u.role, manufacturer: u.manufacturer, ..Default::default() };
                self.users.borrow_mut().retain(|x| x.name != *name);
                self.users.borrow_mut().push(user.clone());
                return Ok(json(&user));
            }
            (["users", name], ehttp::Method::DELETE) => {
                self.users.borrow_mut().retain(|x| x.name != *name);
                return Ok(response(204, Vec::new()));
            }
            (["access"], method) => {
                if !authorized {
                    return Ok(error(401, "invalid token"));
                }
                if *method == ehttp::Method::PUT {
                    let policy: Policy<PartsRight> = serde_json::from_slice(&req.body).map_err(|e| e.to_string())?;
                    if let Err(e) = policy.validate() {
                        return Ok(error(400, &e));
                    }
                    *self.policy.borrow_mut() = policy;
                }
                return Ok(json(&Settings { policy: self.policy.borrow().clone(), providers: Vec::new() }));
            }
            (["files", id], _) => {
                let files = self.files.borrow();
                return Ok(match files.iter().find(|f| f.0 == *id) {
                    Some(f) => response(200, f.2.clone()),
                    None => error(404, "file not found"),
                });
            }
            _ => {}
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

impl FakeServer {
    /// `/parts/{number}/history[/{revision}]` and `/parts/{number}/attachments[/{id}]`.
    fn part_route(&self, rest: &str, query: &str, req: &ehttp::Request) -> ehttp::Response {
        let segments: Vec<String> = rest.split('/').map(decode).collect();
        let mut parts = self.parts.borrow_mut();
        let Some(part) = parts.iter_mut().find(|p| p.number == segments[0]) else { return error(404, "part not found") };
        match (segments[1].as_str(), segments.get(2)) {
            ("history", None) => json(&vec![HistoryEntry { revision: 1, time: 0, user: "ann".into(), change: ChangeKind::Created }]),
            ("history", Some(rev)) if rev == "1" => json(&*part),
            ("history", Some(_)) => error(404, "no such version"),
            ("attachments", None) => {
                let param = |key: &str| query.split('&').filter_map(|kv| kv.split_once('=')).find(|(k, _)| *k == key).map(|(_, v)| decode(v));
                let name = param("name").unwrap_or_default();
                let kind = param("kind").and_then(|k| AttachmentKind::ALL.into_iter().find(|x| format!("{x:?}") == k)).unwrap_or_else(|| AttachmentKind::guess(&name));
                let id = format!("f{}", self.files.borrow().len() + 1);
                part.attachments.push(Attachment { id: id.clone(), name: name.clone(), kind, size: req.body.len() as u64 });
                self.files.borrow_mut().push((id, name, req.body.clone()));
                json(&*part)
            }
            ("attachments", Some(id)) => {
                part.attachments.retain(|a| a.id != *id);
                json(&*part)
            }
            _ => error(404, "unknown path"),
        }
    }
}

pub fn part(number: &str, group: &str) -> Part {
    Part { number: number.into(), manufacturer: "ACME".into(), group: group.into(), ..Default::default() }
}

/// Session connected to `server` (with the write token).
pub fn session_with(server: &FakeServer) -> Session {
    Session::new(Server { endpoint: Endpoint { url: URL.into(), token: TOKEN.into() }, kind: PortalKind::OpenECraft, transport: Box::new(server.clone()) })
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

/// Runs a tool or a command of a plugin with JSON arguments.
pub fn run(s: &mut Session, tool: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    let entry = crate::tools::lookup(s, tool).unwrap_or_else(|| panic!("no tool or command {tool}"));
    crate::tools::call(s, entry, args.as_object().unwrap())
}

/// Session with a new, empty project.
pub fn new_session() -> Session {
    let mut s = session();
    run(&mut s, "new_project", serde_json::json!({ "name": "Test" })).unwrap();
    s
}
