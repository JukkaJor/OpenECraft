//! User accounts of the project server, stored in `users.json` of the data directory
//! (written atomically on every change): the user with role, projects and groups, the
//! password hash (none for users of a directory) and the hashes of the session tokens.

use openecraft_core::collab::ProjectUser;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

const FILE: &str = "users.json";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Account {
    pub user: ProjectUser,
    /// Argon2 hash (PHC string).
    pub password: String,
    /// SHA-256 of the session tokens, oldest first.
    pub sessions: Vec<String>,
}

#[derive(Default)]
pub struct Users {
    path: Option<PathBuf>,
    pub accounts: Vec<Account>,
}

impl Users {
    /// The accounts of a data directory (none if it has no user file).
    pub fn open(dir: &Path) -> io::Result<Users> {
        let path = dir.join(FILE);
        let accounts = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", path.display())))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        Ok(Users { path: Some(path), accounts })
    }

    pub fn save(&self) -> io::Result<()> {
        let Some(path) = &self.path else { return Ok(()) };
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&self.accounts).unwrap_or_default())?;
        std::fs::rename(&tmp, path)
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut Account> {
        self.accounts.iter_mut().find(|a| a.user.name.eq_ignore_ascii_case(name.trim()))
    }

    /// The account a session token belongs to.
    pub fn by_token(&self, token: &str) -> Option<&Account> {
        self.accounts.iter().find(|a| openecraft_auth::has_session(&a.sessions, token))
    }

    /// Creates or replaces the account of an administrator (start-up option).
    pub fn set_admin(&mut self, name: &str, password: &str) -> Result<(), String> {
        openecraft_auth::check_password(password)?;
        let hash = openecraft_auth::hash_password(password)?;
        let user = ProjectUser { name: name.trim().to_string(), role: Some(openecraft_core::collab::ProjectRole::Admin), ..Default::default() };
        match self.find_mut(name) {
            Some(a) => {
                a.user = user;
                a.password = hash;
                a.sessions.clear();
            }
            None => self.accounts.push(Account { user, password: hash, sessions: Vec::new() }),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
