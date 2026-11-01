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
    Request { kind: RouteKind::Cable, class: Class::Signal, ignore, at: [[0.0; 3]; 2], leave: [[0.0; 3]; 2], out: [0.0; 2], area: 50.0, turn: 12.5, half: 3.0 }
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
fn close_bends_leave_out_the_ends_and_lane_changes() {
    let lane_change = [[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [100.0, 10.0, 0.0], [300.0, 10.0, 0.0], [300.0, 200.0, 0.0]];
    assert_eq!(close_bends(&lane_change, 50.0), 0);
    let staircase = [[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [100.0, 10.0, 0.0], [100.0, 10.0, 200.0], [300.0, 10.0, 200.0]];
    assert_eq!(close_bends(&staircase, 50.0), 1);
    assert_eq!(close_bends(&staircase, 5.0), 0);
    assert_eq!(close_bends(&[[0.0; 3], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0]], 50.0), 0, "the stretches at the ends do not count");
    // Out of a nozzle (50), a short way onto the grid, 50 on, then across and on: the
    // ways onto the grid are no bends, the stretch between two corners of it is.
    let nozzle = [[0.0, 100.0, 0.0], [0.0, 50.0, 0.0], [15.0, 45.0, 0.0], [15.0, -5.0, 0.0], [60.0, -5.0, 0.0], [60.0, -5.0, 300.0], [40.0, 20.0, 310.0]];
    assert_eq!(close_bends(&nozzle, 40.0), 0);
    assert_eq!(close_bends(&nozzle, 50.0), 1);
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
fn stretches_between_bends_are_at_least_half_the_bend_radius() {
    let prj = project(with_block());
    let s = space(&prj);
    let rules = RoutingRules::default();
    // Round the device and up to a point just beside its far corner: a thick cable with a
    // large bend radius bends only where it can run on for half of it.
    let (a, b) = ([60.0, 375.0, 60.0], [550.0, 375.0, 60.0]);
    for turn in [12.5, 150.0, 300.0] {
        let r = Request { at: [a, b], leave: [[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]], turn, half: 5.0, ..req(&[]) };
        let w = s.route(a, b, &r, &rules).expect("a way");
        let (n, shortest) = bends(&w.points[1..w.points.len() - 1]);
        assert!(n >= 2, "{:?}", w.points);
        assert!(shortest >= turn * 0.5 - 1e-3, "turn {turn}: stretch of {shortest} between bends: {:?}", w.points);
    }
}

#[test]
fn routes_leave_their_ends_straight_on() {
    let mut e = with_block();
    e.items[0].d = 80.0;
    let prj = project(e);
    let s = space(&prj);
    let rules = RoutingRules::default();
    // From the front of the device straight out (+z) for half the bend radius before the
    // first bend, and into the end point from above (it leaves it upwards, -y).
    let (a, b) = ([250.0, 375.0, 105.0], [550.0, 600.0, 100.0]);
    let r = Request { at: [a, b], leave: [[0.0, 0.0, 1.0], [0.0, -1.0, 0.0]], turn: 150.0, half: 5.0, ..req(&[7]) };
    let w = s.route(a, b, &r, &rules).expect("a way");
    let p = &w.points;
    // The short way to the first cell, then the straight run.
    let (jog, first) = (sub(p[1], p[0]), sub(p[2], p[1]));
    assert!(jog[2] >= 0.0 && jog.iter().all(|v| v.abs() <= 12.5), "{p:?}");
    assert_eq!((first[0], first[1]), (0.0, 0.0), "{p:?}");
    assert!(first[2] >= 75.0 - 1e-3, "{p:?}");
    let n = p.len();
    let (last, jog) = (sub(p[n - 2], p[n - 3]), sub(p[n - 1], p[n - 2]));
    assert!(jog[1] >= 0.0 && jog.iter().all(|v| v.abs() <= 12.5), "{p:?}");
    assert_eq!((last[0], last[2]), (0.0, 0.0), "{p:?}");
    assert!(last[1] >= 75.0 - 1e-3, "{p:?}");
    // Without directions the first stretch is the short way to the nearest cell.
    let any = s.route(a, b, &Request { leave: [[0.0; 3]; 2], ..r }, &rules).expect("a way");
    assert!(length(&any.points) <= length(&w.points) + 1e-3);
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
    // Along under the device, round its lower right corner up its right side (the bend
    // swings into the corner), and away from it into the open.
    let at = |q: V3| s.centre(s.cell_of(q).unwrap());
    let corner = at([hi[0] + 12.0, hi[1] + 12.0, 60.0]);
    let open = at([corner[0], 150.0, 60.0]);
    let radii = s.corner_radii(&[[200.0, corner[1], 60.0], corner, open, [560.0, 150.0, 60.0]], 5.0, 100.0, &[]);
    assert_eq!((radii[0], radii[3], radii[2]), (0.0, 0.0, 100.0));
    assert!(radii[1] > 0.0 && radii[1] <= 25.0, "{radii:?}");
    // The middle of what is left of the bend keeps clear of the device: (√2 − 1) r
    // inside the corner.
    let middle = add(corner, scale(norm([-1.0, -1.0, 0.0]), CORNER_CUT * radii[1]));
    assert!(from_box(middle, lo, hi) >= 5.0 - 0.01, "{radii:?}");
    // The same corner turning away from the device has room for the whole bend.
    let away = s.corner_radii(&[[corner[0], 600.0, 60.0], corner, [560.0, corner[1], 60.0]], 5.0, 100.0, &[]);
    assert_eq!(away[1], 100.0, "{away:?}");
    // A space never measured has room everywhere.
    let mut unmeasured = Space::empty([0.0; 3], [100.0; 3], 25.0);
    unmeasured.block.iter_mut().for_each(|b| *b = 0);
    assert_eq!(unmeasured.corner_radii(&[[0.0; 3], [50.0; 3], [100.0; 3]], 5.0, 80.0, &[])[1], 80.0);
}

#[test]
fn thin_obstacles_between_the_cells_block_the_way() {
    // A busbar 10 mm thick lying between two rows of cell centres (its clearance covers
    // none of them): no route crosses it, a thick one keeps half its thickness from it.
    let mut e = single();
    e.items.push(PanelItem { id: 7, kind: ItemKind::Busbar, tag: "-B1".into(), x: 50.0, y: 118.0, w: 500.0, h: 10.0, d: 240.0, ..Default::default() });
    let prj = project(e);
    let s = space(&prj);
    let (lo, hi) = crate::panel::item_box(&prj.layout.enclosures[0], &prj.layout.enclosures[0].items[0]);
    let around = (lo.map(|v| v - 5.0), hi.map(|v| v + 5.0));
    assert!(s.cells_in(around).is_empty() && !s.cells_in(s.widened(around)).is_empty(), "the bar with its clearance covers no cell centre, widened it does");
    let (a, b) = ([300.0, 60.0, 260.0], [300.0, 740.0, 260.0]);
    for half in [2.0_f32, 6.0, 15.0] {
        let w = s.route(a, b, &Request { at: [a, b], half, ..req(&[]) }, &RoutingRules::default()).expect("a way round");
        let nearest = w.points.windows(2).map(|p| segment_from_box(p[0], p[1], lo, hi)).fold(f32::INFINITY, f32::min);
        assert!(nearest >= half - 0.01, "half {half}: {nearest} from the bar: {:?}", w.points);
    }
}

/// Distance of a segment from a box (sampled), 0 where it enters it.
fn segment_from_box(a: V3, b: V3, lo: V3, hi: V3) -> f32 {
    let n = (dist(a, b) / 2.0).ceil().max(1.0) as usize;
    (0..=n).map(|k| from_box(add(a, scale(sub(b, a), k as f32 / n as f32)), lo, hi)).fold(f32::INFINITY, f32::min)
}

#[test]
fn routes_keep_half_their_thickness_from_the_walls_and_the_plate() {
    // Along the bottom of an empty enclosure (600 wide, 800 high, 250 deep): a thick
    // cable stays its half thickness off the bottom, the sides, the plate and the door.
    let prj = project(single());
    let e = &prj.layout.enclosures[0];
    let s = space(&prj);
    let (a, b) = ([30.0, e.height - 30.0, 40.0], [e.width - 30.0, e.height - 30.0, 40.0]);
    for half in [4.0_f32, 15.0] {
        let w = s.route(a, b, &Request { at: [a, b], half, ..req(&[]) }, &RoutingRules::default()).expect("a way");
        assert!(!w.squeezed);
        for q in &w.points[1..w.points.len() - 1] {
            assert!(q[1] <= e.height - e.wall() - half + 0.01 && q[1] >= e.wall() + half - 0.01, "half {half}: {q:?}");
            assert!(q[0] >= e.wall() + half - 0.01 && q[0] <= e.width - e.wall() - half + 0.01, "half {half}: {q:?}");
            assert!(q[2] >= PLATE_Z + half - 0.01 && q[2] <= e.depth - e.wall() - half + 0.01, "half {half}: {q:?}");
        }
    }
}

#[test]
fn a_route_without_room_is_squeezed_through_and_says_so() {
    // Two shelves 50 mm apart across the whole width: a 44 mm cable (half 22) cannot
    // keep 22 from both, but gets through between them, squeezed.
    let mut e = single();
    for (id, y) in [(7, 300.0), (8, 350.0)] {
        e.items.push(PanelItem { id, kind: ItemKind::Device, tag: format!("-K{id}"), x: 0.0, y, w: 550.0, h: 1.0, d: 200.0, ..Default::default() });
    }
    let prj = project(e);
    let s = space(&prj);
    let (a, b) = ([60.0, 100.0, 100.0], [60.0, 700.0, 100.0]);
    let w = s.route(a, b, &Request { at: [a, b], half: 22.0, ..req(&[]) }, &RoutingRules::default()).expect("squeezed through");
    assert!(w.squeezed, "{:?}", w.points);
    let thin = s.route(a, b, &Request { at: [a, b], half: 10.0, ..req(&[]) }, &RoutingRules::default()).unwrap();
    assert!(!thin.squeezed);
}

#[test]
fn cables_keep_apart_in_free_air() {
    let prj = project(single());
    let mut s = space(&prj);
    let rules = RoutingRules::default();
    let (a, b) = ([300.0, 60.0, 250.0], [300.0, 740.0, 250.0]);
    let first = s.route(a, b, &req(&[]), &rules).unwrap();
    s.take(&first.points, 3.0, 30.0);
    // The same way again: beside the first, not inside it, but for the ends.
    let second = s.route(a, b, &req(&[]), &rules).unwrap();
    assert!(!s.still_apart(&first.cells, &req(&[])));
    assert!(s.still_apart(&second.cells, &req(&[])));
    let far = second.points.iter().filter(|q| q[1] > 150.0 && q[1] < 650.0).all(|q| first.points.iter().all(|p| (p[0] - q[0]).hypot(p[2] - q[2]) >= 6.0 - 1e-3 || (p[1] - q[1]).abs() > 1.0));
    assert!(far, "{:?} / {:?}", first.points, second.points);
    // Wires are not kept apart.
    assert!(s.still_apart(&first.cells, &Request { kind: RouteKind::Wire, ..req(&[]) }));
}

#[test]
fn ducts_are_entered_through_their_walls_and_kept_off_outside() {
    // A horizontal duct 100 wide and 80 deep across the plate (y 400 .. 500).
    let mut e = single();
    e.items.push(PanelItem { id: 3, kind: ItemKind::Duct, x: 100.0, y: 400.0, w: 400.0, h: 100.0, d: 80.0, ..Default::default() });
    let prj = project(e);
    let s = space(&prj);
    let rules = RoutingRules::default();
    let (lo, hi) = (add([100.0, 400.0, 0.0], [crate::panel::PLATE_MARGIN, crate::panel::PLATE_MARGIN, PLATE_Z]), add([500.0, 500.0, 80.0], [crate::panel::PLATE_MARGIN, crate::panel::PLATE_MARGIN, PLATE_Z]));
    let in_duct = |c: usize| s.duct_of[c] != 0;
    // A cable 30 mm thick across the duct: through its walls at the height of their
    // slots (above the rim of 20, below the top at 77.5), never through its cover.
    let (a, b) = ([325.0, 325.0, lo[2] + 40.0], [325.0, 625.0, lo[2] + 40.0]);
    let cable = Request { at: [a, b], half: 15.0, turn: 50.0, ..req(&[]) };
    let w = s.route(a, b, &cable, &rules).expect("across the duct");
    let crossings: Vec<&[usize]> = w.cells.windows(2).filter(|c| in_duct(c[0]) != in_duct(c[1])).collect();
    assert_eq!(crossings.len(), 2, "{:?}", w.points);
    for c in crossings {
        let (p, q) = (s.centre(c[0]), s.centre(c[1]));
        assert!(p[2] == q[2] && p[0] == q[0], "through the wall: {p:?} {q:?}");
        let z = p[2] - lo[2];
        assert!(z - 15.0 >= 20.0 - 12.5 && z + 15.0 <= 77.5 + 12.5, "at the slots: {z}");
    }
    // From in front of the duct to behind it: not down through the cover.
    let (a, b) = ([325.0, 475.0, hi[2] + 60.0], [325.0, 625.0, lo[2] + 40.0]);
    let w = s.route(a, b, &Request { at: [a, b], ..cable }, &rules).expect("round the duct");
    assert!(w.cells.windows(2).all(|c| in_duct(c[0]) == in_duct(c[1]) || s.centre(c[0])[2] == s.centre(c[1])[2]), "{:?}", w.points);
    // A cable too thick for the duct running along it keeps half its thickness off it
    // (but near its ends).
    let (a, b) = ([125.0, 545.0, lo[2] + 40.0], [525.0, 545.0, lo[2] + 40.0]);
    let thick = Request { at: [a, b], half: 40.0, ..cable };
    let w = s.route(a, b, &thick, &rules).expect("along the duct");
    for &c in w.cells.iter().filter(|&&c| s.centre(c)[0] > 200.0 && s.centre(c)[0] < 450.0) {
        assert!(from_box(s.centre(c), lo, hi) >= 40.0 - 0.5, "{:?}", w.points);
    }
}
