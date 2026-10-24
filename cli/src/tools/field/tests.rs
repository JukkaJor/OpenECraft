use super::*;
use crate::testing::*;

/// Demo with a plant layout: +CAB1 – tray T1 – junction J1 – free run F1 – motor -M1.
fn plant() -> Session {
    let mut s = session();
    run(&mut s, "new_project", json!({ "name": "Demo", "demo": true })).unwrap();
    run(&mut s, "field_add_node", json!({ "name": "CAB", "x": 0, "y": 0 })).unwrap();
    run(&mut s, "field_add_node", json!({ "name": "J1", "x": 20, "y": 0 })).unwrap();
    run(&mut s, "field_add_node", json!({ "name": "M", "x": 20, "y": 0, "z": 3 })).unwrap();
    let t = run(&mut s, "field_add_segment", json!({ "a": "CAB", "b": "J1", "name": "T1", "width": 200, "height": 60, "part": "TRAY-200" })).unwrap();
    assert_eq!(t["length_m"], 20.0);
    run(&mut s, "field_add_segment", json!({ "a": "J1", "b": "M", "name": "F1", "kind": "free" })).unwrap();
    run(&mut s, "field_place", json!({ "tag": "+CAB1", "node": "CAB" })).unwrap();
    run(&mut s, "field_place", json!({ "tag": "-M1", "node": "M", "drop": 1 })).unwrap();
    s
}

#[test]
fn network_places_and_plan() {
    let mut s = plant();
    assert!(run(&mut s, "field_add_node", json!({ "name": "J1", "x": 1, "y": 1 })).unwrap_err().contains("exists"));
    assert!(run(&mut s, "field_add_segment", json!({ "a": "J1", "b": "X" })).unwrap_err().contains("Unknown node"));
    assert!(run(&mut s, "field_add_segment", json!({ "a": "J1", "b": "J1" })).unwrap_err().contains("two different"));
    assert!(run(&mut s, "field_add_segment", json!({ "a": "J1", "b": "CAB", "kind": "rope" })).unwrap_err().contains("Unknown segment kind"));
    let p = run(&mut s, "field_plan", json!({})).unwrap();
    let c = &p["cables"][0];
    assert_eq!((c["cable"].as_str(), c["routed"].as_bool()), (Some("-W1"), Some(true)));
    assert_eq!(c["segments"], json!(["T1", "F1"]));
    assert_eq!(c["network_m"], 23.0);
    assert_eq!(p["segments"][0]["segment"], "T1");
    assert_eq!(p["material"][0]["part"], "TRAY-200");
    assert_eq!(p["islands"], 1);
    let p = run(&mut s, "field_plan", json!({ "apply": true })).unwrap();
    assert_eq!(p["applied"], 1);
    assert_eq!(s.project().unwrap().cable("-W1").unwrap().length as f64, p["cables"][0]["cut_m"].as_f64().unwrap());
}

