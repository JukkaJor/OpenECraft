//! Client for the global parts server (`openecraft-server`). Requests run in the
//! background (thread on desktop, fetch in the browser); results are collected with
//! [`ServerClient::poll`].

use openecraft_i18n::{tr, tr_fmt};
use openecraft_parts::api::{self, ApiError, ImportResult, PartList, ServerInfo, API_PREFIX, DEFAULT_PORT};
use openecraft_parts::Part;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

/// Result of a finished request.
#[derive(Debug)]
pub enum Reply {
    Info(Result<ServerInfo, String>),
    Parts(Result<PartList, String>),
    Imported(Result<ImportResult, String>),
    Saved(Result<Box<Part>, String>),
    /// Part number and result.
    Deleted(String, Result<(), String>),
}

impl Reply {
    pub fn error(&self) -> Option<&str> {
        match self {
            Reply::Info(Err(e)) | Reply::Parts(Err(e)) | Reply::Imported(Err(e)) | Reply::Saved(Err(e)) | Reply::Deleted(_, Err(e)) => {
                Some(e)
            }
            _ => None,
        }
    }

    /// Text for a status line.
    pub fn describe(&self) -> String {
        if let Some(e) = self.error() {
            return tr_fmt!("Parts server: {}", e);
        }
        match self {
            Reply::Info(Ok(info)) => format!(
                "Connected: {} {} – {} parts, revision {}, {}",
                info.name,
                info.version,
                info.parts,
                info.revision,
                if info.writable { tr("changes with token") } else { tr("read-only") }
            ),
            Reply::Parts(Ok(list)) => tr_fmt!("Loaded {} parts (revision {})", list.parts.len(), list.revision),
            Reply::Imported(Ok(r)) => tr_fmt!("Uploaded {} parts, server revision {}", r.imported, r.revision),
            Reply::Saved(Ok(p)) => tr_fmt!("Saved part {}", p.number),
            Reply::Deleted(number, Ok(())) => tr_fmt!("Deleted part {}", number),
            _ => String::new(),
        }
    }
}

pub struct ServerClient {
    pub url: String,
    /// Access token for changes (kept in memory only).
    pub token: String,
    pub info: Option<ServerInfo>,
    pub status: String,
    pending: usize,
    inbox: Arc<Mutex<Vec<Reply>>>,
}

impl Default for ServerClient {
    fn default() -> Self {
        ServerClient {
            url: format!("http://127.0.0.1:{DEFAULT_PORT}"),
            token: String::new(),
            info: None,
            status: String::new(),
            pending: 0,
            inbox: Arc::default(),
        }
    }
}

/// Turns an HTTP result into the expected JSON type or a readable error.
pub fn decode<T: DeserializeOwned>(result: ehttp::Result<ehttp::Response>) -> Result<T, String> {
    let resp = check(result)?;
    serde_json::from_slice(&resp.bytes).map_err(|e| tr_fmt!("unexpected answer: {}", e))
}

/// Successful response or a readable error (server message, HTTP status, connection error).
fn check(result: ehttp::Result<ehttp::Response>) -> Result<ehttp::Response, String> {
    match result {
        Err(e) => Err(tr_fmt!("connection failed: {}", e)),
        Ok(resp) if resp.ok => Ok(resp),
        Ok(resp) => Err(serde_json::from_slice::<ApiError>(&resp.bytes)
            .map(|e| e.error)
            .unwrap_or_else(|_| format!("{} {}", resp.status, resp.status_text))),
    }
}

impl ServerClient {
    pub fn busy(&self) -> bool {
        self.pending > 0
    }

    /// True if the last connection check found a server that accepts changes.
    pub fn writable(&self) -> bool {
        self.info.as_ref().is_some_and(|i| i.writable)
    }

    fn endpoint(&self, path: &str) -> String {
        api::url(&self.url, path)
    }

