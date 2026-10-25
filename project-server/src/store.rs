//! Project storage: per project a project file (`<name>.oec`, opens in the editor
//! like any other project) and a state file (`<name>.state.json`: revision and id
//! counter) in the data directory. Files are written atomically on every change.

use openecraft_core::collab::{valid_name, ID_BLOCK};
use openecraft_core::model::Project;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

const PROJECT_EXT: &str = "oec";
const STATE_EXT: &str = "state.json";

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    revision: u64,
    next_id: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub project: Project,
    /// Incremented on every change of the project.
    pub revision: u64,
    /// First object id not given to a session yet.
    pub next_id: u64,
}

pub struct Store {
    dir: Option<PathBuf>,
    pub projects: BTreeMap<String, Entry>,
}

impl Store {
    /// A store that is not persisted (tests).
    #[cfg(test)]
    pub fn in_memory() -> Store {
        Store { dir: None, projects: BTreeMap::new() }
    }

    /// Opens (and creates) the data directory and loads its projects.
    pub fn open(dir: &Path) -> io::Result<Store> {
        std::fs::create_dir_all(dir)?;
        let mut projects = BTreeMap::new();
        for file in std::fs::read_dir(dir)? {
            let path = file?.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(&format!(".{PROJECT_EXT}"))) else {
                continue;
            };
            if !valid_name(name) {
                log::warn!("skipping {}: not a valid project name", path.display());
                continue;
            }
            let invalid = |e: String| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", path.display()));
            let project = Project::from_json(&std::fs::read_to_string(&path)?).map_err(invalid)?;
            let state_path = dir.join(format!("{name}.{STATE_EXT}"));
            let state: State = match std::fs::read_to_string(&state_path) {
                Ok(text) => serde_json::from_str(&text).map_err(|e| invalid(e.to_string()))?,
                Err(e) if e.kind() == io::ErrorKind::NotFound => State::default(),
                Err(e) => return Err(e),
            };
            let next_id = state.next_id.max(project.next_id);
            projects.insert(name.to_string(), Entry { project, revision: state.revision.max(1), next_id });
        }
        Ok(Store { dir: Some(dir.to_path_buf()), projects })
    }

    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.projects.get(name)
    }

    /// Stores `entry` under `name`: in memory only if writing the files succeeded.
    fn put(&mut self, name: &str, mut entry: Entry) -> io::Result<()> {
        entry.project.next_id = entry.next_id;
        if let Some(dir) = &self.dir {
            write(&dir.join(format!("{name}.{PROJECT_EXT}")), &entry.project.to_json())?;
            let state = State { revision: entry.revision, next_id: entry.next_id };
            write(&dir.join(format!("{name}.{STATE_EXT}")), &serde_json::to_string_pretty(&state).map_err(io::Error::other)?)?;
        }
        self.projects.insert(name.to_string(), entry);
        Ok(())
    }

    /// Adds a new project (the caller checks that the name is free).
    pub fn create(&mut self, name: &str, mut project: Project) -> io::Result<()> {
        project.normalize();
        let next_id = project.next_id;
        self.put(name, Entry { project, revision: 1, next_id })
    }

    /// Replaces the project; a new revision if it changed. Returns the revision.
    pub fn save(&mut self, name: &str, project: Project) -> io::Result<u64> {
        let Some(old) = self.projects.get(name) else {
            return Err(io::Error::new(io::ErrorKind::NotFound, name.to_string()));
        };
        let mut project = project;
        project.next_id = old.next_id;
        if project == old.project {
            return Ok(old.revision);
        }
        let entry = Entry { project, revision: old.revision + 1, next_id: old.next_id };
        let revision = entry.revision;
        self.put(name, entry)?;
        Ok(revision)
    }

    /// Removes the project and its files.
    pub fn delete(&mut self, name: &str) -> io::Result<()> {
        if self.projects.remove(name).is_none() {
            return Err(io::Error::new(io::ErrorKind::NotFound, name.to_string()));
        }
        if let Some(dir) = &self.dir {
            for ext in [PROJECT_EXT, STATE_EXT] {
                match std::fs::remove_file(dir.join(format!("{name}.{ext}"))) {
                    Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Reserves [`ID_BLOCK`] object ids for a session; returns the first one.
    pub fn reserve_ids(&mut self, name: &str) -> io::Result<u64> {
        let Some(old) = self.projects.get(name) else {
            return Err(io::Error::new(io::ErrorKind::NotFound, name.to_string()));
        };
        let mut entry = old.clone();
        let first = entry.next_id;
        entry.next_id += ID_BLOCK;
        self.put(name, entry)?;
        Ok(first)
    }
}

/// Writes a temporary file first so a crash never leaves a half written file.
fn write(path: &Path, content: &str) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests;
