use super::*;
use openecraft_core::collab::ProjectRole;

#[test]
fn accounts_are_stored_and_found() {
    let dir = std::env::temp_dir().join(format!("oec-users-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut users = Users::open(&dir).unwrap();
    assert!(users.accounts.is_empty());
    assert!(users.set_admin("Anna", "pw").is_err(), "too short");
    users.set_admin("Anna", "long-password").unwrap();
    let token = openecraft_auth::start_session(&mut users.find_mut("anna").unwrap().sessions, openecraft_auth::unix_now());
    users.save().unwrap();
    let back = Users::open(&dir).unwrap();
    let a = back.by_token(&token).unwrap();
    assert_eq!((a.user.name.as_str(), a.user.role), ("Anna", Some(ProjectRole::Admin)));
    assert!(openecraft_auth::verify_password("long-password", &a.password));
    assert!(back.by_token("nope").is_none());
    std::fs::write(dir.join("users.json"), "not json").unwrap();
    assert!(Users::open(&dir).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