    fn authorized(&self, mut req: ehttp::Request) -> ehttp::Request {
        if !self.token.trim().is_empty() {
            req.headers.insert("Authorization", format!("Bearer {}", self.token.trim()));
        }
        req
    }

    fn send(
        &mut self,
        ctx: &egui::Context,
        request: ehttp::Request,
        status: String,
        wrap: impl 'static + Send + FnOnce(ehttp::Result<ehttp::Response>) -> Reply,
    ) {
        self.status = status;
        self.pending += 1;
        let inbox = self.inbox.clone();
        let ctx = ctx.clone();
        ehttp::fetch(request, move |result| {
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(wrap(result));
            }
            ctx.request_repaint();
        });
    }

    /// GET /api/v1/info
    pub fn check(&mut self, ctx: &egui::Context) {
        let req = ehttp::Request::get(self.endpoint(&format!("{API_PREFIX}/info")));
        self.send(ctx, req, tr("Connecting…").into(), |r| Reply::Info(decode(r)));
    }

    /// GET /api/v1/parts (all parts)
    pub fn download(&mut self, ctx: &egui::Context) {
        let req = ehttp::Request::get(self.endpoint(&format!("{API_PREFIX}/parts")));
        self.send(ctx, req, tr("Downloading parts…").into(), |r| Reply::Parts(decode(r)));
    }

    /// POST /api/v1/import with the parts as JSON (needs the access token).
    pub fn upload(&mut self, ctx: &egui::Context, parts: &[Part]) {
        let body = serde_json::to_vec(parts).unwrap_or_default();
        self.import(ctx, body, "application/json", tr_fmt!("Uploading {} parts…", parts.len()));
    }

    /// POST /api/v1/import with the content of a CSV or JSON parts file.
    pub fn upload_file(&mut self, ctx: &egui::Context, content: Vec<u8>) {
        self.import(ctx, content, "text/plain; charset=utf-8", tr("Uploading parts file…").into());
    }

    fn import(&mut self, ctx: &egui::Context, body: Vec<u8>, content_type: &str, status: String) {
        let mut req = ehttp::Request::post(self.endpoint(&format!("{API_PREFIX}/import")), body);
        req.headers.insert("Content-Type", content_type);
        let req = self.authorized(req);
        self.send(ctx, req, status, |r| Reply::Imported(decode(r)));
    }

    /// PUT /api/v1/parts/{number}: creates or replaces a part.
    pub fn save(&mut self, ctx: &egui::Context, part: &Part) {
        let body = serde_json::to_vec(part).unwrap_or_default();
        let mut req = ehttp::Request::put(self.endpoint(&api::part_path(&part.number)), body);
        req.headers.insert("Content-Type", "application/json");
        let req = self.authorized(req);
        self.send(ctx, req, tr_fmt!("Saving {}…", part.number), |r| Reply::Saved(decode(r).map(Box::new)));
    }

    /// DELETE /api/v1/parts/{number}
    pub fn delete(&mut self, ctx: &egui::Context, number: &str) {
        let req = self.authorized(ehttp::Request::delete(&self.endpoint(&api::part_path(number))));
        let n = number.to_string();
        self.send(ctx, req, tr_fmt!("Deleting {}…", number), move |r| Reply::Deleted(n, check(r).map(|_| ())));
    }

    /// Takes the finished requests and updates `status` / `info`.
    pub fn poll(&mut self) -> Vec<Reply> {
        let replies: Vec<Reply> = self.inbox.lock().map(|mut i| std::mem::take(&mut *i)).unwrap_or_default();
        for reply in &replies {
            self.pending = self.pending.saturating_sub(1);
            match reply {
                Reply::Info(Ok(info)) => self.info = Some(info.clone()),
                Reply::Info(Err(_)) => self.info = None,
                _ => {}
            }
            self.status = reply.describe();
        }
        replies
    }

    /// True if the status line shows an error.
    pub fn failed(&self) -> bool {
        self.status.starts_with(tr("Parts server: {}").split("{}").next().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests;
