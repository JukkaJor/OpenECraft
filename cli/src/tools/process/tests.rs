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

#[test]
fn hose_configuration_and_fluid_netlist() {
    let mut s = new_session();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "fl_source", "x": 100, "y": 120 })).unwrap();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "fl_filter", "x": 100, "y": 100 })).unwrap();
    run(&mut s, "add_wire", json!({ "page": "1", "points": [[100, 124], [100, 140], [130, 140], [130, 96], [100, 96]] })).unwrap();
    let nets = run(&mut s, "list_fluid_nets", json!({})).unwrap();
    assert_eq!(nets[0]["line"], "pressure", "{nets}");
    assert!(nets[0]["ports"].as_array().unwrap().iter().any(|p| p["kind"] == "supply"));
    let net = nets[0]["net"].as_str().unwrap().to_string();
    let h = run(&mut s, "configure_hose", json!({ "hose": net, "line": "tube", "part": "PUN-8", "fitting_a": "QS-8", "pressure": 10, "length": 1.5 })).unwrap();
    assert_eq!((h[0]["line"].as_str(), h[0]["part"].as_str(), h[0]["length_m"].as_f64()), (Some("tube"), Some("PUN-8"), Some(1.5)), "{h}");
    assert!(run(&mut s, "configure_hose", json!({ "hose": net, "line": "rope" })).unwrap_err().contains("Unknown line type"));
    assert!(run(&mut s, "configure_hose", json!({ "hose": "H99" })).unwrap_err().contains("No hose"));
    let d = run(&mut s, "set_fluid_defaults", json!({ "pressure": 12, "allowance": 25 })).unwrap();
    assert_eq!(d["defaults"]["pressure_bar"], 12.0);
    assert!((d["hoses"][0]["cut_length_m"].as_f64().unwrap() - 1.55).abs() < 1e-4);
    let c = run(&mut s, "check", json!({})).unwrap().to_string();
    assert!(c.contains("rated for 10 bar"), "{c}");
    for kind in ["hose_configuration", "tubing_list"] {
        let r = run(&mut s, "report", json!({ "kind": kind })).unwrap();
        assert!(r.to_string().contains("PUN-8"), "{kind}: {r}");
    }
    assert!(run(&mut s, "report", json!({ "kind": "fluid_netlist" })).unwrap().to_string().contains("Pressure line"));
}

#[test]
fn device_groups_and_electro_fluid_links() {
    let mut s = new_session();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "fl_valve52", "x": 100, "y": 100, "tag": "-QM1" })).unwrap();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "fl_valve42", "x": 140, "y": 100, "tag": "-QM2" })).unwrap();
    run(&mut s, "place_symbol", json!({ "page": "1", "symbol": "coil", "x": 300, "y": 100, "tag": "-MB1" })).unwrap();
    let g = run(&mut s, "set_fluid_group", json!({ "tag": "-KH1", "kind": "valve_terminal", "slots": 4, "members": "-QM1 -QM2" })).unwrap();
    assert_eq!(g[0]["members"], json!(["-QM1", "-QM2"]));
    assert!(run(&mut s, "set_fluid_group", json!({ "tag": "-KH1", "kind": "bucket" })).unwrap_err().contains("Unknown group kind"));
    run(&mut s, "set_fluid_group", json!({ "tag": "-KH2" })).unwrap();
    assert!(run(&mut s, "set_fluid_group", json!({ "tag": "-KH2", "rename": "-KH1" })).unwrap_err().contains("already exists"));
    assert_eq!(run(&mut s, "delete_fluid_group", json!({ "tag": "-KH2" })).unwrap().as_array().unwrap().len(), 1);
    let v = run(&mut s, "sync_electro_fluid", json!({})).unwrap();
    assert_eq!(v["linked"], json!([{ "fluid": "-QM1", "electrical": "-MB1" }]), "{v}");
    let list = run(&mut s, "list_electro_fluid", json!({})).unwrap();
    let qm2 = list.as_array().unwrap().iter().find(|e| e["tag"] == "-QM2").unwrap();
    assert_eq!(qm2["electrical"], json!([]));
    run(&mut s, "link_fluid_device", json!({ "tag": "-QM2", "devices": "-MB2" })).unwrap();
    let c = run(&mut s, "check", json!({})).unwrap().to_string();
    assert!(c.contains("linked electrical device -MB2 is not in the schematic"), "{c}");
    run(&mut s, "link_fluid_device", json!({ "tag": "-QM2", "devices": "" })).unwrap();
    let r = run(&mut s, "report", json!({ "kind": "fluid_device_groups" })).unwrap().to_string();
    assert!(r.contains("-KH1") && r.contains("-QM2"), "{r}");
    assert!(run(&mut s, "report", json!({ "kind": "electro_fluid" })).unwrap().to_string().contains("-MB1"));
}
