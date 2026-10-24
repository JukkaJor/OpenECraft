//! Client for the global parts server (`openecraft-parts-server`, the data portal).
//! Requests run in the background (thread on desktop, fetch in the browser); results
//! are collected with [`ServerClient::poll`]. The requests themselves are built by
//! `openecraft_client`.

use openecraft_i18n::{tr, tr_fmt};
use openecraft_client::{check, decode, ehttp, Endpoint};
use openecraft_parts::api::{ImportResult, PartList, PartQuery, ServerInfo, Session, UserUpdate};
use openecraft_parts::portal::{AttachmentKind, Catalog, HistoryEntry, SearchResult, User};
use openecraft_parts::Part;
use std::sync::{Arc, Mutex};

/// Result of a finished request.
#[derive(Debug)]
pub enum Reply {
    Info(Result<ServerInfo, String>),
    Parts(Result<PartList, String>),
    Imported(Result<ImportResult, String>),
    /// A part was saved, or a file attached to / removed from it.
    Saved(Result<Box<Part>, String>),
    /// Part number and result.
    Deleted(String, Result<(), String>),
    Search(Result<SearchResult, String>),
    /// Server versions of local parts (update check).
    Versions(Result<SearchResult, String>),
    /// Part number and its changes.
    History(String, Result<Vec<HistoryEntry>, String>),
    Catalogs(Result<Vec<Catalog>, String>),
    CatalogSaved(Result<Catalog, String>),
    CatalogDeleted(String, Result<(), String>),
    SignedIn(Result<Session, String>),
    SignedOut,
    Favorites(Result<Vec<String>, String>),
    Users(Result<Vec<User>, String>),
    UserSaved(Result<User, String>),
    UserDeleted(String, Result<(), String>),
    /// File name, purpose given with the request and the content.
    File(String, FileUse, Result<Vec<u8>, String>),
}

/// What a downloaded file is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileUse {
    /// Save it as a file.
    Save,
    /// Merge it into the macro library.
    Macros,
}

impl Reply {
    pub fn error(&self) -> Option<&str> {
        match self {
            Reply::Info(Err(e))
            | Reply::Parts(Err(e))
            | Reply::Imported(Err(e))
            | Reply::Saved(Err(e))
            | Reply::Deleted(_, Err(e))
            | Reply::Search(Err(e))
            | Reply::Versions(Err(e))
            | Reply::History(_, Err(e))
            | Reply::Catalogs(Err(e))
            | Reply::CatalogSaved(Err(e))
            | Reply::CatalogDeleted(_, Err(e))
            | Reply::SignedIn(Err(e))
            | Reply::Favorites(Err(e))
            | Reply::Users(Err(e))
            | Reply::UserSaved(Err(e))
            | Reply::UserDeleted(_, Err(e))
            | Reply::File(_, _, Err(e)) => Some(e),
            _ => None,
        }
    }

