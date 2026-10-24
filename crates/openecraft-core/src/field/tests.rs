use super::*;
use crate::model::ReportKind;
use crate::analysis::Analysis;
use crate::sample::demo_project;

fn node(prj: &mut Project, name: &str, x: f32, y: f32, z: f32) -> u64 {
    let id = prj.new_id();
    prj.field.nodes.push(Node { id, name: name.into(), x, y, z });
    id
}

fn segment(prj: &mut Project, name: &str, a: u64, b: u64, kind: SegmentKind) -> u64 {
    let id = prj.new_id();
    prj.field.segments.push(Segment { id, name: name.into(), a, b, kind, ..Default::default() });
    id
}

/// Demo with a plant layout: +CAB1 at the origin, a tray to a junction 10 m away and
/// a free run up to the motor -M1 (2 m higher, drop 1 m); a detour over a ladder.
fn plant() -> (Project, [u64; 4], [u64; 4]) {
    let mut prj = demo_project();
    let cab = node(&mut prj, "CAB", 0.0, 0.0, 0.0);
    let j = node(&mut prj, "J1", 10.0, 0.0, 0.0);
    let m = node(&mut prj, "M", 10.0, 0.0, 2.0);
    let d = node(&mut prj, "D", 0.0, 10.0, 0.0);
    let t1 = segment(&mut prj, "T1", cab, j, SegmentKind::Tray);
    let f1 = segment(&mut prj, "F1", j, m, SegmentKind::Free);
    let l1 = segment(&mut prj, "L1", cab, d, SegmentKind::Ladder);
    let l2 = segment(&mut prj, "L2", d, j, SegmentKind::Ladder);
    prj.field.places.push(Place { tag: "+CAB1".into(), node: cab, drop: 0.0 });
    prj.field.places.push(Place { tag: "-M1".into(), node: m, drop: 1.0 });
    (prj, [cab, j, m, d], [t1, f1, l1, l2])
}

fn planned(prj: &Project) -> Plan {
    Analysis::checked(prj).field
}

#[test]
fn empty_field_plans_nothing() {
    let prj = demo_project();
    let plan = planned(&prj);
    assert!(plan.cables.is_empty() && plan.issues.is_empty());
    assert!(Analysis::checked(&prj).report(&prj, ReportKind::FieldRouting).unwrap().rows.is_empty());
}

#[test]
fn cable_is_planned_along_the_shortest_path() {
    let (prj, n, s) = plant();
    // -X1 is mounted in +CAB1: its cable starts at the place of the enclosure.
    assert_eq!(place_of(&prj, "-X1"), Some((0, true)));
    assert_eq!(place_of(&prj, "-M1"), Some((1, false)));
    let plan = planned(&prj);
    let c = plan.cable("-W1").unwrap();
    assert_eq!(c.status, Status::Routed);
    assert_eq!(c.class, Class::Power);
    assert_eq!(c.nodes, vec![n[0], n[1], n[2]]);
    assert_eq!(c.segments, vec![s[0], s[1]]);
    assert!((c.network - 12.0).abs() < 1e-4);
    // Network + drop + one enclosure end + two reserves, cut to 0.5 m.
    let r = &prj.field.rules;
    assert!((c.length - (12.0 + 1.0 + r.cabinet + 2.0 * r.reserve)).abs() < 1e-4);
    assert_eq!(c.cut, 15.5);
    assert_eq!(c.in_enclosure, [true, false]);
    let u = plan.segment(s[0]).unwrap();
    assert_eq!(u.cables, vec!["-W1".to_string()]);
    assert_eq!(u.classes, Class::Power.bit());
    assert_eq!(plan.islands, 1);
    assert!(plan.issues.is_empty(), "{:?}", plan.issues);
}

