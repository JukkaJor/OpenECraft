use super::*;
use openecraft_parts::api::Session;
use openecraft_parts::portal::{FacetValue, UpdateKind, User};
use openecraft_parts::TopGroup;

fn result(total: usize, numbers: &[&str]) -> SearchResult {
    let parts = numbers.iter().map(|n| Part { number: n.to_string(), ..Default::default() }).collect();
    SearchResult { total, parts, facets: vec![Facet { name: "group".into(), values: vec![FacetValue { value: "Relays, contactors".into(), count: 2 }] }], ..Default::default() }
}

#[test]
fn searches_once_until_something_changes() {
    let mut b = Browser::new();
    b.text = "  contactor ".into();
    let q = b.take_search().unwrap();
    assert_eq!((q.q.as_deref(), q.brief, q.limit), (Some("contactor"), true, Some(PAGE)));
    assert!(b.take_search().is_none());
    b.toggle_facet("manufacturer", "ACME");
    assert_eq!(b.take_search().unwrap().manufacturer.as_deref(), Some("ACME"));
}

#[test]
fn facets_toggle_and_clear() {
    let mut b = Browser::new();
    b.query.offset = Some(100);
    b.toggle_facet("top_group", "Fluid");
    assert_eq!(b.query.top_group, Some(TopGroup::Fluid));
    assert_eq!(b.query.offset, None, "a new filter starts on the first page");
    assert_eq!(b.facet_filter("top_group").as_deref(), Some("Fluid"));
    b.toggle_facet("top_group", "Fluid");
    assert_eq!(b.query.top_group, None);
    b.toggle_facet("lifecycle", "Discontinued");
    assert_eq!(b.query.lifecycle, Some(Lifecycle::Discontinued));
    b.toggle_attribute("Poles", "3");
    b.toggle_attribute("poles", "4");
    assert_eq!(b.query.attributes, [("poles".to_string(), "4".to_string())]);
    assert_eq!(b.attribute_filter("POLES"), Some("4"));
    b.toggle_attribute("Poles", "4");
    assert!(b.query.attributes.is_empty());
    b.query.sort = Sort::Price;
    assert!(b.has_filters());
    b.clear_filters();
    assert!(!b.has_filters());
    assert_eq!(b.query.sort, Sort::Price, "the order is kept");
}

#[test]
fn pages() {
    let mut b = Browser::new();
    b.result = Some(result(120, &[]));
    assert_eq!(b.page(), (1, 3));
    b.go_to_page(3);
    assert_eq!(b.query.offset, Some(2 * PAGE));
    assert_eq!(b.page(), (3, 3));
    b.go_to_page(9);
    assert_eq!(b.page(), (3, 3));
    b.result = Some(result(0, &[]));
    assert_eq!(b.page().1, 1);
}

#[test]
fn favorites() {
    let mut b = Browser::new();
    b.take_search();
    b.apply(&Reply::Favorites(Ok(vec!["A".into()])), &[]);
    assert!(b.is_favorite("A"));
    assert_eq!(b.toggled_favorite("B"), ["A", "B"]);
    assert!(b.toggled_favorite("A").is_empty());
    b.only_favorites = true;
    assert_eq!(b.search_query().numbers, ["A"]);
    b.apply(&Reply::Favorites(Ok(Vec::new())), &[]);
    assert_eq!(b.search_query().numbers, [""], "no favorites: no parts");
    assert!(b.take_search().is_some(), "changed favorites refresh the list");
    b.apply(&Reply::SignedOut, &[]);
    assert!(!b.only_favorites && b.favorites.is_empty());
}

#[test]
fn replies_update_results_history_and_updates() {
    let mut b = Browser::new();
    b.selected = Some("GONE".into());
    b.apply(&Reply::Search(Ok(result(2, &["A", "B"]))), &[]);
    assert_eq!(b.selected, None);
    b.selected = Some("B".into());
    assert_eq!(b.selected_part().unwrap().number, "B");
    b.apply(&Reply::History("B".into(), Ok(vec![HistoryEntry::default()])), &[]);
    assert_eq!(b.history.as_ref().unwrap().0, "B");

    let local = vec![Part { number: "A".into(), revision: 1, ..Default::default() }];
    let remote = vec![Part { number: "A".into(), revision: 2, description: "new".into(), ..Default::default() }];
    b.apply(&Reply::Versions(Ok(SearchResult { parts: remote.clone(), ..Default::default() })), &local);
    assert_eq!(b.updates.as_ref().unwrap()[0].kind, UpdateKind::Changed);
    assert_eq!(b.remote, remote);
    b.updates_applied();
    assert!(b.updates.is_none() && b.remote.is_empty());

    b.password = "secret".into();
    b.apply(&Reply::SignedIn(Ok(Session { token: "t".into(), user: User::default(), ..Default::default() })), &[]);
    assert!(b.password.is_empty(), "the password is not kept");
}

#[test]
fn values_are_shown_translated() {
    assert_eq!(value_text("top_group", "Fluid"), tr("Fluid power"));
    assert_eq!(value_text("lifecycle", "PhaseOut"), tr("Phase-out"));
    assert_eq!(value_text("group", "relays, contactors"), tr("Relays, contactors"));
    assert_eq!(value_text("group", "own group"), "own group");
    assert_eq!(value_text("manufacturer", "ACME"), "ACME");
}
