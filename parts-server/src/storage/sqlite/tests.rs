use super::*;
use crate::storage::tests::temp_file;
use crate::storage::Store;
use openecraft_parts::default_catalog;

#[test]
fn database_file_persists() {
    let path = temp_file("parts.sqlite");
    let mut store = Store::with_backend(Box::new(SqliteBackend::open(&path).unwrap()), default_catalog()).unwrap();
    store.put_parts("u", vec![Part { number: "TEST.1".into(), catalog: "c".into(), ..Default::default() }]).unwrap();
    store.attach("u", "TEST.1", "a.pdf", None, b"data".to_vec()).unwrap();
    let rev = store.revision;
    drop(store);

    let again = Store::with_backend(Box::new(SqliteBackend::open(&path).unwrap()), default_catalog()).unwrap();
    assert_eq!(again.revision, rev);
    assert_eq!(again.count().unwrap(), default_catalog().len() + 1);
    let p = again.get("TEST.1").unwrap().unwrap();
    assert_eq!(again.file(&p.attachments[0].id).unwrap().unwrap().name, "a.pdf");
    drop(again);
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn failed_commits_change_nothing() {
    let mut db = SqliteBackend::in_memory().unwrap();
    db.commit(Change { revision: 1, parts: vec![Part { number: "A".into(), ..Default::default() }], ..Default::default() }).unwrap();
    db.conn.execute_batch("CREATE TRIGGER fail BEFORE INSERT ON catalogs BEGIN SELECT RAISE(ABORT, 'no'); END;").unwrap();
    let change = Change {
        revision: 2,
        parts: vec![Part { number: "B".into(), ..Default::default() }],
        catalogs: vec![Catalog { id: "x".into(), ..Default::default() }],
        ..Default::default()
    };
    assert!(db.commit(change).is_err());
    assert_eq!(db.revision(), 1);
    assert_eq!(db.count().unwrap(), 1);
}

#[test]
fn like_patterns_are_literal() {
    assert_eq!(like("a%b_c\\"), "%a\\%b\\_c\\\\%");
}
