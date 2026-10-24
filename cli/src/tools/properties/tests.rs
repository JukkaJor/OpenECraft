use super::*;
use crate::testing::*;

fn motor_session() -> (Session, u64) {
    let mut s = new_session();
    let m = run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "motor3", "x": 100, "y": 100 })).unwrap();
    (s, m["id"].as_u64().unwrap())
}

#[test]
fn libraries_are_exported_and_imported() {
    let (mut s, _) = motor_session();
    run(&mut s, "define_property", json!({ "key": "user.zone", "name": "Zone", "class": "device" })).unwrap();
    run(&mut s, "define_function", json!({ "key": "user.coil", "from": "fn.coil" })).unwrap();
    let path = temp_dir("property_library").join("lib.json");
    let path = path.to_string_lossy().to_string();
    run(&mut s, "export_property_library", json!({ "path": path })).unwrap();
    let inline = run(&mut s, "export_property_library", json!({})).unwrap();
    assert_eq!(inline["properties"][0]["key"], "user.zone");

    let mut other = new_session();
    let r = run(&mut other, "import_property_library", json!({ "path": path })).unwrap();
    assert_eq!(r["imported"], 2);
    assert_eq!(run(&mut other, "list_function_definitions", json!({ "filter": "user.coil" })).unwrap().as_array().unwrap().len(), 1);
}
