use super::*;
use crate::harness::wires;
use crate::sample::demo_project;

fn wire(n: &str, a: &str, b: &str, length: f32) -> HarnessWire {
    HarnessWire { number: n.into(), from: (a.into(), "1".into()), to: (b.into(), n.into()), cross_section: "0.75 mm²".into(), length, ..Default::default() }
}

#[test]
fn branch_lengths_fit_the_wire_lengths() {
    let b = branch_lengths(3, &[(0, 1, 1000.0), (0, 2, 700.0), (1, 2, 900.0)]);
    assert!((b[0].0 - 400.0).abs() < 1.0 && (b[1].0 - 600.0).abs() < 1.0 && (b[2].0 - 300.0).abs() < 1.0, "{b:?}");
}

#[test]
fn a_harness_outside_the_layout_is_a_star() {
    let prj = Project::default();
    let an = Analysis::default();
    let ws = vec![wire("1", "-A1", "-B1", 1000.0), wire("2", "-A1", "-C1", 700.0), wire("3", "-B1", "-C1", 900.0)];
    let t = derive(&prj, &an, &ws);
    assert_eq!(t.source, Source::Generated);
    // Three connectors and the common node, a branch to each connector.
    assert_eq!(t.nodes.len(), 4);
    assert_eq!(t.segments.len(), 3);
    assert!(t.nodes[3].connectors.is_empty());
    let a = t.node_of("-A1").unwrap();
    let branch = t.segments.iter().find(|s| s.end == a).unwrap();
    assert!((branch.length - 400.0).abs() < 1.0, "{}", branch.length);
    // Each wire runs over two branches; its length along the topology fits.
    assert_eq!(t.routes[0].as_ref().unwrap().len(), 2);
    assert!((t.wire_length(0).unwrap() - 1000.0).abs() < 2.0);
    assert!(t.segments.iter().all(|s| s.wires.len() == 2 && s.diameter > 0.0));
}

#[test]
fn the_demo_harness_follows_the_panel_routes() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let t = of(&prj, &an, "-WH1").unwrap();
    assert_eq!(t.source, Source::Layout);
    let ws = wires(&prj, &an, "-WH1");
    // Every connector is at a node, every wire is routed through connected segments.
    for w in &ws {
        assert!(t.node_of(&w.from.0).is_some() && t.node_of(&w.to.0).is_some());
    }
    for (i, r) in t.routes.iter().enumerate() {
        let r = r.as_ref().unwrap_or_else(|| panic!("wire {} not routed", ws[i].number));
        let mut at = t.node_of(&ws[i].from.0).unwrap();
        for &s in r {
            let seg = &t.segments[s];
            assert!(seg.start == at || seg.end == at, "wire {}: segment {} does not continue at {}", ws[i].number, seg.id, t.nodes[at].id);
            at = if seg.start == at { seg.end } else { seg.start };
        }
        assert!(t.nodes[at].connectors.contains(&ws[i].to.0), "wire {} ends at {}", ws[i].number, t.nodes[at].id);
    }
    // Shared stretches: some segment carries more than one wire, none is empty.
    assert!(t.segments.iter().any(|s| s.wires.len() > 1));
    assert!(t.segments.iter().all(|s| !s.wires.is_empty() && s.points.len() >= 2));
    // Unique ids.
    let mut ids: Vec<&str> = t.nodes.iter().map(|n| n.id.as_str()).chain(t.segments.iter().map(|s| s.id.as_str())).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), t.nodes.len() + t.segments.len());
    let (min, max) = t.bounds();
    assert!(max[0] > min[0]);
}

