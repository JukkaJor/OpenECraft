use super::*;
use crate::testing::*;

#[test]
fn single_line_diagram_and_schematic_are_transformed() {
    let mut s = new_session();
    run(&mut s, "add_page", json!({ "kind": "overview", "name": "SL" })).unwrap();
    run(&mut s, "place_symbol", json!({ "page": "SL", "symbol": "sl_cb", "x": 40, "y": 40, "tag": "-Q1" })).unwrap();
    run(&mut s, "place_symbol", json!({ "page": "SL", "symbol": "sl_motor", "x": 40, "y": 64, "tag": "-M1" })).unwrap();
    run(&mut s, "add_wire", json!({ "page": "SL", "points": [[40, 48], [40, 56]] })).unwrap();
    let v = run(&mut s, "list_single_line", json!({})).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 2, "{v}");
    assert_eq!(v[0]["status"], "planned");

    let g = run(&mut s, "single_line_to_schematic", json!({ "page": "SL" })).unwrap();
    assert_eq!((g["symbols"].as_u64(), g["wires"].as_u64()), (Some(2), Some(3)), "{g}");
    let v = run(&mut s, "list_single_line", json!({})).unwrap();
    assert!(v.as_array().unwrap().iter().all(|d| d["status"] == "engineered"), "{v}");
    assert_eq!((v[0]["tag"].as_str(), v[0]["schematic"]["symbol"].as_str()), (Some("-M1"), Some("motor3")));
    assert_eq!(v[1]["schematic"]["symbol"], "cb3");
    assert!(run(&mut s, "single_line_to_schematic", json!({ "page": "SL" })).is_err());
    assert!(run(&mut s, "schematic_to_single_line", json!({ "page": "SL" })).unwrap_err().contains("No devices"));
    assert_eq!(run(&mut s, "sync_single_line", json!({})).unwrap()["texts_copied"], 0);
}
