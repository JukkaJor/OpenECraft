use super::*;
use crate::testing::*;

#[test]
fn new_projects_start_with_the_iec_60204_preset() {
    let mut s = new_session();
    let v = run(&mut s, "list_potentials", json!({})).unwrap();
    let l2 = v["potentials"].as_array().unwrap().iter().find(|p| p["name"] == "L2").unwrap().clone();
    assert_eq!(l2["color"], "BK");
    assert_eq!(l2["kind"], "l");
    assert_eq!(v["wire_defaults"]["color"], "BU");
}

#[test]
fn potentials_are_set_renamed_and_deleted() {
    let mut s = new_session();
    let d = run(&mut s, "set_potential", json!({ "name": "+48V", "kind": "dc_plus", "color": "RD", "cross_section": "1 mm²" })).unwrap();
    assert_eq!((d["kind"].as_str(), d["color"].as_str()), (Some("dc_plus"), Some("RD")));
    assert!(run(&mut s, "set_potential", json!({ "name": "+48V", "kind": "hot" })).unwrap_err().contains("Unknown potential kind"));
    assert!(run(&mut s, "set_potential", json!({ "name": "+48V", "rename": "L1" })).unwrap_err().contains("already exists"));
    run(&mut s, "set_potential", json!({ "name": "+48V", "rename": "+48VDC" })).unwrap();
    assert!(s.project().unwrap().potential("+48VDC").is_some());
    run(&mut s, "delete_potential", json!({ "name": "+48VDC" })).unwrap();
    assert!(run(&mut s, "delete_potential", json!({ "name": "+48VDC" })).is_err());
}

#[test]
fn presets_and_defaults() {
    let mut s = new_session();
    run(&mut s, "load_potential_preset", json!({ "preset": "IEC 60445" })).unwrap();
    assert_eq!(s.project().unwrap().potential("L1").unwrap().color, "BN");
    assert!(run(&mut s, "load_potential_preset", json!({ "preset": "nec" })).is_err());
    let v = run(&mut s, "set_wire_defaults", json!({ "color": "", "cross_section": "1 mm²" })).unwrap();
    assert_eq!(v["wire_defaults"], json!({ "color": "", "cross_section": "1 mm²" }));
}

#[test]
fn wire_colors_are_switched_on_and_off() {
    let mut s = new_session();
    assert_eq!(run(&mut s, "list_potentials", json!({})).unwrap()["wire_colors"], false);
    assert_eq!(run(&mut s, "set_wire_colors", json!({ "enabled": true })).unwrap()["wire_colors"], true);
    assert!(s.project().unwrap().wire_colors);
    run(&mut s, "set_wire_colors", json!({ "enabled": false })).unwrap();
    assert!(!s.project().unwrap().wire_colors);
}

#[test]
fn pages_show_the_resulting_wire_properties() {
    let mut s = new_session();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "pot_n", "x": 100, "y": 96 })).unwrap();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 100, "y": 120 })).unwrap();
    run(&mut s, "add_wire", json!({ "page": "1", "points": [[100, 100], [100, 116]] })).unwrap();
    let w = &run(&mut s, "get_page", json!({ "page": "1" })).unwrap()["wires"][0];
    assert_eq!(w["color"], "");
    assert_eq!(w["potential"], "N");
    assert_eq!(w["resulting_color"], "BU");
    assert_eq!(w["resulting_cross_section"], "2.5 mm²");
}