#[test]
fn route_definition_rules_and_removal() {
    let mut s = plant();
    run(&mut s, "field_add_node", json!({ "name": "D", "x": 0, "y": 20 })).unwrap();
    run(&mut s, "field_add_segment", json!({ "a": "CAB", "b": "D", "name": "L1", "kind": "ladder" })).unwrap();
    run(&mut s, "field_add_segment", json!({ "a": "D", "b": "J1", "name": "L2", "kind": "ladder", "classes": "power, control" })).unwrap();
    assert!(run(&mut s, "field_add_segment", json!({ "a": "D", "b": "J1", "classes": "mains" })).unwrap_err().contains("Unknown class"));
    let r = run(&mut s, "field_route", json!({ "cable": "-W1", "avoid": "T1" })).unwrap();
    assert_eq!(r["avoid"].as_array().unwrap().len(), 1);
    assert!(run(&mut s, "field_route", json!({ "cable": "-W9" })).unwrap_err().contains("Unknown cable"));
    assert!(run(&mut s, "field_route", json!({ "cable": "-W1", "via": "Q" })).unwrap_err().contains("Unknown node"));
    let p = run(&mut s, "field_plan", json!({})).unwrap();
    assert_eq!(p["cables"][0]["segments"], json!(["L1", "L2", "F1"]));
    run(&mut s, "field_route", json!({ "cable": "-W1" })).unwrap();
    assert!(s.project().unwrap().field.routes.is_empty());
    let r = run(&mut s, "field_rules", json!({ "reserve": 1, "cut": 1, "fill": 300, "main_earth": "PAS" })).unwrap();
    assert_eq!((r["reserve"].as_f64(), r["fill"].as_f64(), r["main_earth"].as_str()), (Some(1.0), Some(100.0), Some("PAS")));
    run(&mut s, "field_remove", json!({ "what": "segment", "id": "L2" })).unwrap();
    run(&mut s, "field_remove", json!({ "what": "node", "id": "D" })).unwrap();
    assert_eq!(s.project().unwrap().field.segments.len(), 2);
    run(&mut s, "field_remove", json!({ "what": "place", "id": "-M1" })).unwrap();
    assert!(run(&mut s, "field_remove", json!({ "what": "place", "id": "-M1" })).unwrap_err().contains("Nothing to remove"));
    assert!(run(&mut s, "field_remove", json!({ "what": "pipe", "id": "1" })).unwrap_err().contains("Unknown field object"));
    let p = run(&mut s, "field_plan", json!({})).unwrap();
    assert!(p["cables"][0]["status"].as_str().unwrap().contains("-M1"));
}

#[test]
fn bonding_outlines_and_accessories() {
    let mut s = plant();
    assert!(run(&mut s, "field_add_outline", json!({ "name": "+M1", "points": [[18, -2], [22, -2]] })).unwrap_err().contains("3 points"));
    run(&mut s, "field_add_outline", json!({ "name": "+M1", "points": [[18, -2], [22, -2], [22, 2]], "conductive": true })).unwrap();
    assert!(run(&mut s, "field_add_bond", json!({ "a": "MET", "b": "+M9" })).unwrap_err().contains("Unknown bonding point"));
    assert!(run(&mut s, "field_add_bond", json!({ "a": "MET", "b": "+M1", "kind": "loose" })).unwrap_err().contains("Unknown bonding kind"));
    let b = run(&mut s, "field_add_bond", json!({ "a": "MET", "b": "+CAB1", "cross_section": "16", "length": 5, "part": "H07V-K 16" })).unwrap();
    assert_eq!(b["kind"], "Main");
    run(&mut s, "field_add_bond", json!({ "a": "+CAB1", "b": "+M1", "kind": "supplementary", "cross_section": "1.5" })).unwrap();
    let p = run(&mut s, "field_plan", json!({})).unwrap();
    let bonded: Vec<(String, bool)> = p["bonding"].as_array().unwrap().iter().map(|b| (b["point"].as_str().unwrap().to_string(), b["bonded"].as_bool().unwrap())).collect();
    assert!(bonded.contains(&("+M1".into(), true)) && bonded.contains(&("T1".into(), false)));
    let problems: Vec<&str> = p["problems"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    assert!(problems.iter().any(|m| m.contains("1.5 mm²")));
    assert!(problems.iter().any(|m| m.contains("T1")));
    assert!(run(&mut s, "field_add_accessory", json!({ "cable": "-W1", "kind": "rivet" })).unwrap_err().contains("Unknown accessory kind"));
    assert!(run(&mut s, "field_add_accessory", json!({ "cable": "-W5", "kind": "gland" })).unwrap_err().contains("Unknown cable"));
    let acc = run(&mut s, "field_add_accessory", json!({ "cable": "-W1", "kind": "emc_gland", "part": "EMC-M25", "count": 2 })).unwrap();
    assert_eq!(acc["count"], 2);
    let p = run(&mut s, "field_plan", json!({})).unwrap();
    assert!(p["material"].as_array().unwrap().iter().any(|m| m["part"] == "EMC-M25" && m["quantity"] == 4.0));
    let id = acc["id"].as_u64().unwrap().to_string();
    run(&mut s, "field_remove", json!({ "what": "accessory", "id": id })).unwrap();
    run(&mut s, "field_remove", json!({ "what": "outline", "id": "+M1" })).unwrap();
    assert!(s.project().unwrap().field.accessories.is_empty() && s.project().unwrap().field.outlines.is_empty());
}
