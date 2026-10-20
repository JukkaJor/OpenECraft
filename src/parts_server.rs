//! Client for the global parts server (`openecraft-server`). Requests run in the
//! background (thread on desktop, fetch in the browser); results are collected with
//! [`ServerClient::poll`].

use crate::parts::{Part, PartsDb};
use openecraft_parts::api::{self, ApiError, ImportResult, PartList, ServerInfo, API_PREFIX, DEFAULT_PORT};
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

enum Reply {
    Info(Result<ServerInfo, String>),
    Parts(Result<PartList, String>),
    Uploaded(Result<ImportResult, String>),
}

pub struct ServerClient {
    pub url: String,
    /// Access token for uploads (kept in memory only).
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
    match result {
        Err(e) => Err(format!("connection failed: {e}")),
        Ok(resp) if resp.ok => serde_json::from_slice(&resp.bytes).map_err(|e| format!("unexpected answer: {e}")),
        Ok(resp) => Err(serde_json::from_slice::<ApiError>(&resp.bytes)
            .map(|e| e.error)
            .unwrap_or_else(|_| format!("{} {}", resp.status, resp.status_text))),
    }
}

/// Adds / replaces downloaded parts in the local database. Returns (added, updated).
pub fn merge(db: &mut PartsDb, parts: Vec<Part>) -> (usize, usize) {
    let (mut added, mut updated) = (0, 0);
    for part in parts {
        match db.parts.iter_mut().find(|p| p.number == part.number) {
            Some(existing) if *existing != part => {
                *existing = part;
                updated += 1;
            }
            Some(_) => {}
            None => {
                db.parts.push(part);
                added += 1;
            }
        }
    }
    db.sort();
    (added, updated)
}

impl ServerClient {
    pub fn busy(&self) -> bool {
        self.pending > 0
    }

    fn send<T: DeserializeOwned + 'static>(
        &mut self,
        ctx: &egui::Context,
        request: ehttp::Request,
        wrap: impl 'static + Send + FnOnce(Result<T, String>) -> Reply,
    ) {
        self.pending += 1;
        let inbox = self.inbox.clone();
        let ctx = ctx.clone();
        ehttp::fetch(request, move |result| {
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(wrap(decode(result)));
            }
            ctx.request_repaint();
        });
    }

    /// GET /api/v1/info
    pub fn check(&mut self, ctx: &egui::Context) {
        self.status = "Connecting…".into();
        let req = ehttp::Request::get(api::url(&self.url, &format!("{API_PREFIX}/info")));
        self.send(ctx, req, Reply::Info);
    }

    /// GET /api/v1/parts (all parts)
    pub fn download(&mut self, ctx: &egui::Context) {
        self.status = "Downloading parts…".into();
        let req = ehttp::Request::get(api::url(&self.url, &format!("{API_PREFIX}/parts")));
        self.send(ctx, req, Reply::Parts);
    }

    /// POST /api/v1/import with the parts as JSON (needs the access token).
    pub fn upload(&mut self, ctx: &egui::Context, parts: &[Part]) {
        let body = serde_json::to_vec(parts).unwrap_or_default();
        let mut req = ehttp::Request::post(api::url(&self.url, &format!("{API_PREFIX}/import")), body);
        req.headers.insert("Content-Type", "application/json");
        if !self.token.trim().is_empty() {
            req.headers.insert("Authorization", format!("Bearer {}", self.token.trim()));
        }
        self.status = format!("Uploading {} parts…", parts.len());
        self.send(ctx, req, Reply::Uploaded);
    }

    /// Applies finished requests. Returns a message for the status bar.
    pub fn poll(&mut self, db: &mut PartsDb) -> Option<String> {
        let replies: Vec<Reply> = self.inbox.lock().map(|mut i| std::mem::take(&mut *i)).unwrap_or_default();
        let mut message = None;
        for reply in replies {
            self.pending = self.pending.saturating_sub(1);
            let text = match reply {
                Reply::Info(Ok(info)) => {
                    let text = format!(
                        "Connected: {} {} – {} parts, revision {}, {}",
                        info.name,
                        info.version,
                        info.parts,
                        info.revision,
                        if info.writable { "uploads with token" } else { "read-only" }
                    );
                    self.info = Some(info);
                    text
                }
                Reply::Parts(Ok(list)) => {
                    let n = list.parts.len();
                    let (added, updated) = merge(db, list.parts);
                    format!("Downloaded {n} parts (revision {}): {added} new, {updated} updated", list.revision)
                }
                Reply::Uploaded(Ok(r)) => format!("Uploaded {} parts, server revision {}", r.imported, r.revision),
                Reply::Info(Err(e)) => {
                    self.info = None;
                    format!("Parts server: {e}")
                }
                Reply::Parts(Err(e)) | Reply::Uploaded(Err(e)) => format!("Parts server: {e}"),
            };
            self.status = text.clone();
            message = Some(text);
        }
        message
    }
}

#[cfg(test)]
mod tests;
