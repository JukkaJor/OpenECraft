//! The open project and what the editor derives from it: undo history, analysis,
//! device index, saved state and file name. No UI code, so the rules (dirty, first
//! page, when the analysis is rebuilt) are unit tested; `App` adds the windows.

use openecraft_api::Plugins;
use openecraft_core::analysis::{Analysis, Cache};
use openecraft_core::history::History;
use openecraft_core::index::Index;
use openecraft_core::model::{PageKind, Project};

pub struct Session {
    pub prj: Project,
    /// Index of the page shown.
    pub page: usize,
    pub history: History,
    pub analysis: Analysis,
    /// Page scans of the last analysis: only changed pages are scanned again.
    cache: Cache,
    analysis_rev: u64,
    analysis_lang: openecraft_i18n::Lang,
    /// Device index; while an edit is in progress it follows every frame.
    pub index: Index,
    index_rev: u64,
    /// History state of the last save; none: the project differs from every file.
    saved: Option<u64>,
    /// File the project was opened from or saved to.
    pub file_name: Option<String>,
}

impl Session {
    /// Opens `prj` (normalised) on its first schematic page; it counts as saved.
    pub fn new(mut prj: Project, file_name: Option<String>) -> Session {
        prj.normalize();
        let history = History::new(&prj);
        let saved = Some(history.state());
        let page = first_schematic(&prj);
        Session { prj, page, history, analysis: Analysis::default(), cache: Cache::default(), analysis_rev: 0, analysis_lang: openecraft_i18n::lang(), index: Index::default(), index_rev: 0, saved, file_name }
    }

    /// Replaces the project; the replacement counts as saved. Its history starts
    /// anew: undo must not bring back the project before (whose changes may have been
    /// discarded) under the file name of this one.
    pub fn replace(&mut self, prj: Project, file_name: Option<String>) {
        self.prj = prj;
        self.prj.normalize();
        self.file_name = file_name;
        self.page = first_schematic(&self.prj);
        // A new revision, so that the analysis and the index are built again.
        let rev = self.history.rev + 1;
        self.history = History::new(&self.prj);
        self.history.rev = rev;
        self.mark_saved();
    }

    /// Changes since the last save (undo back to the saved state leaves none).
    pub fn dirty(&self) -> bool {
        self.saved != Some(self.history.state())
    }

    pub fn mark_saved(&mut self) {
        self.saved = Some(self.history.state());
    }

    /// The project has changes no file holds (e.g. restored after the app was closed
    /// with unsaved changes).
    pub fn mark_unsaved(&mut self) {
        self.saved = None;
    }

    /// Nothing happened since the session started: no edit, undo or replacement.
    pub fn untouched(&self) -> bool {
        self.history.rev == 1
    }

    /// Keeps the page index inside the project.
    pub fn clamp_page(&mut self) {
        self.page = self.page.min(self.prj.pages.len().saturating_sub(1));
    }

    /// Makes the finished edits one undo step (`can_commit` is false while the user
    /// is still dragging or typing).
    /// Returns whether a change was committed (the shell shows it at once).
    pub fn commit(&mut self, can_commit: bool) -> bool {
        self.history.track(&self.prj, can_commit)
    }

    /// Rebuilds the analysis after a committed change or a change of the language
    /// (reports and checks of `plugins`), and the device index after a committed
    /// change or, while `editing`, every frame.
    pub fn refresh(&mut self, plugins: &Plugins, editing: bool) {
        if self.analysis_rev != self.history.rev || self.analysis_lang != openecraft_i18n::lang() {
            self.analysis = Analysis::build_with(&self.prj, &mut self.cache);
            self.analysis.set_reports(plugins.reports().cloned());
            self.analysis.issues = plugins.check(&self.prj, &self.analysis);
            self.analysis.issues.sort_by_key(|i| i.severity);
            self.analysis_rev = self.history.rev;
            self.analysis_lang = openecraft_i18n::lang();
        }
        if editing || self.index_rev != self.history.rev {
            self.index = Index::build(&self.prj);
            self.index_rev = self.history.rev;
        }
    }

