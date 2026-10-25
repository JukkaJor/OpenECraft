use super::*;

#[test]
fn the_default_trees_have_their_panes_once() {
    let tree = default_tree();
    for pane in [Pane::Page, Pane::Library, Pane::Properties, Pane::Navigator(LeftTab::Check)].into_iter().chain(NAVIGATORS) {
        assert!(find(&tree, pane).is_some(), "{pane:?}");
    }
    assert_eq!(active_tab(&default_tree()), Some(LeftTab::Pages));
    for w in Workspace::ALL {
        let tree = default_tree_for(w);
        let all = panes(&tree);
        assert!(!all.is_empty(), "{w:?}");
        for p in &all {
            assert_eq!(all.iter().filter(|q| *q == p).count(), 1, "{p:?} twice in {w:?}");
        }
        assert!(from_json(&to_json(&tree)).is_some(), "{w:?} can be saved and read");
    }
    assert!(find(&default_tree_for(Workspace::Panel), Pane::LayoutView).is_some());
    assert!(find(&default_tree_for(Workspace::Wiring), Pane::WiringWorkbench).is_some());
}

#[test]
fn every_pane_has_an_area_and_the_lists_are_complete() {
    let all = Pane::all();
    assert_eq!(all.len(), 1 + LeftTab::ALL.len() + 2 + 5 + ToolPane::ALL.len());
    for p in &all {
        assert_eq!(all.iter().filter(|q| *q == p).count(), 1, "{p:?}");
    }
    assert_eq!(Pane::Tool(ToolPane::Reports).area(), Area::Center);
    assert_eq!(Pane::Tool(ToolPane::Find).area(), Area::Right);
    assert_eq!(Pane::Navigator(LeftTab::Check).area(), Area::Bottom);
}

#[test]
fn showing_a_tab_activates_it_and_brings_closed_panes_back() {
    let mut tree = default_tree();
    show(&mut tree, Pane::Navigator(LeftTab::Devices));
    assert_eq!(active_tab(&tree), Some(LeftTab::Devices));

    close(&mut tree, Pane::Navigator(LeftTab::Review));
    close(&mut tree, Pane::Library);
    close(&mut tree, Pane::Page);
    assert!(find(&tree, Pane::Navigator(LeftTab::Review)).is_none());
    assert!(find(&tree, Pane::Library).is_none());
    assert!(find(&tree, Pane::Page).is_some(), "the only document cannot be closed");

    show(&mut tree, Pane::Navigator(LeftTab::Review));
    let review = find(&tree, Pane::Navigator(LeftTab::Review)).unwrap();
    let pages = find(&tree, Pane::Navigator(LeftTab::Pages)).unwrap();
    assert_eq!(tree.tiles.parent_of(review), tree.tiles.parent_of(pages), "back among the navigator tabs");
    assert_eq!(active_tab(&tree), Some(LeftTab::Review));
    show(&mut tree, Pane::Library);
    assert!(find(&tree, Pane::Library).is_some());
}

#[test]
fn tools_open_beside_the_page_and_the_page_can_close_then() {
    let mut tree = default_tree();
    show(&mut tree, Pane::Tool(ToolPane::Reports));
    let reports = find(&tree, Pane::Tool(ToolPane::Reports)).unwrap();
    let page = find(&tree, Pane::Page).unwrap();
    assert_eq!(tree.tiles.parent_of(reports), tree.tiles.parent_of(page));
    assert!(visible(&tree).contains(&Pane::Tool(ToolPane::Reports)));
    assert!(can_close(&tree, Pane::Page), "another document is open");
    close(&mut tree, Pane::Page);
    assert!(find(&tree, Pane::Page).is_none());
    assert!(!can_close(&tree, Pane::Tool(ToolPane::Reports)));
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
    let mut no_document = default_tree();
    let page = find(&no_document, Pane::Page).unwrap();
    no_document.remove_recursively(page);
    assert!(from_json(&to_json(&no_document)).is_none());
    assert_eq!(LeftTab::from_key("plc"), Some(LeftTab::Plc));
}

#[test]
fn arrangements_saved_before_tools_were_panes_are_read() {
    // A tree of the earlier editor: navigator, page, library and properties.
    let mut tiles = Tiles::default();
    let nav = tabs(&mut tiles, &[Pane::Navigator(LeftTab::Pages)]);
    let page = tabs(&mut tiles, &[Pane::Page]);
    let side = tabs(&mut tiles, &[Pane::Library, Pane::Properties]);
    let root = linear(&mut tiles, LinearDir::Horizontal, &[(nav, 1.0), (page, 3.0), (side, 1.0)]);
    let old = Tree::new(egui::Id::unique("old"), root, tiles);
    assert!(from_json(&to_json(&old)).is_some());
}
