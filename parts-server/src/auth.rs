//! User accounts of the parts server: a user with credentials (see `openecraft_auth`
//! for the hashes and session tokens).

pub use openecraft_auth::{hash_password, sha256_hex, verify_account_password};
use openecraft_auth::{unix_now, SessionKey};
use openecraft_parts::portal::User;
use serde::{Deserialize, Serialize};

/// A user with credentials, as stored by the server.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Account {
    pub user: User,
    /// Argon2 hash (PHC string).
    pub password: String,
    /// SHA-256 of the session tokens with their start, oldest first.
    pub sessions: Vec<SessionKey>,
}

impl Account {
    /// Starts a session; returns the token for the client.
    pub fn start_session(&mut self) -> String {
        openecraft_auth::start_session(&mut self.sessions, unix_now())
    }

    pub fn has_session(&self, token: &str) -> bool {
        openecraft_auth::has_session(&self.sessions, token, unix_now())
    }

    pub fn end_session(&mut self, token: &str) -> bool {
        openecraft_auth::end_session(&mut self.sessions, token)
    }
}

#[cfg(test)]
mod tests;