    /// The next `refresh` rebuilds the analysis (plugins changed).
    pub fn invalidate(&mut self) {
        self.analysis_rev = self.history.rev.wrapping_add(1);
    }

    pub fn undo(&mut self) {
        self.history.undo(&mut self.prj);
    }

    pub fn redo(&mut self) {
        self.history.redo(&mut self.prj);
    }

    /// Whether the items of page `page` may be changed: not released, not generated
    /// (reports, mounting layouts), not read only for this user (locked by another user
    /// of a shared project, a viewer).
    pub fn may_edit_page(&self, page: usize, read_only: bool) -> bool {
        self.prj.pages.get(page).is_some_and(|p| p.is_editable()) && !read_only
    }

    /// Whether page `page` may be deleted: not the last one, not released, not locked
    /// by another user of a shared project.
    pub fn may_delete_page(&self, page: usize, locked_by_other: bool) -> bool {
        self.prj.pages.len() > 1 && self.prj.pages.get(page).is_some_and(|p| !p.locked) && !locked_by_other
    }

    /// What deleting page `page` loses: its items (a document page counts its file);
    /// an empty page is deleted without asking.
    pub fn page_items(&self, page: usize) -> usize {
        let Some(p) = self.prj.pages.get(page) else { return 0 };
        p.symbols.len() + p.wires.len() + p.texts.len() + p.shapes.len() + p.dimensions.len() + p.images.len() + p.conn_defs.len() + p.cable_defs.len() + p.macro_boxes.len() + usize::from(p.document.is_some())
    }
}

/// The question before an action drops unsaved changes (a new or opened project,
/// closing the window): the action waits for the answer, after Save until the save
/// has finished.
#[derive(Debug)]
pub struct Guard<A> {
    pending: Option<A>,
    saving: bool,
    /// The last save of the question ended with changes left (cancelled or failed).
    pub not_saved: bool,
}

impl<A> Default for Guard<A> {
    fn default() -> Self {
        Guard { pending: None, saving: false, not_saved: false }
    }
}

impl<A> Guard<A> {
    /// The action runs now (returned) unless it would drop unsaved changes: then it
    /// waits for the answer.
    pub fn request(&mut self, dirty: bool, action: A) -> Option<A> {
        if !dirty {
            return Some(action);
        }
        self.pending = Some(action);
        self.saving = false;
        self.not_saved = false;
        None
    }

    /// The action waiting for the answer.
    pub fn pending(&self) -> Option<&A> {
        self.pending.as_ref()
    }

    /// Discard: the waiting action runs.
    pub fn discard(&mut self) -> Option<A> {
        self.saving = false;
        self.pending.take()
    }

    pub fn cancel(&mut self) {
        self.pending = None;
        self.saving = false;
    }

    /// Save was chosen: the action waits for the save.
    pub fn save(&mut self) {
        self.saving = self.pending.is_some();
        self.not_saved = false;
    }

    /// A save is running for the question.
    pub fn saving(&self) -> bool {
        self.saving
    }

    /// Call after a save attempt and every frame while saving (`busy`: a save on a
    /// server is still running). Returns the action once the project is clean; a save
    /// that ended with changes left keeps the question open.
    pub fn after_save(&mut self, dirty: bool, busy: bool) -> Option<A> {
        if !self.saving {
            return None;
        }
        if !dirty {
            self.saving = false;
            return self.pending.take();
        }
        if !busy {
            self.saving = false;
            self.not_saved = true;
        }
        None
    }
}

fn first_schematic(prj: &Project) -> usize {
    prj.pages.iter().position(|p| p.kind == PageKind::Schematic).unwrap_or(0)
}

#[cfg(test)]
mod tests;
