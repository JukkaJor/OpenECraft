use super::*;
use crate::model::ReportKind;
use crate::reports::Analysis;
use crate::sample::demo_project;

/// The demo with a cable gland in the bottom of +CAB1, below the terminal strip.
fn demo_with_gland() -> Project {
    let mut prj = demo_project();
    crate::panel::place_accessory(&mut prj, 0, "GEN.GLAND-M25", Face::Bottom, crate::geom::p(80.0, 100.0)).unwrap();
    prj
}

#[test]
fn classes_diameters_and_kinds() {
    let prj = demo_project();
    assert_eq!(class_of_net(&prj, "L1"), Class::Power);
    assert_eq!(class_of_net(&prj, "PE"), Class::Power);
    assert_eq!(class_of_net(&prj, "+24V"), Class::Control);
    assert_eq!(class_of_net(&prj, "unknown"), Class::Control);
    assert!(Class::Power.keeps_apart(Class::Signal) && !Class::Control.keeps_apart(Class::Signal));
    assert_eq!(Class::Signal.apart_bits(), Class::Power.bit());
    assert_eq!(tube_diameter("8 mm"), 8.0);
    assert_eq!(tube_diameter("DN25"), 25.0);
    assert_eq!(tube_diameter(""), 8.0);
    let w1 = prj.cable("-W1").unwrap();
    assert!(cable_diameter(w1) > 8.0 && cable_diameter(w1) < 15.0, "{}", cable_diameter(w1));
    assert!(PathKind::Cables.accepts(RouteKind::Cable) && !PathKind::Cables.accepts(RouteKind::Wire) && PathKind::Any.accepts(RouteKind::Tube));
    assert!(ShieldEnds::First.connects(0) && !ShieldEnds::First.connects(1) && !ShieldEnds::None.connects(0));
    let p = RoutingPath { width: 20.0, reserve: 25.0, points: vec![[0.0; 3], [0.0, 300.0, 0.0], [400.0, 300.0, 0.0]], ..Default::default() };
    assert!((p.capacity() - std::f32::consts::PI * 75.0).abs() < 0.01);
    assert_eq!(p.length(), 700.0);
}

#[test]
fn field_cable_runs_to_its_gland() {
    let an = Analysis::checked(&demo_project());
    let w1 = an.routing.item("-W1").expect("motor cable");
    assert_eq!((w1.kind, w1.class), (RouteKind::Cable, Class::Power));
    assert!(w1.routed(), "{:?}", w1.status);
    assert_eq!(w1.ends[0].tag, "-X1");
    assert!(an.issues.iter().any(|i| i.message.contains("no cable gland")));
    let an = Analysis::checked(&demo_with_gland());
    let w1 = an.routing.item("-W1").unwrap();
    assert_eq!(w1.ends[1].tag, "-M1");
    assert!(w1.ends[1].item.is_some());
    assert!(!an.issues.iter().any(|i| i.message.contains("no cable gland")));
    // Length with both cable end reserves, cut length in steps of 100 mm.
    let rules = RoutingRules::default();
    assert!(w1.length > 2.0 * rules.cable_reserve);
    assert_eq!(w1.cut % rules.cut_cable, 0.0);
    assert!(w1.cut >= w1.length && w1.cut < w1.length + rules.cut_cable);
    assert_eq!(w1.enclosures, vec![0]);
}

#[test]
fn wires_in_ducts_form_bundles_with_branches() {
    let prj = demo_with_gland();
    let an = Analysis::checked(&prj);
    let wires: Vec<&RoutedItem> = an.routing.items.iter().filter(|i| i.kind == RouteKind::Wire).collect();
    assert!(!wires.is_empty());
    // Cores of -W1 are routed with their cable, not as wires.
    assert!(!wires.iter().any(|w| w.label.contains("-M1")));
    for w in &wires {
        let rw = &an.wiring[w.wire.unwrap()];
        assert_eq!(w.length, rw.length);
        assert_eq!(w.free, 0.0);
    }
    assert!(an.routing.bundles.iter().any(|b| b.items.len() >= 3));
    assert!(!an.routing.branches.is_empty());
    let report = &an.report(&prj, ReportKind::RoutingList);
    assert_eq!(report.rows.len(), an.routing.items.len());
    assert_eq!(an.report(&prj, ReportKind::BundleList).rows.len(), an.routing.bundles.len());
}

#[test]
fn wires_routed_in_3d_feed_the_duct_fill() {
    let mut prj = demo_with_gland();
    prj.layout.routing.wires_3d = true;
    let ducts = Analysis::checked(&demo_with_gland());
    let an = Analysis::checked(&prj);
    let free: Vec<&RoutedItem> = an.routing.items.iter().filter(|i| i.kind == RouteKind::Wire && an.wiring[i.wire.unwrap()].door == (None, None)).collect();
    assert!(!free.is_empty());
    for it in &free {
        assert!(it.routed(), "{} {:?}", it.label, it.status);
        let w = &an.wiring[it.wire.unwrap()];
        assert_eq!(it.length, w.length);
        assert!(w.route.points.len() >= 2);
    }
    // The 3D routes use the ducts: they still carry wires, and most of the way is in them.
    assert!(!an.duct_fill.is_empty());
    let (in_air, total): (f32, f32) = free.iter().fold((0.0, 0.0), |(a, t), i| (a + i.free, t + i.length));
    assert!(in_air < total * 0.5, "{in_air} of {total}");
    // Lengths are of the same order as through the ducts on the plate.
    let sum = |an: &Analysis| an.wiring.iter().map(|w| w.length).sum::<f32>();
    assert!(sum(&an) < sum(&ducts) * 1.5 && sum(&an) > sum(&ducts) * 0.5, "{} {}", sum(&an), sum(&ducts));
}

