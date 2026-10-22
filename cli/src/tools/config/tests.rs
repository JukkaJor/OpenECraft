use super::*;
use crate::testing::*;
use openecraft_core::macros::from_page;
use openecraft_core::sample::demo_project;

#[test]
fn options_rules_and_generation() {
    let dir = temp_dir("config");
    let file = dir.join("macros.json");
    let mut lib = MacroLibrary::default();
    let demo = demo_project();
    let mut m = from_page(&demo.pages[1], "Motor");
    m.page_description = "Motor {LINE}".into();
    lib.upsert_variant(m, "");
    std::fs::write(&file, serde_json::to_string(&lib).unwrap()).unwrap();

    let mut s = new_session();
    run(&mut s, "set_option", json!({ "name": "LINE", "choices": "L1, L2", "value": "L2" })).unwrap();
    assert!(run(&mut s, "set_option", json!({ "name": "LINE", "value": "L9" })).unwrap_err().contains("not one of its choices"));
    let cfg = run(&mut s, "add_rule", json!({ "macro": "Motor", "condition": "LINE=L2" })).unwrap();
    assert_eq!(cfg["rules"][0]["number"], 1);
    let g = run(&mut s, "generate_pages", json!({ "macros": file.to_str().unwrap() })).unwrap();
    assert_eq!(g["pages"].as_array().unwrap().len(), 1);
    assert_eq!(s.project().unwrap().pages[1].description, "Motor L2");
    run(&mut s, "set_option", json!({ "name": "LINE", "value": "L1" })).unwrap();
    let g = run(&mut s, "generate_pages", json!({ "macros": file.to_str().unwrap() })).unwrap();
    assert_eq!((g["removed"].as_u64(), g["pages"].as_array().unwrap().len()), (Some(1), 0));
    run(&mut s, "delete_rule", json!({ "number": 1 })).unwrap();
    assert!(run(&mut s, "delete_rule", json!({ "number": 1 })).is_err());
    run(&mut s, "delete_option", json!({ "name": "LINE" })).unwrap();
    assert_eq!(run(&mut s, "get_configuration", json!({})).unwrap(), json!({ "options": [], "rules": [] }));
}