    /// Text for a status line.
    pub fn describe(&self) -> String {
        if let Some(e) = self.error() {
            return tr_fmt!("Parts server: {}", e);
        }
        match self {
            Reply::Info(Ok(info)) => {
                let access = match &info.user {
                    Some(u) => tr_fmt!("signed in as {} ({})", u.name, tr(u.role.title())),
                    None if info.writable => tr("changes with token").to_string(),
                    None => tr("read-only").to_string(),
                };
                tr_fmt!("Connected: {} {} – {} parts, revision {}, {}", info.name, info.version, info.parts, info.revision, access)
            }
            Reply::Parts(Ok(list)) => tr_fmt!("Loaded {} parts (revision {})", list.parts.len(), list.revision),
            Reply::Imported(Ok(r)) => tr_fmt!("Uploaded {} parts, server revision {}", r.imported, r.revision),
            Reply::Saved(Ok(p)) => tr_fmt!("Saved part {}", p.number),
            Reply::Deleted(number, Ok(())) => tr_fmt!("Deleted part {}", number),
            Reply::Search(Ok(r)) => tr_fmt!("{} parts found", r.total),
            Reply::Versions(Ok(r)) => tr_fmt!("Compared with {} parts of the server", r.total),
            Reply::History(number, Ok(h)) => tr_fmt!("{} changes of part {}", h.len(), number),
            Reply::Catalogs(Ok(c)) => tr_fmt!("{} catalogs", c.len()),
            Reply::CatalogSaved(Ok(c)) => tr_fmt!("Saved catalog {}", c.name),
            Reply::CatalogDeleted(id, Ok(())) => tr_fmt!("Deleted catalog {}", id),
            Reply::SignedIn(Ok(s)) => tr_fmt!("Signed in as {} ({})", s.user.name, tr(s.user.role.title())),
            Reply::SignedOut => tr("Signed out").to_string(),
            Reply::Favorites(Ok(f)) => tr_fmt!("{} favorites", f.len()),
            Reply::Users(Ok(u)) => tr_fmt!("{} users", u.len()),
            Reply::UserSaved(Ok(u)) => tr_fmt!("Saved user {}", u.name),
            Reply::UserDeleted(name, Ok(())) => tr_fmt!("Deleted user {}", name),
            Reply::File(name, _, Ok(data)) => tr_fmt!("Downloaded {} ({} bytes)", name, data.len()),
            _ => String::new(),
        }
    }
}

pub struct ServerClient {
    pub url: String,
    /// Access token for changes: the administrator token or the token of a session
    /// (kept in memory only).
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

    /// The signed in user (as of the last connection check or sign-in).
    pub fn user(&self) -> Option<&User> {
        self.info.as_ref().and_then(|i| i.user.as_ref())
    }

