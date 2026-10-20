//! Local copy of the server's parts and the part being edited. No UI code, so the
//! editing rules (new parts, renames, unsaved changes) are unit testable.

use openecraft_parts::{natural_cmp, unique_number, Part, PartsDb};
use openecraft_ui::client::Reply;

/// The part in the editor.
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    /// Part number on the server; `None` for a part that was never saved.
    pub original: Option<String>,
    pub part: Part,
}

/// Request the application has to send after a server reply.
#[derive(Clone, Debug, PartialEq)]
pub enum FollowUp {
    /// Load all parts again.
    Reload,
    /// Remove the old number of a renamed part.
    Delete(String),
}

#[derive(Default)]
pub struct Session {
    /// Parts on the server (as of the last download).
    pub parts: Vec<Part>,
    pub revision: u64,
    pub loaded: bool,
    pub draft: Option<Draft>,
}

impl Session {
    pub fn get(&self, number: &str) -> Option<&Part> {
        self.parts.iter().find(|p| p.number == number)
    }

    /// Server version of the draft.
    fn saved(&self) -> Option<&Part> {
        self.draft.as_ref()?.original.as_deref().and_then(|n| self.get(n))
    }

    /// True if the draft has changes that are not on the server.
    pub fn dirty(&self) -> bool {
        match &self.draft {
            None => false,
            Some(d) => self.saved() != Some(&d.part),
        }
    }

    /// The draft's part number belongs to another part on the server.
    pub fn number_taken(&self) -> bool {
        self.draft.as_ref().is_some_and(|d| {
            d.original.as_deref() != Some(d.part.number.as_str()) && self.get(&d.part.number).is_some()
        })
    }

    /// Why the draft cannot be saved, if it cannot.
    pub fn save_problem(&self) -> Option<&'static str> {
        let d = self.draft.as_ref()?;
        if d.part.number.trim().is_empty() {
            Some("The part number must not be empty")
        } else if self.number_taken() {
            Some("Part number already exists")
        } else {
            None
        }
    }

    pub fn selected(&self) -> Option<&str> {
        self.draft.as_ref().map(|d| d.original.as_deref().unwrap_or(&d.part.number))
    }

    /// Opens a server part in the editor.
    pub fn select(&mut self, number: &str) {
        self.draft = self.get(number).map(|p| Draft { original: Some(p.number.clone()), part: p.clone() });
    }

    /// Discards the changes of the draft.
    pub fn revert(&mut self) {
        match self.draft.as_ref().and_then(|d| d.original.clone()) {
            Some(n) => self.select(&n),
            None => self.draft = None,
        }
    }

    /// Starts a new part.
    pub fn new_part(&mut self) {
        let number = unique_number(&self.parts, "NEW.PART");
        self.draft = Some(Draft { original: None, part: Part { number, manufacturer: "NEW".into(), ..Default::default() } });
    }

    /// Starts a new part as a copy of the draft.
    pub fn copy_part(&mut self) {
        if let Some(d) = &self.draft {
            let mut part = d.part.clone();
            part.number = unique_number(&self.parts, &format!("{}_COPY", part.number));
            self.draft = Some(Draft { original: None, part });
        }
    }

    /// All parts as a parts database (export).
    pub fn database(&self) -> PartsDb {
        PartsDb { parts: self.parts.clone() }
    }

    /// Updates the local copy with a server reply.
    pub fn apply(&mut self, reply: &Reply) -> Option<FollowUp> {
        match reply {
            Reply::Info(Ok(info)) if !self.loaded || info.revision != self.revision => Some(FollowUp::Reload),
            Reply::Parts(Ok(list)) => {
                let keep = self.dirty();
                self.parts = list.parts.clone();
                self.parts.sort_by(|a, b| natural_cmp(&a.number, &b.number));
                self.revision = list.revision;
                self.loaded = true;
                if !keep {
                    // Show the current server state of the selected part.
                    match self.draft.as_ref().and_then(|d| d.original.clone()) {
                        Some(n) => self.select(&n),
                        None => self.draft = None,
                    }
                }
                None
            }
            Reply::Saved(Ok(part)) => {
                match self.parts.iter_mut().find(|p| p.number == part.number) {
                    Some(p) => *p = (**part).clone(),
                    None => {
                        self.parts.push((**part).clone());
                        self.parts.sort_by(|a, b| natural_cmp(&a.number, &b.number));
                    }
                }
                let mut follow = None;
                if let Some(d) = self.draft.as_mut().filter(|d| d.part.number == part.number) {
                    if let Some(old) = d.original.replace(part.number.clone()).filter(|old| *old != part.number) {
                        follow = Some(FollowUp::Delete(old));
                    }
                    d.part = (**part).clone();
                }
                follow
            }
            Reply::Deleted(number, Ok(())) => {
                self.parts.retain(|p| &p.number != number);
                if self.draft.as_ref().is_some_and(|d| d.original.as_ref() == Some(number)) {
                    self.draft = None;
                }
                None
            }
            Reply::Imported(Ok(_)) => Some(FollowUp::Reload),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