#[test]
fn route_definition_forces_via_points_and_avoids_segments() {
    let (mut prj, n, s) = plant();
    prj.field.route_mut("-W1").via.push(n[3]);
    let c = planned(&prj).cable("-W1").unwrap().clone();
    assert_eq!(c.segments, vec![s[2], s[3], s[1]]);
    prj.field.route_mut("-W1").via.clear();
    prj.field.route_mut("-W1").avoid.push(s[0]);
    let c = planned(&prj).cable("-W1").unwrap().clone();
    assert_eq!(c.segments, vec![s[2], s[3], s[1]]);
    assert!((c.network - (10.0 + 200f32.sqrt() + 2.0)).abs() < 1e-3);
    // Without the free run there is no way to the motor.
    prj.field.route_mut("-W1").avoid.push(s[1]);
    let plan = planned(&prj);
    assert_eq!(plan.cable("-W1").unwrap().status, Status::NoPath);
    assert!(plan.issues.iter().any(|i| i.severity == Severity::Error && i.message.contains("-W1")));
}

#[test]
fn classes_fill_and_dividers() {
    let (mut prj, _, s) = plant();
    // The tray only for signals: power goes over the ladders.
    prj.field.segments.iter_mut().find(|x| x.id == s[0]).unwrap().classes = Class::Signal.bit();
    assert_eq!(planned(&prj).cable("-W1").unwrap().segments, vec![s[2], s[3], s[1]]);
    // A tray too small for the cable is avoided while there is another way …
    let t = prj.field.segments.iter_mut().find(|x| x.id == s[0]).unwrap();
    t.classes = 0;
    t.width = 10.0;
    t.height = 10.0;
    assert_eq!(planned(&prj).cable("-W1").unwrap().segments, vec![s[2], s[3], s[1]]);
    // … and overfilled when there is none.
    prj.field.remove_segment(s[2]);
    let plan = planned(&prj);
    assert_eq!(plan.cable("-W1").unwrap().segments, vec![s[0], s[1]]);
    assert!(plan.issues.iter().any(|i| i.message.contains("T1") && i.message.contains("%")));
    // A shielded cable is a signal cable: with power in the same tray it needs a divider.
    let mut w2 = prj.cables[0].clone();
    w2.tag = "-W2".into();
    w2.shielded = Some(true);
    prj.cables.push(w2);
    let wires: Vec<u64> = prj.pages.iter().flat_map(|p| p.wires.iter()).filter(|w| w.cable == "-W1").map(|w| w.id).take(1).collect();
    for w in prj.pages.iter_mut().flat_map(|p| p.wires.iter_mut()).filter(|w| wires.contains(&w.id)) {
        w.cable = "-W2".into();
    }
    let t = prj.field.segments.iter_mut().find(|x| x.id == s[0]).unwrap();
    t.width = 0.0;
    let plan = planned(&prj);
    assert_eq!(plan.segment(s[0]).unwrap().classes, Class::Power.bit() | Class::Signal.bit());
    assert!(plan.issues.iter().any(|i| i.message.contains("divider")));
    for x in prj.field.segments.iter_mut().filter(|x| x.id == s[0] || x.id == s[1]) {
        x.divider = true;
    }
    assert!(!planned(&prj).issues.iter().any(|i| i.message.contains("divider")));
}

#[test]
fn drag_chain_bend_radius() {
    let (mut prj, _, s) = plant();
    let f1 = prj.field.segments.iter_mut().find(|x| x.id == s[1]).unwrap();
    f1.kind = SegmentKind::DragChain;
    f1.bend_radius = 50.0;
    let plan = planned(&prj);
    let need = plan.cable("-W1").unwrap().diameter * prj.field.rules.bend_factor;
    assert!(need > 50.0);
    assert!(plan.issues.iter().any(|i| i.severity == Severity::Error && i.message.contains("bend radius")));
    prj.field.segments.iter_mut().find(|x| x.id == s[1]).unwrap().bend_radius = need + 1.0;
    assert!(!planned(&prj).issues.iter().any(|i| i.message.contains("bend radius")));
}

