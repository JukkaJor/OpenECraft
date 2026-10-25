use super::*;
use openecraft_parts::access::{Grant, Subject};
use openecraft_parts::portal::PartsRight;

fn dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("oec-access-{}-{:?}", std::process::id(), std::thread::current().id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn missing_files_give_the_default_and_changes_are_stored() {
    let dir = dir();
    let path = dir.join("access.json");
    let _ = std::fs::remove_file(&path);
    let default = AccessConfig::<PartsRight> { policy: Policy { anonymous: "reader".into(), ..Default::default() }, ..Default::default() };
    let mut file = AccessFile::open(&path, default).unwrap();
    assert_eq!(file.config.policy.anonymous, "reader");
    assert!(!path.exists(), "written on the first change only");
    let policy = Policy { grants: vec![Grant::new(Subject::Group("corp:Purchasing".into()), "editor", "ABB")], ..Default::default() };
    file.set_policy(policy.clone()).unwrap();
    let back = AccessFile::<PartsRight>::open(&path, AccessConfig::default()).unwrap();
    assert_eq!(back.config.policy, policy);
    let bad = Policy { grants: vec![Grant::new(Subject::User("x".into()), "nobody", "*")], ..Default::default() };
    assert!(file.set_policy(bad).is_err());
    assert_eq!(file.config.policy, policy, "unchanged");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn files_are_checked() {
    let dir = dir();
    let path = dir.join("bad.json");
    let d = r#"{"id":"corp","url":"ldaps://dc","bind_dn":"CN=svc","base_dn":"DC=corp"}"#;
    std::fs::write(&path, format!(r#"{{"directories":[{d}, {d}]}}"#)).unwrap();
    let err = AccessFile::<PartsRight>::open(&path, AccessConfig::default()).unwrap_err();
    assert!(err.to_string().contains("defined twice"), "{err}");
    std::fs::write(&path, r#"{"grants":[{"subject":{"user":"a"},"role":"nobody"}]}"#).unwrap();
    assert!(AccessFile::<PartsRight>::open(&path, AccessConfig::default()).is_err());
    std::fs::write(&path, "not json").unwrap();
    assert!(AccessFile::<PartsRight>::open(&path, AccessConfig::default()).is_err());
    std::fs::write(&path, format!(r#"{{"directories":[{d}],"anonymous":"reader","roles":[{{"name":"buyer","rights":["read"]}}]}}"#)).unwrap();
    let file = AccessFile::<PartsRight>::open(&path, AccessConfig::default()).unwrap();
    let settings = file.settings();
    assert_eq!(settings.providers.len(), 1);
    assert_eq!(settings.policy.roles[0].name, "buyer");
    assert!(!serde_json::to_string(&settings).unwrap().contains("CN=svc"), "directory settings stay on the server");
    let _ = std::fs::remove_dir_all(&dir);
}
