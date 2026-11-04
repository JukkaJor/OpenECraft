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

#[test]
fn releasing_without_workflow_stores_a_baseline() {
    let mut prj = demo_project();
    create(&mut prj, "A", "2026-10-23", "anna", "", false);
    assert_eq!(status(&prj, 0), Status::Released);
    assert_eq!(prj.baselines.len(), 1);
    assert!(prj.baselines[0].name.contains('A'));
    assert!(open_revision(&prj).is_none());
    // Files of older versions: revisions without state are released.
    let old: Revision = serde_json::from_str(r#"{"name": "X"}"#).unwrap();
    assert_eq!(old.state, State::Released);
}

#[test]
fn approval_workflow() {
    let mut prj = demo_project();
    create(&mut prj, "A", "", "", "", false);
    prj.workflow = Workflow { approvers: vec!["Anna".into(), "Ben".into(), "Cleo".into()], required: 2 };
    assert_eq!(prj.workflow.needed(), 2);

    prj.pages[1].description = "changed".into();
    create(&mut prj, "B", "2026-10-23", "dev", "motor changed", true);
    let i = 1;
    assert_eq!(status(&prj, i), Status::Draft);
    assert_eq!(next_name(&prj), "B", "marks show the revision being prepared");
    assert_eq!(prj.info.revision, "A");
    assert_eq!(pending(&prj).len(), 1, "changes since the released revision");
    assert!(decide(&mut prj, i, "anna", "", true, "").is_err(), "not in review");
    assert!(release(&mut prj, i, true).is_err());

    submit(&mut prj, i).unwrap();
    assert_eq!(status(&prj, i), Status::InReview);
    assert!(decide(&mut prj, i, "mallory", "", true, "").is_err());
    decide(&mut prj, i, "anna", "2026-10-24", true, "").unwrap();
    assert_eq!(status(&prj, i), Status::InReview);
    decide(&mut prj, i, "Ben", "2026-10-24", false, "wrong fuse").unwrap();
    assert_eq!(status(&prj, i), Status::Rejected);
    // Ben changes his mind: one decision per user.
    decide(&mut prj, i, "ben", "2026-10-25", true, "").unwrap();
    assert_eq!(decisions(&prj, i).len(), 2);
    assert_eq!(status(&prj, i), Status::Approved);

    // Changed after the approval: release refused, submit again voids the approvals.
    prj.pages[1].description = "changed again".into();
    assert!(release(&mut prj, i, true).is_err());
    submit(&mut prj, i).unwrap();
    assert_eq!(prj.revisions[i].round, 2);
    assert!(decisions(&prj, i).is_empty());
    decide(&mut prj, i, "cleo", "", true, "").unwrap();
    decide(&mut prj, i, "anna", "", true, "").unwrap();
    release(&mut prj, i, true).unwrap();
    assert_eq!(status(&prj, i), Status::Released);
    assert_eq!(prj.info.revision, "B");
    assert!(prj.pages.iter().all(|p| p.locked));
    assert_eq!(prj.baselines.len(), 2);
    assert!(pending(&prj).is_empty());
    assert_eq!(next_name(&prj), "C");
}

#[test]
fn withdraw_and_discard() {
    let mut prj = demo_project();
    prj.workflow.approvers = vec!["anna".into()];
    create(&mut prj, "A", "", "", "", false);
    submit(&mut prj, 0).unwrap();
    withdraw(&mut prj, 0);
    assert_eq!(status(&prj, 0), Status::Draft);
    discard(&mut prj, 0);
    assert!(prj.revisions.is_empty());
}

#[test]
fn changed_items_list_their_changed_properties() {
    let old = crate::sample::demo_project().pages;
    let mut new = old.clone();
    let pi = new.iter().position(|p| p.symbols.iter().any(|s| !s.tag.is_empty()) && !p.wires.is_empty()).unwrap();
    let si = new[pi].symbols.iter().position(|s| !s.tag.is_empty()).unwrap();
    let old_tag = new[pi].symbols[si].tag.clone();
    new[pi].symbols[si].tag = "-K99".into();
    new[pi].symbols[si].pos.x += 8.0;
    new[pi].wires[0].color = "VT".into();
    new[pi].description = "Changed page".into();
    let changes = compare(&old, &new);
    let pc = changes.iter().find(|c| c.page == Some(pi)).unwrap();
    let sym = pc.changes.iter().find(|c| c.what == "Symbol").unwrap();
    let fields: Vec<&str> = sym.details.iter().map(|d| d.field).collect();
    assert_eq!(fields, ["Device tag", "Position"]);
    assert_eq!((sym.details[0].old.as_str(), sym.details[0].new.as_str()), (old_tag.as_str(), "-K99"));
    assert!(sym.details_text().starts_with(&format!("Device tag: {old_tag} › -K99")));
    let wire = pc.changes.iter().find(|c| c.what == "Wire").unwrap();
    assert_eq!(wire.details[0].field, "Colour");
    assert_eq!(wire.details[0].new, "VT");
    let page = pc.changes.iter().find(|c| c.what == "Page").unwrap();
    assert_eq!(page.details[0].field, "Description");
}

#[test]
fn all_page_content_counts_as_a_change() {
    let mut prj = demo_project();
    create(&mut prj, "A", "", "", "", false);
    let old = prj.pages.clone();
    let pg = &mut prj.pages[1];
    pg.dimensions.push(crate::model::Dimension { id: 900_001, ..Default::default() });
    pg.cable_defs.push(crate::model::CableDef { id: 900_002, cable: "-W9".into(), ..Default::default() });
    pg.conn_defs.push(crate::model::ConnDef { id: 900_003, ..Default::default() });
    pg.values.insert("page.drawing_number".into(), "D-17".into());
    let changes = compare(&old, &prj.pages);
    let whats: Vec<&str> = changes.iter().flat_map(|pc| pc.changes.iter().map(|c| c.what)).collect();
    for what in ["Dimension", "Cable definition", "Connection definition point", "Page"] {
        assert!(whats.contains(&what), "{what}: {whats:?}");
    }
    let page = changes[0].changes.iter().find(|c| c.what == "Page").unwrap();
    assert_eq!(page.details.iter().map(|d| d.field).collect::<Vec<_>>(), ["Page properties"]);
}

#[test]
fn changes_after_the_approval_need_a_new_submission() {
    let mut prj = demo_project();
    create(&mut prj, "A", "", "", "", false);
    prj.workflow = Workflow { approvers: vec!["Anna".into()], required: 1 };
    prj.pages[1].description = "changed".into();
    create(&mut prj, "B", "", "", "", false);
    submit(&mut prj, 1).unwrap();
    decide(&mut prj, 1, "Anna", "", true, "").unwrap();
    prj.pages[1].dimensions.push(crate::model::Dimension { id: 900_001, ..Default::default() });
    assert!(release(&mut prj, 1, false).is_err());
    submit(&mut prj, 1).unwrap();
    decide(&mut prj, 1, "Anna", "", true, "").unwrap();
    release(&mut prj, 1, true).unwrap();
}
