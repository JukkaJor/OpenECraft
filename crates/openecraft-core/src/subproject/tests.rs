use super::*;
use crate::geom::p;
use crate::sample::demo_project;

fn page_names(prj: &Project) -> Vec<String> {
    prj.pages.iter().map(|p| p.name.clone()).collect()
}

#[test]
fn pages_are_chosen_by_name_and_structure() {
    let mut prj = demo_project();
    prj.pages[2].function = "A2".into();
    prj.pages[3].function = "A2".into();
    let ids = select_pages(&prj, &["2".into()], "=A2").unwrap();
    assert_eq!(ids, vec![prj.pages[1].id, prj.pages[2].id, prj.pages[3].id]);
    assert!(select_pages(&prj, &["99".into()], "").is_err());
    assert!(select_pages(&prj, &[], "").unwrap().is_empty());
}

#[test]
fn subproject_carries_the_data_of_its_pages() {
    let mut prj = demo_project();
    let ids = select_pages(&prj, &["2".into()], "").unwrap();
    let sub = extract(&mut prj, "Power", "Power section", &ids, "2026-10-24").unwrap();
    assert_eq!(page_names(&sub), vec!["2"]);
    let tags = tags_of(&sub.pages);
    assert!(sub.devices.keys().all(|t| tags.contains(t)));
    assert!(sub.devices.len() < prj.devices.len());
    assert!(sub.parts.len() <= prj.parts.len());
    assert!(sub.layout.enclosures.is_empty());
    let parent = sub.subprojects.parent.as_ref().unwrap();
    assert_eq!((parent.project.as_str(), parent.name.as_str()), ("Demo conveyor", "Power"));
    assert!(!sub.pages[0].locked);

    // Locked and reserved in the main project.
    assert!(prj.pages[1].locked);
    assert!(prj.next_id >= sub.next_id + ID_RANGE);
    assert!(extract(&mut prj, "Power", "", &ids, "").is_err());
    assert!(extract(&mut prj, "Again", "", &ids, "").is_err());
    assert!(extract(&mut prj, "", "", &ids, "").is_err());
    assert!(extract(&mut prj, "None", "", &[], "").is_err());
}

#[test]
fn edited_subproject_replaces_its_pages() {
    let mut prj = demo_project();
    let before = page_names(&prj);
    let ids = select_pages(&prj, &["2".into(), "3".into()], "").unwrap();
    let mut sub = extract(&mut prj, "Drives", "", &ids, "2026-10-01").unwrap();
    let start = sub.next_id;

    // Edited there: a text, a new page, a new device description.
    let id = sub.new_id();
    sub.pages[0].texts.push(TextItem { id, pos: p(20.0, 20.0), text: "edited".into(), ..Default::default() });
    let pid = sub.new_id();
    sub.pages.push(Page::new(pid, "3a", "Extension", PageKind::Schematic));
    let tag = sub.devices.keys().next().unwrap().clone();
    sub.devices.get_mut(&tag).unwrap().description = "Changed outside".into();
    // Meanwhile the main project creates an id from its own range.
    let main_id = prj.new_id();
    assert!(main_id >= start + ID_RANGE);

    let r = insert(&mut prj, &sub, "2026-10-24").unwrap();
    assert_eq!((r.pages_removed, r.pages_inserted, r.renumbered), (2, 3, 0));
    let mut expected = before.clone();
    expected.insert(3, "3a".into());
    assert_eq!(page_names(&prj), expected);
    assert!(prj.pages[1].texts.iter().any(|t| t.text == "edited" && t.id == id));
    assert_eq!(prj.devices[&tag].description, "Changed outside");
    assert!(!prj.pages[1].locked);
    let c = prj.subprojects.child("Drives").unwrap();
    assert!(!c.out);
    assert_eq!(c.inserted, "2026-10-24");
    assert_eq!(c.pages.len(), 3);

    // Not a subproject of this project.
    assert!(insert(&mut prj, &demo_project(), "").is_err());
}

#[test]
fn colliding_ids_are_renumbered() {
    let mut prj = demo_project();
    let ids = select_pages(&prj, &["4".into()], "").unwrap();
    let mut sub = extract(&mut prj, "PLC", "", &ids, "").unwrap();
    // A hand-edited file reuses an id of another page.
    let other = prj.pages[1].symbols[0].id;
    sub.pages[0].texts.push(TextItem { id: other, text: "dup".into(), ..Default::default() });
    let r = insert(&mut prj, &sub, "").unwrap();
    assert_eq!(r.renumbered, 1);
    let t = prj.pages[3].texts.iter().find(|t| t.text == "dup").unwrap();
    assert_ne!(t.id, other);
}

#[test]
fn discarding_unlocks_the_pages() {
    let mut prj = demo_project();
    prj.pages[2].locked = true;
    let ids = select_pages(&prj, &["2".into(), "3".into()], "").unwrap();
    extract(&mut prj, "S", "", &ids, "").unwrap();
    discard(&mut prj, "S").unwrap();
    assert!(!prj.pages[1].locked);
    assert!(prj.pages[2].locked);
    assert!(prj.subprojects.is_empty());
    assert!(discard(&mut prj, "S").is_err());
}

#[test]
fn a_subproject_is_inserted_once_and_only_from_its_latest_extraction() {
    let mut prj = demo_project();
    let ids = select_pages(&prj, &["2".into()], "").unwrap();
    let first = extract(&mut prj, "Power", "", &ids, "2026-10-24").unwrap();
    insert(&mut prj, &first, "2026-10-25").unwrap();
    // Changed here after the insert: inserting the file again would undo it.
    let at = prj.pages.iter().position(|p| p.name == "2").unwrap();
    prj.pages[at].description = "Edited after the insert".into();
    assert!(insert(&mut prj, &first, "2026-10-26").is_err());
    assert_eq!(prj.pages[at].description, "Edited after the insert");
    // Taken out again: only the new file goes back in.
    let ids = vec![prj.pages[at].id];
    let second = extract(&mut prj, "Power", "", &ids, "2026-10-27").unwrap();
    assert!(insert(&mut prj, &first, "2026-10-28").is_err());
    insert(&mut prj, &second, "2026-10-28").unwrap();
    let at = prj.pages.iter().position(|p| p.name == "2").unwrap();
    assert_eq!(prj.pages[at].description, "Edited after the insert");
}
