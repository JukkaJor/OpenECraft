//! Client of Autodesk Vault (`openecraft_client::vault`): signing in, browsing
//! folders, searching, downloads, uploads and the item lookup run in the background;
//! [`Vault::poll`] collects the answers. The window is `app/vault_ui.rs`.

use openecraft_client::ehttp;
use openecraft_client::vault::{check, decode, entries, Collection, Entry, FileVersion, ItemLookup, ItemVersion, Job, Session, Step, Target, Upload, Uploaded, VaultEndpoint, ROOT};
use openecraft_i18n::{tr, tr_fmt};
use std::sync::{Arc, Mutex};

/// What a file of the vault is used for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    /// Opened as the project.
    Project,
    /// Imported into the parts database.
    Parts,
    /// 2D or 3D model of a part.
    Model,
    /// DXF / DWG drawing as a new page.
    Drawing,
    /// Embedded as a document page.
    Document,
    /// File of another system, read with a connector of the integration layer.
    Integration,
    /// Saved as a local file.
    Save,
}

impl Use {
    pub const ALL: [Use; 7] = [Use::Project, Use::Parts, Use::Model, Use::Drawing, Use::Document, Use::Integration, Use::Save];

    pub fn title(self) -> &'static str {
        match self {
            Use::Project => tr("Open as project"),
            Use::Parts => tr("Import into the parts database"),
            Use::Model => tr("Model of a part (2D / 3D)"),
            Use::Drawing => tr("Drawing as new page"),
            Use::Document => tr("Embed as document page"),
            Use::Integration => tr("File of another system (connector)"),
            Use::Save => tr("Save as local file"),
        }
    }

    /// The use a file name suggests.
    pub fn guess(name: &str) -> Use {
        let ext = name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
        match ext.as_str() {
            "oec" | "oecz" => Use::Project,
            "edz" | "xml" => Use::Parts,
            "stl" | "obj" | "step" | "stp" | "igs" | "iges" | "ifc" | "jt" | "svg" => Use::Model,
            "dxf" | "dwg" => Use::Drawing,
            "pdf" | "docx" | "doc" | "xlsx" | "xls" | "pptx" | "odt" | "ods" | "png" | "jpg" | "jpeg" | "txt" => Use::Document,
            "csv" | "json" => Use::Integration,
            _ => Use::Save,
        }
    }
}

/// Answer of a request.
#[derive(Debug)]
pub enum Reply {
    SignedIn(Result<Session, String>),
    /// Folder (or search) the entries belong to and the answer.
    Contents(String, Result<Collection<Entry>, String>),
    Downloaded(Use, FileVersion, Result<Vec<u8>, String>),
    /// The upload was the project file.
    Uploaded(bool, Result<Uploaded, String>),
    Items(Result<Vec<(String, ItemVersion)>, String>),
    SignedOut,
}

/// The project file the open project came from; checking the project in puts a new
/// version of it into its folder.
#[derive(Clone, Debug, PartialEq)]
pub struct Origin {
    pub folder: String,
    pub name: String,
}

pub struct Vault {
    pub url: String,
    /// Name of the knowledge vault for signing in.
    pub vault_name: String,
    pub user: String,
    /// Password of the Vault account (kept until signing in).
    pub password: String,
    /// Access token of the session or of an Autodesk account (kept in memory only).
    pub token: String,
    pub vault_id: String,
    pub signed_in: bool,
    /// Folders from the root to the one shown: id and name.
    pub path: Vec<(String, String)>,
    pub entries: Vec<Entry>,
    /// Number of results of the listing (it may have more than were sent).
    pub total: usize,
    pub search: String,
    pub selected: Option<usize>,
    pub origin: Option<Origin>,
    pub window: bool,
    pub status: String,
    pending: usize,
    /// Status of the running job (progress of uploads).
    progress: Arc<Mutex<Option<String>>>,
    ctx: Option<egui::Context>,
    inbox: Arc<Mutex<Vec<Reply>>>,
}