#[test]
fn frozen_routes_survive_recalculation_until_invalid() {
    let mut prj = demo_with_gland();
    let an = Analysis::checked(&prj);
    let n = freeze(&mut prj, &an.routing, &[]);
    // Wires that stay on the door have no route in the enclosure to freeze.
    assert_eq!(n, an.routing.items.iter().filter(|i| i.routed() && !i.enclosures.is_empty()).count());
    let an = Analysis::checked(&prj);
    assert_eq!(an.routing.item("-W1").unwrap().status, Status::Frozen);
    assert!(an.routing.changes.is_empty(), "{:?}", an.routing.changes);
    // A keep-out across the frozen cable route: routed again, and reported.
    let w1 = an.routing.item("-W1").unwrap().clone();
    let mid = w1.points[w1.points.len() / 2];
    prj.layout.enclosures[0].reservations.push(Reservation { id: 1, min: [mid[0] - 30.0, mid[1] - 30.0, 0.0], max: [mid[0] + 30.0, mid[1] + 30.0, 500.0], ..Default::default() });
    let an = Analysis::checked(&prj);
    let again = an.routing.item("-W1").unwrap();
    assert!(matches!(again.status, Status::Rerouted | Status::Unrouted(_)), "{:?}", again.status);
    assert!(an.issues.iter().any(|i| i.message.contains("Frozen route of -W1")));
    // Moving the terminal strip changes the length of its wires: a change against the
    // frozen length.
    prj.layout.enclosures[0].reservations.clear();
    let x1 = prj.layout.enclosures[0].items.iter().position(|i| i.tag == "-X1").unwrap();
    prj.layout.enclosures[0].items[x1].x += 200.0;
    let an = Analysis::checked(&prj);
    assert!(!an.routing.changes.is_empty());
    assert!(an.issues.iter().any(|i| i.message.contains("frozen routes changed")));
    // Released routes are calculated afresh.
    assert_eq!(release(&mut prj, &["-W1".to_string()]), 1);
    assert!(prj.layout.frozen.iter().all(|f| f.key != "-W1"));
    let all = prj.layout.frozen.len();
    assert_eq!(release(&mut prj, &[]), all);
    assert!(prj.layout.frozen.is_empty());
}

#[test]
fn bayed_enclosures_route_cables_through_passages() {
    let mut prj = demo_with_gland();
    let cab2 = crate::panel::new_enclosure(&mut prj, crate::panel::ENCLOSURE_PART, "+CAB2");
    prj.layout.enclosures.push(cab2);
    let i = prj.layout.enclosures[0].items.iter().position(|i| i.tag == "-X1").unwrap();
    let x1 = prj.layout.enclosures[0].items.remove(i);
    prj.layout.enclosures[1].items.push(x1);
    // The gland now in +CAB2 as well.
    crate::panel::place_accessory(&mut prj, 1, "GEN.GLAND-M25", Face::Bottom, crate::geom::p(80.0, 100.0)).unwrap();
    let an = Analysis::checked(&prj);
    let k1 = an.routing.items.iter().find(|i| i.kind == RouteKind::Wire && i.label.contains("-X1")).map(|i| i.status.clone());
    assert!(k1.is_none(), "not bayed: the wires are not routed at all");
    crate::panel::bay_right_of(&mut prj, 1, 0);
    crate::panel::add_passage(&mut prj, 0, 600.0, 80.0, 60.0);
    prj.layout.routing.wires_3d = true;
    let an = Analysis::checked(&prj);
    let across = an.routing.items.iter().find(|i| i.kind == RouteKind::Wire && i.enclosures.len() == 2).expect("a wire across");
    assert!(across.routed());
    let w = &an.wiring[across.wire.unwrap()];
    assert_eq!(w.legs.len(), 1);
    assert!(w.in_enclosure(0) && w.in_enclosure(1));
}

#[test]
fn rules_and_frozen_routes_round_trip() {
    let mut prj = demo_with_gland();
    prj.layout.routing.segregation = 150.0;
    prj.layout.enclosures[0].paths.push(RoutingPath { id: 5, name: "Tray".into(), kind: PathKind::Cables, points: vec![[50.0, 50.0, 100.0], [50.0, 700.0, 100.0]], width: 40.0, classes: vec![Class::Signal], reserve: 20.0 });
    prj.layout.enclosures[0].reservations.push(Reservation { id: 6, name: "Spare".into(), min: [300.0, 600.0, 20.0], max: [500.0, 750.0, 200.0], mode: ReserveMode::For(Class::Power) });
    let an = Analysis::checked(&prj);
    freeze(&mut prj, &an.routing, &["-W1".to_string()]);
    let json = serde_json::to_string(&prj.layout).unwrap();
    let back: crate::panel::Layout = serde_json::from_str(&json).unwrap();
    assert_eq!(back, prj.layout);
    assert!(!serde_json::to_string(&crate::panel::Layout::default()).unwrap().contains("routing"));
    // The cable path is reported with its use.
    assert!(an.routing.paths.iter().any(|p| p.path == 5 && p.capacity > 0.0));
}
