use super::*;
use crate::panel::{new_enclosure, Enclosure, PanelItem};
use crate::routing3d::{Reservation, RoutingPath};

fn project(e: Enclosure) -> Project {
    let mut prj = Project::default();
    prj.layout.enclosures.push(e);
    prj
}

fn single() -> Enclosure {
    let mut prj = Project::default();
    new_enclosure(&mut prj, "", "+A")
}

fn req(ignore: &[u64]) -> Request<'_> {
    Request { kind: RouteKind::Cable, class: Class::Signal, ignore, at: [[0.0; 3]; 2], area: 50.0, turn: 12.5, half: 3.0 }
}

fn space(prj: &Project) -> Space {
    let pos = crate::panel::positions(&prj.layout);
    let lu: Vec<usize> = (0..prj.layout.enclosures.len()).collect();
    Space::build(prj, &lu, &pos, &RoutingRules::default())
}

/// A device on the plate from x 125 to 425, y 325 to 425 (enclosure coordinates),
/// 200 mm deep.
fn with_block() -> Enclosure {
    let mut e = single();
    e.items.push(PanelItem { id: 7, kind: ItemKind::Device, tag: "-K1".into(), x: 100.0, y: 300.0, w: 300.0, h: 100.0, d: 200.0, ..Default::default() });
    e
}

fn inside(q: V3, lo: V3, hi: V3) -> bool {
    (0..3).all(|i| q[i] > lo[i] && q[i] < hi[i])
}

#[test]
fn routes_go_around_devices() {
    let prj = project(with_block());
    let s = space(&prj);
    let (a, b) = ([60.0, 375.0, 60.0], [550.0, 375.0, 60.0]);
    let w = s.route(a, b, &req(&[]), &RoutingRules::default()).expect("a way around");
    assert_eq!((w.points[0], *w.points.last().unwrap()), (a, b));
    for &c in &w.cells {
        assert!(!inside(s.centre(c), [120.0, 320.0, 15.0], [430.0, 430.0, 225.0]), "{:?}", s.centre(c));
    }
    assert!(length(&w.points) > 490.0 + 50.0);
    // Orthogonal between the cells.
    for seg in w.points[1..w.points.len() - 1].windows(2) {
        assert_eq!((0..3).filter(|&i| (seg[0][i] - seg[1][i]).abs() > 1e-3).count(), 1, "{:?}", w.points);
    }
    // Its own device is no obstacle for a route ending at it, but only where the route
    // leaves it: past the end the route goes around it as around any other device.
    let end = [250.0, 375.0, 100.0];
    assert!(s.route(a, end, &Request { at: [a, end], ..req(&[7]) }, &RoutingRules::default()).is_some());
    let w = s.route(a, b, &Request { at: [a, b], ..req(&[7]) }, &RoutingRules::default()).expect("a way around");
    assert!(w.cells.iter().all(|&c| !inside(s.centre(c), [120.0, 320.0, 15.0], [430.0, 430.0, 225.0])), "{:?}", w.points);
}

