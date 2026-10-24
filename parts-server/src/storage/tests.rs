use super::*;
use openecraft_parts::portal::{Lifecycle, Role, User};

pub(crate) fn temp_file(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("openecraft-test-{}-{nanos}", std::process::id())).join(name)
}

fn part(number: &str) -> Part {
    Part { number: number.into(), description: format!("Test {number}"), ..Default::default() }
}

/// The same store rules on both backends.
fn stores() -> Vec<Store> {
    vec![
        Store::with_backend(Box::new(JsonBackend::in_memory()), default_catalog()).unwrap(),
        Store::with_backend(Box::new(SqliteBackend::in_memory().unwrap()), default_catalog()).unwrap(),
    ]
}

#[test]
fn new_stores_are_seeded() {
    for store in stores() {
        assert_eq!(store.revision, 1);
        assert_eq!(store.count().unwrap(), default_catalog().len());
        assert!(store.all().unwrap().iter().all(|p| p.revision == 1 && p.modified > 0));
    }
}

#[test]
fn changes_get_revisions_and_history() {
    for mut store in stores() {
        let stored = store.put_parts("alice", vec![part("TEST.1")]).unwrap();
        assert_eq!((stored[0].0.revision, stored[0].1), (2, true));
        assert_eq!(store.revision, 2);
        // Unchanged parts are not stored again.
        let same = store.put_parts("bob", vec![part("TEST.1")]).unwrap();
        assert_eq!((same[0].0.revision, same[0].1), (2, false));
        assert_eq!(store.revision, 2);
        let changed = Part { description: "changed".into(), ..part("TEST.1") };
        store.put_parts("bob", vec![changed]).unwrap();
        assert!(store.delete("carol", "TEST.1").unwrap());
        assert!(!store.delete("carol", "TEST.1").unwrap());
        assert_eq!(store.revision, 4);

        let history = store.history("TEST.1").unwrap();
        let summary: Vec<(u64, &str, ChangeKind)> = history.iter().map(|h| (h.revision, h.user.as_str(), h.change)).collect();
        assert_eq!(summary, [(4, "carol", ChangeKind::Deleted), (3, "bob", ChangeKind::Changed), (2, "alice", ChangeKind::Created)]);
        assert_eq!(store.history_part("TEST.1", 2).unwrap().unwrap().description, "Test TEST.1");
        assert_eq!(store.history_part("TEST.1", 4).unwrap().unwrap().description, "changed");
        assert!(store.history_part("TEST.1", 9).unwrap().is_none());
    }
}

#[test]
fn history_is_limited() {
    for mut store in stores() {
        for i in 0..HISTORY_LIMIT + 5 {
            store.put_parts("u", vec![Part { note: i.to_string(), ..part("TEST.1") }]).unwrap();
        }
        let history = store.history("TEST.1").unwrap();
        assert_eq!(history.len(), HISTORY_LIMIT);
        assert_eq!(history[0].revision, store.revision);
    }
}

#[test]
fn import_with_duplicates_is_one_revision() {
    for mut store in stores() {
        let parts = vec![part("A"), part("B"), Part { note: "second".into(), ..part("A") }];
        let out = store.put_parts("u", parts).unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(store.revision, 2);
        assert_eq!(store.get("A").unwrap().unwrap().note, "second");
        assert_eq!(store.count().unwrap(), default_catalog().len() + 2);
    }
}

#[test]
fn search_uses_the_shared_query() {
    for mut store in stores() {
        let p = Part { manufacturer: "ACME".into(), voltage: "24 V".into(), catalog: "acme_2026".into(), ..part("ACME.1") };
        store.put_parts("u", vec![p, Part { lifecycle: Lifecycle::Discontinued, ..part("ACME.2") }]).unwrap();
        let q = PartQuery { q: Some("acme_2026".into()), ..Default::default() };
        let r = store.search(&q).unwrap();
        assert_eq!(r.total, 1);
        assert_eq!(r.facet("Voltage").unwrap().values[0].value, "24 V");
        let q = PartQuery { numbers: vec!["ACME.2".into(), "GEN.CT-9A-24DC".into()], brief: true, ..Default::default() };
        let r = store.search(&q).unwrap();
        assert_eq!(r.total, 2);
        assert!(r.parts.iter().all(|p| p.model_3d.is_none()));
        // Words with LIKE wildcards are looked up literally.
        assert_eq!(store.list(&PartQuery { q: Some("%".into()), ..Default::default() }).unwrap().0, 0);
        assert_eq!(store.list(&PartQuery { q: Some("acme_".into()), ..Default::default() }).unwrap().0, 1);
    }
}

#[test]
fn models_are_kept_with_the_part() {
    for mut store in stores() {
        let full = default_catalog().into_iter().find(|p| p.model_3d.is_some() || p.model_2d.is_some()).expect("catalog part with a model");
        let got = store.get(&full.number).unwrap().unwrap();
        assert_eq!(content(&got), content(&full));
        let listed = store.list(&PartQuery { numbers: vec![full.number.clone()], ..Default::default() }).unwrap().1;
        assert_eq!(content(&listed[0]), content(&full));
        store.put_parts("u", vec![Part { model_3d: None, model_2d: None, ..full.clone() }]).unwrap();
        assert!(store.get(&full.number).unwrap().unwrap().model_3d.is_none());
    }
}

