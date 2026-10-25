use super::*;
use crate::builtin::testing::*;

#[test]
fn harnesses_are_listed_with_their_topology() {
    let mut s = demo_session();
    let list = run(&mut s, "list_harnesses", json!({})).unwrap();
    let h = &list["harnesses"][0];
    assert_eq!(h["tag"], "-WH1");
    assert_eq!(h["topology"], "layout");
    assert!(h["wires"].as_u64().unwrap() > 0 && h["segments"].as_u64().unwrap() > 0);
    let data = run(&mut s, "harness_data", json!({ "harness": "-WH1" })).unwrap();
    assert_eq!(data["format"], "openecraft.harness");
    assert_eq!(data["harnesses"][0]["wire_count"], h["wires"]);
    assert!(run(&mut s, "harness_data", json!({ "harness": "-WH9" })).unwrap_err().contains("No harness"));
    let t = run(&mut s, "harness_topology", json!({ "harness": "-WH1" })).unwrap();
    assert_eq!(t["source"], "layout");
    assert!(t["routes"].as_array().unwrap().iter().all(|r| r["segments"].is_array()));
    assert!(t["problems"].as_array().unwrap().is_empty());
}

#[test]
fn the_topology_is_stored_and_edited() {
    let mut s = demo_session();
    let stored = run(&mut s, "store_harness_topology", json!({ "harness": "-WH1" })).unwrap();
    assert!(stored["nodes"].as_u64().unwrap() > 1);
    // A new branch node in the middle of the first segment.
    let t = run(&mut s, "harness_topology", json!({ "harness": "-WH1" })).unwrap();
    let seg = t["segments"][0].clone();
    let (start, end) = (seg["start"].as_str().unwrap().to_string(), seg["end"].as_str().unwrap().to_string());
    run(&mut s, "set_harness_node", json!({ "harness": "-WH1", "id": "NX", "x": 10.0, "y": 20.0, "z": 30.0, "bend_radius": 25.0 })).unwrap();
    run(&mut s, "set_harness_segment", json!({ "harness": "-WH1", "id": seg["id"], "end": "NX" })).unwrap();
    let t = run(&mut s, "set_harness_segment", json!({ "harness": "-WH1", "id": "SX", "start": "NX", "end": end, "points": "10,20,100; 50,20,100" })).unwrap();
    assert_eq!(t["source"], "stored");
    let sx = t["segments"].as_array().unwrap().iter().find(|x| x["id"] == "SX").unwrap().clone();
    assert_eq!(sx["points"].as_array().unwrap().len(), 4);
    assert!(t["problems"].as_array().unwrap().iter().all(|p| p["problem"] != "unknown_node"), "{t}");
    let prj = s.project().unwrap();
    let nx = prj.harnesses[0].nodes.iter().find(|n| n.id == "NX").unwrap();
    assert_eq!((nx.pos, nx.bend_radius), ([10.0, 20.0, 30.0], 25.0));
    assert!(prj.harnesses[0].segments.iter().any(|x| x.start == start && x.end == "NX"));
    // Wrong input.
    assert!(run(&mut s, "set_harness_segment", json!({ "harness": "-WH1", "id": "SY", "start": "NX" })).is_err());
    assert!(run(&mut s, "set_harness_segment", json!({ "harness": "-WH1", "id": "SX", "start": "NOPE" })).unwrap_err().contains("No node"));
    assert!(run(&mut s, "set_harness_segment", json!({ "harness": "-WH1", "id": "SX", "points": "1,2,3,4" })).unwrap_err().contains("Not a point"));
    // Connectors move from node to node; removing a node removes its segments.
    let prj = s.project().unwrap();
    let dev = prj.harnesses[0].nodes.iter().find(|n| !n.connectors.is_empty()).unwrap().connectors[0].clone();
    run(&mut s, "set_harness_node", json!({ "harness": "-WH1", "id": "NX", "connectors": dev })).unwrap();
    let prj = s.project().unwrap();
    assert_eq!(prj.harnesses[0].nodes.iter().filter(|n| n.connectors.contains(&dev)).count(), 1);
    run(&mut s, "set_harness_node", json!({ "harness": "-WH1", "id": "NX", "remove": true })).unwrap();
    assert!(!s.project().unwrap().harnesses[0].segments.iter().any(|x| x.start == "NX" || x.end == "NX"));
    assert!(run(&mut s, "set_harness_node", json!({ "harness": "-WH1", "id": "NX", "remove": true })).is_err());
    let cleared = run(&mut s, "clear_harness_topology", json!({ "harness": "-WH1" })).unwrap();
    assert_eq!(cleared["cleared"], true);
    assert!(s.project().unwrap().harnesses[0].nodes.is_empty());
}