#[test]
fn ducts_and_paths_are_preferred_and_reservations_kept() {
    let mut e = single();
    // A vertical duct on the left of the plate.
    e.items.push(PanelItem { id: 3, kind: ItemKind::Duct, x: 0.0, y: 0.0, w: 40.0, h: 750.0, d: 60.0, ..Default::default() });
    let prj = project(e.clone());
    let rules = RoutingRules::default();
    let (a, b) = ([60.0, 60.0, 50.0], [60.0, 740.0, 50.0]);
    let s = space(&prj);
    let w = s.route(a, b, &req(&[]), &rules).unwrap();
    assert!(w.free < 100.0, "runs in the duct: {}", w.free);
    // Without the duct it is free air all the way.
    let free = space(&project(single())).route(a, b, &req(&[]), &rules).unwrap();
    assert!(free.free > 600.0);
    // A routing path for tubes only is no channel for cables.
    let mut e2 = single();
    e2.paths.push(RoutingPath { id: 1, kind: PathKind::Tubes, points: vec![[300.0, 60.0, 150.0], [300.0, 740.0, 150.0]], width: 50.0, ..Default::default() });
    let s2 = space(&project(e2));
    let (a2, b2) = ([300.0, 60.0, 150.0], [300.0, 740.0, 150.0]);
    assert!(s2.route(a2, b2, &req(&[]), &rules).unwrap().free > 600.0);
    let tube = Request { kind: RouteKind::Tube, class: Class::Fluid, ..req(&[]) };
    assert!(s2.route(a2, b2, &tube, &rules).unwrap().free < 100.0);
    // A keep-out across the enclosure leaves no way; a reservation for power keeps
    // signals out.
    let mut e3 = single();
    e3.reservations.push(Reservation { id: 1, min: [0.0, 380.0, 0.0], max: [600.0, 420.0, 250.0], ..Default::default() });
    assert!(space(&project(e3.clone())).route(a2, b2, &req(&[]), &rules).is_none());
    e3.reservations[0].mode = ReserveMode::For(Class::Power);
    let s3 = space(&project(e3));
    assert!(s3.route(a2, b2, &req(&[]), &rules).is_none());
    let power = Request { class: Class::Power, ..req(&[]) };
    assert!(s3.route(a2, b2, &power, &rules).is_some());
}

#[test]
fn walls_between_bayed_enclosures_only_at_passages() {
    let mut prj = Project::default();
    for name in ["+A", "+B"] {
        let e = new_enclosure(&mut prj, "", name);
        prj.layout.enclosures.push(e);
    }
    crate::panel::bay_right_of(&mut prj, 1, 0);
    let rules = RoutingRules::default();
    let (a, b) = ([300.0, 400.0, 100.0], [900.0, 400.0, 100.0]);
    assert!(space(&prj).route(a, b, &req(&[]), &rules).is_none(), "closed walls");
    crate::panel::add_passage(&mut prj, 0, 200.0, 80.0, 60.0);
    let w = space(&prj).route(a, b, &req(&[]), &rules).expect("through the passage");
    // Where it crosses x = 600 it is at the passage.
    let cross = w.points.windows(2).find(|s| (s[0][0] - 600.0) * (s[1][0] - 600.0) <= 0.0 && s[0][0] != s[1][0]).unwrap();
    assert!((cross[0][1] - 200.0).abs() <= 45.0 && (cross[0][2] - 80.0).abs() <= 45.0, "{cross:?}");
}

#[test]
fn later_classes_keep_their_distance() {
    let prj = project(single());
    let rules = RoutingRules::default();
    let mut s = space(&prj);
    // Power along the middle.
    let power = s.cells_along(&[[300.0, 60.0, 100.0], [300.0, 740.0, 100.0]]);
    s.occupy(&power, Class::Power, 10.0);
    let (a, b) = ([280.0, 60.0, 100.0], [280.0, 740.0, 100.0]);
    let near = s.route(a, b, &req(&[]), &rules).unwrap();
    s.update_zones(rules.segregation);
    let apart = s.route(a, b, &req(&[]), &rules).unwrap();
    let dist_to_power = |w: &Way| w.cells.iter().map(|&c| (s.centre(c)[0] - 300.0).abs().max((s.centre(c)[2] - 100.0).abs())).sum::<f32>() / w.cells.len() as f32;
    assert!(dist_to_power(&apart) > dist_to_power(&near) + 30.0);
    // Control may run close to power.
    let control = Request { class: Class::Control, ..req(&[]) };
    assert!(length(&s.route(a, b, &control, &rules).unwrap().points) <= length(&near.points) + 1.0);
    assert_eq!(s.classes_at(&power), Class::Power.bit());
}