#[test]
fn catalogs_count_their_parts() {
    for mut store in stores() {
        let c = Catalog { id: "acme".into(), name: "ACME".into(), manufacturer: "ACME".into(), version: "1".into(), ..Default::default() };
        let stored = store.put_catalog(c).unwrap();
        assert!(stored.modified > 0);
        store.put_parts("u", vec![Part { catalog: "ACME".into(), ..part("A") }, Part { catalog: "acme".into(), ..part("B") }]).unwrap();
        assert_eq!(store.catalog("Acme").unwrap().unwrap().parts, 2);
        let rev = store.revision;
        store.delete_catalog("ACME").unwrap();
        assert!(store.catalogs().unwrap().is_empty());
        assert_eq!(store.revision, rev + 1);
    }
}

#[test]
fn accounts_sessions_and_favorites() {
    for mut store in stores() {
        let mut account = Account { user: User { name: "ann".into(), role: Role::Editor, manufacturer: String::new() }, ..Default::default() };
        let token = account.start_session();
        let rev = store.revision;
        store.put_account(account).unwrap();
        assert_eq!(store.revision, rev, "accounts do not change the parts revision");
        assert_eq!(store.session(&token).unwrap().user.name, "ann");
        assert!(store.account("ANN").is_some());
        store.set_favorites("ann", vec!["A".into(), "B".into()]).unwrap();
        assert_eq!(store.favorites("ann").unwrap(), ["A", "B"]);
        assert!(store.favorites("bob").unwrap().is_empty());
        store.delete_account("ann").unwrap();
        assert!(store.session(&token).is_none());
        assert!(store.favorites("ann").unwrap().is_empty());
    }
}

#[test]
fn attachments_store_files() {
    for mut store in stores() {
        store.put_parts("u", vec![part("A")]).unwrap();
        let p = store.attach("u", "A", "sheet.pdf", None, b"%PDF-1".to_vec()).unwrap().unwrap();
        assert_eq!(p.attachments.len(), 1);
        let a = p.attachments[0].clone();
        assert_eq!((a.name.as_str(), a.kind, a.size), ("sheet.pdf", AttachmentKind::Datasheet, 6));
        assert_eq!(store.file(&a.id).unwrap().unwrap().data, b"%PDF-1");
        // Same name: replaced.
        let p = store.attach("u", "A", "sheet.pdf", Some(AttachmentKind::Manual), b"%PDF-2".to_vec()).unwrap().unwrap();
        assert_eq!(p.attachments.len(), 1);
        assert_eq!(p.attachments[0].kind, AttachmentKind::Manual);
        assert!(store.attach("u", "NOPE", "x.pdf", None, vec![1]).unwrap().is_none());
        let p = store.detach("u", "A", &p.attachments[0].id).unwrap().unwrap();
        assert!(p.attachments.is_empty());
        assert!(store.detach("u", "A", "unknown").unwrap().is_none());
        // Files of older versions stay available.
        assert!(store.file(&a.id).unwrap().is_some());
        assert!(store.file("unknown").unwrap().is_none());
        assert_eq!(store.history("A").unwrap().len(), 4);
    }
}

#[test]
fn migrate_copies_everything() {
    let mut store = Store::with_backend(Box::new(JsonBackend::in_memory()), vec![part("A")]).unwrap();
    store.put_catalog(Catalog { id: "c".into(), ..Default::default() }).unwrap();
    store.attach("u", "A", "a.pdf", None, b"data".to_vec()).unwrap();
    let mut account = Account { user: User { name: "ann".into(), ..Default::default() }, ..Default::default() };
    let token = account.start_session();
    store.put_account(account).unwrap();
    store.set_favorites("ann", vec!["A".into()]).unwrap();
    let from = store.into_backend();
    let mut to = SqliteBackend::in_memory().unwrap();
    assert_eq!(migrate(from.as_ref(), &mut to).unwrap(), 1);
    let copy = Store::with_backend(Box::new(to), Vec::new()).unwrap();
    assert_eq!(copy.revision, from.revision());
    assert_eq!(copy.get("A").unwrap(), from.part("A").unwrap());
    assert_eq!(copy.history("A").unwrap().len(), 1);
    assert_eq!(copy.catalogs().unwrap().len(), 1);
    assert!(copy.session(&token).is_some());
    assert_eq!(copy.favorites("ann").unwrap(), ["A"]);
    let id = &copy.get("A").unwrap().unwrap().attachments[0].id;
    assert_eq!(copy.file(id).unwrap().unwrap().data, b"data");
}

#[test]
fn storage_kind_from_the_file_name() {
    let p = std::path::Path::new;
    assert_eq!(StorageKind::resolve(None, p("db/parts-db.json")), StorageKind::Json);
    assert_eq!(StorageKind::resolve(None, p("parts.JSON")), StorageKind::Json);
    assert_eq!(StorageKind::resolve(None, p("parts-db.sqlite")), StorageKind::Sqlite);
    assert_eq!(StorageKind::resolve(Some(StorageKind::Json), p("parts.db")), StorageKind::Json);
}
