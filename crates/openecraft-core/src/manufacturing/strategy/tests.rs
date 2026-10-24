use super::*;
use crate::manufacturing::machines::{builtin, find};
use crate::panel::{Enclosure, PanelItem};
use crate::sample::demo_project;

fn project() -> Project {
    let mut prj = Project::default();
    let mut e = Enclosure::default();
    let item = |id, kind, x, y, w, h, round, face| PanelItem { id, kind, x, y, w, h, d: 30.0, round, face, ..Default::default() };
    e.items.push(item(1, ItemKind::Rail, 50.0, 100.0, 400.0, 35.0, false, Face::Plate));
    e.items.push(item(2, ItemKind::Cutout, 100.0, 300.0, 22.5, 22.5, true, Face::Door));
    e.items.push(item(3, ItemKind::Cutout, 300.0, 300.0, 92.0, 92.0, false, Face::Door));
    e.items.push(item(4, ItemKind::Cutout, 300.0, 500.0, 60.0, 40.0, false, Face::Door));
    prj.layout.enclosures.push(e);
    prj
}

fn machine(id: &str) -> Machine {
    builtin().into_iter().find(|m| m.id == id).unwrap()
}

#[test]
fn drilling_and_tapping_on_a_machining_centre() {
    let prj = project();
    let m = machine("drill_mill_iso");
    let p = plan(&prj, 0, Face::Plate, &m, &Strategy::default());
    let holes = crate::production::holes_on(&prj.layout.enclosures[0], &prj.parts, Face::Plate).len();
    assert_eq!(p.count("drill"), holes);
    assert_eq!(p.count("tap"), holes);
    assert!(p.problems.is_empty(), "{:?}", p.problems);
    // Drills first, then taps; numbered in order.
    let first_tap = p.ops.iter().position(|o| o.kind.key() == "tap").unwrap();
    assert!(p.ops[..first_tap].iter().all(|o| o.kind.key() == "drill"));
    assert!(p.ops.iter().enumerate().all(|(i, o)| o.seq == i + 1));
    assert_eq!(p.tool_changes, 2);
    assert!(p.time > 0.0 && p.travel > 0.0);
    // Without tapping the threads are reported.
    let s = Strategy { tap: false, ..Default::default() };
    let p = plan(&prj, 0, Face::Plate, &m, &s);
    assert_eq!(p.count("tap"), 0);
    assert_eq!(p.problems.len(), 1);
}

#[test]
fn openings_are_milled_or_punched() {
    let prj = project();
    let mill = plan(&prj, 0, Face::Door, &machine("drill_mill_iso"), &Strategy::default());
    assert_eq!(mill.count("mill_circle"), 1);
    assert_eq!(mill.count("mill_rect"), 2);
    assert!(mill.milled > 0.0);
    let punch = machine("punch_turret");
    let p = plan(&prj, 0, Face::Door, &punch, &Strategy::default());
    // Ø22.5 and 92 x 92 have their punch, 60 x 40 is nibbled.
    assert_eq!(p.count("punch"), 2);
    assert_eq!(p.count("nibble"), 1);
    let op = p.ops.iter().find(|o| o.item == 4).unwrap();
    let OpKind::Nibble { hits } = &op.kind else { panic!() };
    assert!(hits.len() > 4);
    assert!(hits.iter().all(|h| (h.x - op.pos.x).abs() <= 30.0 && (h.y - op.pos.y).abs() <= 20.0));
    // Contour strategy: everything nibbled.
    let s = Strategy { cutouts: Method::Contour, ..Default::default() };
    let p = plan(&prj, 0, Face::Door, &punch, &s);
    assert_eq!((p.count("punch"), p.count("nibble")), (0, 3));
}

#[test]
fn limits_of_the_machine() {
    let prj = project();
    let mut m = machine("drill_mill_iso");
    m.area = (300.0, 300.0);
    m.max_thickness = 1.0;
    m.tools.retain(|t| t.kind != ToolKind::Mill);
    let p = plan(&prj, 0, Face::Door, &m, &Strategy::default());
    assert_eq!(p.count("manual"), 3);
    assert_eq!(p.machined(), 0);
    // Working area, thickness and three openings.
    assert_eq!(p.problems.len(), 5, "{:?}", p.problems);
    assert!(!plan(&prj, 0, Face::Plate, &machine("busbar_cnc"), &Strategy::default()).problems.is_empty());
}

#[test]
fn path_optimisation_shortens_the_travel() {
    let prj = demo_project();
    let m = find(&prj, "drill_mill_iso").unwrap();
    let on = plan(&prj, 0, Face::Plate, &m, &Strategy::default());
    let off = plan(&prj, 0, Face::Plate, &m, &Strategy { optimize: false, ..Default::default() });
    assert_eq!(on.ops.len(), off.ops.len());
    assert!(on.travel <= off.travel + 0.01, "{} > {}", on.travel, off.travel);
    assert_eq!(duration(65.0), "1 min 5 s");
    assert_eq!(duration(5.4), "5 s");
}
