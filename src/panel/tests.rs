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

#[test]
fn wire_hit_test_and_cycling() {
    let prj = demo_project();
    let an = Analysis::build(&prj);
    // Keys identify wires uniquely (selection survives re-analysis).
    let mut keys: Vec<_> = an.wiring.iter().map(|w| w.key()).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), an.wiring.len());

    // A point in the middle of a route segment finds that wire.
    let (wi, w) = an.wiring.iter().enumerate().find(|(_, w)| w.from.starts_with("-Q1")).unwrap();
    let seg = &w.route.points[0..2];
    let mid = p((seg[0].x + seg[1].x) / 2.0, (seg[0].y + seg[1].y) / 2.0);
    assert!(route_distance(&w.route.points, mid) < 0.01);
    let hits = wires_at(&an.wiring, w.enclosure, mid, 2.0);
    assert!(hits.contains(&wi));
    // Nearest first, other enclosures and far points excluded.
    assert!(wires_at(&an.wiring, w.enclosure + 1, mid, 2.0).is_empty());
    assert!(wires_at(&an.wiring, w.enclosure, p(-500.0, -500.0), 2.0).is_empty());

    // Repeated clicks step through overlapping wires and wrap around.
    assert_eq!(next_candidate(&[4, 7, 9], None), Some(4));
    assert_eq!(next_candidate(&[4, 7, 9], Some(4)), Some(7));
    assert_eq!(next_candidate(&[4, 7, 9], Some(9)), Some(4));
    assert_eq!(next_candidate(&[4, 7, 9], Some(1)), Some(4));
    assert_eq!(next_candidate(&[], Some(1)), None);
}

fn within(tris: &[[[f32; 3]; 3]], it: &PanelItem) -> bool {
    tris.iter().flatten().all(|v| {
        v[0] >= it.x - 0.01
            && v[0] <= it.x + it.w + 0.01
            && v[1] >= it.y - 0.01
            && v[1] <= it.y + it.h + 0.01
            && v[2] >= it.z - 0.01
            && v[2] <= it.z + it.d + 0.01
    })
}

fn extent(tris: &[[[f32; 3]; 3]]) -> [f32; 3] {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for v in tris.iter().flatten() {
        for k in 0..3 {
            min[k] = min[k].min(v[k]);
            max[k] = max[k].max(v[k]);
        }
    }
    [max[0] - min[0], max[1] - min[1], max[2] - min[2]]
}

#[test]
fn part_models_are_fitted_into_devices() {
    let prj = demo_project();
    let (ei, k1) = prj.layout.find_device("-K1").unwrap();
    let part = prj.parts.iter().find(|p| p.number == k1.part).unwrap();
    let mesh = part.model_3d.as_ref().expect("catalog contactor has a 3D model");
    // Contactor + aux block: the model (73 deep) is stretched to the device depth (113).
    let tris = place_mesh(mesh, k1);
    assert_eq!(tris.len(), mesh.triangles.len());
    assert!(within(&tris, k1));
    let e = extent(&tris);
    assert!((e[0] - k1.w).abs() < 0.01 && (e[1] - k1.h).abs() < 0.01 && (e[2] - k1.d).abs() < 0.01, "{e:?}");

    // Rotated device: the model is turned, not squeezed.
    let mut rotated = k1.clone();
    rotated.rotate();
    let tris = place_mesh(mesh, &rotated);
    assert!(within(&tris, &rotated));
    assert!((extent(&tris)[0] - rotated.w).abs() < 0.01);

    // Terminal strip: one terminal model per terminal.
    let (_, x1) = prj.layout.find_device("-X1").unwrap();
    let tb = prj.parts.iter().find(|p| p.number == x1.part).unwrap().model_3d.as_ref().unwrap();
    assert_eq!(tiles(x1.w, tb.size()[0]), 4);
    let tris = place_mesh(tb, x1);
    assert_eq!(tris.len(), tb.triangles.len() * 4);
    assert!(within(&tris, x1));
    assert_eq!(ei, 0);
}

#[test]
fn model_size_is_used_without_mounting_data() {
    let mut prj = demo_project();
    // Remove the mounting data of the lamp part but give it a 3D model.
    let part = prj.parts.iter_mut().find(|p| p.number == "GEN.LMP-GN-24").unwrap();
    part.width = 0.0;
    part.height = 0.0;
    part.depth = 0.0;
    let mut mesh = crate::parts::Mesh3d::default();
    mesh.add_box([0.0, 0.0, 0.0], [22.0, 22.0, 50.0]);
    part.model_3d = Some(mesh);
    assert_eq!(part_dims(part), Some([22.0, 22.0, 50.0]));
    let size = device_size(&prj, "-P1").unwrap();
    assert_eq!((size.w, size.h, size.d), (22.0, 22.0, 50.0));
    assert_eq!(tiles(20.8, 5.2), 4);
    assert_eq!(tiles(45.0, 45.0), 1);
    assert_eq!(tiles(30.0, 7.0), 1);
}

#[test]
fn mounting_page_draws_part_models() {
    let mut prj = demo_project();
    let page = prj.pages.iter().position(|pg| matches!(pg.kind, crate::model::PageKind::Mounting(_))).unwrap();
    let with = crate::svg::page_svg(&prj, page);
    for p in &mut prj.parts {
        p.model_2d = None;
    }
    let without = crate::svg::page_svg(&prj, page);
    // Terminal screws of the generated front views are circles.
    assert!(with.matches("<circle").count() > without.matches("<circle").count() + 10);
}
