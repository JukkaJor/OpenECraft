use super::*;
use crate::builtin::testing::*;

#[test]
fn strips_levels_jumpers_and_order() {
    let mut s = demo_session();
    let list = run(&mut s, "list_strips", json!({})).unwrap();
    assert_eq!(list[0]["strip"], "-X1");
    assert_eq!(list[0]["terminals"], 4);
    assert!(run(&mut s, "get_strip", json!({ "strip": "-X9" })).is_err());

    let strip = run(&mut s, "set_terminal", json!({ "strip": "-X1", "terminal": "2", "level": 2 })).unwrap();
    let blocks: Vec<Value> = strip["rail"].as_array().unwrap().iter().filter(|e| e["kind"] == "terminal").map(|e| e["terminals"].clone()).collect();
    assert_eq!(blocks[0], json!(["1", "2"]));
    assert_eq!(strip["terminals"][1]["level"], 2);
    assert!(run(&mut s, "set_terminal", json!({ "strip": "-X1", "terminal": "77", "level": 2 })).is_err());

    run(&mut s, "set_terminal", json!({ "strip": "-X1", "terminal": "1", "jumper": 5 })).unwrap();
    let strip = run(&mut s, "set_terminal", json!({ "strip": "-X1", "terminal": "2", "jumper": 5 })).unwrap();
    assert!(strip["jumpers"][0]["problem"].as_str().unwrap().contains("different levels"));
    let strip = run(&mut s, "set_jumper_kind", json!({ "strip": "-X1", "jumper": 5, "kind": "wire" })).unwrap();
    assert_eq!(strip["jumpers"][0]["kind"], "wire");
    assert!(strip["jumpers"][0]["problem"].is_null());
    assert!(run(&mut s, "set_jumper_kind", json!({ "strip": "-X1", "jumper": 5, "kind": "glue" })).is_err());

    let strip = run(&mut s, "set_strip", json!({ "strip": "-X1", "spares": 2, "order": "3,1,2,PE" })).unwrap();
    let order: Vec<&str> = strip["terminals"].as_array().unwrap().iter().map(|t| t["terminal"].as_str().unwrap()).collect();
    assert_eq!(&order[..4], ["3", "1", "2", "PE"]);
    assert_eq!(strip["terminals"].as_array().unwrap().len(), 6);
    assert!(run(&mut s, "set_strip", json!({ "strip": "-X1", "order": "9" })).is_err());
    let strip = run(&mut s, "renumber_strip", json!({ "strip": "-X1" })).unwrap();
    assert_eq!(strip["renumbered"], 3);
}
