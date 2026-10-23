use super::*;
use crate::sample::demo_project;

#[test]
fn equal_projects_have_no_differences() {
    let prj = demo_project();
    assert!(diff(&prj, &prj.clone()).is_empty());
    // Revisions, baselines and the review are not compared.
    let mut other = prj.clone();
    add_baseline(&mut other, "B1", "", "", "");
    crate::review::add_comment(&mut other, 0, crate::geom::p(0.0, 0.0), "anna", "", "x");
    assert!(diff(&prj, &other).is_empty());
}

#[test]
fn differences_and_take_over() {
    let base = demo_project();
    let mut cur = base.clone();
    // Current: changed symbol on page 1, a new device, removed part, other project name.
    cur.pages[1].symbols[0].function_text = "changed".into();
    cur.devices.insert("-NEW1".into(), Default::default());
    let removed_part = cur.parts.remove(0).number;
    cur.info.name = "renamed".into();
    let removed_page = cur.pages.remove(2);
    let new_id = cur.new_id();
    cur.pages.push(Page::new(new_id, "99", "extra", crate::model::PageKind::Schematic));

    let d = diff(&cur, &base);
    let find = |item: &Item| d.iter().find(|x| x.item == *item).map(|x| x.kind);
    assert_eq!(find(&Item::Page(cur.pages[1].id)), Some(ChangeKind::Modified));
    assert!(d.iter().find(|x| x.item == Item::Page(cur.pages[1].id)).unwrap().details.iter().any(|t| t.contains("changed")));
    assert_eq!(find(&Item::Page(new_id)), Some(ChangeKind::Added));
    assert_eq!(find(&Item::Page(removed_page.id)), Some(ChangeKind::Removed));
    assert_eq!(find(&Item::Device("-NEW1".into())), Some(ChangeKind::Added));
    assert_eq!(find(&Item::Part(removed_part.clone())), Some(ChangeKind::Removed));
    assert_eq!(find(&Item::Section(Section::Info)), Some(ChangeKind::Modified));
    // Pages come first.
    assert!(matches!(d[0].item, Item::Page(_)));

    // Taking everything over gives the other project again.
    let items: Vec<Item> = d.iter().map(|x| x.item.clone()).collect();
    take_all(&mut cur, &base, &items);
    assert!(diff(&cur, &base).is_empty(), "{:?}", diff(&cur, &base));
    assert_eq!(cur.pages.iter().map(|p| p.id).collect::<Vec<_>>(), base.pages.iter().map(|p| p.id).collect::<Vec<_>>());
}

#[test]
fn pages_from_another_project_get_free_ids() {
    let mut cur = Project::default();
    let used = cur.pages[0].id;
    let mut other = Project::default();
    // A page whose id is used by the current project for something else.
    let mut page = Page::new(used + 500, "2", "", crate::model::PageKind::Schematic);
    page.texts.push(crate::model::TextItem { id: used, text: "t".into(), ..Default::default() });
    other.pages.push(page);
    take(&mut cur, &other, &Item::Page(used + 500));
    assert_eq!(cur.pages.len(), 2);
    let taken = &cur.pages[1];
    assert_ne!(taken.texts[0].id, used);
    assert!(taken.id < cur.next_id && taken.texts[0].id < cur.next_id);
}

#[test]
fn page_order() {
    let base = demo_project();
    let mut cur = base.clone();
    cur.pages.swap(0, 1);
    let d = diff(&cur, &base);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].item, Item::PageOrder);
    take(&mut cur, &base, &Item::PageOrder);
    assert!(diff(&cur, &base).is_empty());
}

#[test]
fn baselines_are_shared_and_serialized() {
    let mut prj = demo_project();
    add_baseline(&mut prj, "B1", "2026-10-23", "anna", "before changes");
    let copy = prj.clone();
    assert!(Arc::ptr_eq(&copy.baselines[0].project.0, &prj.baselines[0].project.0));
    let json = serde_json::to_string(&prj).unwrap();
    let back: Project = serde_json::from_str(&json).unwrap();
    assert_eq!(back, prj);
    assert!(back.baselines[0].project.0.baselines.is_empty());
    // Comparing with a revision compares its pages.
    crate::revision::create(&mut prj, "A", "", "", "", false);
    prj.pages[0].description = "x".into();
    let rev = with_revision_pages(&prj, &prj.revisions[0].pages);
    let d = diff(&prj, &rev);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].item, Item::Page(prj.pages[0].id));
}