impl Default for Vault {
    fn default() -> Self {
        Vault {
            url: String::new(),
            vault_name: "Vault".into(),
            user: String::new(),
            password: String::new(),
            token: String::new(),
            vault_id: String::new(),
            signed_in: false,
            path: Vec::new(),
            entries: Vec::new(),
            total: 0,
            search: String::new(),
            selected: None,
            origin: None,
            window: false,
            status: String::new(),
            pending: 0,
            progress: Arc::default(),
            ctx: None,
            inbox: Arc::default(),
        }
    }
}

type Wrap<T> = Box<dyn FnOnce(Result<T, String>) -> Reply + Send>;

/// Sends the job's requests one after the other; the result goes to the inbox.
fn drive<J: Job>(mut job: J, req: ehttp::Request, inbox: Arc<Mutex<Vec<Reply>>>, progress: Arc<Mutex<Option<String>>>, ctx: Option<egui::Context>, wrap: Wrap<J::Output>) {
    ehttp::fetch(req, move |result| {
        let reply = match job.next(result) {
            Ok(Step::Send(next)) => {
                if let Ok(mut p) = progress.lock() {
                    *p = Some(job.status());
                }
                if let Some(ctx) = &ctx {
                    ctx.request_repaint();
                }
                return drive(job, next, inbox, progress, ctx, wrap);
            }
            Ok(Step::Done(out)) => wrap(Ok(out)),
            Err(e) => {
                if let Some(undo) = job.cleanup() {
                    ehttp::fetch(undo, |_| {});
                }
                wrap(Err(e))
            }
        };
        if let Ok(mut inbox) = inbox.lock() {
            inbox.push(reply);
        }
        if let Some(ctx) = ctx {
            ctx.request_repaint();
        }
    });
}

impl Vault {
    pub fn endpoint(&self) -> VaultEndpoint {
        VaultEndpoint { url: self.url.clone(), token: self.token.clone(), vault: self.vault_id.clone() }
    }

    pub fn busy(&self) -> bool {
        self.pending > 0
    }

    /// The folder shown.
    pub fn folder(&self) -> String {
        self.path.last().map(|(id, _)| id.clone()).unwrap_or_else(|| ROOT.into())
    }

