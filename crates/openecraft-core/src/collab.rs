//! Shared projects on the project server (`openecraft-project-server`): API types,
//! request paths and the rules that let several users edit one project at a time.
//!
//! Every page and the project-wide data (page list, devices, parts, layout,
//! settings, …) are locked separately ([`Target`]). A user changes only what they
//! have locked; saving merges their locked parts into the server's project and takes
//! everything else from the server ([`merge`]). New pages belong to the project-wide
//! data until they are saved. Review records (comments, markup, approvals, change
//! requests) need no lock: they are merged record by record (`review::merge`).
//!
//! | Method | Path | |
//! | --- | --- | --- |
//! | GET | `/api/v1/info` | [`ServerInfo`] |
//! | GET | `/api/v1/projects` | [`ProjectSummary`] list |
//! | POST | `/api/v1/projects` | [`CreateRequest`] → [`Opened`] |
//! | GET | `/api/v1/projects/{name}` | [`Snapshot`] |
//! | PUT | `/api/v1/projects/{name}` | [`SaveRequest`] → [`Snapshot`] |
//! | POST | `/api/v1/projects/{name}/open` | [`OpenRequest`] → [`Opened`] |
//! | POST | `/api/v1/projects/{name}/status` | [`Session`] → [`Status`] (keeps the session alive) |
//! | POST | `/api/v1/projects/{name}/lock` | [`LockRequest`] → [`Status`] |
//! | POST | `/api/v1/projects/{name}/release` | [`LockRequest`] → [`Status`] |
//! | POST | `/api/v1/projects/{name}/close` | [`Session`] → [`Status`] (releases all locks) |
//! | POST | `/api/v1/login` | [`LoginRequest`] → [`LoginResponse`] |
//! | POST | `/api/v1/logout` | ends the session of the bearer token |
//! | GET | `/api/v1/users` | [`ProjectUser`] list (administrators) |
//! | PUT | `/api/v1/users/{name}` | [`UserUpdate`] → [`ProjectUser`] (administrators) |
//! | DELETE | `/api/v1/users/{name}` | (administrators) |
//!
//! With a server token or user accounts every request needs `Authorization: Bearer
//! <token>`: the server token (full access) or the session token of a signed-in user,
//! whose role ([`ProjectRole`]) and projects decide what they may do. Errors are
//! returned as `openecraft_parts::api::ApiError` JSON.

use crate::model::{Page, Project};
use openecraft_parts::api::encode_component;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const API_PREFIX: &str = "/api/v1";
pub const DEFAULT_PORT: u16 = 8091;
/// Object ids reserved for a session when it opens a project, so that users never
/// create the same id.
pub const ID_BLOCK: u64 = 1_000_000;
/// A session without a status request for this long loses its locks.
pub const SESSION_TIMEOUT_SECS: u64 = 90;
/// How often clients send a status request.
pub const HEARTBEAT_SECS: f64 = 15.0;

/// Something a user can lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// Project-wide data: page list and order, devices, parts, layout, settings.
    Project,
    /// Content and properties of a page (by page id).
    Page(u64),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lock {
    pub target: Target,
    pub user: String,
    pub session: String,
}

/// User and session of a request.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub user: String,
    pub session: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub projects: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectSummary {
    /// Name on the server (file name).
    pub name: String,
    /// Project name from the project info.
    pub title: String,
    pub pages: usize,
    pub revision: u64,
    /// Users working on the project.
    pub users: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenRequest {
    pub user: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CreateRequest {
    pub name: String,
    pub user: String,
    pub project: Project,
}

/// The project as stored on the server.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapshot {
    pub revision: u64,
    pub project: Project,
    pub locks: Vec<Lock>,
}

/// Answer to opening a project: a new session with its id range.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Opened {
    pub session: Session,
    /// First object id of the session; ids up to `first_id + ID_BLOCK` are its own.
    pub first_id: u64,
    pub snapshot: Snapshot,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Status {
    pub revision: u64,
    pub locks: Vec<Lock>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SaveRequest {
    pub session: Session,
    pub project: Project,
    /// Locks to release after saving.
    pub release: Vec<Target>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LockRequest {
    pub session: Session,
    pub targets: Vec<Target>,
}

/// What a user may do on the project server. Ordered by permissions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRole {
    /// Open projects read-only (no locks, no saving): review and comments.
    #[default]
    Viewer,
    /// Lock, change, save and create projects.
    Editor,
    /// Also manage the users.
    Admin,
}

impl ProjectRole {
    pub const ALL: [ProjectRole; 3] = [ProjectRole::Viewer, ProjectRole::Editor, ProjectRole::Admin];

    pub fn title(self) -> &'static str {
        match self {
            ProjectRole::Viewer => openecraft_i18n::tr("Viewer"),
            ProjectRole::Editor => openecraft_i18n::tr("Editor"),
            ProjectRole::Admin => openecraft_i18n::tr("Administrator"),
        }
    }
}

/// A user of the project server (without credentials).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectUser {
    pub name: String,
    pub role: ProjectRole,
    /// Projects the user may open (names, `*` and `?` wildcards); empty = all.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub projects: Vec<String>,
}