    pub fn endpoint(&self) -> Endpoint {
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

    /// Loads the parts with these numbers (e.g. to add them to the project).
    pub fn fetch(&mut self, ctx: &egui::Context, numbers: &[String]) {
        let query = PartQuery { numbers: numbers.to_vec(), ..Default::default() };
        let req = self.endpoint().search(&query);
        self.send(ctx, req, tr("Downloading parts…").into(), |r| {
            Reply::Parts(decode::<SearchResult>(r).map(|s| PartList { revision: s.revision, total: s.total, offset: s.offset, parts: s.parts }))
        });
    }

    /// Faceted search (part lists without models).
    pub fn search(&mut self, ctx: &egui::Context, query: &PartQuery) {
        let req = self.endpoint().search(&PartQuery { brief: true, ..query.clone() });
        self.send(ctx, req, tr("Searching…").into(), |r| Reply::Search(decode(r)));
    }

    /// Loads the server versions of local parts for an update check.
    pub fn versions(&mut self, ctx: &egui::Context, numbers: &[String]) {
        let req = self.endpoint().search(&PartQuery { numbers: numbers.to_vec(), ..Default::default() });
        self.send(ctx, req, tr("Checking for part updates…").into(), |r| Reply::Versions(decode(r)));
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

    pub fn history(&mut self, ctx: &egui::Context, number: &str) {
        let req = self.endpoint().history(number);
        let n = number.to_string();
        self.send(ctx, req, tr("Loading the history…").into(), move |r| Reply::History(n, decode(r)));
    }

    /// Stores a file with a part.
    pub fn attach(&mut self, ctx: &egui::Context, number: &str, name: &str, kind: Option<AttachmentKind>, data: Vec<u8>) {
        let req = self.endpoint().attach(number, name, kind, data);
        self.send(ctx, req, tr_fmt!("Uploading {}…", name), |r| Reply::Saved(decode(r).map(Box::new)));
    }

    pub fn detach(&mut self, ctx: &egui::Context, number: &str, id: &str) {
        let req = self.endpoint().detach(number, id);
        self.send(ctx, req, tr("Removing the file…").into(), |r| Reply::Saved(decode(r).map(Box::new)));
    }

    pub fn download_file(&mut self, ctx: &egui::Context, id: &str, name: &str, purpose: FileUse) {
        let req = self.endpoint().file(id);
        let n = name.to_string();
        self.send(ctx, req, tr_fmt!("Downloading {}…", name), move |r| Reply::File(n, purpose, check(r).map(|resp| resp.bytes)));
    }

    pub fn catalogs(&mut self, ctx: &egui::Context) {
        let req = self.endpoint().catalogs();
        self.send(ctx, req, tr("Loading catalogs…").into(), |r| Reply::Catalogs(decode(r)));
    }

    pub fn save_catalog(&mut self, ctx: &egui::Context, catalog: &Catalog) {
        let req = self.endpoint().save_catalog(catalog);
        self.send(ctx, req, tr_fmt!("Saving {}…", catalog.name), |r| Reply::CatalogSaved(decode(r)));
    }

    pub fn delete_catalog(&mut self, ctx: &egui::Context, id: &str) {
        let req = self.endpoint().delete_catalog(id);
        let id = id.to_string();
        self.send(ctx, req, tr_fmt!("Deleting {}…", id), move |r| Reply::CatalogDeleted(id, check(r).map(|_| ())));
    }

    /// Signs in; the session's token replaces [`ServerClient::token`].
    pub fn sign_in(&mut self, ctx: &egui::Context, name: &str, password: &str) {
        let req = self.endpoint().login(name, password);
        self.send(ctx, req, tr("Signing in…").into(), |r| Reply::SignedIn(decode(r)));
    }

    pub fn sign_out(&mut self, ctx: &egui::Context) {
        let req = self.endpoint().logout();
        self.send(ctx, req, tr("Signing out…").into(), |_| Reply::SignedOut);
    }

    pub fn favorites(&mut self, ctx: &egui::Context) {
        let req = self.endpoint().favorites();
        self.send(ctx, req, tr("Loading favorites…").into(), |r| Reply::Favorites(decode(r)));
    }

    pub fn set_favorites(&mut self, ctx: &egui::Context, numbers: &[String]) {
        let req = self.endpoint().set_favorites(numbers);
        self.send(ctx, req, tr("Saving favorites…").into(), |r| Reply::Favorites(decode(r)));
    }

    pub fn users(&mut self, ctx: &egui::Context) {
        let req = self.endpoint().users();
        self.send(ctx, req, tr("Loading users…").into(), |r| Reply::Users(decode(r)));
    }

    pub fn save_user(&mut self, ctx: &egui::Context, name: &str, update: &UserUpdate) {
        let req = self.endpoint().save_user(name, update);
        self.send(ctx, req, tr_fmt!("Saving {}…", name), |r| Reply::UserSaved(decode(r)));
    }

    pub fn delete_user(&mut self, ctx: &egui::Context, name: &str) {
        let req = self.endpoint().delete_user(name);
        let n = name.to_string();
        self.send(ctx, req, tr_fmt!("Deleting {}…", name), move |r| Reply::UserDeleted(n, check(r).map(|_| ())));
    }

    /// Takes the finished requests and updates `status`, `info` and the token.
    pub fn poll(&mut self) -> Vec<Reply> {
        let replies: Vec<Reply> = self.inbox.lock().map(|mut i| std::mem::take(&mut *i)).unwrap_or_default();
        for reply in &replies {
            self.pending = self.pending.saturating_sub(1);
            match reply {
                Reply::Info(Ok(info)) => self.info = Some(info.clone()),
                Reply::Info(Err(_)) => self.info = None,
                Reply::SignedIn(Ok(session)) => {
                    self.token = session.token.clone();
                    if let Some(info) = &mut self.info {
                        info.user = Some(session.user.clone());
                    }
                }
                Reply::SignedOut => {
                    self.token.clear();
                    if let Some(info) = &mut self.info {
                        info.user = None;
                    }
                }
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
