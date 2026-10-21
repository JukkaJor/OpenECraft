use super::*;
use crate::testing::*;

fn issues(s: &mut Session) -> Vec<String> {
    let v = run(s, "check", json!({})).unwrap();
    v["issues"].as_array().unwrap().iter().map(|i| i["message"].as_str().unwrap().to_string()).collect()
}

#[test]
fn symbols_take_the_device_of_a_given_tag() {
    let mut s = new_session();
    let coil = run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 101, "y": 99 })).unwrap();
    assert_eq!(coil["tag"], "-K1");
    assert_eq!((coil["x"].as_f64(), coil["y"].as_f64()), (Some(100.0), Some(100.0)), "snapped to the grid");
    let first = run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "no", "x": 60, "y": 60, "tag": "-K1" })).unwrap();
    let second = run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "no", "x": 80, "y": 60, "tag": "-K1" })).unwrap();
    let pins = |v: &Value| v["pins"].as_array().unwrap().iter().map(|p| p["pin"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!(pins(&first), ["13", "14"]);
    assert_eq!(pins(&second), ["23", "24"]);
    let next = run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 140, "y": 100, "rotation": 90 })).unwrap();
    assert_eq!(next["tag"], "-K2");
    assert_eq!(next["rotation"], 90);

    assert!(run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 0, "y": 0, "rotation": 45 })).is_err());
    assert!(run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "nonsense", "x": 0, "y": 0 })).is_err());
    let listed = run(&mut s, "list_symbols", json!({ "category": "coils" })).unwrap();
    assert!(listed.as_array().unwrap().iter().any(|d| d["key"] == "coil"));
}

#[test]
fn connect_wires_connection_points() {
    let mut s = new_session();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 100, "y": 100 })).unwrap();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "no", "x": 60, "y": 60, "tag": "-K1" })).unwrap();
    assert!(issues(&mut s).iter().any(|m| m.contains("-K1:A1")));
    let wire = run(&mut s, "connect", json!({ "page": "1", "from": "-K1:A1", "to": "-K1:13" })).unwrap();
    assert_eq!(wire["ids"].as_array().unwrap().len(), 2, "one corner");
    assert!(!issues(&mut s).iter().any(|m| m.contains("-K1:A1") || m.contains("-K1:13")));

    assert!(run(&mut s, "connect", json!({ "page": "1", "from": "-K1:X9", "to": "-K1:14" })).unwrap_err().contains("No connection point"));
    assert!(run(&mut s, "connect", json!({ "page": "1", "from": "K1", "to": "-K1:14" })).is_err());
}

#[test]
fn ambiguous_connection_points_need_an_id() {
    let mut s = new_session();
    let a = run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "term", "x": 40, "y": 40, "tag": "-X1" })).unwrap();
    let b = run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "term", "x": 80, "y": 40, "tag": "-X1" })).unwrap();
    assert_eq!((a["label"].as_str(), b["label"].as_str()), (Some("1"), Some("2")), "terminal numbers");
    assert_eq!(a["pins"][1]["pin"], "2", "points without designation are numbered");
    let pin = "1";
    let err = run(&mut s, "connect", json!({ "page": "1", "from": format!("-X1:{pin}"), "to": format!("-X1:{pin}") })).unwrap_err();
    assert!(err.contains("ambiguous"), "{err}");
    let by_id = format!("#{}:{pin}", a["id"]);
    assert!(pin_position(&s.project().unwrap().pages[0], &by_id).is_ok());
}

#[test]
fn items_move_with_their_wires_and_can_be_deleted() {
    let mut s = new_session();
    let coil = run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 100, "y": 100 })).unwrap();
    let a1 = (coil["pins"][0]["x"].as_f64().unwrap(), coil["pins"][0]["y"].as_f64().unwrap());
    let wire = run(&mut s, "add_wire", json!({ "page": "1", "points": [[a1.0, a1.1], [a1.0, 40]] })).unwrap();
    let id = coil["id"].as_u64().unwrap();
    run(&mut s, "move_items", json!({ "page": "1", "ids": [id], "dx": 0, "dy": 8 })).unwrap();
    let page = run(&mut s, "get_page", json!({ "page": "1" })).unwrap();
    assert_eq!(page["symbols"][0]["y"], 108.0);
    assert_eq!(page["wires"][0]["from"], json!([a1.0, a1.1 + 8.0]), "wire end follows");

    run(&mut s, "update_wire", json!({ "page": "1", "id": wire["ids"][0], "number": "W1", "color": "BU" })).unwrap();
    assert_eq!(run(&mut s, "get_page", json!({ "page": "1" })).unwrap()["wires"][0]["number"], "W1");
    run(&mut s, "add_text", json!({ "page": "1", "x": 20, "y": 20, "text": "Hello" })).unwrap();

    assert!(run(&mut s, "delete_items", json!({ "page": "1", "ids": [999] })).is_err());
    assert_eq!(run(&mut s, "delete_items", json!({ "page": "1", "ids": [id] })).unwrap()["deleted"], 1);
    assert_eq!(run(&mut s, "list_pages", json!({})).unwrap()[0]["symbols"], 0);
}

#[test]
fn locked_pages_are_not_changed() {
    let mut s = new_session();
    s.project_mut().unwrap().pages[0].locked = true;
    let err = run(&mut s, "add_text", json!({ "page": "1", "x": 0, "y": 0, "text": "x" })).unwrap_err();
    assert_eq!(err, "Page 1 is locked");
}

#[test]
fn generated_pages_are_not_drawn_on() {
    let mut s = new_session();
    run(&mut s, "add_page", json!({ "kind": "device_list", "name": "DL" })).unwrap();
    let err = run(&mut s, "place_symbol", json!({ "page": "DL", "symbol": "coil", "x": 100, "y": 100 })).unwrap_err();
    assert_eq!(err, "Page DL is generated and cannot be edited");
    assert!(run(&mut s, "add_text", json!({ "page": "DL", "x": 0, "y": 0, "text": "x" })).is_err());
    // Its properties can still be changed.
    run(&mut s, "set_page", json!({ "page": "DL", "description": "Devices" })).unwrap();
}