#[test]
fn path_capacity_and_frozen_ways() {
    let mut e = single();
    e.paths.push(RoutingPath { id: 4, points: vec![[300.0, 60.0, 150.0], [300.0, 740.0, 150.0]], width: 20.0, reserve: 50.0, ..Default::default() });
    let prj = project(e);
    let rules = RoutingRules::default();
    let mut s = space(&prj);
    let (a, b) = ([300.0, 60.0, 150.0], [300.0, 740.0, 150.0]);
    let w = s.route(a, b, &req(&[]), &rules).unwrap();
    assert!(w.free < 100.0);
    // 157 mm² usable: three items of 60 mm² overfill it.
    for _ in 0..3 {
        s.occupy(&w.cells, Class::Signal, 60.0);
    }
    assert_eq!((s.paths[0].items, s.paths[0].used), (3, 180.0));
    assert!(s.paths[0].used > s.paths[0].capacity);
    assert!(s.route(a, b, &req(&[]), &rules).unwrap().free > 600.0, "full: the next goes beside it");
    assert!(s.still_free(&w.points, &req(&[])));
    let behind_plate = [[300.0, 60.0, 150.0], [300.0, 60.0, 5.0], [300.0, 740.0, 5.0], [300.0, 740.0, 150.0]];
    assert!(!s.still_free(&behind_plate, &req(&[])), "behind the mounting plate");
}

#[test]
fn simplify_and_bends() {
    let pts = simplify(vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [20.0, 0.0, 0.0], [20.0, 0.0, 0.0], [20.0, 30.0, 0.0], [20.0, 30.0, 5.0]]);
    assert_eq!(pts, vec![[0.0, 0.0, 0.0], [20.0, 0.0, 0.0], [20.0, 30.0, 0.0], [20.0, 30.0, 5.0]]);
    assert_eq!(bends(&pts), (2, 30.0));
    assert_eq!(length(&pts), 55.0);
}

#[test]
fn a_short_stretch_between_bends_moves_to_an_end() {
    let prj = project(single());
    let s = space(&prj);
    let at = |q: V3| s.centre(s.cell_of(q).unwrap());
    let c1 = at([100.0, 375.0, 60.0]);
    let c2 = at([250.0, 375.0, 60.0]);
    let c3 = at([250.0, 400.0, 60.0]);
    let c4 = at([400.0, 400.0, 60.0]);
    let (a, b) = ([c1[0] - 10.0, c1[1] - 10.0, c1[2]], [c4[0] + 5.0, c4[1] + 5.0, c4[2]]);
    let mut points = vec![a, c1, c2, c3, c4, b];
    let wide = Request { turn: 100.0, ..req(&[]) };
    assert!(s.spread_bends(&mut points, &wide, &RoutingRules::default()));
    // The short stretch first, then straight on to the end.
    assert_eq!(points, vec![a, c1, [c1[0], c4[1], c1[2]], c4, b]);
    let mut kept = vec![a, c1, c2, c3, c4, b];
    assert!(!s.spread_bends(&mut kept, &req(&[]), &RoutingRules::default()), "long enough for a small bend radius");
}

