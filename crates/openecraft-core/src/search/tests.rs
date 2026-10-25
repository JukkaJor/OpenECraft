use super::*;
use crate::sample::demo_project;

#[test]
fn matching_case_and_whole_words() {
    let q = Query::new("pump");
    assert!(q.matches("Pump 1") && q.matches("Waterpump"));
    let case = Query { case_sensitive: true, ..Query::new("pump") };
    assert!(!case.matches("Pump 1"));
    let word = Query { whole_word: true, ..Query::new("pump") };
    assert!(word.matches("Pump 1") && !word.matches("Waterpump") && !word.matches("pump_2"));
    assert_eq!(q.replace("Pump and pump", "Fan"), Some("Fan and Fan".to_string()));
    assert_eq!(q.replace("motor", "Fan"), None);
    assert!(!Query::new("").matches("anything"));
    assert_eq!(Query::new("ä").replace("Bär bär", "a"), Some("Bar bar".to_string()));
}

#[test]
fn finds_devices_texts_and_terminals() {
    let prj = demo_project();
    let hits = find(&prj, &Query::new("-K1"));
    assert!(hits.iter().any(|h| h.kind == Kind::Device && h.text == "-K1"));
    assert_eq!(hits.iter().filter(|h| h.kind == Kind::Device && h.text == "-K1").count(), 1, "one hit per device");
    let terminals = find(&prj, &Query { kinds: vec![Kind::Terminal], ..Query::new("-X1:") });
    assert_eq!(terminals.len(), 4);
    assert!(terminals.iter().all(|h| h.page.is_some() && h.item != 0));
    let parts = find(&prj, &Query { kinds: vec![Kind::Part], ..Query::new("GEN.") });
    assert!(!parts.is_empty());
    assert_eq!(Kind::from_code("wire_number"), Some(Kind::WireNumber));
    assert!(Kind::Text.replaceable() && !Kind::Device.replaceable());
}

#[test]
fn replace_changes_texts_but_not_released_pages() {
    let mut prj = demo_project();
    let page = prj.pages.iter().position(|p| p.symbols.iter().any(|s| !s.function_text.is_empty())).unwrap();
    let original = prj.pages[page].symbols.iter().find(|s| !s.function_text.is_empty()).unwrap().function_text.clone();
    let word = original.split_whitespace().next().unwrap().to_string();
    let q = Query { kinds: vec![Kind::FunctionText], ..Query::new(&word) };
    prj.pages.iter_mut().for_each(|p| p.locked = true);
    assert_eq!(replace(&mut prj, &q, "X"), 0, "released pages stay");
    prj.pages.iter_mut().for_each(|p| p.locked = false);
    assert!(replace(&mut prj, &q, "XYZ") > 0);
    assert!(find(&prj, &Query::new("XYZ")).iter().any(|h| h.kind == Kind::FunctionText));
}