impl ProjectUser {
    /// The user may open the project.
    pub fn may_see(&self, project: &str) -> bool {
        self.role == ProjectRole::Admin || self.projects.is_empty() || self.projects.iter().any(|p| crate::report_blocks::matches(p, project))
    }

    /// The user may lock, change and save the project.
    pub fn may_edit(&self, project: &str) -> bool {
        self.role >= ProjectRole::Editor && self.may_see(project)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LoginRequest {
    pub user: String,
    pub password: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LoginResponse {
    /// Session token for `Authorization: Bearer`.
    pub token: String,
    pub user: ProjectUser,
}

/// Creates or changes a user; a new user needs a password.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub role: ProjectRole,
    pub projects: Vec<String>,
}

/// Path of a user, e.g. `/api/v1/users/anna`.
pub fn user_path(name: &str) -> String {
    format!("{API_PREFIX}/users/{}", encode_component(name))
}

/// Path of a project, e.g. `/api/v1/projects/Plant%201`.
pub fn project_path(name: &str) -> String {
    format!("{API_PREFIX}/projects/{}", encode_component(name))
}

/// Project names are file names on the server: letters, digits, space, `-`, `_`, `.`;
/// not starting with a dot, at most 64 characters.
pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.trim() == name
        && name.chars().count() <= 64
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
}

/// What a session may change.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Owned {
    pub project: bool,
    pub pages: BTreeSet<u64>,
}

impl Owned {
    /// The locks of `session`.
    pub fn of(locks: &[Lock], session: &str) -> Owned {
        let mut owned = Owned::default();
        for l in locks.iter().filter(|l| l.session == session) {
            owned.add(l.target);
        }
        owned
    }

    /// Everything of `prj` except `targets`.
    pub fn all_but(prj: &Project, targets: &[Target]) -> Owned {
        let mut owned = Owned { project: true, pages: prj.pages.iter().map(|p| p.id).collect() };
        for t in targets {
            owned.remove(*t);
        }
        owned
    }

    pub fn add(&mut self, target: Target) {
        match target {
            Target::Project => self.project = true,
            Target::Page(id) => {
                self.pages.insert(id);
            }
        }
    }

    pub fn remove(&mut self, target: Target) {
        match target {
            Target::Project => self.project = false,
            Target::Page(id) => {
                self.pages.remove(&id);
            }
        }
    }

    pub fn contains(&self, target: Target) -> bool {
        match target {
            Target::Project => self.project,
            Target::Page(id) => self.pages.contains(&id),
        }
    }
}

fn page(prj: &Project, id: u64) -> Option<&Page> {
    prj.pages.iter().find(|p| p.id == id)
}

/// Combines the server's project (`theirs`) with a user's (`mine`): what `owned`
/// covers comes from `mine`, everything else from `theirs`. Pages that only `mine`
/// has are new pages of the project-wide data. `next_id` is the one of `mine`.
pub fn merge(theirs: &Project, mine: &Project, owned: &Owned) -> Project {
    let order = if owned.project { mine } else { theirs };
    let pages = order
        .pages
        .iter()
        .map(|p| {
            let take_mine = owned.pages.contains(&p.id) || page(theirs, p.id).is_none();
            match (take_mine, page(mine, p.id)) {
                (true, Some(m)) => m.clone(),
                _ => page(theirs, p.id).unwrap_or(p).clone(),
            }
        })
        .collect();
    let mut out = if owned.project { data_clone(mine) } else { data_clone(theirs) };
    out.pages = pages;
    out.review = crate::review::merge(&theirs.review, &mine.review);
    out.next_id = mine.next_id;
    out
}

/// Copy of the project without its pages and review.
fn data_clone(prj: &Project) -> Project {
    let mut out = Project::default();
    out.pages.clear();
    with_data(&mut out, prj);
    out
}

