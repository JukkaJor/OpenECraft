use super::*;

#[test]
fn the_default_tree_has_every_pane_once() {
    let tree = default_tree();
    for pane in Pane::all() {
        assert!(find(&tree, pane).is_some(), "{pane:?}");
    }
    assert_eq!(tree.tiles.tiles().filter(|t| matches!(t, Tile::Pane(_))).count(), Pane::all().len());
    assert_eq!(active_tab(&default_tree()), Some(LeftTab::Pages));
}

#[test]
fn showing_a_tab_activates_it_and_brings_closed_panes_back() {
    let mut tree = default_tree();
    show(&mut tree, Pane::Navigator(LeftTab::Check));
    assert_eq!(active_tab(&tree), Some(LeftTab::Check));

    close(&mut tree, Pane::Navigator(LeftTab::Review));
    close(&mut tree, Pane::Library);
    close(&mut tree, Pane::Page);
    assert!(find(&tree, Pane::Navigator(LeftTab::Review)).is_none());
    assert!(find(&tree, Pane::Library).is_none());
    assert!(find(&tree, Pane::Page).is_some(), "the page cannot be closed");

    show(&mut tree, Pane::Navigator(LeftTab::Review));
    let review = find(&tree, Pane::Navigator(LeftTab::Review)).unwrap();
    let pages = find(&tree, Pane::Navigator(LeftTab::Pages)).unwrap();
    assert_eq!(tree.tiles.parent_of(review), tree.tiles.parent_of(pages), "back among the navigator tabs");
    assert_eq!(active_tab(&tree), Some(LeftTab::Review));
    show(&mut tree, Pane::Library);
    assert!(find(&tree, Pane::Library).is_some());
}

#[test]
fn a_closed_navigator_comes_back_beside_the_page() {
    let mut tree = default_tree();
    for t in LeftTab::ALL {
        close(&mut tree, Pane::Navigator(t));
    }
    tree.simplify(&egui_tiles::SimplificationOptions { all_panes_must_have_tabs: true, ..Default::default() });
    show(&mut tree, Pane::Navigator(LeftTab::Devices));
    let devices = find(&tree, Pane::Navigator(LeftTab::Devices)).unwrap();
    let page = find(&tree, Pane::Page).unwrap();
    assert_eq!(tree.tiles.parent_of(devices), tree.tiles.parent_of(page));
}

#[test]
fn arrangements_are_saved_and_checked_when_read() {
    let mut tree = default_tree();
    close(&mut tree, Pane::Properties);
    let json = to_json(&tree);
    let back = from_json(&json).unwrap();
    assert!(find(&back, Pane::Properties).is_none());
    assert!(find(&back, Pane::Library).is_some());
    assert!(from_json("{").is_none());
    let mut no_page = default_tree();
    let page = find(&no_page, Pane::Page).unwrap();
    no_page.remove_recursively(page);
    assert!(from_json(&to_json(&no_page)).is_none());
    assert_eq!(LeftTab::from_key("plc"), Some(LeftTab::Plc));
}
