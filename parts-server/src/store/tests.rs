use super::*;

fn temp_file(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("openecraft-test-{}-{nanos}", std::process::id())).join(name)
}

#[test]
fn new_database_is_seeded_and_changes_persist() {
    let path = temp_file("parts.json");
    let mut store = Store::open(&path).unwrap();
    assert!(path.exists());
    assert_eq!(store.parts.len(), default_catalog().len());
    let rev = store.revision;

    let part = Part { number: "TEST.1".into(), description: "Test part".into(), ..Default::default() };
    assert!(store.upsert(part.clone()).unwrap());
    assert!(!store.upsert(part).unwrap());
    assert!(store.delete("GEN.CT-9A-24DC").unwrap());
    assert!(!store.delete("does-not-exist").unwrap());
    assert_eq!(store.revision, rev + 3);

    // Reopen: everything is still there, no temporary file left.
    let again = Store::open(&path).unwrap();
    assert_eq!(again.revision, rev + 3);
    assert_eq!(again.get("TEST.1").unwrap().description, "Test part");
    assert!(again.get("GEN.CT-9A-24DC").is_none());
    assert!(!path.with_extension("tmp").exists());
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn import_is_one_revision() {
    let mut store = Store::in_memory(Vec::new());
    let parts = vec![
        Part { number: "A".into(), ..Default::default() },
        Part { number: "B".into(), ..Default::default() },
    ];
    assert_eq!(store.import(parts).unwrap(), 2);
    assert_eq!(store.revision, 2);
    assert_eq!(store.import(Vec::new()).unwrap(), 0);
    assert_eq!(store.revision, 2);
}

#[test]
fn invalid_file_is_an_error() {
    let path = temp_file("broken.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "{ not json").unwrap();
    let err = Store::open(&path).err().unwrap();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn failed_save_keeps_the_previous_state() {
    // A directory where the file should be: writing succeeds, replacing fails.
    let dir = temp_file("db.json");
    std::fs::create_dir_all(&dir).unwrap();
    let part = Part { number: "TEST.1".into(), ..Default::default() };
    let mut store = Store { path: Some(dir.clone()), revision: 5, parts: vec![part.clone()] };
    assert!(store.upsert(Part { description: "changed".into(), ..part.clone() }).is_err());
    assert!(store.delete("TEST.1").is_err());
    assert!(store.import(vec![Part { number: "TEST.2".into(), ..Default::default() }]).is_err());
    assert_eq!(store.parts, vec![part]);
    assert_eq!(store.revision, 5);
    std::fs::remove_dir_all(dir.parent().unwrap()).ok();
}

#[test]
fn import_replaces_parts_in_place() {
    let mut store = Store::in_memory(vec![
        Part { number: "A".into(), description: "old".into(), ..Default::default() },
        Part { number: "B".into(), ..Default::default() },
    ]);
    let new = vec![
        Part { number: "C".into(), ..Default::default() },
        Part { number: "A".into(), description: "new".into(), ..Default::default() },
        Part { number: "C".into(), description: "last wins".into(), ..Default::default() },
    ];
    assert_eq!(store.import(new).unwrap(), 3);
    let numbers: Vec<&str> = store.parts.iter().map(|p| p.number.as_str()).collect();
    assert_eq!(numbers, ["A", "B", "C"]);
    assert_eq!(store.parts[0].description, "new");
    assert_eq!(store.parts[2].description, "last wins");
}
