use super::*;
use crate::builtin::testing::*;

fn demo() -> Session {
    let mut s = demo_session();
    s.project_mut().unwrap().info.name = "Demo".into();
    s
}

#[test]
fn wires_are_listed_marked_and_counted() {
    let mut s = demo();
    let list = run(&mut s, "wiring_list", json!({ "order": "cross_section" })).unwrap();
    let wires = list["wires"].as_array().unwrap().clone();
    assert!(!wires.is_empty());
    assert_eq!(list["progress"]["wired"], 0);
    assert!(wires.iter().all(|w| w["status"] == "open" && w["strip_mm"].is_number()));
    let (from, to) = (wires[0]["from"].as_str().unwrap(), wires[0]["to"].as_str().unwrap());
    // The ends may be given in either order.
    let r = run(&mut s, "mark_wire", json!({ "from": to, "to": from, "state": "wired" })).unwrap();
    assert_eq!((r["status"].as_str(), r["wired"].as_i64()), (Some("wired"), Some(1)));
    let open = run(&mut s, "wiring_list", json!({ "status": "open" })).unwrap();
    assert_eq!(open["wires"].as_array().unwrap().len(), wires.len() - 1);
    run(&mut s, "mark_wire", json!({ "from": from, "to": to, "state": "problem", "note": "too short" })).unwrap();
    let p = run(&mut s, "wiring_list", json!({ "status": "problem" })).unwrap();
    assert_eq!(p["wires"][0]["note"], "too short");
    assert_eq!(p["progress"]["problems"], 1);
    run(&mut s, "mark_wire", json!({ "from": from, "to": to, "state": "open" })).unwrap();
    assert!(s.project().unwrap().wiring_marks.is_empty());
}

#[test]
fn wrong_input_is_rejected() {
    let mut s = demo();
    assert!(run(&mut s, "mark_wire", json!({ "from": "-X9:1", "to": "-X9:2", "state": "wired" })).unwrap_err().contains("No routed wire"));
    assert!(run(&mut s, "mark_wire", json!({ "from": "a", "to": "b", "state": "half" })).unwrap_err().contains("Unknown state"));
    assert!(run(&mut s, "wiring_list", json!({ "order": "colour" })).unwrap_err().contains("Unknown order"));
    assert!(run(&mut s, "wiring_list", json!({ "enclosure": "+NOPE" })).unwrap_err().contains("No enclosure"));
    assert_eq!(run(&mut s, "remove_old_wiring_marks", json!({})).unwrap()["removed"], 0);
}
