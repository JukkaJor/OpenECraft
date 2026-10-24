use super::*;

#[test]
fn passwords_are_hashed_and_verified() {
    let hash = hash_password("s3cret").unwrap();
    assert!(hash.starts_with("$argon2"));
    assert!(verify_password("s3cret", &hash));
    assert!(!verify_password("S3cret", &hash));
    assert!(!verify_password("s3cret", "not a hash"));
    assert_ne!(hash_password("s3cret").unwrap(), hash, "salted");
}

#[test]
fn sessions_store_only_token_hashes() {
    let mut account = Account::default();
    let token = account.start_session();
    assert_eq!(token.len(), 64);
    assert!(!account.sessions.contains(&token));
    assert!(account.has_session(&token));
    assert!(!account.has_session(&new_token()));
    assert!(account.end_session(&token));
    assert!(!account.has_session(&token));
    assert!(!account.end_session(&token));
}

#[test]
fn old_sessions_are_dropped() {
    let mut account = Account::default();
    let first = account.start_session();
    for _ in 0..MAX_SESSIONS {
        account.start_session();
    }
    assert_eq!(account.sessions.len(), MAX_SESSIONS);
    assert!(!account.has_session(&first));
}

#[test]
fn content_ids() {
    assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
}