/// Sets the project-wide data of `out` to that of `src` (all but pages, review and
/// `next_id`).
fn with_data(out: &mut Project, src: &Project) {
    let Project {
        info,
        pages: _,
        parts,
        devices,
        layout,
        cables,
        wire_numbering,
        potentials,
        wire_defaults,
        wire_colors,
        strips,
        plugs,
        plc_cards,
        plc,
        revisions,
        harnesses,
        symbols,
        forms,
        report_blocks,
        configuration,
        marking,
        fluid,
        pct_loops,
        languages,
        wiring_marks,
        structures,
        page_templates,
        layers,
        workflow,
        baselines,
        electrical,
        property_db,
        format: _,
        field,
        subprojects,
        model_options,
        integration,
        check_settings,
        xref,
        mounting_marks,
        review: _,
        next_id: _,
        symbol_cache: _,
    } = src;
    out.info = info.clone();
    out.parts = parts.clone();
    out.devices = devices.clone();
    out.layout = layout.clone();
    out.cables = cables.clone();
    out.wire_numbering = wire_numbering.clone();
    out.potentials = potentials.clone();
    out.wire_defaults = wire_defaults.clone();
    out.wire_colors = *wire_colors;
    out.strips = strips.clone();
    out.plugs = plugs.clone();
    out.plc_cards = plc_cards.clone();
    out.plc = plc.clone();
    out.revisions = revisions.clone();
    out.harnesses = harnesses.clone();
    out.symbols = symbols.clone();
    out.forms = forms.clone();
    out.report_blocks = report_blocks.clone();
    out.configuration = configuration.clone();
    out.marking = marking.clone();
    out.fluid = fluid.clone();
    out.pct_loops = pct_loops.clone();
    out.languages = languages.clone();
    out.wiring_marks = wiring_marks.clone();
    out.structures = structures.clone();
    out.page_templates = page_templates.clone();
    out.layers = layers.clone();
    out.workflow = workflow.clone();
    out.baselines = baselines.clone();
    out.electrical = electrical.clone();
    out.property_db = property_db.clone();
    out.field = field.clone();
    out.subprojects = subprojects.clone();
    out.model_options = model_options.clone();
    out.integration = integration.clone();
    out.check_settings = check_settings.clone();
    out.xref = xref.clone();
    out.mounting_marks = mounting_marks.clone();
}

/// Project-wide data equal (all but page contents, review and `next_id`; the page
/// list counts). The review needs no lock, its records are merged one by one.
fn same_data(a: &Project, b: &Project) -> bool {
    let Project {
        info,
        pages,
        parts,
        devices,
        layout,
        cables,
        wire_numbering,
        potentials,
        wire_defaults,
        wire_colors,
        strips,
        plugs,
        plc_cards,
        plc,
        revisions,
        harnesses,
        symbols,
        forms,
        report_blocks,
        configuration,
        marking,
        fluid,
        pct_loops,
        languages,
        wiring_marks,
        structures,
        page_templates,
        layers,
        workflow,
        baselines,
        electrical,
        property_db,
        field,
        subprojects,
        model_options,
        format: _,
        integration,
        check_settings,
        xref,
        mounting_marks,
        review: _,
        next_id: _,
        symbol_cache: _,
    } = a;
    pages.iter().map(|p| p.id).eq(b.pages.iter().map(|p| p.id))
        && *info == b.info
        && *parts == b.parts
        && *devices == b.devices
        && *layout == b.layout
        && *cables == b.cables
        && *wire_numbering == b.wire_numbering
        && *potentials == b.potentials
        && *wire_defaults == b.wire_defaults
        && *wire_colors == b.wire_colors
        && *strips == b.strips
        && *plugs == b.plugs
        && *plc_cards == b.plc_cards
        && *plc == b.plc
        && *revisions == b.revisions
        && *harnesses == b.harnesses
        && *symbols == b.symbols
        && *forms == b.forms
        && *report_blocks == b.report_blocks
        && *configuration == b.configuration
        && *marking == b.marking
        && *fluid == b.fluid
        && *pct_loops == b.pct_loops
        && *languages == b.languages
        && *wiring_marks == b.wiring_marks
        && *structures == b.structures
        && *page_templates == b.page_templates
        && *layers == b.layers
        && *workflow == b.workflow
        && *baselines == b.baselines
        && *electrical == b.electrical
        && *property_db == b.property_db
        && *field == b.field
        && *subprojects == b.subprojects
        && *model_options == b.model_options
        && *integration == b.integration
        && *check_settings == b.check_settings
        && *xref == b.xref
        && *mounting_marks == b.mounting_marks
}

