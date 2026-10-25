use super::*;
use openecraft_client::vault::{Pagination, UserInfo, VaultInfo};

fn page(names: &[&str]) -> Collection<Entry> {
    let results = names.iter().map(|n| Entry::File(FileVersion { name: n.to_string(), ..Default::default() })).collect();
    Collection { pagination: Pagination { total_results: 1200, next_url: Some("/next".into()) }, results }
}

fn poll_with(v: &mut Vault, replies: Vec<Reply>) -> Vec<Reply> {
    v.pending += replies.len();
    v.inbox.lock().unwrap().extend(replies);
    v.poll(&egui::Context::default())
}

#[test]
fn uses_follow_the_file_name() {
    assert_eq!(Use::guess("Plant 1.oec"), Use::Project);
    assert_eq!(Use::guess("Siemens.EDZ"), Use::Parts);
    assert_eq!(Use::guess("motor.STEP"), Use::Model);
    assert_eq!(Use::guess("housing.jt"), Use::Model);
    assert_eq!(Use::guess("old.dwg"), Use::Drawing);
    assert_eq!(Use::guess("manual.pdf"), Use::Document);
    assert_eq!(Use::guess("bom.csv"), Use::Integration);
    assert_eq!(Use::guess("README"), Use::Save);
    for u in Use::ALL {
        assert!(!u.title().is_empty());
    }
}

#[test]
fn signing_in_takes_the_session() {
    let mut v = Vault { path: vec![("7".into(), "Old".into())], ..Default::default() };
    let session = Session {
        access_token: "V:abc".into(),
        vault_information: Some(VaultInfo { id: "101".into(), name: "Engineering".into() }),
        user_information: Some(UserInfo { name: "anna".into(), ..Default::default() }),
        ..Default::default()
    };
    poll_with(&mut v, vec![Reply::SignedIn(Ok(session))]);
    assert!(v.signed_in);
    assert_eq!((v.token.as_str(), v.vault_id.as_str(), v.vault_name.as_str(), v.user.as_str()), ("V:abc", "101", "Engineering", "anna"));
    // The root folder is listed.
    assert!(v.path.is_empty());
    assert!(v.busy());
    // A session without a vault is no sign-in.
    let mut w = Vault::default();
    poll_with(&mut w, vec![Reply::SignedIn(Ok(Session { access_token: "x".into(), ..Default::default() }))]);
    assert!(!w.signed_in);
}

#[test]
fn listings_of_other_folders_are_dropped() {
    let mut v = Vault { path: vec![("7".into(), "Projects".into())], ..Default::default() };
    assert_eq!((v.folder().as_str(), v.folder_path().as_str()), ("7", "$/Projects"));
    poll_with(&mut v, vec![Reply::Contents(ROOT.into(), Ok(page(&["a.oec"])))]);
    assert!(v.entries.is_empty());
    v.selected = Some(0);
    poll_with(&mut v, vec![Reply::Contents("7".into(), Ok(page(&["Plant.oec", "Panel.stp"])))]);
    assert_eq!(v.entries.len(), 2);
    assert_eq!((v.total, v.selected), (1200, None));
    assert!(v.status.contains("$/Projects"), "{}", v.status);
}

#[test]
fn errors_are_shown() {
    let mut v = Vault::default();
    let replies = poll_with(&mut v, vec![Reply::Uploaded(true, Err("The file is checked out by ben".into()))]);
    assert_eq!(replies.len(), 1);
    assert!(v.status.contains("checked out by ben"), "{}", v.status);
    assert!(!v.busy());
}
