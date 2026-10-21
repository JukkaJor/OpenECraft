use super::*;
use crate::testing::{session, temp_dir};
use crate::tools::{call, find};

fn run(s: &mut Session, tool: &str, args: Value) -> Result<Value, String> {
    call(s, find(tool).unwrap(), args.as_object().unwrap())
}

fn new_session() -> Session {
    let mut s = session();
    run(&mut s, "new_project", json!({ "name": "Test" })).unwrap();
    s
}

fn issues(s: &mut Session) -> Vec<String> {
    let v = run(s, "check", json!({})).unwrap();
    v["issues"].as_array().unwrap().iter().map(|i| i["message"].as_str().unwrap().to_string()).collect()
}

#[test]
fn new_project_saves_and_refuses_to_drop_changes() {
    let dir = temp_dir("project-new");
    let file = dir.join("p.oec");
    let mut s = session();
    let info = run(&mut s, "new_project", json!({ "name": "Plant", "path": file.to_str().unwrap() })).unwrap();
    assert_eq!(info["name"], "Plant");
    assert_eq!(info["unsaved"], false);
    assert!(file.exists());

    run(&mut s, "set_project_info", json!({ "company": "ACME", "properties": { "Line": "3" } })).unwrap();
    let err = run(&mut s, "open_project", json!({ "path": file.to_str().unwrap() })).unwrap_err();
    assert!(err.contains("unsaved changes"), "{err}");
    let info = run(&mut s, "open_project", json!({ "path": file.to_str().unwrap(), "discard": true })).unwrap();
    assert_eq!(info["company"], "");

    let demo = run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    assert!(demo["pages"].as_u64().unwrap() > 1);
    assert_eq!(demo["errors"], 0);
}

#[test]
fn pages_are_added_renamed_and_deleted() {
    let mut s = new_session();
    assert_eq!(run(&mut s, "add_page", json!({ "kind": "schematic" })).unwrap()["name"], "2");
    let bom = run(&mut s, "add_page", json!({ "kind": "bill-of-materials", "after": "1", "name": "BOM" })).unwrap();
    assert_eq!(bom["kind"], "bill_of_materials");
    let names: Vec<Value> = run(&mut s, "list_pages", json!({})).unwrap().as_array().unwrap().iter().map(|p| p["name"].clone()).collect();
    assert_eq!(names, vec![json!("1"), json!("BOM"), json!("2")]);

    assert!(run(&mut s, "add_page", json!({ "kind": "drawing" })).is_err());
    assert!(run(&mut s, "set_page", json!({ "page": "2", "name": "BOM" })).unwrap_err().contains("already exists"));
    let pg = run(&mut s, "set_page", json!({ "page": "2", "name": "10", "function": "=A1" })).unwrap();
    assert_eq!(pg["function"], "A1");

    run(&mut s, "delete_page", json!({ "page": "BOM" })).unwrap();
    run(&mut s, "delete_page", json!({ "page": "10" })).unwrap();
    assert!(run(&mut s, "delete_page", json!({ "page": "1" })).unwrap_err().contains("last page"));
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
fn devices_are_renamed_and_get_parts() {
    let mut s = new_session();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 100, "y": 100 })).unwrap();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "no", "x": 60, "y": 60, "tag": "-K1" })).unwrap();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 140, "y": 100 })).unwrap();
    assert!(run(&mut s, "rename_device", json!({ "old": "-K1", "new": "-K2" })).unwrap_err().contains("already exists"));
    run(&mut s, "rename_device", json!({ "old": "-K1", "new": "-K10" })).unwrap();
    let devices = run(&mut s, "list_devices", json!({})).unwrap();
    let k10 = devices.as_array().unwrap().iter().find(|d| d["tag"] == "-K10").unwrap();
    assert_eq!(k10["main"], json!(["/1.3"]));
    assert_eq!(k10["auxiliary"].as_array().unwrap().len(), 1);

    let number = parts::catalog()[0].number.clone();
    run(&mut s, "assign_part", json!({ "tag": "-K10", "number": number })).unwrap();
    let out = run(&mut s, "assign_part", json!({ "tag": "-K10", "number": number, "count": 2 })).unwrap();
    assert_eq!(out["parts"], json!([{ "number": number, "count": 3 }]));
    assert!(s.project().unwrap().parts.iter().any(|p| p.number == number));
    assert!(run(&mut s, "assign_part", json!({ "tag": "-K99", "number": number })).is_err());
}

#[test]
fn reports_and_exports() {
    let dir = temp_dir("project-export");
    let mut s = session();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    let bom = run(&mut s, "report", json!({ "kind": "bill_of_materials" })).unwrap();
    assert!(!bom["rows"].as_array().unwrap().is_empty());
    assert!(run(&mut s, "report", json!({ "kind": "device_list", "format": "csv" })).unwrap().as_str().unwrap().contains(';'));
    assert!(run(&mut s, "report", json!({ "kind": "parts" })).is_err());

    let svg = run(&mut s, "export", json!({ "format": "svg", "page": "1" })).unwrap();
    assert!(svg.as_str().unwrap().contains("<svg"));
    assert!(run(&mut s, "export", json!({ "format": "svg" })).is_err(), "svg needs a page");
    let file = dir.join("all.dxf");
    let out = run(&mut s, "export", json!({ "format": "dxf", "path": file.to_str().unwrap() })).unwrap();
    assert!(out["bytes"].as_u64().unwrap() > 0);
    assert!(std::fs::read_to_string(&file).unwrap().contains("SECTION"));
    for format in ["html", "pdf", "wire_list", "plc_table", "kbl"] {
        assert!(run(&mut s, "export", json!({ "format": format })).is_ok(), "{format}");
    }
}

#[test]
fn renumbering_and_undo() {
    let mut s = session();
    run(&mut s, "new_project", json!({ "demo": true })).unwrap();
    run(&mut s, "renumber_wires", json!({ "overwrite_fixed": true })).unwrap();
    let before = s.project().unwrap().clone();
    run(&mut s, "add_text", json!({ "page": "1", "x": 0, "y": 0, "text": "x" })).unwrap();
    assert_eq!(run(&mut s, "undo", json!({})).unwrap()["undone"], true);
    assert_eq!(s.project().unwrap(), &before);
    assert_eq!(run(&mut s, "redo", json!({})).unwrap()["redone"], true);
    assert_ne!(s.project().unwrap(), &before);
}
