use super::*;
use crate::testing::*;

#[test]
fn plugs_are_listed_with_their_pins() {
    let mut s = new_session();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "pot_24", "x": 100, "y": 76 })).unwrap();
    let pin = run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "plug_pair", "x": 100, "y": 100, "tag": "-X10" })).unwrap();
    assert_eq!(pin["label"], "1");
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 100, "y": 120 })).unwrap();
    run(&mut s, "add_wire", json!({ "page": "1", "points": [[100, 80], [100, 96]] })).unwrap();
    run(&mut s, "add_wire", json!({ "page": "1", "points": [[100, 104], [100, 116]] })).unwrap();

    let list = run(&mut s, "list_plugs", json!({})).unwrap();
    assert_eq!(list, json!([{ "plug": "-X10", "pins_used": 1, "pin_count": null, "parts": [] }]));
    let plug = run(&mut s, "get_plug", json!({ "plug": "-X10" })).unwrap();
    let p = &plug["pins"][0];
    assert_eq!((p["pin"].as_str(), p["socket_side"].as_str(), p["pin_side"].as_str()), (Some("1"), Some("=+24V"), Some("-K1:A1")));
    assert_eq!(p["symbols"][0]["kind"], "pair");
    assert!(run(&mut s, "get_plug", json!({ "plug": "-X99" })).unwrap_err().contains("No plug"));
}
