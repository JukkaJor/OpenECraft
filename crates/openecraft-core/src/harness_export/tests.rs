use super::*;
use crate::sample::demo_project;

#[test]
fn the_interface_document_holds_the_whole_harness() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let doc = document(&prj, &an, None).unwrap();
    assert_eq!(doc["format"], FORMAT);
    assert_eq!(doc["version"], FORMAT_VERSION);
    let h = &doc["harnesses"][0];
    assert_eq!(h["tag"], "-WH1");
    let ws = harness::wires(&prj, &an, "-WH1");
    assert_eq!(h["wires"].as_array().unwrap().len(), ws.len());
    assert_eq!(h["wire_count"], ws.len());
    let w = &h["wires"][0];
    for key in ["number", "net", "color", "cross_section_mm2", "length_mm", "length_source", "cut_length_mm", "from", "to", "segments"] {
        assert!(!w[key].is_null(), "{key}");
    }
    assert!(w["from"]["strip_mm"].as_f64().unwrap() > 0.0);
    assert!(w["cut_length_mm"].as_f64().unwrap() >= w["length_mm"].as_f64().unwrap());
    assert_eq!(h["topology"]["source"], "layout");
    assert!(!h["topology"]["segments"].as_array().unwrap().is_empty());
    // Every connector is at a node of the topology.
    assert!(h["connectors"].as_array().unwrap().iter().all(|c| c["node"].is_string()));
    // One harness by its tag, an unknown one is an error.
    assert_eq!(document(&prj, &an, Some("-WH1")).unwrap()["harnesses"].as_array().unwrap().len(), 1);
    assert!(document(&prj, &an, Some("-WH9")).is_err());
}

#[test]
fn processing_list_in_processing_order() {
    let mut prj = demo_project();
    prj.layout.routing.cut_wire = 50.0;
    let an = Analysis::checked(&prj);
    let csv = processing_list(&prj, &an, None).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], PROCESSING_COLUMNS.join(";"));
    assert_eq!(lines.len(), harness::wires(&prj, &an, "-WH1").len() + 1);
    let rows: Vec<Vec<&str>> = lines[1..].iter().map(|l| l.split(';').collect()).collect();
    assert!(rows.iter().all(|r| r.len() == PROCESSING_COLUMNS.len()));
    let col = |n: &str| PROCESSING_COLUMNS.iter().position(|c| *c == n).unwrap();
    // Numbered in order, cut lengths in steps of 50 mm, sorted by cross-section.
    for (i, r) in rows.iter().enumerate() {
        assert_eq!(r[col("sequence")], (i + 1).to_string());
        let cut: f32 = r[col("cut_length_mm")].parse().unwrap();
        assert_eq!(cut % 50.0, 0.0, "{cut}");
    }
    let cs: Vec<f32> = rows.iter().map(|r| r[col("cross_section_mm2")].parse().unwrap()).collect();
    assert!(cs.windows(2).all(|w| w[0] <= w[1]));
    assert_eq!(cut_length(&prj, 101.0), 150.0);
}

#[test]
fn the_bundle_in_3d() {
    let mut prj = demo_project();
    let an = Analysis::checked(&prj);
    crate::harness::topology::store(&mut prj, &an, "-WH1").unwrap();
    let seg = prj.harnesses[0].segments[0].id.clone();
    prj.harnesses[0].protections.push(harness::Protection { id: "P1".into(), segment: seg.clone(), start: 0.1, end: 0.9, ..Default::default() });
    prj.harnesses[0].fixings.push(harness::Fixing { id: "F1".into(), segment: seg, location: 0.5, ..Default::default() });
    let s = solids(&prj, &an, &prj.harnesses[0]);
    let t = crate::harness::topology::of(&prj, &an, "-WH1").unwrap();
    assert_eq!(s.len(), t.segments.len() + 2 + t.nodes.len());
    assert!(s.iter().any(|x| x.name == "P1" && !x.triangles.is_empty()));
    let g = gltf(&prj, &an, None).unwrap();
    assert!(g.contains("\"version\":\"2.0\"") && g.contains("\"-WH1\""));
    assert!(serde_json::from_str::<Value>(&g).is_ok());
    assert!(!jt(&prj, &an, Some("-WH1")).unwrap().is_empty());
}

#[test]
fn tubes_and_sub_polylines() {
    let p = vec![[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [100.0, 100.0, 0.0]];
    assert_eq!(tube(&p, 2.0).len(), 32);
    let part = sub_polyline(&p, 0.25, 0.75);
    assert_eq!(part, vec![[50.0, 0.0, 0.0], [100.0, 0.0, 0.0], [100.0, 50.0, 0.0]]);
    assert_eq!(view([1.0, 2.0, 3.0]), [1.0, -2.0, 3.0]);
}
