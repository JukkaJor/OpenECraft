use super::*;

#[test]
fn accounts_keep_their_sessions() {
    let mut account = Account::default();
    let token = account.start_session();
    assert!(account.has_session(&token));
    assert!(account.sessions.iter().all(|s| s.hash != token), "only the hash is stored");
    assert!(account.end_session(&token));
    assert!(!account.has_session(&token));
}
