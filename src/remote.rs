//! Client of the project server (`openecraft-project-server`): opening and saving
//! shared projects, page locks. Requests run in the background; [`Remote::poll`]
//! collects the answers. The rules of shared editing are in `openecraft_core::collab`.

use openecraft_client::{decode, ehttp, ProjectEndpoint};
use openecraft_core::collab::{Opened, ProjectSummary, Shared, Snapshot, Status, Target};
use openecraft_core::model::Project;
use openecraft_i18n::{tr, tr_fmt};
use std::sync::{Arc, Mutex};

/// Answer of a request. Answers for a session that is no longer open are dropped.
#[derive(Debug)]
pub enum Reply {
    List(Result<Vec<ProjectSummary>, String>),
    /// Name on the server and the opened project.
    Opened(String, Result<Box<Opened>, String>),
    Saved(String, Result<Box<Snapshot>, String>),
    Snapshot(String, Result<Box<Snapshot>, String>),
    Status(String, Result<Status, String>),
    /// Requested targets and the answer.
    Locked(String, Vec<Target>, Result<Status, String>),
    Closed,
}

impl Reply {
    fn session(&self) -> Option<&str> {
        match self {
            Reply::Saved(s, _) | Reply::Snapshot(s, _) | Reply::Status(s, _) | Reply::Locked(s, _, _) => Some(s),
            Reply::List(_) | Reply::Opened(..) | Reply::Closed => None,
        }
    }
}

pub struct Remote {
    pub url: String,
    /// Access token of the server (kept in memory only).
    pub token: String,
    pub user: String,
    /// Projects on the server (last list).
    pub projects: Vec<ProjectSummary>,
    /// The open shared project.
    pub shared: Option<Shared>,
    /// Name for uploading the current project.
    pub new_name: String,
    pub status: String,
    pub window: bool,
    pending: usize,
    /// A status request is under way.
    polling: bool,
    /// A newer project is being loaded.
    fetching: bool,
    last_status: f64,
    /// History revision checked for changes without lock.
    pub checked_rev: u64,
    /// History revision of the last save request.
    pub saving_rev: u64,
    ctx: Option<egui::Context>,
    inbox: Arc<Mutex<Vec<Reply>>>,
}

impl Default for Remote {
    fn default() -> Self {
        let ProjectEndpoint { url, token } = ProjectEndpoint::default();
        let user = std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default();
        Remote {
            url,
            token,
            user,
            projects: Vec::new(),
            shared: None,
            new_name: String::new(),
            status: String::new(),
            window: false,
            pending: 0,
            polling: false,
            fetching: false,
            last_status: 0.0,
            checked_rev: 0,
            saving_rev: 0,
            ctx: None,
            inbox: Arc::default(),
        }
    }
}

impl Remote {
    fn endpoint(&self) -> ProjectEndpoint {
        ProjectEndpoint { url: self.url.clone(), token: self.token.clone() }
    }

    pub fn busy(&self) -> bool {
        self.pending > 0
    }

