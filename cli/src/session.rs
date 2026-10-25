//! State the tools work on: the open project (with its file, undo history and saved
//! state), the parts server and Autodesk Vault. The command line uses one session per
//! call, the MCP server one for the whole connection.

use crate::parts_server::{Http, Server};
use crate::vault::Vault;
use openecraft_core::archive::{self, Conversion};
use openecraft_api::Plugins;
use openecraft_core::history::History;
use openecraft_core::model::Project;
use openecraft_i18n::{tr, tr_fmt};
use std::path::PathBuf;

pub struct Session {
    project: Option<Project>,
    pub path: Option<PathBuf>,
    /// Project as last opened / saved; `None` if it was never saved.
    saved: Option<Project>,
    history: Option<History>,
    pub server: Server,
    pub vault: Vault,
    /// Built-in plugins; the command line adds the external ones (see `main`).
    pub plugins: Plugins,
}

impl Session {
    pub fn new(server: Server) -> Session {
        let vault = Vault::new(Box::new(Http));
        Session { project: None, path: None, saved: None, history: None, server, vault, plugins: Plugins::new(crate::tools::all().map(|t| t.name)) }
    }

    pub fn project(&self) -> Result<&Project, String> {
        self.project.as_ref().ok_or_else(no_project)
    }

    pub fn project_mut(&mut self) -> Result<&mut Project, String> {
        self.project.as_mut().ok_or_else(no_project)
    }

    /// Replaces the project. `path` is the file it was read from (counts as saved).
    pub fn replace(&mut self, mut prj: Project, path: Option<PathBuf>) {
        prj.normalize();
        self.history = Some(History::new(&prj));
        self.saved = path.is_some().then(|| prj.clone());
        self.path = path;
        self.project = Some(prj);
    }

    /// Opens a project file or archive (converted to the current file format).
    pub fn open(&mut self, path: &str) -> Result<Conversion, String> {
        let bytes = std::fs::read(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
        let (prj, conv) = archive::read_project(&bytes)?;
        // Saving writes a project file, not the archive.
        let file = (!archive::is_archive(&bytes)).then(|| PathBuf::from(path));
        self.replace(prj, file);
        Ok(conv)
    }

    /// Saves to `path`, or to the file of the project. Returns the file.
    pub fn save(&mut self, path: Option<&str>) -> Result<PathBuf, String> {
        let prj = self.project()?;
        let path = path.map(PathBuf::from).or_else(|| self.path.clone()).ok_or_else(|| tr("The project has no file yet: pass a path").to_string())?;
        std::fs::write(&path, prj.to_json()).map_err(|e| tr_fmt!("Cannot write {}: {}", path.display(), e))?;
        self.saved = Some(prj.clone());
        self.path = Some(path.clone());
        Ok(path)
    }

    /// The project has changes that are not in its file.
    pub fn unsaved(&self) -> bool {
        self.project.is_some() && self.saved != self.project
    }

    /// Makes the changes of a tool one undo step.
    pub fn commit(&mut self) {
        if let (Some(h), Some(prj)) = (&mut self.history, &self.project) {
            h.track(prj, true);
        }
    }

    pub fn can_undo(&self) -> bool {
        self.history.as_ref().is_some_and(|h| h.can_undo())
    }

    pub fn can_redo(&self) -> bool {
        self.history.as_ref().is_some_and(|h| h.can_redo())
    }

    /// Undoes the last change; false if there is none.
    pub fn undo(&mut self) -> Result<bool, String> {
        let (Some(h), Some(prj)) = (&mut self.history, &mut self.project) else { return Err(no_project()) };
        let can = h.can_undo();
        h.undo(prj);
        Ok(can)
    }

    pub fn redo(&mut self) -> Result<bool, String> {
        let (Some(h), Some(prj)) = (&mut self.history, &mut self.project) else { return Err(no_project()) };
        let can = h.can_redo();
        h.redo(prj);
        Ok(can)
    }
}

fn no_project() -> String {
    tr("No project is open: use open_project or new_project first").to_string()
}

#[cfg(test)]
mod tests;
