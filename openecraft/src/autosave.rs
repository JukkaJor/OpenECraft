//! The autosaved project and parts database: kept in the app's [`Store`] (IndexedDB in
//! the browser, whose local storage is too small for large projects), read back
//! asynchronously at start-up. No UI code; `App` calls it.

use crate::session::Session;
use openecraft_parts::PartsDb;
use openecraft_ui::io::Store;

const PROJECT: &str = "project";
const PARTS: &str = "parts_db";

pub struct Autosave {
    store: Store,
    /// The stored project is still expected: it replaces the start-up project unless
    /// that was changed or replaced meanwhile.
    awaiting_project: bool,
    /// The stored parts database is still expected; the start-up one, which it replaces
    /// unless the parts were changed meanwhile.
    awaiting_parts: Option<PartsDb>,
}

impl Autosave {
    /// Starts reading the stored project and parts database (`parts`: the database the
    /// app starts with).
    pub fn start(store: Store, ctx: &egui::Context, parts: &PartsDb) -> Autosave {
        store.load(ctx, PROJECT);
        store.load(ctx, PARTS);
        Autosave { store, awaiting_project: true, awaiting_parts: Some(parts.clone()) }
    }

    /// Takes values that finished loading. Returns true if the project was replaced.
    pub fn poll(&mut self, session: &mut Session, parts: &mut PartsDb) -> bool {
        let mut replaced = false;
        while let Some((key, value)) = self.store.poll() {
            match key.as_str() {
                PROJECT => {
                    let waited = std::mem::take(&mut self.awaiting_project);
                    let prj = value.and_then(|v| openecraft_core::archive::unpack(&String::from_utf8_lossy(&v)).ok());
                    if let Some(prj) = prj.filter(|_| waited && session.untouched()) {
                        // Unsaved changes at the last close stay unsaved.
                        let unsaved = session.dirty();
                        *session = Session::new(prj, session.file_name.clone());
                        if unsaved {
                            session.mark_unsaved();
                        }
                        replaced = true;
                    }
                }
                PARTS => {
                    let start = self.awaiting_parts.take();
                    let db = value.and_then(|v| serde_json::from_slice::<PartsDb>(&v).ok());
                    if let Some(db) = db.filter(|_| start.as_ref() == Some(parts)) {
                        *parts = db;
                    }
                }
                _ => {}
            }
        }
        replaced
    }

    /// Writes the project and the parts database (not before the stored ones arrived,
    /// which would be overwritten by the start-up values). Returns whether the store
    /// has worked so far: until it does, the caller keeps both in eframe's storage too.
    pub fn save(&self, session: &Session, parts: &PartsDb) -> bool {
        if !self.awaiting_project {
            self.store.store(PROJECT, openecraft_core::archive::pack(&session.prj).into_bytes());
        }
        if self.awaiting_parts.is_none() {
            self.store.store(PARTS, serde_json::to_vec(parts).unwrap_or_default());
        }
        !self.awaiting_project && self.awaiting_parts.is_none() && self.store.works()
    }
}

#[cfg(test)]
mod tests;
