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
    let mut sessions = Vec::new();
    let token = start_session(&mut sessions);
    assert_eq!(token.len(), 64);
    assert!(!sessions.contains(&token));
    assert!(has_session(&sessions, &token));
    assert!(!has_session(&sessions, &new_token()));
    assert!(end_session(&mut sessions, &token));
    assert!(!has_session(&sessions, &token));
    assert!(!end_session(&mut sessions, &token));
}

#[test]
fn old_sessions_are_dropped() {
    let mut sessions = Vec::new();
    let first = start_session(&mut sessions);
    for _ in 0..MAX_SESSIONS {
        start_session(&mut sessions);
    }
    assert_eq!(sessions.len(), MAX_SESSIONS);
    assert!(!has_session(&sessions, &first));
}

#[test]
fn content_ids() {
    assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
}

#[test]
fn short_passwords_are_refused() {
    assert!(check_password("1234567").is_err());
    assert!(check_password("        ").is_err());
    assert!(check_password("correct horse").is_ok());
}
