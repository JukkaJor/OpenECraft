use super::*;
use crate::storage::tests::temp_file;
use crate::storage::Store;
use openecraft_parts::default_catalog;

#[test]
fn new_database_is_seeded_and_changes_persist() {
    let path = temp_file("parts.json");
    let mut store = Store::with_backend(Box::new(JsonBackend::open(&path).unwrap()), default_catalog()).unwrap();
    assert!(path.exists());
    let part = Part { number: "TEST.1".into(), description: "Test part".into(), ..Default::default() };
    store.put_parts("u", vec![part]).unwrap();
    store.attach("u", "TEST.1", "a.pdf", None, b"data".to_vec()).unwrap();
    assert!(store.delete("u", "GEN.CT-9A-24DC").unwrap());
    let rev = store.revision;

    // Reopen: everything is still there, no temporary file left.
    let again = Store::with_backend(Box::new(JsonBackend::open(&path).unwrap()), Vec::new()).unwrap();
    assert_eq!(again.revision, rev);
    let p = again.get("TEST.1").unwrap().unwrap();
    assert_eq!(p.description, "Test part");
    assert_eq!(again.file(&p.attachments[0].id).unwrap().unwrap().data, b"data");
    assert!(path.with_file_name("parts.files").join(&p.attachments[0].id).exists());
    assert!(again.get("GEN.CT-9A-24DC").unwrap().is_none());
    assert_eq!(again.history("TEST.1").unwrap().len(), 2);
    assert!(!path.with_extension("tmp").exists());
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn databases_of_earlier_versions_are_read() {
    let path = temp_file("old.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, r#"{"revision": 0, "parts": [{"number": "OLD.1"}]}"#).unwrap();
    let store = Store::with_backend(Box::new(JsonBackend::open(&path).unwrap()), default_catalog()).unwrap();
    // Not seeded again: the database already has parts.
    assert_eq!(store.count().unwrap(), 1);
    assert_eq!(store.revision, 1);
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn broken_files_are_reported() {
    let path = temp_file("broken.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "{ not json").unwrap();
    let e = JsonBackend::open(&path).err().unwrap();
    assert_eq!(e.kind(), io::ErrorKind::InvalidData);
    assert!(e.to_string().contains("broken.json"));
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}
