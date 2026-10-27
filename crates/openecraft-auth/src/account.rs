//! Password hashes, session tokens (stored as SHA-256) and content ids.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use sha2::{Digest, Sha256};

/// Sessions kept per user; the oldest is dropped when a new one starts.
pub const MAX_SESSIONS: usize = 20;

/// Shortest password the servers accept for local accounts.
pub const MIN_PASSWORD_LEN: usize = 8;

/// A password a server may set for a local account: long enough, not only blanks.
pub fn check_password(password: &str) -> Result<(), String> {
    if password.trim().chars().count() < MIN_PASSWORD_LEN {
        return Err(format!("the password needs at least {MIN_PASSWORD_LEN} characters"));
    }
    Ok(())
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

/// Starts a session in a list of token hashes; returns the token for the client.
pub fn start_session(sessions: &mut Vec<String>) -> String {
    let token = new_token();
    sessions.push(sha256_hex(token.as_bytes()));
    if sessions.len() > MAX_SESSIONS {
        sessions.remove(0);
    }
    token
}

pub fn has_session(sessions: &[String], token: &str) -> bool {
    let hash = sha256_hex(token.as_bytes());
    sessions.iter().any(|s| openecraft_parts::api::token_matches(s, &hash))
}

pub fn end_session(sessions: &mut Vec<String>, token: &str) -> bool {
    let hash = sha256_hex(token.as_bytes());
    let before = sessions.len();
    sessions.retain(|s| *s != hash);
    sessions.len() != before
}

#[cfg(test)]
mod tests;
