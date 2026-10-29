use super::*;
use crate::analysis::Analysis;
use crate::geom::p;
use crate::model::routing3d::RouteKind;
use crate::panel::{SitePlace, ENCLOSURE_PART};
use crate::parts::FnKind;
use crate::routing3d::{freeze, FrozenRoute, RoutedItem, Status};
use crate::testkit::Kit;

/// Room +ROOM1 (4 x 3 m, 2.8 m high) with the cabinet +CAB1 (600 x 800 x 250) against
/// its rear wall at x 500. Ball valve -QM1 in the cabinet, -QM2 and -QM3 on the right
/// wall; hoses -QM1:2 – -QM2:1 and -QM2:2 – -QM3:1. Returns the project, the room and
/// the cabinet.
fn pump_room() -> (Project, usize, usize) {
    let mut k = Kit::new();
    let q1 = k.sym("fl_shutoff", 100.0, 40.0, "-QM1", "");
    let q2 = k.sym("fl_shutoff", 100.0, 80.0, "-QM2", "");
    let q3 = k.sym("fl_shutoff", 100.0, 120.0, "-QM3", "");
    k.wire((q1, "2"), (q2, "1"));
    k.wire((q2, "2"), (q3, "1"));
    k.part("-QM1", FnKind::FluidComponent, "1 2", |p| (p.width, p.height, p.depth) = (40.0, 60.0, 50.0));
    for tag in ["-QM2", "-QM3"] {
        k.part(tag, FnKind::FluidComponent, "1 2", |p| {
            (p.width, p.height, p.depth) = (160.0, 120.0, 140.0);
            p.mounting_data.wall_mount = true;
        });
    }
    let mut prj = k.prj;
    let room = panel::new_room(&mut prj, "+ROOM1", 4000.0, 3000.0, 2800.0);
    prj.layout.enclosures.push(room);
    let cab = panel::new_enclosure(&mut prj, ENCLOSURE_PART, "+CAB1");
    prj.layout.enclosures.push(cab);
    let (r, c) = (0, 1);
    panel::set_room(&mut prj, c, Some(r), Some(SitePlace { x: 500.0, z: 0.0, angle: 0.0 })).unwrap();
    panel::place_device_on(&mut prj, c, "-QM1", p(250.0, 300.0), Face::Plate).unwrap();
    panel::mount_on_wall(&mut prj, r, "-QM2", Face::Wall(1), p(1000.0, 1200.0)).unwrap();
    panel::mount_on_wall(&mut prj, r, "-QM3", Face::Wall(1), p(2400.0, 1200.0)).unwrap();
    (prj, r, c)
}

fn with_roof_gland(prj: &mut Project, c: usize) -> u64 {
    panel::place_accessory(prj, c, "GEN.GLAND-M25", Face::Roof, p(282.0, 107.0)).unwrap()
}

fn tube<'a>(an: &'a Analysis, tag: &str) -> &'a RoutedItem {
    an.routing.items.iter().find(|i| i.kind == RouteKind::Tube && i.key.contains(tag)).unwrap_or_else(|| panic!("hose at {tag}"))
}

#[test]
fn the_room_space_is_free_inside_the_walls_only() {
    let (mut prj, r, c) = pump_room();
    panel::set_outline(&mut prj.layout.enclosures[r], &[[0.0, 0.0], [4000.0, 0.0], [4000.0, 3000.0], [2000.0, 3000.0], [2000.0, 1500.0], [0.0, 1500.0]]).unwrap();
    panel::add_inner_wall(&mut prj, r, [3000.0, 0.0], [3000.0, 1000.0], 100.0).unwrap();
    panel::add_opening(&mut prj, r, panel::Opening::Door, Face::Wall(1), Some(1500.0), None, None).unwrap();
    let pos = panel::positions(&prj.layout);
    let poses = panel::site_poses(&prj);
    let s = room_space(&prj, r, &pos, &poses, &prj.layout.routing);
    let at = |x: f32, y: f32, z: f32| add(pos[r], [x, y, z]);
    assert!(!s.blocked(at(1000.0, 1000.0, 1000.0)), "inside");
    assert!(s.blocked(at(1000.0, 1000.0, 2500.0)), "in the cut-out corner of the L");
    assert!(s.blocked(at(3000.0, 1400.0, 500.0)), "in the inner wall");
    assert!(!s.blocked(at(3000.0, 1400.0, 1300.0)), "beyond the end of the inner wall");
    // The cabinet from the floor to its top (800 mm), free above it.
    assert!(s.blocked(at(800.0, 2500.0, 125.0)));
    assert!(!s.blocked(at(800.0, 1500.0, 125.0)));
    assert_eq!(cabinet_box(&prj, &poses, r, c), ([500.0, 2000.0, 0.0], [1100.0, 2800.0, 250.0]));
    // The door in the right wall swings 1 m into the room, up to its height.
    assert!(s.blocked(at(3500.0, 2500.0, 2000.0)));
    assert!(!s.blocked(at(3500.0, 300.0, 2000.0)), "above the door");
}

