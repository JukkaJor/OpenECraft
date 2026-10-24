//! User accounts: password hashes (Argon2), session tokens (stored as SHA-256) and
//! content ids of files.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use openecraft_parts::portal::User;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Sessions kept per user; the oldest is dropped when a new one starts.
const MAX_SESSIONS: usize = 20;

/// A user with credentials, as stored by the server.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Account {
    pub user: User,
    /// Argon2 hash (PHC string).
    pub password: String,
    /// SHA-256 of the session tokens, oldest first.
    pub sessions: Vec<String>,
}

impl Account {
    /// Starts a session; returns the token for the client.
    pub fn start_session(&mut self) -> String {
        let token = new_token();
        self.sessions.push(sha256_hex(token.as_bytes()));
        if self.sessions.len() > MAX_SESSIONS {
            self.sessions.remove(0);
        }
        token
    }

    pub fn has_session(&self, token: &str) -> bool {
        let hash = sha256_hex(token.as_bytes());
        self.sessions.iter().any(|s| openecraft_parts::api::token_matches(s, &hash))
    }

    pub fn end_session(&mut self, token: &str) -> bool {
        let hash = sha256_hex(token.as_bytes());
        let before = self.sessions.len();
        self.sessions.retain(|s| *s != hash);
        self.sessions.len() != before
    }
}

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut password_hash::rand_core::OsRng);
    Argon2::default().hash_password(password.as_bytes(), &salt).map(|h| h.to_string()).map_err(|e| e.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
}

/// A random token (256 bit, hex).
pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("no random numbers available");
    hex(&bytes)
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests;
