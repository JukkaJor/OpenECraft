use super::*;
use crate::sample::demo_project;

#[test]
fn changes_since_the_last_revision() {
    let mut prj = demo_project();
    assert!(pending(&prj).is_empty(), "no revision yet");
    create(&mut prj, "A", "2026-10-20", "NS", "first release", true);
    assert_eq!(prj.info.revision, "A");
    assert!(prj.pages.iter().all(|p| p.locked));
    assert!(pending(&prj).is_empty());

    // Unlock, then change a symbol, delete a wire, add a text, add a page.
    for pg in &mut prj.pages {
        pg.locked = false;
    }
    assert!(pending(&prj).is_empty(), "locking alone is no change");
    let pg = &mut prj.pages[1];
    pg.symbols[0].function_text = "changed".into();
    let removed = pg.wires.remove(0).id;
    pg.texts.push(crate::model::TextItem { id: 9999, text: "note".into(), ..Default::default() });
    prj.pages.push(Page::new(10_000, "9", "new page", crate::model::PageKind::Schematic));
    let changes = pending(&prj);
    let page1 = changes.iter().find(|c| c.page == Some(1)).unwrap();
    let kinds: Vec<(ChangeKind, &str)> = page1.changes.iter().map(|c| (c.kind, c.what)).collect();
    assert!(kinds.contains(&(ChangeKind::Modified, "Symbol")));
    assert!(kinds.contains(&(ChangeKind::Added, "Text")));
    assert!(page1.changes.iter().any(|c| c.kind == ChangeKind::Removed && c.id == removed));
    assert!(changes.iter().any(|c| c.name == "9" && c.changes[0].what == "Page"));
    assert_eq!(overview(&prj).len(), changes.iter().map(|c| c.changes.len()).sum::<usize>());

    // A deleted page is reported as removed.
    prj.pages.remove(3);
    assert!(pending(&prj).iter().any(|c| c.page.is_none() && c.changes[0].kind == ChangeKind::Removed));
}

#[test]
fn revision_names_count_up() {
    let mut prj = Project::default();
    assert_eq!(next_name(&prj), "A");
    for (last, next) in [("A", "B"), ("Z", "ZA"), ("01", "02"), ("9", "10"), ("B1", "B1A")] {
        prj.revisions = vec![Revision { name: last.into(), ..Default::default() }];
        assert_eq!(next_name(&prj), next, "after {last}");
    }
}

#[test]
fn snapshots_are_shared_and_serialized_as_pages() {
    let mut prj = demo_project();
    create(&mut prj, "A", "", "", "", false);
    let copy = prj.clone();
    assert!(Arc::ptr_eq(&copy.revisions[0].pages.0, &prj.revisions[0].pages.0));
    assert_eq!(copy, prj);
    let json = serde_json::to_string(&prj).unwrap();
    let back: Project = serde_json::from_str(&json).unwrap();
    assert_eq!(back.revisions[0].pages.0.len(), prj.pages.len());
    assert_eq!(back, prj);
}
