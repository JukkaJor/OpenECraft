use super::*;
use crate::testing::*;

fn demo() -> Session {
    let mut s = session();
    run(&mut s, "new_project", json!({ "name": "Demo", "demo": true })).unwrap();
    s
}

#[test]
fn demo_layout_has_no_warnings() {
    let mut s = demo();
    let r = run(&mut s, "check_panel", json!({})).unwrap();
    assert_eq!(r["warnings"], 0, "{r:#}");
    assert!(run(&mut s, "check_panel", json!({ "enclosure": "+CAB9" })).unwrap_err().contains("+CAB9"));
}

#[test]
fn rules_classes_and_heights() {
    let mut s = demo();
    let r = run(&mut s, "panel_rules", json!({ "device_gap": 30, "stratification": 5 })).unwrap();
    assert_eq!((r["device_gap"].as_f64(), r["stratification"].as_f64()), (Some(30.0), Some(2.0)));
    let r = run(&mut s, "check_panel", json!({ "enclosure": "+CAB1" })).unwrap();
    let mounting = r["findings"].as_array().unwrap().iter().filter(|f| f["check"] == "mounting").count();
    assert!(mounting > 0, "{r:#}");
    run(&mut s, "panel_rules", json!({ "reset": true })).unwrap();
    // Contactor and PLC 30 mm apart: a power / control rule of 50 mm.
    let r = run(&mut s, "set_collision_rule", json!({ "a": "power", "b": "control", "distance": 50 })).unwrap();
    assert_eq!(r["classes"].as_array().unwrap().len(), 2);
    let r = run(&mut s, "check_panel", json!({})).unwrap();
    assert!(r["findings"].as_array().unwrap().iter().any(|f| f["check"] == "class_distance"), "{r:#}");
    run(&mut s, "set_collision_rule", json!({ "a": "control", "b": "power", "remove": true })).unwrap();
    assert!(run(&mut s, "set_collision_rule", json!({ "a": "control", "b": "power", "remove": true })).is_err());
    // Classes of items.
    let id = s.project().unwrap().layout.find_device("-K1").unwrap().1.id;
    let r = run(&mut s, "set_item_class", json!({ "enclosure": "+CAB1", "id": id, "class": "" })).unwrap();
    assert_eq!((r["class"].as_str(), r["automatic"].as_bool()), (Some("power"), Some(true)));
    let r = run(&mut s, "set_item_class", json!({ "enclosure": "+CAB1", "id": id, "class": "Noisy" })).unwrap();
    assert_eq!(r["class"], "noisy");
    // Wall mounted at 1.9 m: devices reach above 2 m.
    let r = run(&mut s, "set_enclosure_elevation", json!({ "enclosure": "+CAB1", "elevation": 1900 })).unwrap();
    assert_eq!(r["floor_height"].as_f64(), Some(1900.0));
    let r = run(&mut s, "check_panel", json!({})).unwrap();
    assert!(r["findings"].as_array().unwrap().iter().any(|f| f["check"] == "height"), "{r:#}");
}

#[test]
fn thermal_simulation() {
    let mut s = demo();
    let r = run(&mut s, "panel_thermal", json!({ "enclosure": "+CAB1" })).unwrap();
    assert!(r["max_c"].as_f64().unwrap() >= r["min_c"].as_f64().unwrap());
    assert!(r["top_c"].as_f64().unwrap() > r["bottom_c"].as_f64().unwrap());
    let devices = r["devices"].as_array().unwrap();
    assert!(devices.iter().any(|d| d["device"] == "-K1"), "{r:#}");
    assert!(devices.iter().all(|d| d["too_warm"] == false), "{r:#}");
}
