use super::*;
use crate::sample::demo_project;

fn project(name: &str, company: &str) -> Project {
    let mut prj = Project::new_default();
    prj.info.name = name.into();
    prj.info.company = company.into();
    prj
}

#[test]
fn entries_are_read_from_files_and_archives() {
    let prj = demo_project();
    let e = Entry::read("/p/demo.oec", prj.to_json().as_bytes(), true);
    assert_eq!((e.name.as_str(), e.pages, e.format, e.archive), ("Demo conveyor", prj.pages.len(), crate::model::FORMAT, false));
    assert!(e.devices > 3);
    assert_eq!(e.verified, Some((0, 0)));
    let z = archive::archive(&prj, &Default::default());
    let a = Entry::read("/p/demo.oecz", &z, false);
    assert!(a.archive && a.verified.is_none() && a.error.is_empty());
    let bad = Entry::read("/p/bad.oec", b"nope", false);
    assert!(!bad.error.is_empty());
    assert!(is_project_file("/x/A.OEC") && is_project_file("b.oecz") && !is_project_file("c.json"));
}

#[test]
fn queries_match_text_fields_and_properties() {
    let mut prj = project("Line 4 conveyor", "ACME");
    prj.info.properties.insert("customer".into(), "Big Plant Ltd".into());
    let e = Entry::of("/jobs/2026/line4.oec", &prj, false);
    assert!(e.matches(""));
    assert!(e.matches("conveyor"));
    assert!(e.matches("plant"));
    assert!(e.matches("company=acme"));
    assert!(!e.matches("company=AC"));
    assert!(e.matches("company~AC; customer~big"));
    assert!(e.matches("path~2026"));
    assert!(!e.matches("conveyor; company=Other"));
}

#[test]
fn database_keeps_one_entry_per_file() {
    let mut db = Database::default();
    db.update_folder("/a", vec![Entry::of("/a/2.oec", &project("B", ""), false), Entry::of("/a/1.oec", &project("A", ""), false)]);
    db.upsert(Entry::of("/b/3.oec", &project("A", "X"), false));
    assert_eq!(db.entries.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(), vec!["/a/1.oec", "/b/3.oec", "/a/2.oec"]);
    db.update_folder("/a", vec![Entry::of("/a/1.oec", &project("A2", ""), false)]);
    assert_eq!(db.entries.len(), 2);
    assert_eq!(db.folders, vec!["/a"]);
    assert_eq!(db.search("company=x").len(), 1);
    let back = Database::from_json(&db.to_json()).unwrap();
    assert_eq!(back, db);
    db.remove_folder("/a");
    assert_eq!(db.entries.len(), 1);
    assert!(Database::from_json("[").is_err());
}

#[test]
fn batch_sets_removes_replaces_and_renames() {
    let mut prj = project("Line 1", "ACME GmbH");
    prj.info.properties.insert("plant".into(), "North".into());
    prj.info.properties.insert("old".into(), "x".into());
    let mut b = Batch::default();
    assert!(b.is_empty());
    b.set.insert("creator".into(), "NS".into());
    b.set.insert("project.customer".into(), "Big Plant".into());
    b.remove.push("plant".into());
    b.replace.push(("*".into(), "ACME".into(), "Acme".into()));
    b.rename.push(("old".into(), "legacy".into()));
    let changes = b.apply(&mut prj).unwrap();
    assert_eq!(prj.info.creator, "NS");
    assert_eq!(prj.info.company, "Acme GmbH");
    assert_eq!(get(&prj, "project.customer"), "Big Plant");
    assert!(!prj.info.properties.contains_key("plant"));
    assert_eq!(prj.info.properties.get("legacy").map(String::as_str), Some("x"));
    assert!(changes.iter().any(|c| c.key == "company" && c.old == "ACME GmbH" && c.new == "Acme GmbH"));
    assert_eq!(changes.len(), 6);
    // Nothing changes the second time.
    assert!(b.apply(&mut prj).unwrap().len() <= 1);

    let bad = |f: fn(&mut Batch)| {
        let mut b = Batch::default();
        f(&mut b);
        b.apply(&mut project("A", "")).is_err()
    };
    assert!(bad(|b| b.remove.push("name".into())));
    assert!(bad(|b| b.rename.push(("name".into(), "x".into()))));
    assert!(bad(|b| b.replace.push(("name".into(), String::new(), "x".into()))));
}

#[test]
fn batch_rewrites_files_only_when_something_changes() {
    let prj = project("A", "ACME");
    let mut b = Batch::default();
    b.set.insert("company".into(), "ACME".into());
    let (changes, out) = b.apply_to_file(prj.to_json().as_bytes(), "").unwrap();
    assert!(changes.is_empty() && out.is_none());
    b.set.insert("company".into(), "Other".into());
    let (_, out) = b.apply_to_file(prj.to_json().as_bytes(), "").unwrap();
    assert_eq!(Project::from_json(&String::from_utf8(out.unwrap()).unwrap()).unwrap().info.company, "Other");
    let z = archive::archive(&prj, &archive::ArchiveOptions { comment: "c".into(), ..Default::default() });
    let (_, out) = b.apply_to_file(&z, "2026-10-24").unwrap();
    let out = out.unwrap();
    assert!(archive::is_archive(&out));
    assert_eq!(archive::manifest(&out).unwrap().comment, "c");
}
