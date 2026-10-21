//! Client for the global parts server (`openecraft-server`). Requests run in the
//! background (thread on desktop, fetch in the browser); results are collected with
//! [`ServerClient::poll`]. The requests themselves are built by `openecraft_client`.

use openecraft_i18n::{tr, tr_fmt};
use openecraft_client::{check, decode, ehttp, Endpoint};
use openecraft_parts::api::{ImportResult, PartList, PartQuery, ServerInfo};
use openecraft_parts::Part;
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
    /// The last finished request failed (`status` shows why).
    failed: bool,
    pending: usize,
    inbox: Arc<Mutex<Vec<Reply>>>,
}

impl Default for ServerClient {
    fn default() -> Self {
        let Endpoint { url, token } = Endpoint::default();
        ServerClient { url, token, info: None, status: String::new(), failed: false, pending: 0, inbox: Arc::default() }
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

    fn endpoint(&self) -> Endpoint {
        Endpoint { url: self.url.clone(), token: self.token.clone() }
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

    /// Checks the connection (server info).
    pub fn check(&mut self, ctx: &egui::Context) {
        let req = self.endpoint().info();
        self.send(ctx, req, tr("Connecting…").into(), |r| Reply::Info(decode(r)));
    }

    /// Loads all parts.
    pub fn download(&mut self, ctx: &egui::Context) {
        let req = self.endpoint().list(&PartQuery::default());
        self.send(ctx, req, tr("Downloading parts…").into(), |r| Reply::Parts(decode(r)));
    }

    /// Uploads parts as JSON (needs the access token).
    pub fn upload(&mut self, ctx: &egui::Context, parts: &[Part]) {
        let req = self.endpoint().import(serde_json::to_vec(parts).unwrap_or_default(), "application/json");
        self.send(ctx, req, tr_fmt!("Uploading {} parts…", parts.len()), |r| Reply::Imported(decode(r)));
    }

    /// Uploads the content of a CSV or JSON parts file.
    pub fn upload_file(&mut self, ctx: &egui::Context, content: Vec<u8>) {
        let req = self.endpoint().import(content, "text/plain; charset=utf-8");
        self.send(ctx, req, tr("Uploading parts file…").into(), |r| Reply::Imported(decode(r)));
    }

    /// Creates or replaces a part.
    pub fn save(&mut self, ctx: &egui::Context, part: &Part) {
        let req = self.endpoint().save(part);
        self.send(ctx, req, tr_fmt!("Saving {}…", part.number), |r| Reply::Saved(decode(r).map(Box::new)));
    }

    pub fn delete(&mut self, ctx: &egui::Context, number: &str) {
        let req = self.endpoint().delete(number);
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
            self.failed = reply.error().is_some();
        }
        replies
    }

    /// True if the status line shows an error.
    pub fn failed(&self) -> bool {
        self.failed
    }
}

#[cfg(test)]
mod tests;