#[test]
fn a_hose_from_a_cabinet_to_the_wall_runs_through_its_gland() {
    let (mut prj, r, c) = pump_room();
    let an = Analysis::checked(&prj);
    let h = tube(&an, "-QM1");
    assert!(h.routed(), "{:?}", h.status);
    // Without a gland through the middle of the roof, and reported.
    assert_eq!(h.legs.len(), 2);
    assert!(an.issues.iter().any(|i| i.message().contains("enters through the roof")), "{:?}", an.issues);
    let gland = with_roof_gland(&mut prj, c);
    let an = Analysis::checked(&prj);
    assert!(!an.issues.iter().any(|i| i.message().contains("enters through the roof")));
    let h = tube(&an, "-QM1");
    assert!(h.routed(), "{:?}", h.status);
    assert_eq!(h.legs.iter().map(|l| l.enclosure).collect::<Vec<_>>(), vec![c, r]);
    assert_eq!(h.enclosures, vec![c, r]);
    assert_eq!(h.points.len(), h.legs.iter().map(|l| l.points.len()).sum::<usize>());
    // The cabinet leg ends at the gland, the room leg starts above the roof.
    let pos = panel::positions(&prj.layout);
    let poses = panel::site_poses(&prj);
    let cab = &prj.layout.enclosures[c];
    let g = cab.item(gland).unwrap();
    let inside = add(panel::face_point(cab, Face::Roof, [g.x + g.w / 2.0, g.y + g.h / 2.0, 5.0]), pos[c]);
    assert!(manhattan(*h.legs[0].points.last().unwrap(), inside) < 1.0);
    let start = sub(h.legs[1].points[0], pos[r]);
    assert!(start[1] < 2800.0 - 800.0, "above the cabinet: {start:?}");
    let outside = panel::face_point(cab, Face::Roof, [g.x + g.w / 2.0, g.y + g.h / 2.0, -super::OUTSIDE]);
    assert!(manhattan(start, cabinet_in_room(&prj, &poses, r, c, outside)) < 1.0);
    // The length is that of both legs (less the rounded bends) and the allowances.
    let legs: f32 = h.legs.iter().map(|l| grid_length(&l.points)).sum();
    let extra = 2.0 * prj.fluid.allowance;
    assert!(h.length <= legs + extra + 0.5 && h.length > 0.9 * legs, "{} {}", h.length, legs);
    // The hose between the two wall valves stays in the room.
    let h2 = tube(&an, "-QM3");
    assert!(h2.routed() && h2.legs.is_empty());
    assert_eq!(h2.enclosures, vec![r]);
}

fn grid_length(points: &[V3]) -> f32 {
    super::super::grid::length(points)
}

#[test]
fn routes_in_the_room_go_round_inner_walls_and_prefer_pipe_runs() {
    let (mut prj, r, _) = pump_room();
    // An inner wall from the right wall between both wall valves.
    let wall = panel::add_inner_wall(&mut prj, r, [3000.0, 1900.0], [4000.0, 1900.0], 100.0).unwrap();
    let an = Analysis::checked(&prj);
    let h = tube(&an, "-QM3");
    assert!(h.routed(), "{:?}", h.status);
    let fp = panel::inner_footprint(prj.layout.enclosures[r].inner_walls.iter().find(|w| w.id == wall).unwrap());
    let pos = panel::positions(&prj.layout);
    for s in h.points.windows(2) {
        for t in 0..=20 {
            let q = sub([0, 1, 2].map(|i| s[0][i] + (s[1][i] - s[0][i]) * t as f32 / 20.0), pos[r]);
            assert!(!panel::inside_outline(&fp, [q[0], q[2]]) || q[1] < 1.0, "through the inner wall at {q:?}");
        }
    }
    // Without the inner wall the hose runs in free air; a pipe run along the right wall
    // above the valves takes it.
    panel::remove_inner_wall(&mut prj.layout.enclosures[r], wall);
    let free = tube(&Analysis::checked(&prj), "-QM3").free;
    panel::add_pipe_run(&mut prj, r, Face::Wall(1), &[p(1000.0, 1000.0), p(2600.0, 1000.0)], None, None, None).unwrap();
    let an = Analysis::checked(&prj);
    let h = tube(&an, "-QM3");
    assert!(h.routed() && h.free < free * 0.5, "{} {}", h.free, free);
    let path = an.routing.paths.iter().find(|u| u.enclosure == r).expect("the pipe run is used");
    assert!(path.items >= 1);
}