#[test]
fn stored_topology_is_edited_and_routed() {
    let mut prj = demo_project();
    let an = Analysis::checked(&prj);
    let derived = of(&prj, &an, "-WH1").unwrap();
    assert_eq!(store(&mut prj, &an, "-WH1"), Some((derived.nodes.len(), derived.segments.len())));
    let t = of(&prj, &an, "-WH1").unwrap();
    assert_eq!(t.source, Source::Stored);
    assert_eq!(t.routes.iter().filter(|r| r.is_some()).count(), derived.routes.iter().filter(|r| r.is_some()).count());
    assert!((t.length() - derived.length()).abs() < 1.0);
    assert!(problems(&prj, &an, &prj.harnesses[0]).is_empty(), "{:?}", problems(&prj, &an, &prj.harnesses[0]));
    // A formboard length overrides the centre curve.
    prj.harnesses[0].segments[0].length = 1234.0;
    let t = of(&prj, &an, "-WH1").unwrap();
    assert_eq!(t.segments[0].length, 1234.0);
    assert!(t.segments[0].physical != 1234.0);
    assert!(clear(&mut prj, "-WH1"));
    assert!(!clear(&mut prj, "-WH1"));
    assert_eq!(of(&prj, &an, "-WH1").unwrap().source, Source::Layout);
    assert_eq!(store(&mut prj, &an, "-WH9"), None);
}

#[test]
fn problems_of_a_stored_topology() {
    let mut prj = demo_project();
    let an = Analysis::checked(&prj);
    store(&mut prj, &an, "-WH1").unwrap();
    let h = &mut prj.harnesses[0];
    let connector = h.nodes.iter().find(|n| !n.connectors.is_empty()).unwrap().connectors[0].clone();
    for n in &mut h.nodes {
        n.connectors.retain(|c| *c != connector);
    }
    h.segments[0].end = "N99".into();
    let dup = h.nodes[1].id.clone();
    h.nodes[0].id = dup.clone();
    h.nodes[2].bend_radius = 0.5;
    h.protections.push(crate::harness::Protection { id: "P1".into(), segment: "S99".into(), start: 0.0, end: 2.0, ..Default::default() });
    h.fixings.push(crate::harness::Fixing { id: "F1".into(), segment: h.segments[1].id.clone(), location: 0.5, ..Default::default() });
    let p = problems(&prj, &an, &prj.harnesses[0]);
    assert!(p.contains(&Problem::DuplicateId(dup)), "{p:?}");
    assert!(p.contains(&Problem::UnknownNode("S1".into(), "N99".into())), "{p:?}");
    assert!(p.contains(&Problem::UnknownSegment("P1".into(), "S99".into())));
    assert!(p.contains(&Problem::OutOfRange("P1".into())));
    assert!(p.contains(&Problem::ConnectorWithoutNode(connector)));
    assert!(p.iter().any(|x| matches!(x, Problem::BendRadius(..))), "{p:?}");
    assert!(!p.iter().any(|x| matches!(x, Problem::UnknownSegment(id, _) if id == "F1")));
}

#[test]
fn shortest_way_through_the_segments() {
    let node = |id: &str| Node { id: id.into(), pos: [0.0; 3], connectors: Vec::new(), bend_radius: 0.0 };
    let seg = |a: usize, b: usize, l: f32| Segment { id: String::new(), start: a, end: b, points: Vec::new(), physical: l, length: l, wires: Vec::new(), diameter: 0.0 };
    let nodes = vec![node("A"), node("B"), node("C"), node("D")];
    // A–B–D is 2 + 2, A–C–D is 1 + 1.
    let segments = vec![seg(0, 1, 2.0), seg(1, 3, 2.0), seg(0, 2, 1.0), seg(2, 3, 1.0)];
    assert_eq!(shortest(&nodes, &segments, 0, 3), Some(vec![2, 3]));
    assert_eq!(shortest(&nodes, &segments, 3, 0), Some(vec![3, 2]));
    assert_eq!(shortest(&nodes, &segments[..1], 0, 3), None);
    assert_eq!(shortest(&nodes, &segments, 1, 1), Some(vec![]));
}

#[test]
fn polylines() {
    let p = vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0]];
    assert_eq!(polyline_length(&p), 20.0);
    assert_eq!(point_at(&p, 0.75), [10.0, 5.0, 0.0]);
    assert_eq!(point_at(&p, 0.0), [0.0; 3]);
    assert_eq!(simplify(vec![[0.0; 3], [5.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 3.0, 0.0]]), vec![[0.0; 3], [10.0, 0.0, 0.0], [10.0, 3.0, 0.0]]);
    // The cells of a stretch are the same in both directions.
    let a = cells(&[[0.0; 3], [37.0, 81.0, 5.0]], 10.0);
    let mut b = cells(&[[37.0, 81.0, 5.0], [0.0; 3]], 10.0);
    b.reverse();
    assert_eq!(a, b);
}
