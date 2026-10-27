use super::*;
use crate::builtin::testing::*;

#[test]
fn cabinets_in_a_room_with_rails_and_ducts_on_the_walls() {
    let mut s = demo_session();
    let r = run(&mut s, "add_room", json!({ "name": "+ROOM1", "width": 5000, "depth": 4000, "height": 3000, "x": 1000, "z": 2000 })).unwrap();
    assert_eq!(r["size"]["width"], 5000.0);
    assert!(run(&mut s, "add_room", json!({ "name": "+ROOM1" })).unwrap_err().contains("exists"));
    let r = run(&mut s, "place_in_room", json!({ "enclosure": "+CAB1", "room": "+ROOM1", "x": 800, "z": 100 })).unwrap();
    let cab = &r["cabinets"][0];
    assert_eq!(cab["enclosure"], "+CAB1");
    assert!((cab["x"].as_f64().unwrap() - 800.0).abs() < 0.01 && (cab["z"].as_f64().unwrap() - 100.0).abs() < 0.01, "{cab}");
    assert!(run(&mut s, "place_in_room", json!({ "enclosure": "+ROOM1", "room": "+ROOM1" })).is_err());
    assert!(run(&mut s, "place_in_room", json!({ "enclosure": "+CAB1", "room": "+CAB1" })).unwrap_err().contains("not a room"));
    // A contactor needs a rail on the wall.
    assert!(run(&mut s, "mount_on_wall", json!({ "room": "+ROOM1", "device": "-K1", "face": "wall1", "x": 3000, "y": 1000 })).unwrap_err().contains("wall mounting"));
    run(&mut s, "add_wall_profile", json!({ "room": "+ROOM1", "kind": "rail", "face": "wall1", "x1": 2900, "y1": 1040, "x2": 3400, "y2": 1040 })).unwrap();
    let m = run(&mut s, "mount_on_wall", json!({ "room": "+ROOM1", "device": "-K1", "face": "wall1", "x": 3000, "y": 1000 })).unwrap();
    assert_eq!(m["on_rail"], true);
    run(&mut s, "add_wall_profile", json!({ "room": "+ROOM1", "kind": "duct", "face": "ceiling", "x1": 100, "y1": 300, "x2": 4900, "y2": 300 })).unwrap();
    assert!(run(&mut s, "add_wall_profile", json!({ "room": "+ROOM1", "kind": "busbar", "face": "wall1", "x1": 0, "y1": 0, "x2": 1, "y2": 0 })).is_err());
    assert!(run(&mut s, "add_wall_profile", json!({ "room": "+ROOM1", "kind": "rail", "face": "wall9", "x1": 0, "y1": 0, "x2": 1, "y2": 0 })).is_err());
    let list = run(&mut s, "list_rooms", json!({})).unwrap();
    let faces: Vec<&str> = list[0]["faces"].as_array().unwrap().iter().map(|f| f["face"].as_str().unwrap()).collect();
    assert_eq!(faces, ["wall1", "ceiling"]);
    assert_eq!(list[0]["problems"], json!([]));
    // The room moves, its cabinets with it.
    let r = run(&mut s, "set_room", json!({ "room": "+ROOM1", "x": 0 })).unwrap();
    assert!((r["cabinets"][0]["x"].as_f64().unwrap() - 800.0).abs() < 0.01);
    assert_eq!(r["at"][0], 0.0);
    let r = run(&mut s, "place_in_room", json!({ "enclosure": "+CAB1" })).unwrap();
    assert_eq!(r["room"], Value::Null);
}

