use super::*;
use crate::reports::Analysis;
use crate::sample::demo_project;

fn device(id: u64, x: f32, y: f32, w: f32, h: f32, z: f32, d: f32) -> PanelItem {
    PanelItem { id, kind: ItemKind::Device, tag: format!("-D{id}"), x, y, w, h, z, d, ..Default::default() }
}

fn duct(id: u64, x: f32, y: f32, w: f32, h: f32) -> PanelItem {
    PanelItem { id, kind: ItemKind::Duct, x, y, w, h, d: 60.0, ..Default::default() }
}

#[test]
fn device_size_from_parts() {
    let prj = demo_project();
    // Contactor 45 x 74 x 73 plus front auxiliary block (40 deep)
    let k1 = device_size(&prj, "-K1").unwrap();
    assert_eq!((k1.w, k1.h, k1.d), (45.0, 74.0, 113.0));
    assert_eq!(k1.part, "GEN.CT-9A-24DC");
    // Terminal strip: 4 terminals of 5.2 mm
    let x1 = device_size(&prj, "-X1").unwrap();
    assert!((x1.w - 20.8).abs() < 0.01, "{}", x1.w);
    // Motors have no mounting data: not placeable
    assert!(device_size(&prj, "-M1").is_none());
}

#[test]
fn devices_snap_onto_rails() {
    let prj = demo_project();
    let (_, k1) = prj.layout.find_device("-K1").unwrap();
    assert_eq!(k1.z, RAIL_PROFILE.1);
    assert!((k1.center().y - 385.0).abs() < 0.01);
    // Placed devices are no longer offered for placement.
    assert!(!devices_to_place(&prj).iter().any(|d| d.tag == "-K1"));
}

#[test]
fn collisions_and_outside() {
    let mut e = Enclosure::default();
    e.items.push(device(1, 10.0, 10.0, 40.0, 40.0, 0.0, 50.0));
    e.items.push(device(2, 30.0, 30.0, 40.0, 40.0, 0.0, 50.0)); // overlaps 1
    e.items.push(device(3, 100.0, 10.0, 40.0, 40.0, 0.0, 50.0)); // free
    // Crossing ducts are joined, not colliding.
    e.items.push(duct(4, 200.0, 0.0, 300.0, 40.0));
    e.items.push(duct(5, 300.0, 0.0, 40.0, 300.0));
    // Device beyond the plate edge
    let (pw, _) = e.plate_size();
    e.items.push(device(6, pw - 10.0, 400.0, 40.0, 40.0, 0.0, 50.0));
    assert_eq!(collisions(&e), vec![(1, 2)]);
    assert_eq!(outside(&e), vec![6]);
    // Too deep for the enclosure
    e.items.push(device(7, 100.0, 400.0, 40.0, 40.0, 0.0, 400.0));
    assert!(outside(&e).contains(&7));
}

#[test]
fn routing_follows_the_ducts() {
    let mut e = Enclosure::default();
    // Without ducts: orthogonal direct connection
    let r = route(&e, p(100.0, 100.0), p(200.0, 300.0));
    assert!(!r.via_ducts);
    assert_eq!(r.plate_length, 300.0);
    // L-shaped duct network: horizontal at y=20 (centre), vertical at x=20 (centre)
    e.items.push(duct(1, 0.0, 0.0, 400.0, 40.0));
    e.items.push(duct(2, 0.0, 40.0, 40.0, 400.0));
    let a = p(300.0, 100.0);
    let b = p(100.0, 400.0);
    let r = route(&e, a, b);
    assert!(r.via_ducts);
    // a -> (300,20): 80, along the duct to (20,20): 280, down to (20,400): 380, to b: 80
    assert!((r.plate_length - 820.0).abs() < 1.0, "{}", r.plate_length);
    assert!(r.points.first().unwrap().same(a) && r.points.last().unwrap().same(b));
    // All legs are orthogonal
    for w in r.points.windows(2) {
        assert!(w[0].x == w[1].x || w[0].y == w[1].y, "{:?}", r.points);
    }
}

#[test]
fn virtual_wiring_of_demo() {
    let prj = demo_project();
    let an = Analysis::build(&prj);
    assert!(!an.wiring.is_empty());
    for w in &an.wiring {
        assert!(w.route.via_ducts, "{} - {}", w.from, w.to);
        assert!(w.length > prj.layout.wire_reserve);
    }
    // Q1 feeds the main contacts of K1 inside the cabinet.
    assert!(an.wiring.iter().any(|w| w.from.starts_with("-Q1") && w.to.starts_with("-K1")));
    // The motor is in the field: its connections stay open.
    let report = &an.reports[&crate::model::ReportKind::PanelWiring];
    assert!(report.rows.iter().any(|r| r[2].starts_with("-M1") && r[7] == "device not placed"));
}

#[test]
fn layout_material_in_bom() {
    let prj = demo_project();
    let material = layout_material(&prj);
    let get = |n: &str| material.iter().find(|m| m.0 == n).map(|m| m.1);
    assert_eq!(get(ENCLOSURE_PART), Some(1.0));
    // Three rails of 430 mm, sold per metre
    assert_eq!(get(RAIL_PART), Some(1.29));
    // 3380 mm of duct in 2 m pieces
    assert_eq!(get(DUCT_PART), Some(2.0));
    let an = Analysis::build(&prj);
    let bom = &an.reports[&crate::model::ReportKind::BillOfMaterials];
    let rail = bom.rows.iter().find(|r| r[3] == RAIL_PART).unwrap();
    assert_eq!((rail[1].as_str(), rail[2].as_str()), ("1.29", "m"));
}

#[test]
fn rotate_keeps_centre() {
    let mut it = duct(1, 0.0, 0.0, 200.0, 40.0);
    let c = it.center();
    it.rotate();
    assert!(it.center().same(c));
    assert_eq!((it.w, it.h), (40.0, 200.0));
    assert!(!it.horizontal());
}

#[test]
fn mounting_page_renders() {
    let prj = demo_project();
    let page = prj.pages.iter().position(|pg| matches!(pg.kind, crate::model::PageKind::Mounting(_))).unwrap();
    let svg = crate::svg::page_svg(&prj, page);
    assert!(svg.contains("+CAB1"));
    assert!(svg.contains("Scale 1:"));
    assert!(svg.contains("-K1"));
}
