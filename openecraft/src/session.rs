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
    /// History revision of the last save.
    pub saved_rev: u64,
    /// File the project was opened from or saved to.
    pub file_name: Option<String>,
}

impl Session {
    /// Opens `prj` (normalised) on its first schematic page; it counts as saved.
    pub fn new(mut prj: Project, file_name: Option<String>) -> Session {
        prj.normalize();
        let history = History::new(&prj);
        let rev = history.rev;
        let page = first_schematic(&prj);
        Session { prj, page, history, analysis: Analysis::default(), cache: Cache::default(), analysis_rev: 0, analysis_lang: openecraft_i18n::lang(), index: Index::default(), index_rev: 0, saved_rev: rev, file_name }
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
        self.saved_rev = rev;
    }

    /// Changes since the last save.
    pub fn dirty(&self) -> bool {
        self.history.rev != self.saved_rev
    }

    pub fn mark_saved(&mut self) {
        self.saved_rev = self.history.rev;
    }

    /// Keeps the page index inside the project.
    pub fn clamp_page(&mut self) {
        self.page = self.page.min(self.prj.pages.len().saturating_sub(1));
    }

    /// Makes the finished edits one undo step (`can_commit` is false while the user
    /// is still dragging or typing).
    pub fn commit(&mut self, can_commit: bool) {
        self.history.track(&self.prj, can_commit);
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
}

fn first_schematic(prj: &Project) -> usize {
    prj.pages.iter().position(|p| p.kind == PageKind::Schematic).unwrap_or(0)
}

#[cfg(test)]
mod tests;