#[test]
fn unplaced_ends_and_broken_references() {
    let (mut prj, n, _) = plant();
    prj.field.places.retain(|p| p.tag != "-M1");
    let plan = planned(&prj);
    assert_eq!(plan.cable("-W1").unwrap().status, Status::NotPlaced("-M1".into()));
    assert!(plan.issues.iter().any(|i| i.message.contains("-M1")));
    // A machine location places all its devices.
    prj.field.places.push(Place { tag: "+M1".into(), node: n[2], drop: 0.0 });
    assert_eq!(place_of(&prj, "+M1-M1"), Some((1, false)));
    assert_eq!(place_of(&prj, "-M1"), None);
    prj.field.route_mut("-W9").via.push(999);
    prj.field.places.push(Place { tag: "-Q9".into(), node: 998, drop: 0.0 });
    let msgs: Vec<String> = planned(&prj).issues.into_iter().map(|i| i.message).collect();
    assert!(msgs.iter().any(|m| m.contains("-W9")));
    assert!(msgs.iter().any(|m| m.contains("998")));
}

#[test]
fn lengths_are_taken_over_and_material_counted() {
    let (mut prj, _, s) = plant();
    prj.field.segments.iter_mut().find(|x| x.id == s[0]).unwrap().part = "TRAY-100".into();
    let plan = planned(&prj);
    assert_eq!(apply_lengths(&mut prj, &plan), 1);
    assert_eq!(prj.cable("-W1").unwrap().length, 15.5);
    assert_eq!(apply_lengths(&mut prj, &plan), 0);
    let m = material(&prj);
    assert_eq!(m, vec![("TRAY-100".to_string(), 10.0, vec!["T1".to_string()])]);
    let bom = &Analysis::checked(&prj).report(&prj, ReportKind::BillOfMaterials).unwrap();
    assert!(bom.rows.iter().any(|r| r[3] == "TRAY-100" && r[1] == "10"));
}

#[test]
fn bonding_points_and_conductors() {
    let (mut prj, _, s) = plant();
    let outline = prj.new_id();
    prj.field.outlines.push(Outline { id: outline, name: "+M1".into(), points: vec![[8.0, -2.0], [12.0, -2.0], [12.0, 2.0], [8.0, 2.0]], conductive: true });
    let names: Vec<String> = bonding::names(&prj).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, vec!["MET", "+CAB1", "+M1", "T1", "L1", "L2"]);
    // Not checked before any bonding is planned.
    assert!(bonding::problems(&prj).is_empty());
    prj.field.bonds.push(Bond { id: 1, a: "MET".into(), b: "+CAB1".into(), kind: BondKind::Main, cross_section: "16".into(), length: 3.0, part: "H07V-K 16".into() });
    prj.field.bonds.push(Bond { id: 2, a: "+CAB1".into(), b: "T1".into(), kind: BondKind::Main, cross_section: "4".into(), ..Default::default() });
    let pts = bonding::points(&prj);
    // T1 meets L1 and L2 at nodes: tray continuity bonds the ladders.
    assert!(pts.iter().filter(|p| p.name != "+M1").all(|p| p.bonded));
    let msgs: Vec<String> = bonding::problems(&prj).into_iter().map(|i| i.message).collect();
    assert_eq!(msgs.len(), 2, "{msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("4 mm²") && m.contains("6 mm²")));
    assert!(msgs.iter().any(|m| m.contains("+M1")));
    prj.field.rules.continuity = false;
    assert!(!bonding::points(&prj).iter().find(|p| p.name == "L1").unwrap().bonded);
    prj.field.rules.continuity = true;
    prj.field.bonds.push(Bond { id: 3, a: "+M1".into(), b: "L9".into(), kind: BondKind::Supplementary, cross_section: "2.5".into(), ..Default::default() });
    assert!(bonding::problems(&prj).iter().any(|i| i.message.contains("L9")));
    let report = &Analysis::checked(&prj).report(&prj, ReportKind::Bonding).unwrap();
    assert_eq!(report.rows.len(), 3 + names.len());
    assert!(material(&prj).iter().any(|(p, q, _)| p == "H07V-K 16" && *q == 3.0));
    let _ = s;
}

