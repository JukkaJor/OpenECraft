//! Sessions and their locks. Kept in memory only: a session ends when it is closed,
//! when the server stops or after `SESSION_TIMEOUT_SECS` without a request.

use openecraft_core::collab::{Lock, Session, Target, SESSION_TIMEOUT_SECS};
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{BuildHasher, RandomState};
use std::time::{Duration, Instant};

struct Entry {
    user: String,
    project: String,
    seen: Instant,
    locks: BTreeSet<Target>,
    /// Hash of the sign-in token the session was opened with: signing out ends it.
    owner: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum LockError {
    /// Unknown or expired session.
    Expired,
    /// Held by another session.
    Held(Lock),
}

#[derive(Default)]
pub struct Sessions {
    sessions: BTreeMap<String, Entry>,
    counter: u64,
}

impl Sessions {
    /// Starts a session of `user` on `project`.
    pub fn open(&mut self, project: &str, user: &str, now: Instant) -> Session {
        self.counter += 1;
        let random = RandomState::new().hash_one(self.counter);
        let id = format!("{}-{random:016x}", self.counter);
        self.sessions.insert(id.clone(), Entry { user: user.to_string(), project: project.to_string(), seen: now, locks: BTreeSet::new(), owner: None });
        Session { user: user.to_string(), session: id }
    }

    /// Marks a session as opened with the sign-in token whose hash is `owner`.
    pub fn set_owner(&mut self, session: &str, owner: String) {
        if let Some(e) = self.sessions.get_mut(session) {
            e.owner = Some(owner);
        }
    }

    /// Ends the sessions opened with the sign-in token whose hash is `owner` (the
    /// user signed out); returns how many.
    pub fn close_owned(&mut self, owner: &str) -> usize {
        let before = self.sessions.len();
        self.sessions.retain(|_, s| s.owner.as_deref() != Some(owner));
        before - self.sessions.len()
    }

    /// Ends all sessions of `user` (the account was deleted); returns how many.
    pub fn close_user(&mut self, user: &str) -> usize {
        let before = self.sessions.len();
        self.sessions.retain(|_, s| !s.user.eq_ignore_ascii_case(user.trim()));
        before - self.sessions.len()
    }

    /// Ends the sessions without request for `SESSION_TIMEOUT_SECS`.
    pub fn expire(&mut self, now: Instant) {
        let timeout = Duration::from_secs(SESSION_TIMEOUT_SECS);
        self.sessions.retain(|id, s| {
            let alive = now.saturating_duration_since(s.seen) < timeout;
            if !alive {
                log::info!("session {id} of {} expired", s.user);
            }
            alive
        });
    }

    fn entry(&mut self, project: &str, session: &Session) -> Result<&mut Entry, LockError> {
        match self.sessions.get_mut(&session.session) {
            Some(e) if e.project == project && e.user == session.user => Ok(e),
            _ => Err(LockError::Expired),
        }
    }

    /// Keeps the session alive.
    pub fn touch(&mut self, project: &str, session: &Session, now: Instant) -> Result<(), LockError> {
        self.entry(project, session)?.seen = now;
        Ok(())
    }

    /// All locks on `project`.
    pub fn locks(&self, project: &str) -> Vec<Lock> {
        let mut out: Vec<Lock> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.project == project)
            .flat_map(|(id, s)| s.locks.iter().map(|t| Lock { target: *t, user: s.user.clone(), session: id.clone() }))
            .collect();
        out.sort_by_key(|l| l.target);
        out
    }

    /// Users with a session on `project`.
    pub fn users(&self, project: &str) -> Vec<String> {
        let users: BTreeSet<&String> = self.sessions.values().filter(|s| s.project == project).map(|s| &s.user).collect();
        users.into_iter().cloned().collect()
    }

    /// Locks all `targets` or none of them.
    pub fn lock(&mut self, project: &str, session: &Session, targets: &[Target], now: Instant) -> Result<(), LockError> {
        self.touch(project, session, now)?;
        if let Some(held) = self.locks(project).into_iter().find(|l| l.session != session.session && targets.contains(&l.target)) {
            return Err(LockError::Held(held));
        }
        self.entry(project, session)?.locks.extend(targets.iter().copied());
        Ok(())
    }

    pub fn release(&mut self, project: &str, session: &Session, targets: &[Target], now: Instant) -> Result<(), LockError> {
        let entry = self.entry(project, session)?;
        entry.seen = now;
        for t in targets {
            entry.locks.remove(t);
        }
        Ok(())
    }

    /// Ends a session and releases its locks.
    pub fn close(&mut self, project: &str, session: &Session) -> Result<(), LockError> {
        self.entry(project, session)?;
        self.sessions.remove(&session.session);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