/// Distance of a point from a box (0 inside).
fn from_box(q: V3, lo: V3, hi: V3) -> f32 {
    let d = [0, 1, 2].map(|i| (lo[i] - q[i]).max(q[i] - hi[i]).max(0.0));
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

#[test]
fn thick_cables_keep_half_their_thickness_from_devices() {
    let prj = project(with_block());
    let s = space(&prj);
    let (lo, hi) = crate::panel::item_box(&prj.layout.enclosures[0], &prj.layout.enclosures[0].items[0]);
    let (a, b) = ([60.0, 375.0, 60.0], [550.0, 375.0, 60.0]);
    let nearest = |half: f32| {
        let r = Request { at: [a, b], half, ..req(&[]) };
        let w = s.route(a, b, &r, &RoutingRules::default()).expect("a way around");
        w.cells.iter().map(|&c| s.centre(c)).filter(|q| dist(*q, a) > 120.0 && dist(*q, b) > 120.0).map(|q| from_box(q, lo, hi)).fold(f32::INFINITY, f32::min)
    };
    let (thin, thick) = (nearest(3.0), nearest(40.0));
    assert!(thick >= 40.0 && thick > thin, "thin {thin}, thick {thick}");
}

#[test]
fn cables_keep_out_of_ducts_too_small_for_them() {
    let mut e = single();
    e.items.push(PanelItem { id: 3, kind: ItemKind::Duct, x: 0.0, y: 0.0, w: 40.0, h: 750.0, d: 60.0, ..Default::default() });
    let prj = project(e);
    let s = space(&prj);
    let rules = RoutingRules::default();
    let (a, b) = ([60.0, 60.0, 50.0], [60.0, 740.0, 50.0]);
    // A thin cable runs in the duct, one thicker than its inside not.
    assert!(s.route(a, b, &Request { at: [a, b], ..req(&[]) }, &rules).unwrap().free < 100.0);
    let thick = s.route(a, b, &Request { at: [a, b], half: 25.0, ..req(&[]) }, &rules).unwrap();
    assert!(thick.free > 500.0, "{}", thick.free);
    let duct = |q: V3| q[0] < 25.0 + 40.0 && q[1] > 25.0 + 100.0 && q[1] < 25.0 + 650.0;
    assert!(!thick.cells.iter().any(|&c| duct(s.centre(c))), "{:?}", thick.points);
}

#[test]
fn bends_are_rounded_as_far_as_the_room_around_them_allows() {
    let prj = project(with_block());
    let s = space(&prj);
    let (lo, hi) = crate::panel::item_box(&prj.layout.enclosures[0], &prj.layout.enclosures[0].items[0]);
    // A corner just beside the device and one in the open.
    let at = |q: V3| s.centre(s.cell_of(q).unwrap());
    let close = at([hi[0] + 20.0, 375.0, 60.0]);
    let open = at([550.0, 150.0, 120.0]);
    let radii = s.corner_radii(&[[0.0; 3], close, open, [600.0, 150.0, 120.0]], 5.0, 100.0);
    assert_eq!((radii[0], radii[3], radii[2]), (0.0, 0.0, 100.0));
    assert!(radii[1] < 100.0, "{radii:?}");
    // What is left of the bend keeps clear of the device: (√2 − 1) r inside the corner.
    assert!(from_box(close, lo, hi) - CORNER_CUT * radii[1] >= 5.0 - 0.01, "{radii:?}");
    // A space never measured has room everywhere.
    let mut unmeasured = Space::empty([0.0; 3], [100.0; 3], 25.0);
    unmeasured.block.iter_mut().for_each(|b| *b = 0);
    assert_eq!(unmeasured.corner_radii(&[[0.0; 3], [50.0; 3], [100.0; 3]], 5.0, 80.0)[1], 80.0);
}

#[test]
fn cables_keep_apart_in_free_air() {
    let prj = project(single());
    let mut s = space(&prj);
    let rules = RoutingRules::default();
    let (a, b) = ([300.0, 60.0, 250.0], [300.0, 740.0, 250.0]);
    let first = s.route(a, b, &req(&[]), &rules).unwrap();
    s.take(&first.points, 3.0);
    // The same way again: beside the first, not inside it, but for the ends.
    let second = s.route(a, b, &req(&[]), &rules).unwrap();
    assert!(!s.still_apart(&first.cells, &req(&[])));
    assert!(s.still_apart(&second.cells, &req(&[])));
    let far = second.points.iter().filter(|q| q[1] > 150.0 && q[1] < 650.0).all(|q| first.points.iter().all(|p| (p[0] - q[0]).hypot(p[2] - q[2]) >= 6.0 - 1e-3 || (p[1] - q[1]).abs() > 1.0));
    assert!(far, "{:?} / {:?}", first.points, second.points);
    // Wires are not kept apart.
    assert!(s.still_apart(&first.cells, &Request { kind: RouteKind::Wire, ..req(&[]) }));
}
