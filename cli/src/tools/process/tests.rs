use super::*;
use crate::testing::*;

#[test]
fn hoses_between_fluid_ports() {
    let mut s = new_session();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "fl_source", "x": 100, "y": 120 })).unwrap();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "fl_filter", "x": 100, "y": 100 })).unwrap();
    let ids = run(&mut s, "add_wire", json!({ "page": "1", "points": [[100, 124], [100, 140], [130, 140], [130, 96], [100, 96]] })).unwrap();
    let v = run(&mut s, "list_hoses", json!({})).unwrap();
    let hoses = v["hoses"].as_array().unwrap();
    assert_eq!(hoses.len(), 1, "{v}");
    assert_eq!((hoses[0]["medium"].as_str(), hoses[0]["size"].as_str()), (Some("compressed_air"), Some("8 mm")));
    let id = ids["ids"][0].as_u64().unwrap();
    run(&mut s, "update_wire", json!({ "page": "1", "id": id, "medium": "water", "length": 2.5 })).unwrap();
    assert!(run(&mut s, "update_wire", json!({ "page": "1", "id": id, "medium": "lava" })).unwrap_err().contains("Unknown medium"));
    let h = &run(&mut s, "list_hoses", json!({})).unwrap()["hoses"][0];
    assert_eq!((h["medium"].as_str(), h["length_m"].as_f64()), (Some("water"), Some(2.5)));
    let d = run(&mut s, "set_fluid_defaults", json!({ "medium": "hydraulic_oil", "size": "DN10", "prefix": "S" })).unwrap();
    assert_eq!(d["defaults"]["size"], "DN10");
    assert_eq!(d["hoses"][0]["size"], "DN10");
    let r = run(&mut s, "report", json!({ "kind": "hose_list" })).unwrap();
    assert!(r.to_string().contains("Water"), "{r}");
}

#[test]
fn pct_loops_of_the_pre_planning() {
    let mut s = new_session();
    let v = run(&mut s, "set_pct_loop", json!({ "name": "101", "description": "Tank temperature", "devices": "-KF101 -TA101" })).unwrap();
    assert_eq!(v[0]["devices"], json!(["-KF101", "-TA101"]));
    assert_eq!(v[0]["status"], "planned");
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "pid_instrument", "x": 100, "y": 100, "tag": "-BT101", "label": "TIC 101" })).unwrap();
    let v = run(&mut s, "list_pct_loops", json!({})).unwrap();
    assert_eq!(v[0]["functions"], json!(["TIC"]));
    assert_eq!(v[0]["measured_variable"], "Temperature");
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 200, "y": 100, "tag": "-KF101" })).unwrap();
    assert_eq!(run(&mut s, "list_pct_loops", json!({})).unwrap()[0]["status"], "in_engineering");
    assert!(run(&mut s, "set_pct_loop", json!({ "name": "102", "rename": "101" })).unwrap_err().contains("already exists"));
    run(&mut s, "set_pct_loop", json!({ "name": "101", "rename": "110" })).unwrap();
    let v = run(&mut s, "delete_pct_loop", json!({ "name": "110" })).unwrap();
    // The instrument still names loop 101.
    assert_eq!(v.as_array().unwrap().iter().filter(|l| l["defined"] == true).count(), 0);
    assert!(run(&mut s, "delete_pct_loop", json!({ "name": "110" })).is_err());
}
