use super::*;
use crate::testing::*;

fn demo() -> Session {
    let mut s = session();
    run(&mut s, "new_project", json!({ "name": "Demo", "demo": true })).unwrap();
    s
}

#[test]
fn enclosure_of_a_system_with_components() {
    let mut s = demo();
    let systems = run(&mut s, "enclosure_systems", json!({})).unwrap();
    assert!(systems.as_array().unwrap().iter().any(|x| x["system"] == "baying" && x["bayable"] == true));
    let e = run(&mut s, "add_enclosure", json!({ "name": "+CAB2", "system": "baying" })).unwrap();
    assert_eq!(e["part"], "GEN.TS-600X1800X500");
    assert_eq!(e["plinth"], 100.0);
    let e = run(&mut s, "set_enclosure", json!({ "enclosure": "+CAB2", "width": 800, "height": 2000, "right_open": true, "gland_plate": true })).unwrap();
    assert_eq!(e["part"], "GEN.TS-800X2000X500");
    let parts: Vec<&str> = e["components"].as_array().unwrap().iter().filter_map(|c| c["part"].as_str()).collect();
    assert!(parts.contains(&"GEN.TS-SP-2000X500") && parts.contains(&"GEN.GP-800X500") && parts.contains(&"GEN.TS-BAY"), "{parts:?}");
    assert!(run(&mut s, "add_enclosure", json!({ "name": "+CAB2" })).unwrap_err().contains("exists"));
    assert!(run(&mut s, "set_enclosure", json!({ "enclosure": "+CAB2", "system": "tent" })).unwrap_err().contains("Unknown enclosure system"));
}

#[test]
fn accessories_items_and_arranging() {
    let mut s = demo();
    let g = run(&mut s, "place_accessory", json!({ "enclosure": "+CAB1", "part": "GEN.GLAND-M25", "face": "bottom", "anchor_x": "center:0", "y": 60 })).unwrap();
    assert_eq!((g["face"].as_str(), g["fastening"].as_str()), (Some("bottom"), Some("clip")));
    assert_eq!(g["x"], (600.0 - 36.0) / 2.0);
    assert!(run(&mut s, "place_accessory", json!({ "enclosure": "+CAB1", "part": "GEN.HANDLE-SWING", "face": "roof" })).unwrap_err().contains("cannot be mounted"));
    assert!(run(&mut s, "place_accessory", json!({ "enclosure": "+CAB1", "part": "GEN.GLAND-M25", "face": "lid" })).unwrap_err().contains("Unknown face"));
    let id = g["id"].as_u64().unwrap();
    let r = run(&mut s, "arrange_panel_items", json!({ "enclosure": "+CAB1", "ids": [id], "action": "pattern", "columns": 4, "pitch_x": 50 })).unwrap();
    assert_eq!(r["created"].as_array().unwrap().len(), 3);
    let m = run(&mut s, "set_panel_item", json!({ "enclosure": "+CAB1", "id": id, "fastening": "rivet_nut", "anchor_y": "end:40" })).unwrap();
    assert_eq!((m["fastening"].as_str(), m["anchor"][1].as_str()), (Some("rivet_nut"), Some("end:40")));
    assert!(run(&mut s, "arrange_panel_items", json!({ "enclosure": "+CAB1", "ids": [id], "action": "spin" })).unwrap_err().contains("Unknown action"));
}

#[test]
fn assemblies_and_macros() {
    let mut s = demo();
    let list = run(&mut s, "list_panel_macros", json!({})).unwrap();
    assert!(list.as_array().unwrap().iter().any(|m| m["name"] == "Filter fan with outlet filter" && m["faces"] == json!(["left", "right"])));
    let r = run(&mut s, "insert_panel_macro", json!({ "enclosure": "+CAB1", "macro": "Cable entry 5 x M25", "x": 100, "y": 60 })).unwrap();
    let ids: Vec<u64> = r["items"].as_array().unwrap().iter().map(|i| i["id"].as_u64().unwrap()).collect();
    assert_eq!(ids.len(), 5);
    let e = &run(&mut s, "list_enclosures", json!({ "enclosure": "+CAB1" })).unwrap()[0];
    assert_eq!(e["assemblies"][0]["items"].as_array().unwrap().len(), 5);
    run(&mut s, "save_panel_macro", json!({ "enclosure": "+CAB1", "ids": &ids[..2], "name": "Two glands" })).unwrap();
    let list = run(&mut s, "list_panel_macros", json!({})).unwrap();
    assert_eq!(list[0]["name"], "Two glands");
    assert_eq!(list[0]["built_in"], false);
    let g = run(&mut s, "group_panel_items", json!({ "enclosure": "+CAB1", "ids": ids })).unwrap();
    assert_eq!(g["group"], 0);
    let e = &run(&mut s, "list_enclosures", json!({ "enclosure": "+CAB1" })).unwrap()[0];
    assert!(e["assemblies"].as_array().unwrap().is_empty());
}
