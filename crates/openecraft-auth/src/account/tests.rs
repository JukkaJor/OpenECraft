use super::*;

const NOW: u64 = 1_800_000_000;

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
fn unknown_accounts_check_a_password_too() {
    let hash = hash_password("s3cret").unwrap();
    assert!(verify_account_password("s3cret", Some(&hash)));
    assert!(!verify_account_password("s3cret", None));
    // The dummy hash is a real Argon2 hash, so both answers take the same time.
    let start = std::time::Instant::now();
    verify_account_password("x", None);
    let unknown = start.elapsed();
    let start = std::time::Instant::now();
    verify_account_password("x", Some(&hash));
    let known = start.elapsed();
    assert!(unknown * 4 > known, "unknown {unknown:?}, known {known:?}");
}

#[test]
fn sessions_store_only_token_hashes() {
    let mut sessions = Vec::new();
    let token = start_session(&mut sessions, NOW);
    assert_eq!(token.len(), 64);
    assert!(sessions.iter().all(|s| s.hash != token));
    assert!(has_session(&sessions, &token, NOW));
    assert!(!has_session(&sessions, &new_token(), NOW));
    assert!(end_session(&mut sessions, &token));
    assert!(!has_session(&sessions, &token, NOW));
    assert!(!end_session(&mut sessions, &token));
}

#[test]
fn sessions_expire() {
    let mut sessions = Vec::new();
    let token = start_session(&mut sessions, NOW);
    assert!(has_session(&sessions, &token, NOW + SESSION_LIFETIME_SECS - 1));
    assert!(!has_session(&sessions, &token, NOW + SESSION_LIFETIME_SECS));
    // A new sign-in drops the expired sessions.
    start_session(&mut sessions, NOW + SESSION_LIFETIME_SECS);
    assert_eq!(sessions.len(), 1);
}

#[test]
fn sessions_stored_without_start_are_expired() {
    let token = new_token();
    let stored = format!("[\"{}\"]", sha256_hex(token.as_bytes()));
    let sessions: Vec<SessionKey> = serde_json::from_str(&stored).unwrap();
    assert_eq!(sessions[0].started, 0);
    assert!(!has_session(&sessions, &token, NOW));
    let back: Vec<SessionKey> = serde_json::from_str(&serde_json::to_string(&[SessionKey { hash: "h".into(), started: NOW }]).unwrap()).unwrap();
    assert_eq!(back, vec![SessionKey { hash: "h".into(), started: NOW }]);
}

#[test]
fn old_sessions_are_dropped() {
    let mut sessions = Vec::new();
    let first = start_session(&mut sessions, NOW);
    for _ in 0..MAX_SESSIONS {
        start_session(&mut sessions, NOW);
    }
    assert_eq!(sessions.len(), MAX_SESSIONS);
    assert!(!has_session(&sessions, &first, NOW));
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