#[test]
fn frozen_routes_through_the_room_keep_their_legs() {
    let (mut prj, r, c) = pump_room();
    with_roof_gland(&mut prj, c);
    let an = Analysis::checked(&prj);
    let key = tube(&an, "-QM1").key.clone();
    assert_eq!(freeze(&mut prj, &an.routing, std::slice::from_ref(&key)), 1);
    let f = prj.layout.frozen.iter().find(|f| f.key == key).unwrap();
    assert_eq!(f.legs.len(), 2);
    assert!(f.points.is_empty());
    assert_eq!((f.legs[0].enclosure, f.legs[1].enclosure), (prj.layout.enclosures[c].id, prj.layout.enclosures[r].id));
    let json = serde_json::to_string(&prj.layout.frozen).unwrap();
    assert_eq!(serde_json::from_str::<Vec<FrozenRoute>>(&json).unwrap(), prj.layout.frozen);
    let an = Analysis::checked(&prj);
    let h = tube(&an, "-QM1");
    assert_eq!(h.status, Status::Frozen);
    assert!(an.routing.changes.is_empty(), "{:?}", an.routing.changes);
    // The cabinet moves along the wall: its gland with it, the frozen route no longer fits.
    panel::set_room(&mut prj, c, Some(r), Some(SitePlace { x: 1500.0, z: 0.0, angle: 0.0 })).unwrap();
    let an = Analysis::checked(&prj);
    assert_eq!(tube(&an, "-QM1").status, Status::Rerouted);
    assert!(an.issues.iter().any(|i| i.message().contains("Frozen route of")));
}

#[test]
fn cables_from_a_cabinet_to_a_room_wall_and_ends_elsewhere() {
    let mut k = Kit::new();
    let f1 = k.sym("cb1", 100.0, 60.0, "-F1", "");
    let e1 = k.sym("heater", 100.0, 80.0, "-E1", "");
    k.cable("-W1", "1 2", "1.5 mm\u{b2}", 0.0);
    k.core((f1, "2"), (e1, "1"), "-W1", "1");
    k.part("-F1", FnKind::Breaker1, "1 2", |p| (p.width, p.height, p.depth) = (18.0, 90.0, 70.0));
    k.part("-E1", FnKind::Generic, "1 2", |p| {
        (p.width, p.height, p.depth) = (300.0, 200.0, 100.0);
        p.mounting_data.wall_mount = true;
    });
    let mut prj = k.prj;
    let room = panel::new_room(&mut prj, "+ROOM1", 4000.0, 3000.0, 2800.0);
    prj.layout.enclosures.push(room);
    let cab = panel::new_enclosure(&mut prj, ENCLOSURE_PART, "+CAB1");
    prj.layout.enclosures.push(cab);
    panel::place_device_on(&mut prj, 1, "-F1", p(250.0, 300.0), Face::Plate).unwrap();
    panel::mount_on_wall(&mut prj, 0, "-E1", Face::Wall(3), p(1000.0, 1000.0)).unwrap();
    // The cabinet outside the room: not routed.
    let an = Analysis::checked(&prj);
    let w1 = an.routing.item("-W1").expect("cable");
    assert_eq!(w1.status, Status::Unrouted(openecraft_i18n::tr("not in the same line-up or room").to_string()));
    // In the room: through the cabinet and the room.
    panel::set_room(&mut prj, 1, Some(0), Some(SitePlace { x: 1500.0, z: 0.0, angle: 0.0 })).unwrap();
    with_roof_gland(&mut prj, 1);
    let an = Analysis::checked(&prj);
    let w1 = an.routing.item("-W1").unwrap();
    assert!(w1.routed() && w1.legs.len() == 2, "{:?}", w1.status);
    let mut tags: Vec<&str> = w1.ends.iter().map(|e| e.tag.as_str()).collect();
    tags.sort_unstable();
    assert_eq!(tags, ["-E1", "-F1"]);
    // A heater on the wall alone, with the breaker not placed: nothing to route.
    let i = prj.layout.enclosures[1].items.iter().position(|i| i.tag == "-F1").unwrap();
    prj.layout.enclosures[1].items.remove(i);
    let an = Analysis::checked(&prj);
    assert!(an.routing.item("-W1").is_none());
}