#[test]
fn protections_fixings_ends_and_parts() {
    let mut s = demo_session();
    let p = run(&mut s, "set_harness_protection", json!({ "harness": "-WH1", "id": "P1", "kind": "corrugated_tube", "part": "NW10", "start": 0.1 })).unwrap();
    assert_eq!((p["kind"].as_str(), p["segment"].as_str(), p["end"].as_f64()), (Some("corrugated_tube"), Some(""), Some(1.0)));
    assert!(run(&mut s, "set_harness_protection", json!({ "harness": "-WH1", "id": "P2", "kind": "glue" })).unwrap_err().contains("Unknown protection"));
    assert!(run(&mut s, "set_harness_protection", json!({ "harness": "-WH1", "id": "P1", "end": 1.5 })).is_err());
    let f = run(&mut s, "set_harness_fixing", json!({ "harness": "-WH1", "id": "F1", "kind": "cable_tie", "segment": "S1" })).unwrap();
    assert_eq!((f["kind"].as_str(), f["location"].as_f64()), (Some("cable_tie"), Some(0.5)));
    let e = run(&mut s, "set_harness_end", json!({ "harness": "-WH1", "device": "-S1", "terminal": "RING-M4", "strip": 6.0, "usage": "ring terminal" })).unwrap();
    assert_eq!(e["usage"], "ring terminal");
    assert!(run(&mut s, "set_harness_end", json!({ "harness": "-WH1", "device": "-S1", "usage": "glued" })).unwrap_err().contains("Unknown usage"));
    let part = run(&mut s, "set_harness_part", json!({ "harness": "-WH1", "part": "BAG-1", "kind": "co_pack", "count": 3 })).unwrap();
    assert_eq!(part["count"], 3);
    let h = &s.project().unwrap().harnesses[0];
    assert_eq!((h.protections.len(), h.fixings.len(), h.ends.len(), h.parts.len()), (1, 1, 1, 1));
    // The KBL export carries them.
    let kbl = openecraft_core::kbl::export(s.project().unwrap(), &Analysis::checked(s.project().unwrap()), None).unwrap();
    assert!(kbl.contains("<Part_number>NW10</Part_number>") && kbl.contains("<Part_number>RING-M4</Part_number>") && kbl.contains("<Co_pack_occurrence "));
    for (cmd, args) in [
        ("set_harness_protection", json!({ "harness": "-WH1", "id": "P1", "remove": true })),
        ("set_harness_fixing", json!({ "harness": "-WH1", "id": "F1", "remove": true })),
        ("set_harness_end", json!({ "harness": "-WH1", "device": "-S1", "remove": true })),
        ("set_harness_part", json!({ "harness": "-WH1", "part": "BAG-1", "kind": "co_pack", "remove": true })),
    ] {
        run(&mut s, cmd, args).unwrap();
    }
    let h = &s.project().unwrap().harnesses[0];
    assert!(h.protections.is_empty() && h.fixings.is_empty() && h.ends.is_empty() && h.parts.is_empty());
    assert!(run(&mut s, "set_harness_fixing", json!({ "harness": "-WH9", "id": "F1" })).unwrap_err().contains("No harness"));
}