#[test]
fn irregular_rooms_from_corners() {
    let mut s = demo_session();
    run(&mut s, "add_room", json!({ "name": "+ROOM1" })).unwrap();
    let r = run(&mut s, "set_room_outline", json!({ "room": "+ROOM1", "corners": "0,0; 6000,0; 6000,6000; 3000,6000; 3000,3000; 0,3000" })).unwrap();
    assert_eq!(r["walls"].as_array().unwrap().len(), 6);
    assert_eq!(r["walls"][3]["wall"], "wall4");
    assert_eq!(r["size"]["width"], 6000.0);
    assert!(run(&mut s, "set_room_outline", json!({ "room": "+ROOM1", "corners": "0,0; 6000,0; 3000,3000" })).is_err());
    assert!(run(&mut s, "set_room_outline", json!({ "room": "+ROOM1", "corners": "0,0; x" })).unwrap_err().contains("corner"));
    // Wall 2 outwards by 500 mm, then wall 1 shortened to 5 m.
    let r = run(&mut s, "set_room_wall", json!({ "room": "+ROOM1", "wall": 2, "offset": 500 })).unwrap();
    assert_eq!(r["walls"][0]["length"], 6500.0);
    let r = run(&mut s, "set_room_wall", json!({ "room": "+ROOM1", "wall": 1, "length": 5000 })).unwrap();
    assert_eq!(r["outline"][2], json!([5000.0, 6000.0]));
    assert!(run(&mut s, "set_room_wall", json!({ "room": "+ROOM1", "wall": 4, "offset": -2500 })).is_err(), "into the other wall");
    // Width and depth make it rectangular again.
    let r = run(&mut s, "set_room", json!({ "room": "+ROOM1", "width": 4000, "depth": 3000 })).unwrap();
    assert_eq!(r["walls"].as_array().unwrap().len(), 4);
    let ceiling = run(&mut s, "add_wall_profile", json!({ "room": "+ROOM1", "kind": "duct", "face": "ceiling", "x1": 100, "y1": 300, "x2": 3900, "y2": 300 }));
    assert!(ceiling.is_ok());
}

#[test]
fn doors_and_windows() {
    let mut s = demo_session();
    run(&mut s, "add_room", json!({ "name": "+ROOM1" })).unwrap();
    let r = run(&mut s, "add_wall_opening", json!({ "room": "+ROOM1", "kind": "door", "wall": "wall1" })).unwrap();
    let item = &r["room"]["faces"][0]["items"][0];
    assert_eq!((item["kind"].as_str(), item["x"].as_f64(), item["w"].as_f64()), (Some("door"), Some(1500.0), Some(1000.0)));
    let r = run(&mut s, "add_wall_opening", json!({ "room": "+ROOM1", "kind": "window", "wall": "wall2", "x": 500, "width": 800, "sill": 1000 })).unwrap();
    let item = &r["room"]["faces"][1]["items"][0];
    assert_eq!((item["kind"].as_str(), item["w"].as_f64(), item["h"].as_f64()), (Some("window"), Some(800.0), Some(1200.0)));
    assert!(run(&mut s, "add_wall_opening", json!({ "room": "+ROOM1", "kind": "hatch", "wall": "wall1" })).is_err());
    assert!(run(&mut s, "add_wall_opening", json!({ "room": "+ROOM1", "kind": "door", "wall": "ceiling" })).is_err());
    // A cabinet in front of the door.
    let r = run(&mut s, "place_in_room", json!({ "enclosure": "+CAB1", "room": "+ROOM1", "x": 1700, "z": 300 })).unwrap();
    assert!(r["problems"].as_array().unwrap().iter().any(|p| p.as_str().unwrap().contains("swing")), "{}", r["problems"]);
}

#[test]
fn inner_walls() {
    let mut s = demo_session();
    run(&mut s, "add_room", json!({ "name": "+ROOM1" })).unwrap();
    let r = run(&mut s, "add_inner_wall", json!({ "room": "+ROOM1", "x1": 2000, "z1": 0, "x2": 2000, "z2": 2000 })).unwrap();
    let inner = &r["room"]["inner_walls"][0];
    assert_eq!((inner["wall"].as_str(), inner["thickness"].as_f64()), (Some("inner1"), Some(100.0)));
    assert_eq!(inner["sides"], json!(["inner1a", "inner1b"]));
    run(&mut s, "add_wall_profile", json!({ "room": "+ROOM1", "kind": "rail", "face": "inner1a", "x1": 100, "y1": 1000, "x2": 900, "y2": 1000 })).unwrap();
    run(&mut s, "add_wall_opening", json!({ "room": "+ROOM1", "kind": "door", "wall": "inner1b", "x": 1000 })).unwrap();
    assert!(run(&mut s, "add_inner_wall", json!({ "room": "+ROOM1", "x1": 0, "z1": 0, "x2": 5000, "z2": 0 })).unwrap_err().contains("leaves"));
    let r = run(&mut s, "set_inner_wall", json!({ "room": "+ROOM1", "wall": "inner1", "offset": 500, "thickness": 150 })).unwrap();
    assert_eq!(r["inner_walls"][0]["from"], json!([2500.0, 0.0]));
    let r = run(&mut s, "set_inner_wall", json!({ "room": "+ROOM1", "wall": "inner1a", "remove": true })).unwrap();
    assert_eq!(r["inner_walls"], json!([]));
    assert_eq!(r["faces"], json!([]), "its rail and door went with it");
}