/// What differs between `a` and `b`. Changes of pages that `a` does not have count
/// as project-wide changes (new pages).
pub fn changes(a: &Project, b: &Project) -> BTreeSet<Target> {
    let mut out = BTreeSet::new();
    for pb in &b.pages {
        match page(a, pb.id) {
            Some(pa) if pa == pb => {}
            Some(_) => {
                out.insert(Target::Page(pb.id));
            }
            None => {
                out.insert(Target::Project);
            }
        }
    }
    if !same_data(a, b) {
        out.insert(Target::Project);
    }
    out
}

/// Client side state of a project opened from the server.
#[derive(Clone, Debug, PartialEq)]
pub struct Shared {
    /// Name on the server.
    pub name: String,
    pub session: Session,
    /// Server revision of `base`.
    pub revision: u64,
    /// The server's project at `revision`.
    pub base: Project,
    pub locks: Vec<Lock>,
    /// Targets whose lock was requested but not answered yet.
    pub requested: BTreeSet<Target>,
    /// Locks granted while `base` was older than the server's project: not used
    /// until the newer project is loaded ([`Shared::update`]).
    pub fresh: BTreeSet<Target>,
    /// First object id after the session's id range.
    pub id_end: u64,
}

impl Shared {
    /// State after opening; the returned project is the one to edit.
    pub fn open(name: &str, opened: Opened) -> (Shared, Project) {
        let mut base = opened.snapshot.project;
        base.normalize();
        let mut prj = base.clone();
        prj.next_id = opened.first_id;
        let shared = Shared {
            name: name.to_string(),
            session: opened.session,
            revision: opened.snapshot.revision,
            base,
            locks: opened.snapshot.locks,
            requested: BTreeSet::new(),
            fresh: BTreeSet::new(),
            id_end: opened.first_id + ID_BLOCK,
        };
        (shared, prj)
    }

    /// What this session may change now.
    pub fn owned(&self) -> Owned {
        let mut owned = Owned::of(&self.locks, &self.session.session);
        for t in &self.fresh {
            owned.remove(*t);
        }
        owned
    }

    pub fn owns(&self, target: Target) -> bool {
        self.locks.iter().any(|l| l.target == target && l.session == self.session.session)
    }

    /// The lock of another session on `target`.
    pub fn holder(&self, target: Target) -> Option<&Lock> {
        self.locks.iter().find(|l| l.target == target && l.session != self.session.session)
    }

    /// Changes of `prj` that need a lock this session has neither got nor requested.
    pub fn unlocked_changes(&self, prj: &Project) -> Vec<Target> {
        let owned = self.owned();
        changes(&self.base, prj)
            .into_iter()
            .filter(|t| !owned.contains(*t) && !self.requested.contains(t) && !self.fresh.contains(t))
            .collect()
    }

    /// `prj` has changes the server takes on saving.
    pub fn has_changes(&self, prj: &Project) -> bool {
        let owned = self.owned();
        self.base.review != prj.review || changes(&self.base, prj).into_iter().any(|t| owned.contains(t))
    }

    /// `prj` with `targets` set back to the server's version.
    pub fn revert(&self, prj: &Project, targets: &[Target]) -> Project {
        merge(&self.base, prj, &Owned::all_but(prj, targets))
    }

    /// Answer to a status, lock or release request. `granted` are the targets of a
    /// successful lock request. Returns true if the server has a newer project.
    pub fn status(&mut self, status: Status, granted: &[Target]) -> bool {
        let newer = status.revision > self.revision;
        for t in granted {
            self.requested.remove(t);
            if newer {
                self.fresh.insert(*t);
            }
        }
        self.locks = status.locks;
        newer
    }

    /// A lock request was refused.
    pub fn denied(&mut self, targets: &[Target]) {
        for t in targets {
            self.requested.remove(t);
        }
    }

    /// Takes a newer server state. Returns the fresh locks whose target another user
    /// changed in the meantime (the local change of them is replaced), and the
    /// function that brings a local version of the project (the edited one and the
    /// undo steps) up to date: it keeps what this session owns, the rest comes from
    /// the server.
    pub fn update(&mut self, mut snapshot: Snapshot) -> (Vec<Target>, impl Fn(&Project) -> Project) {
        snapshot.project.normalize();
        let changed = changes(&self.base, &snapshot.project);
        let lost: Vec<Target> = self.fresh.iter().copied().filter(|t| changed.contains(t)).collect();
        let mut owned = Owned::of(&snapshot.locks, &self.session.session);
        for t in &lost {
            owned.remove(*t);
        }
        self.fresh.clear();
        self.revision = snapshot.revision;
        self.locks = snapshot.locks;
        self.base = snapshot.project;
        let base = self.base.clone();
        (lost, move |mine: &Project| merge(&base, mine, &owned))
    }
}

#[cfg(test)]
mod tests;
