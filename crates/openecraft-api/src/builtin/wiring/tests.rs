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

#[test]
fn numbering_of_wires_devices_and_pages() {
    let mut s = demo();
    let set = run(&mut s, "set_wire_numbering", json!({ "scheme": "format", "format": "{page}.{n}", "digits": 2, "skip_kinds": "pe" })).unwrap();
    assert_eq!((set["scheme"].as_str(), set["digits"].as_u64()), (Some("format"), Some(2)));
    assert_eq!(set["skip_kinds"], json!(["pe"]));
    assert!(run(&mut s, "set_wire_numbering", json!({ "scheme": "random" })).is_err());
    assert!(run(&mut s, "set_wire_numbering", json!({ "skip_kinds": "earth" })).is_err());
    assert!(run(&mut s, "renumber_wires", json!({})).unwrap()["renumbered"].as_u64().unwrap() > 0);
    let numbers: Vec<String> = s.project().unwrap().pages.iter().flat_map(|p| &p.wires).map(|w| w.number.clone()).filter(|n| !n.is_empty()).collect();
    assert!(numbers.iter().all(|n| n.split_once('.').is_some_and(|(_, c)| c.len() >= 2)), "{numbers:?}");

    let plan = run(&mut s, "renumber_devices", json!({ "format": "{letter}{page}{column}", "dry_run": true })).unwrap();
    let renamed = plan["renamed"].as_array().unwrap().clone();
    assert!(!renamed.is_empty());
    let first_old = renamed[0]["old"].as_str().unwrap().to_string();
    let first_new = renamed[0]["new"].as_str().unwrap().to_string();
    assert!(s.project().unwrap().all_tags().contains(&first_old), "a dry run changes nothing");
    run(&mut s, "renumber_devices", json!({ "format": "{letter}{page}{column}" })).unwrap();
    assert!(s.project().unwrap().all_tags().contains(&first_new));
    assert!(run(&mut s, "renumber_devices", json!({ "pages": "nope" })).is_err());

    let pages = run(&mut s, "renumber_pages", json!({ "start": 10, "step": 10 })).unwrap();
    assert_eq!(pages["pages"][0], "10");
    assert_eq!(pages["pages"][1], "20");
}

#[test]
fn cross_reference_settings() {
    let mut s = demo();
    let x = run(&mut s, "set_cross_references", json!({ "row": true, "interruptions": "pairs", "contact_image": false })).unwrap();
    assert_eq!((x["row"].as_bool(), x["interruptions"].as_str(), x["contact_image"].as_bool()), (Some(true), Some("pairs"), Some(false)));
    assert!(run(&mut s, "set_cross_references", json!({ "structure": "sometimes" })).is_err());
    let x = s.project().unwrap().xref.clone();
    assert!(x.row && x.hide_contact_image);
}
