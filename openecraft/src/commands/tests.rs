use super::*;
use crate::ribbon;

#[test]
fn ids_are_unique_snake_case_and_every_command_is_defined_once() {
    let mut ids = std::collections::BTreeSet::new();
    let mut cmds = std::collections::BTreeSet::new();
    for d in ALL {
        assert!(ids.insert(d.id), "id {} twice", d.id);
        assert!(cmds.insert(d.cmd), "{:?} defined twice", d.cmd);
        assert!(d.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'), "{} is not snake_case", d.id);
        assert!(!d.title.is_empty(), "{:?} has no title", d.cmd);
        assert_eq!(def(d.cmd).id, d.id);
        assert_eq!(by_id(d.id).map(|x| x.cmd), Some(d.cmd));
    }
}

#[test]
fn every_command_has_a_place_in_the_shell() {
    let mut missing = Vec::new();
    for d in ALL {
        if ribbon::places(d.cmd).is_empty() {
            missing.push(d.id);
        }
    }
    // Commands only reachable by key or from the page tabs are listed here on purpose.
    let by_key_only = ["previous_page_tab"];
    missing.retain(|id| !by_key_only.contains(id));
    assert!(missing.is_empty(), "commands without a place in the ribbon, the backstage or View › Panes: {missing:?}");
}

#[test]
fn every_pane_is_shown_by_exactly_one_command() {
    for pane in Pane::all() {
        if pane == Pane::Page {
            continue;
        }
        let n = ALL.iter().filter(|d| d.pane == Some(pane)).count();
        assert_eq!(n, 1, "{pane:?} is shown by {n} commands");
        assert!(for_pane(pane).is_some());
    }
}

#[test]
fn toggles_and_menus_have_no_pane() {
    for d in ALL {
        if d.pane.is_some() {
            assert_eq!(d.kind, Kind::Button, "{}", d.id);
        }
    }
}

#[test]
fn wildcards_match_like_in_eplan() {
    assert!(wildcard_match("coil", "Relay coil"));
    assert!(wildcard_match("", "anything"));
    assert!(wildcard_match("c*l", "Relay coil"));
    assert!(wildcard_match("co?l", "COIL"));
    assert!(!wildcard_match("co?l", "cot"));
    assert!(wildcard_match("*contact*NO", "Contact, NO"));
    assert!(!wildcard_match("x*z", "abc"));
    assert!(wildcard_match("a*", "abc"));
}

#[test]
fn search_puts_title_matches_first() {
    let items = vec![
        ("wire", vec!["Wire numbering…".to_string()]),
        ("pages", vec!["Pages".to_string(), "navigator".to_string()]),
        ("new", vec!["New schematic page".to_string()]),
        ("renumber", vec!["Renumber pages…".to_string()]),
    ];
    let hits = rank("page", &items);
    let order: Vec<&str> = hits.iter().map(|h| h.item).collect();
    assert_eq!(order[0], "pages", "title starts with the query");
    assert!(order.contains(&"new") && order.contains(&"renumber"));
    assert!(!order.contains(&"wire"));
    assert_eq!(rank("navigator", &items).len(), 1, "found by its further texts");
    assert_eq!(rank("", &items).len(), 4);
    assert!(rank("page renumber", &items).iter().all(|h| h.item == "renumber"));
}