    fn send(&mut self, request: ehttp::Request, status: String, wrap: impl 'static + Send + FnOnce(ehttp::Result<ehttp::Response>) -> Reply) {
        self.status = status;
        self.pending += 1;
        let inbox = self.inbox.clone();
        let ctx = self.ctx.clone();
        ehttp::fetch(request, move |result| {
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(wrap(result));
            }
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
    }

    fn session_id(&self) -> String {
        self.shared.as_ref().map(|s| s.session.session.clone()).unwrap_or_default()
    }

    /// Lists the projects of the server.
    pub fn list(&mut self) {
        let req = self.endpoint().list();
        self.send(req, tr("Connecting…").into(), |r| Reply::List(decode(r)));
    }

    pub fn open(&mut self, name: &str) {
        let req = self.endpoint().open(name, self.user.trim());
        let n = name.to_string();
        self.send(req, tr_fmt!("Opening {}…", name), move |r| Reply::Opened(n, decode(r).map(Box::new)));
    }

    /// Stores `prj` as a new project on the server and opens it.
    pub fn create(&mut self, name: &str, prj: &Project) {
        let req = self.endpoint().create(name, self.user.trim(), prj);
        let n = name.to_string();
        self.send(req, tr_fmt!("Uploading {}…", name), move |r| Reply::Opened(n, decode(r).map(Box::new)));
    }

    /// Saves what this session has locked, then releases `release`.
    pub fn save(&mut self, prj: &Project, release: Vec<Target>) {
        let Some(sh) = &self.shared else { return };
        let req = self.endpoint().save(&sh.name, &sh.session, prj, release);
        let s = self.session_id();
        self.send(req, tr("Saving to the project server…").into(), move |r| Reply::Saved(s, decode(r).map(Box::new)));
    }

    pub fn lock(&mut self, targets: Vec<Target>) {
        let endpoint = self.endpoint();
        let Some(sh) = &mut self.shared else { return };
        sh.requested.extend(targets.iter().copied());
        let req = endpoint.lock(&sh.name, &sh.session, targets.clone());
        let s = self.session_id();
        self.send(req, tr("Locking…").into(), move |r| Reply::Locked(s, targets, decode(r)));
    }

    /// Releases locks without saving.
    pub fn release(&mut self, targets: Vec<Target>) {
        let Some(sh) = &self.shared else { return };
        let req = self.endpoint().release(&sh.name, &sh.session, targets);
        let s = self.session_id();
        self.send(req, tr("Releasing…").into(), move |r| Reply::Status(s, decode(r)));
    }

    /// Ends the session (releases its locks); the project stays open locally.
    pub fn close(&mut self) {
        let Some(sh) = self.shared.take() else { return };
        let req = self.endpoint().close(&sh.name, &sh.session);
        self.send(req, tr_fmt!("Closed {} on the project server", sh.name), |_| Reply::Closed);
    }

    fn fetch(&mut self) {
        let Some(sh) = &self.shared else { return };
        if self.fetching {
            return;
        }
        self.fetching = true;
        let req = self.endpoint().snapshot(&sh.name);
        let s = self.session_id();
        self.send(req, tr("Loading changes…").into(), move |r| Reply::Snapshot(s, decode(r).map(Box::new)));
    }

    /// Takes the finished requests (of the open session) and keeps the session alive.
    pub fn poll(&mut self, ctx: &egui::Context) -> Vec<Reply> {
        self.ctx = Some(ctx.clone());
        let replies: Vec<Reply> = self.inbox.lock().map(|mut i| std::mem::take(&mut *i)).unwrap_or_default();
        let current = self.session_id();
        let mut out = Vec::new();
        for reply in replies {
            self.pending = self.pending.saturating_sub(1);
            match &reply {
                Reply::Status(..) => self.polling = false,
                Reply::Snapshot(..) => self.fetching = false,
                _ => {}
            }
            if reply.session().is_some_and(|s| s != current) {
                continue;
            }
            self.status = match &reply {
                Reply::List(Ok(list)) => tr_fmt!("{} projects on the server", list.len()),
                Reply::Opened(name, Ok(_)) => tr_fmt!("Opened {} from the project server", name),
                Reply::Saved(_, Ok(s)) => tr_fmt!("Saved to the project server (revision {})", s.revision),
                Reply::List(Err(e))
                | Reply::Opened(_, Err(e))
                | Reply::Saved(_, Err(e))
                | Reply::Snapshot(_, Err(e))
                | Reply::Status(_, Err(e))
                | Reply::Locked(_, _, Err(e)) => tr_fmt!("Project server: {}", e),
                _ => std::mem::take(&mut self.status),
            };
            if let Reply::List(Ok(list)) = &reply {
                self.projects = list.clone();
            }
            out.push(reply);
        }
        if self.shared.is_some() {
            let now = ctx.input(|i| i.time);
            if !self.polling && now - self.last_status >= openecraft_core::collab::HEARTBEAT_SECS {
                self.heartbeat(now);
            }
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(openecraft_core::collab::HEARTBEAT_SECS));
        }
        out
    }

    fn heartbeat(&mut self, now: f64) {
        let Some(sh) = &self.shared else { return };
        self.polling = true;
        self.last_status = now;
        let req = self.endpoint().status(&sh.name, &sh.session);
        let s = self.session_id();
        let status = std::mem::take(&mut self.status);
        self.send(req, status, move |r| Reply::Status(s, decode(r)));
    }

    /// Answer to a status or lock request: loads the newer project if there is one.
    pub fn take_status(&mut self, status: Status, granted: &[Target]) {
        let Some(sh) = &mut self.shared else { return };
        if sh.status(status, granted) {
            self.fetch();
        }
    }

    /// Text for a target, e.g. "page 3".
    pub fn describe(prj: &Project, target: Target) -> String {
        match target {
            Target::Project => tr("the project data (page list, devices, settings)").into(),
            Target::Page(id) => match prj.pages.iter().find(|p| p.id == id) {
                Some(p) => tr_fmt!("page {}", openecraft_core::structure::page_full_name(p)),
                None => tr_fmt!("page {}", id),
            },
        }
    }
}

#[cfg(test)]
mod tests;
