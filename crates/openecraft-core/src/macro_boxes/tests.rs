use super::*;
use crate::macros::from_selection;
use crate::sample::demo_project;

/// The demo with an empty schematic page at the end and a library with a macro made of
/// the first symbol (with a wire) of page 2.
fn setup() -> (Project, MacroLibrary) {
    let mut prj = demo_project();
    let src = prj.pages.iter().position(|p| p.symbols.iter().any(|s| s.tag.starts_with("-K")) && !p.wires.is_empty()).unwrap();
    let sym = prj.pages[src].symbols.iter().find(|s| s.tag.starts_with("-K")).unwrap().id;
    let sel: BTreeSet<u64> = [sym, prj.pages[src].wires[0].id].into();
    let m = from_selection(&prj.pages[src], &sel, "Unit");
    let id = prj.new_id();
    prj.pages.push(crate::model::Page::new(id, "99", "Macro test", crate::model::PageKind::Schematic));
    (prj, MacroLibrary { macros: vec![m] })
}

#[test]
fn placed_macros_are_recorded_and_updated_from_the_library() {
    let (mut prj, mut lib) = setup();
    let pi = prj.pages.len() - 1;
    let m = lib.get("Unit").unwrap().clone();
    let ids = place(&mut prj, pi, &m, &BTreeMap::new(), p(100.0, 100.0));
    assert_eq!(ids.len(), 2);
    let b = prj.pages[pi].macro_boxes[0].clone();
    assert_eq!(b.items.len(), 2);
    assert_eq!(state(&lib, &b), State::Current);
    let area = bounds(&prj, pi, &b).unwrap();
    assert!(area.min.x > 80.0 && area.min.y > 80.0, "{area:?}");
    // The library macro changes: the box is outdated and updated in place.
    let mut changed = m.clone();
    changed.clip.texts.push(crate::model::TextItem { id: 1, pos: m.clip.bounds().min, text: "NEW".into(), size: 3.0, ..Default::default() });
    lib.upsert(changed);
    assert_eq!(state(&lib, &prj.pages[pi].macro_boxes[0]), State::Outdated);
    let res = update(&mut prj, &lib);
    assert_eq!(res.updated, 1);
    assert!(prj.pages[pi].texts.iter().any(|t| t.text == "NEW"));
    let b = &prj.pages[pi].macro_boxes[0];
    assert_eq!(b.items.len(), 3);
    assert!(ids.iter().all(|i| !b.items.contains(i)), "old items replaced");
    assert_eq!(state(&lib, b), State::Current);
    assert_eq!(update(&mut prj, &lib).updated, 0);
}

#[test]
fn boxes_edited_on_the_page_are_saved_back() {
    let (mut prj, mut lib) = setup();
    let pi = prj.pages.len() - 1;
    let m = lib.get("Unit").unwrap().clone();
    place(&mut prj, pi, &m, &BTreeMap::new(), p(50.0, 50.0));
    let b = prj.pages[pi].macro_boxes[0].clone();
    let sym = b.items.iter().find(|i| prj.pages[pi].symbols.iter().any(|s| s.id == **i)).copied().unwrap();
    prj.pages[pi].symbols.iter_mut().find(|s| s.id == sym).unwrap().function_text = "Edited".into();
    save_to_library(&mut prj, pi, b.id, &mut lib).unwrap();
    assert!(lib.get("Unit").unwrap().clip.symbols.iter().any(|s| s.function_text == "Edited"));
    assert_eq!(state(&lib, &prj.pages[pi].macro_boxes[0]), State::Current);
}

#[test]
fn missing_macros_locked_pages_and_empty_boxes() {
    let (mut prj, lib) = setup();
    let pi = prj.pages.len() - 1;
    let m = lib.get("Unit").unwrap().clone();
    let ids = place(&mut prj, pi, &m, &BTreeMap::new(), p(50.0, 50.0));
    let empty = MacroLibrary::default();
    assert_eq!(state(&empty, &prj.pages[pi].macro_boxes[0]), State::Missing);
    assert_eq!(update(&mut prj, &empty).missing, vec!["Unit".to_string()]);
    let mut other = lib.clone();
    let mut changed = m.clone();
    changed.clip.texts.push(crate::model::TextItem { id: 1, text: "X".into(), size: 3.0, ..Default::default() });
    other.upsert(changed);
    prj.pages[pi].locked = true;
    assert_eq!(update(&mut prj, &other).locked, 1);
    prj.pages[pi].locked = false;
    ops::delete(&mut prj.pages[pi], &ids);
    assert_eq!(remove_empty(&mut prj), 1);
    assert!(prj.pages[pi].macro_boxes.is_empty());
}

#[test]
fn renamed_devices_keep_their_tags_when_updated() {
    let (mut prj, mut lib) = setup();
    let pi = prj.pages.len() - 1;
    let m = lib.get("Unit").unwrap().clone();
    place(&mut prj, pi, &m, &BTreeMap::new(), p(50.0, 50.0));
    // The second copy gets a new tag: the macro's tag is taken.
    let ids = place(&mut prj, pi, &m, &BTreeMap::new(), p(50.0, 150.0));
    let tag_of = |prj: &Project, ids: &[u64]| prj.pages[pi].symbols.iter().find(|s| ids.contains(&s.id)).map(|s| s.tag.clone()).unwrap();
    let second: Vec<u64> = ids.into_iter().collect();
    let renamed = tag_of(&prj, &second);
    assert_ne!(renamed, m.clip.symbols[0].tag, "renamed when placed");
    assert!(!renamed.is_empty());
    let mut changed = m.clone();
    changed.clip.texts.push(crate::model::TextItem { id: 1, pos: m.clip.bounds().min, text: "NEW".into(), size: 3.0, ..Default::default() });
    lib.upsert(changed);
    assert_eq!(update(&mut prj, &lib).updated, 2);
    let b = prj.pages[pi].macro_boxes[1].clone();
    assert_eq!(tag_of(&prj, &b.items), renamed, "the renamed tag stays");
}

#[test]
fn updates_keep_the_content_in_place() {
    // A box of a single symbol: its drawing reaches beyond its insertion point.
    let mut prj = demo_project();
    let src = prj.pages.iter().position(|p| p.symbols.iter().any(|s| s.tag.starts_with("-K"))).unwrap();
    let sym = prj.pages[src].symbols.iter().find(|s| s.tag.starts_with("-K")).unwrap().id;
    let m = from_selection(&prj.pages[src], &[sym].into(), "Unit");
    let id = prj.new_id();
    prj.pages.push(crate::model::Page::new(id, "99", "Macro test", crate::model::PageKind::Schematic));
    let mut lib = MacroLibrary { macros: vec![m.clone()] };
    let pi = prj.pages.len() - 1;
    place(&mut prj, pi, &m, &BTreeMap::new(), p(100.0, 100.0));
    let before = prj.pages[pi].symbols[0].pos;
    for n in 0..3 {
        let mut changed = lib.get("Unit").unwrap().clone();
        changed.clip.symbols[0].function_text = format!("v{n}");
        lib.upsert(changed);
        assert_eq!(update(&mut prj, &lib).updated, 1);
        assert_eq!(prj.pages[pi].symbols[0].pos, before, "update {n}");
    }
}
