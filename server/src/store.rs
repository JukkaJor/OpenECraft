//! Persistent parts storage: one JSON file, written atomically on every change.

use openecraft_parts::{default_catalog, Part};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

/// File format of the database.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct StoredDb {
    revision: u64,
    parts: Vec<Part>,
}

pub struct Store {
    path: Option<PathBuf>,
    /// Incremented on every change (clients can detect updates).
    pub revision: u64,
    pub parts: Vec<Part>,
}

impl Store {
    /// A store that is not persisted (tests).
    #[cfg(test)]
    pub fn in_memory(parts: Vec<Part>) -> Store {
        Store { path: None, revision: 1, parts }
    }

    /// Opens the database file; a missing file is created with the example catalog.
    pub fn open(path: &Path) -> io::Result<Store> {
        if path.exists() {
            let text = std::fs::read_to_string(path)?;
            let db: StoredDb =
                serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", path.display())))?;
            Ok(Store { path: Some(path.to_path_buf()), revision: db.revision.max(1), parts: db.parts })
        } else {
            let store = Store { path: Some(path.to_path_buf()), revision: 1, parts: default_catalog() };
            store.save()?;
            Ok(store)
        }
    }

    fn save(&self) -> io::Result<()> {
        let Some(path) = &self.path else { return Ok(()) };
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        let db = StoredDb { revision: self.revision, parts: self.parts.clone() };
        let json = serde_json::to_string_pretty(&db).map_err(io::Error::other)?;
        // Write a temporary file first so a crash never leaves a half written database.
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path)
    }

    /// Applies a change as a new revision and saves it. If saving fails, the store
    /// stays as it was, so memory and file never differ.
    fn change<T>(&mut self, f: impl FnOnce(&mut Vec<Part>) -> T) -> io::Result<T> {
        let before = self.parts.clone();
        let result = f(&mut self.parts);
        self.revision += 1;
        if let Err(e) = self.save() {
            self.parts = before;
            self.revision -= 1;
            return Err(e);
        }
        Ok(result)
    }

    pub fn get(&self, number: &str) -> Option<&Part> {
        self.parts.iter().find(|p| p.number == number)
    }

    /// Inserts or replaces a part. Returns true if it was created.
    pub fn upsert(&mut self, part: Part) -> io::Result<bool> {
        self.change(|parts| put(parts, part))
    }

    /// Removes a part. Returns false if it did not exist.
    pub fn delete(&mut self, number: &str) -> io::Result<bool> {
        if self.get(number).is_none() {
            return Ok(false);
        }
        self.change(|parts| parts.retain(|p| p.number != number))?;
        Ok(true)
    }

    /// Upserts many parts with a single revision step.
    pub fn import(&mut self, new: Vec<Part>) -> io::Result<usize> {
        let n = new.len();
        if n > 0 {
            self.change(|parts| {
                for p in new {
                    put(parts, p);
                }
            })?;
        }
        Ok(n)
    }
}

/// Inserts or replaces a part. Returns true if it was created.
fn put(parts: &mut Vec<Part>, part: Part) -> bool {
    match parts.iter_mut().find(|p| p.number == part.number) {
        Some(existing) => {
            *existing = part;
            false
        }
        None => {
            parts.push(part);
            true
        }
    }
}

#[cfg(test)]
mod tests;
