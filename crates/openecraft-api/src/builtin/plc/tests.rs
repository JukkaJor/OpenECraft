use super::*;
use crate::builtin::testing::*;

fn demo() -> Session {
    demo_session()
}

#[test]
fn hardware_is_configured_and_changed() {
    let mut s = demo();
    let hw = run(&mut s, "get_plc_hardware", json!({})).unwrap();
    assert_eq!(hw["unassigned_cards"], json!(["-A1", "-A2"]));
    let hw = run(&mut s, "configure_plc_hardware", json!({ "cables": true })).unwrap();
    assert_eq!(hw["created"]["stations"], 1);
    assert_eq!(hw["created"]["cables"], 0, "a single station has no connections");
    assert_eq!(hw["stations"][0]["modules"][1]["card"], "-A2");
    assert_eq!(hw["stations"][0]["modules"][1]["slot"], 3);
    assert_eq!(hw["problems"], json!([]));

    let hw = run(&mut s, "set_station", json!({ "name": "PLC1", "scheme": "iec", "new_name": "Main" })).unwrap();
    assert_eq!(hw["addresses_changed"], 5);
    assert_eq!(hw["stations"][0]["name"], "Main");
    assert!(run(&mut s, "set_station", json!({ "name": "Main", "scheme": "x" })).unwrap_err().contains("Unknown address scheme"));

    run(&mut s, "set_station", json!({ "name": "ET1", "kind": "io_device", "device": "GEN.IM-PN", "network": "BUS_1", "controller": "Main", "address": "192.168.0.5" })).unwrap();
    let hw = run(&mut s, "set_plc_card", json!({ "card": "-A2", "station": "ET1", "slot": 1, "start_byte": 4 })).unwrap();
    assert_eq!(hw["addresses_changed"], 2);
    assert!(run(&mut s, "set_plc_card", json!({ "card": "-A2", "station": "NONE" })).is_err());

    let hw = run(&mut s, "connect_ports", json!({ "a": "Main:X1 P2", "b": "ET1:X1 P1", "cable": "-W5" })).unwrap();
    assert_eq!(hw["connections"], json!([{ "network": "BUS_1", "a": "Main:X1 P2", "b": "ET1:X1 P1", "cable": "-W5" }]));
    assert!(hw["problems"].as_array().unwrap().iter().any(|p| p.as_str().unwrap().contains("cable -W5 does not exist")));
    let hw = run(&mut s, "create_bus_cables", json!({})).unwrap();
    assert_eq!(hw["created"], 0, "the connection names a cable");
    run(&mut s, "connect_ports", json!({ "a": "Main:X1 P2", "b": "ET1:X1 P1", "create_cable": true })).unwrap();
    let hw = run(&mut s, "get_plc_hardware", json!({})).unwrap();
    let cable = hw["connections"][0]["cable"].as_str().unwrap().to_string();
    let bus = hw["bus_cables"].as_array().unwrap().iter().find(|c| c["cable"] == cable.as_str()).unwrap();
    assert_eq!((bus["bus"].as_str(), bus["connections"].as_u64()), (Some("profinet"), Some(1)));
    run(&mut s, "disconnect_port", json!({ "port": "ET1:X1 P1" })).unwrap();
    assert!(run(&mut s, "disconnect_port", json!({ "port": "ET1:X1 P1" })).is_err());
    let hw = run(&mut s, "set_bus_system", json!({ "name": "BUS_1", "remove": true })).unwrap();
    assert_eq!(hw["bus_systems"], json!([]));
    let hw = run(&mut s, "remove_station", json!({ "name": "ET1" })).unwrap();
    assert_eq!(hw["unassigned_cards"], json!(["-A2"]));
}