#[test]
fn accessories_material_glands_and_shields() {
    let (mut prj, _, _) = plant();
    let d = crate::routing3d::cable_diameter(prj.cable("-W1").unwrap());
    prj.field.accessories.push(Accessory { id: 1, cable: "-W1".into(), at: String::new(), kind: AccessoryKind::StrainRelief, part: "SR-1".into(), count: 1 });
    prj.field.accessories.push(Accessory { id: 2, cable: "-W1".into(), at: "-X1".into(), kind: AccessoryKind::Gland, part: "GEN.GLAND-M32".into(), count: 1 });
    assert_eq!(accessories::clamping_range(&prj, "GEN.GLAND-M32"), Some((13.0, 21.0)));
    let m = accessories::material(&prj);
    assert!(m.contains(&("SR-1".to_string(), 2.0, vec!["-W1".to_string()])));
    assert!(m.contains(&("GEN.GLAND-M32".to_string(), 1.0, vec!["-W1".to_string()])));
    let plan = planned(&prj);
    let misfit = plan.issues.iter().any(|i| i.message.contains("clamping range"));
    assert_eq!(misfit, !(13.0..=21.0).contains(&d));
    // A shielded cable needs a shield contact at its enclosure end only.
    prj.cables[0].shielded = Some(true);
    let plan = planned(&prj);
    let shield: Vec<&Issue> = plan.issues.iter().filter(|i| i.message.contains("Shielded cable")).collect();
    assert_eq!(shield.len(), 1);
    assert!(shield[0].message.contains("-X1"));
    prj.field.accessories[1].kind = AccessoryKind::EmcGland;
    assert!(!planned(&prj).issues.iter().any(|i| i.message.contains("Shielded cable")));
    prj.field.accessories.push(Accessory { id: 3, cable: "-W7".into(), kind: AccessoryKind::Marker, ..Default::default() });
    assert!(planned(&prj).issues.iter().any(|i| i.message.contains("-W7")));
}

#[test]
fn renames_and_removal_keep_references() {
    let (mut prj, n, s) = plant();
    prj.field.route_mut("-W1").via.push(n[3]);
    prj.field.route_mut("-W1").avoid.push(s[0]);
    prj.field.accessories.push(Accessory { id: 1, cable: "-W1".into(), at: "-M1".into(), ..Default::default() });
    crate::ops::rename_device(&mut prj, "-M1", "-M2");
    assert!(prj.field.places.iter().any(|p| p.tag == "-M2"));
    assert_eq!(prj.field.accessories[0].at, "-M2");
    prj.cables[0].tag = "-W5".into();
    crate::ops::rename_cable(&mut prj, "-W1", "-W5");
    assert_eq!(prj.field.routes[0].cable, "-W5");
    assert_eq!(prj.field.accessories[0].cable, "-W5");
    assert!(prj.field.remove_node(n[3]));
    assert!(prj.field.routes[0].via.is_empty());
    assert_eq!(prj.field.segments.len(), 2);
    assert!(prj.field.remove_segment(s[0]));
    assert!(prj.field.routes[0].avoid.is_empty());
}

#[test]
fn ids_survive_reload_and_drawing_shows_the_network() {
    let (prj, _, _) = plant();
    let json = serde_json::to_string(&prj).unwrap();
    let mut back: Project = serde_json::from_str(&json).unwrap();
    assert_eq!(back.field, prj.field);
    back.next_id = 0;
    back.fix_next_id();
    assert!(back.next_id > prj.field.segments.iter().map(|s| s.id).max().unwrap());
    let plan = planned(&prj);
    let svg = draw::svg(&prj, &plan, Some("-W1"), 200.0);
    assert!(svg.starts_with("<svg") && svg.contains("+CAB1") && svg.contains("-M1") && svg.contains("T1"));
    let (min, max) = draw::bounds(&prj).unwrap();
    assert_eq!((min.x, min.y, max.x, max.y), (0.0, 0.0, 10.0, 10.0));
}

#[test]
fn field_routing_report_lists_the_cables() {
    let (prj, _, _) = plant();
    let t = &Analysis::checked(&prj).report(&prj, ReportKind::FieldRouting).unwrap();
    assert_eq!(t.rows.len(), 1);
    assert_eq!(t.rows[0][0], "-W1");
    assert!(t.rows[0][4].contains("T1") && t.rows[0][4].contains("F1"));
    assert_eq!(t.rows[0][7], "15.5");
}