    /// Vault path of the folder shown, e.g. `$/Projects`.
    pub fn folder_path(&self) -> String {
        std::iter::once("$").chain(self.path.iter().map(|(_, n)| n.as_str())).collect::<Vec<_>>().join("/")
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

    fn spawn<J: Job>(&mut self, mut job: J, wrap: impl 'static + Send + FnOnce(Result<J::Output, String>) -> Reply) {
        self.status = job.status();
        self.pending += 1;
        let req = job.start();
        drive(job, req, self.inbox.clone(), self.progress.clone(), self.ctx.clone(), Box::new(wrap));
    }

    /// Signs in with the Vault account, or takes the session of the access token.
    pub fn sign_in(&mut self) {
        let endpoint = self.endpoint();
        let req = if self.user.trim().is_empty() && !self.token.trim().is_empty() {
            endpoint.session()
        } else {
            endpoint.sign_in(&self.vault_name, &self.user, &std::mem::take(&mut self.password))
        };
        self.send(req, tr("Signing in…").into(), |r| Reply::SignedIn(decode(r)));
    }

    /// Ends the session; the project and its origin stay.
    pub fn sign_out(&mut self) {
        let req = self.endpoint().sign_out();
        self.token.clear();
        self.signed_in = false;
        self.entries.clear();
        self.selected = None;
        self.send(req, tr("Signed out of Vault").into(), |_| Reply::SignedOut);
    }

    /// Lists the folder shown again.
    pub fn refresh(&mut self) {
        let folder = self.folder();
        let req = self.endpoint().contents(&folder);
        self.send(req, tr("Reading the folder…").into(), move |r| Reply::Contents(folder, entries(r)));
    }

    /// Shows the subfolder `id`.
    pub fn enter(&mut self, id: &str, name: &str) {
        self.path.push((id.to_string(), name.to_string()));
        self.search.clear();
        self.refresh();
    }

    /// Goes back to the folder at `depth` (0: the root).
    pub fn up_to(&mut self, depth: usize) {
        self.path.truncate(depth);
        self.search.clear();
        self.refresh();
    }

    /// Searches the file properties below the folder shown.
    pub fn find(&mut self) {
        if self.search.trim().is_empty() {
            return self.refresh();
        }
        let folder = self.folder();
        let req = self.endpoint().search(&self.search, Some(&folder));
        self.send(req, tr("Searching…").into(), move |r| Reply::Contents(folder, entries(r)));
    }

    pub fn download(&mut self, file: FileVersion, using: Use) {
        let req = self.endpoint().download(&file.id);
        let status = tr_fmt!("Downloading {}…", file.name);
        self.send(req, status, move |r| Reply::Downloaded(using, file, check(r).map(|r| r.bytes)));
    }

    /// Puts a file into `folder`: a new version of a file of the same name there, or a new file.
    pub fn upload(&mut self, folder: &str, name: &str, data: Vec<u8>, comment: &str, project: bool) {
        let job = Upload::new(self.endpoint(), Target::Folder(folder.to_string()), name, data, comment);
        self.spawn(job, move |r| Reply::Uploaded(project, r));
    }

    /// Looks up the Vault items of the part numbers.
    pub fn lookup_items(&mut self, numbers: Vec<String>) {
        match ItemLookup::new(self.endpoint(), numbers) {
            Ok(job) => self.spawn(job, Reply::Items),
            Err(e) => self.status = e,
        }
    }

    /// Takes the finished requests.
    pub fn poll(&mut self, ctx: &egui::Context) -> Vec<Reply> {
        self.ctx = Some(ctx.clone());
        if let Some(p) = self.progress.lock().ok().and_then(|mut p| p.take()) {
            self.status = p;
        }
        let replies: Vec<Reply> = self.inbox.lock().map(|mut i| std::mem::take(&mut *i)).unwrap_or_default();
        for reply in &replies {
            self.pending = self.pending.saturating_sub(1);
            self.status = match reply {
                Reply::SignedIn(Ok(s)) => {
                    if !s.access_token.is_empty() {
                        self.token = s.access_token.clone();
                    }
                    if let Some(v) = &s.vault_information {
                        self.vault_id = v.id.clone();
                        self.vault_name = v.name.clone();
                    }
                    if let Some(u) = &s.user_information {
                        self.user = u.name.clone();
                    }
                    self.signed_in = !self.vault_id.is_empty();
                    if !self.signed_in {
                        tr("Vault: the session names no vault").to_string()
                    } else {
                        self.path.clear();
                        self.refresh();
                        tr_fmt!("Signed in to {} as {}", self.vault_name, self.user)
                    }
                }
                Reply::Contents(folder, Ok(page)) if *folder == self.folder() => {
                    self.entries = page.results.clone();
                    self.total = page.pagination.total_results.max(self.entries.len());
                    self.selected = None;
                    tr_fmt!("{} entries in {}", self.total, self.folder_path())
                }
                Reply::Contents(_, Ok(_)) => std::mem::take(&mut self.status),
                Reply::Uploaded(_, Ok(u)) => {
                    let msg = if u.added {
                        tr_fmt!("Added {} to Vault", u.version.name)
                    } else {
                        tr_fmt!("Checked in {} as version {}", u.version.name, u.version.version_number())
                    };
                    self.refresh();
                    msg
                }
                Reply::Downloaded(_, file, Ok(_)) => tr_fmt!("Downloaded {} (version {})", file.name, file.version_number()),
                Reply::Items(Ok(found)) => tr_fmt!("{} Vault items found for the project parts", found.len()),
                Reply::SignedOut => std::mem::take(&mut self.status),
                Reply::SignedIn(Err(e)) | Reply::Contents(_, Err(e)) | Reply::Downloaded(_, _, Err(e)) | Reply::Uploaded(_, Err(e)) | Reply::Items(Err(e)) => tr_fmt!("Vault: {}", e),
            };
        }
        replies
    }
}

#[cfg(test)]
mod tests;
