use super::*;
use crate::builtin::testing::*;

#[test]
fn racks_with_parts_on_their_units() {
    let mut s = demo_session();
    let r = run(&mut s, "add_rack", json!({ "name": "+RACK1" })).unwrap();
    assert_eq!((r["units"].as_u64(), r["free"].as_u64()), (Some(42), Some(42)));
    assert!(run(&mut s, "add_rack", json!({ "name": "+RACK1" })).unwrap_err().contains("exists"));
    assert!(run(&mut s, "add_rack", json!({ "name": "+RACK2", "width": 400 })).unwrap_err().contains("19-inch"));
    let m = run(&mut s, "mount_in_rack", json!({ "rack": "+RACK1", "device": "GEN.RK-PATCH-24P" })).unwrap();
    assert_eq!((m["unit"].as_u64(), m["units"].as_u64()), (Some(42), Some(1)));
    let m = run(&mut s, "mount_in_rack", json!({ "rack": "+RACK1", "device": "GEN.RK-UPS-2U", "unit": 1 })).unwrap();
    assert_eq!(m["rack"]["free"].as_u64(), Some(39));
    assert!(run(&mut s, "mount_in_rack", json!({ "rack": "+RACK1", "device": "GEN.RK-SERVER-2U", "unit": 2 })).unwrap_err().contains("taken"));
    assert!(run(&mut s, "mount_in_rack", json!({ "rack": "+RACK1", "device": "GEN.RK-SERVER-2U", "unit": 0 })).is_err());
    assert!(run(&mut s, "mount_in_rack", json!({ "rack": "+CAB1", "device": "GEN.RK-SERVER-2U" })).unwrap_err().contains("19-inch rack"));
    let list = run(&mut s, "list_racks", json!({})).unwrap();
    let units: Vec<u64> = list[0]["parts"].as_array().unwrap().iter().map(|p| p["unit"].as_u64().unwrap()).collect();
    assert_eq!(units, [42, 1]);
    assert_eq!(list[0]["problems"], json!([]));
    // In a room, at its rear.
    run(&mut s, "add_room", json!({ "name": "+ROOM1" })).unwrap();
    run(&mut s, "add_rack", json!({ "name": "+RACK3", "room": "+ROOM1" })).unwrap();
    let rooms = run(&mut s, "list_rooms", json!({ "room": "+ROOM1" })).unwrap();
    assert_eq!(rooms[0]["cabinets"][0]["enclosure"], "+RACK3");
}
