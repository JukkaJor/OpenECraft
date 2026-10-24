use super::*;
use crate::sample::demo_project;

fn item(id: u64, kind: ItemKind, x: f32, y: f32, w: f32, h: f32) -> PanelItem {
    PanelItem { id, kind, tag: if kind == ItemKind::Device { format!("-D{id}") } else { String::new() }, x, y, w, h, d: 60.0, ..Default::default() }
}

fn project(items: Vec<PanelItem>, parts: Vec<Part>) -> Project {
    let mut prj = Project::new_default();
    prj.parts = parts;
    prj.layout.enclosures.push(Enclosure { items, ..Default::default() });
    prj
}

fn kinds_of(prj: &Project) -> Vec<CheckKind> {
    validate(prj, &prj.layout.enclosures[0]).into_iter().filter(|f| f.severity == Severity::Warning).map(|f| f.kind).collect()
}

#[test]
fn clearances_from_the_part_data() {
    let part = Part { tech_data: "Clearance: 25 mm (convection cooling, above and below)\nClearance left: 10\nEMC clearance: 100 mm\nService space: 80".into(), ..Default::default() };
    let c = clearances(&part);
    assert_eq!((c.top, c.bottom, c.left, c.right, c.front, c.emc, c.service), (25.0, 25.0, 10.0, 0.0, 0.0, 100.0, 80.0));
    let part = Part { tech_data: "Clearance: 50\nClearance top: 100".into(), ..Default::default() };
    assert_eq!((clearances(&part).top, clearances(&part).bottom), (100.0, 50.0));
}

#[test]
fn ventilation_zones_above_and_below() {
    let part = Part { number: "PS".into(), tech_data: "Clearance: 25 mm".into(), ..Default::default() };
    let dev = PanelItem { part: "PS".into(), z: 7.5, ..item(1, ItemKind::Device, 100.0, 100.0, 40.0, 125.0) };
    let rail = PanelItem { d: 7.5, ..item(2, ItemKind::Rail, 0.0, 145.0, 500.0, 35.0) };
    let duct_close = item(3, ItemKind::Duct, 0.0, 40.0, 500.0, 40.0);
    let duct_far = item(4, ItemKind::Duct, 0.0, 300.0, 500.0, 40.0);
    let prj = project(vec![dev, rail, duct_close, duct_far], vec![part]);
    let f = validate(&prj, &prj.layout.enclosures[0]);
    let vent: Vec<_> = f.iter().filter(|f| f.kind == CheckKind::Ventilation).collect();
    assert_eq!(vent.len(), 1, "{f:#?}");
    assert_eq!(vent[0].items, vec![1, 3]);
    assert!(crate::panel::problem_items(&prj, &prj.layout.enclosures[0]).contains(&3));
}

#[test]
fn mounting_clearance_and_duct_gap() {
    let a = item(1, ItemKind::Device, 100.0, 100.0, 40.0, 80.0);
    let b = item(2, ItemKind::Device, 145.0, 100.0, 40.0, 80.0);
    let duct = item(3, ItemKind::Duct, 0.0, 200.0, 500.0, 40.0);
    let mut prj = project(vec![a, b, duct], Vec::new());
    assert!(kinds_of(&prj).is_empty());
    prj.layout.checks.device_gap = 10.0;
    let f = validate(&prj, &prj.layout.enclosures[0]);
    let mounting: Vec<_> = f.iter().filter(|f| f.kind == CheckKind::Mounting).collect();
    assert_eq!(mounting.len(), 1, "reported once per pair: {mounting:#?}");
    prj.layout.checks.device_gap = 0.0;
    prj.layout.checks.duct_gap = 30.0;
    assert_eq!(kinds_of(&prj), vec![CheckKind::Mounting, CheckKind::Mounting]);
}

#[test]
fn emc_clearance_and_collision_classes() {
    let mut prj = project(vec![item(1, ItemKind::Device, 100.0, 100.0, 40.0, 80.0), item(2, ItemKind::Device, 200.0, 100.0, 40.0, 80.0)], Vec::new());
    let mut kit = crate::testkit::Kit::new();
    kit.sym("vfd", 0.0, 0.0, "-D1", "");
    kit.sym("plc_ai", 100.0, 0.0, "-D2", "");
    prj.pages = kit.prj.pages;
    let kinds = device_kinds(&prj);
    let e = &prj.layout.enclosures[0];
    assert_eq!((item_class(&prj, &kinds, &e.items[0]).as_str(), item_class(&prj, &kinds, &e.items[1]).as_str()), ("drive", "signal"));
    // Drive and signal device 60 mm apart, the default rule asks for 100 mm.
    assert_eq!(kinds_of(&prj), vec![CheckKind::ClassDistance]);
    prj.layout.checks.classes.clear();
    assert!(kinds_of(&prj).is_empty());
    // EMC clearance of the part.
    prj.parts.push(Part { number: "FC".into(), tech_data: "EMC clearance: 80".into(), ..Default::default() });
    prj.layout.enclosures[0].items[0].part = "FC".into();
    assert_eq!(kinds_of(&prj), vec![CheckKind::Emc]);
    // Classes set on the item win.
    prj.layout.enclosures[0].items[1].class = "power".into();
    assert!(kinds_of(&prj).is_empty());
}

#[test]
fn ignored_classes_do_not_collide() {
    let mut prj = project(vec![item(1, ItemKind::Device, 100.0, 100.0, 40.0, 80.0), item(2, ItemKind::Accessory, 110.0, 110.0, 40.0, 40.0)], Vec::new());
    assert_eq!(kinds_of(&prj), vec![CheckKind::Collision]);
    prj.layout.checks.classes.push(ClassRule { a: "accessory".into(), b: "*".into(), ignore: true, ..Default::default() });
    assert!(class_collisions(&prj, &prj.layout.enclosures[0]).is_empty());
    assert!(kinds_of(&prj).is_empty());
}

#[test]
fn service_space_up_to_the_door() {
    let mut dev = item(1, ItemKind::Device, 100.0, 100.0, 40.0, 80.0);
    dev.d = 150.0;
    let mut prj = project(vec![dev], Vec::new());
    prj.layout.checks.service_space = 50.0;
    // Depth 250: plate at 23, device front at 173, door at 248.
    assert!(kinds_of(&prj).is_empty());
    prj.layout.checks.service_space = 100.0;
    assert_eq!(kinds_of(&prj), vec![CheckKind::Service]);
    // A door device in front of it blocks the space.
    prj.layout.checks.service_space = 50.0;
    let door = PanelItem { face: Face::Door, d: 60.0, ..item(2, ItemKind::Device, 120.0, 120.0, 40.0, 40.0) };
    prj.layout.enclosures[0].items.push(door);
    let f = validate(&prj, &prj.layout.enclosures[0]);
    assert!(f.iter().any(|f| f.kind == CheckKind::Service && f.items == vec![1, 2]), "{f:#?}");
}

#[test]
fn heights_above_the_floor() {
    let mut kit = crate::testkit::Kit::new();
    kit.sym("term", 0.0, 0.0, "-X1", "");
    kit.sym("pb_no", 0.0, 50.0, "-S1", "");
    let mut prj = kit.prj;
    let e = Enclosure { height: 800.0, items: vec![item(1, ItemKind::Device, 100.0, 720.0, 40.0, 20.0)], ..Default::default() };
    prj.layout.enclosures.push(e);
    prj.layout.enclosures[0].items[0].tag = "-X1".into();
    let s1 = PanelItem { face: Face::Door, tag: "-S1".into(), ..item(2, ItemKind::Device, 200.0, 100.0, 30.0, 30.0) };
    prj.layout.enclosures[0].items.push(s1);
    // Wall mounted, height unknown: not checked.
    assert!(kinds_of(&prj).is_empty());
    prj.layout.enclosures[0].elevation = Some(0.0);
    let e = &prj.layout.enclosures[0];
    // Terminals 35 … 55 mm above the floor (plate bottom edge at 775 mm).
    assert_eq!(heights(e, &e.items[0]), Some((35.0, 55.0)));
    let f = validate(&prj, e);
    assert!(f.iter().any(|f| f.kind == CheckKind::Height && f.severity == Severity::Warning && f.items == vec![1]), "{f:#?}");
    // The pushbutton (685 mm) is within the recommended range.
    assert!(!f.iter().any(|f| f.items == vec![2]), "{f:#?}");
    prj.layout.enclosures[0].elevation = Some(1500.0);
    let f = validate(&prj, &prj.layout.enclosures[0]);
    assert!(f.iter().any(|f| f.kind == CheckKind::Height && f.severity == Severity::Info && f.items == vec![2]), "{f:#?}");
    assert!(f.iter().any(|f| f.kind == CheckKind::Height && f.severity == Severity::Warning && f.items == vec![2]), "above 2 m: {f:#?}");
    // On a plinth the floor is known.
    let e = Enclosure { build: super::super::Build { plinth: 100.0, ..Default::default() }, ..Default::default() };
    assert_eq!(floor_height(&e), Some(100.0));
}

#[test]
fn demo_layout_passes() {
    let prj = demo_project();
    let f = validate(&prj, &prj.layout.enclosures[0]);
    assert!(f.iter().all(|f| f.severity != Severity::Warning), "{f:#?}");
}

#[test]
fn rules_round_trip() {
    let mut l = crate::panel::Layout::default();
    let json = serde_json::to_string(&l).unwrap();
    assert!(!json.contains("checks"), "default rules are not stored");
    l.checks.device_gap = 5.0;
    l.checks.classes.push(ClassRule { a: "power".into(), b: "signal".into(), distance: 50.0, ignore: false });
    let back: crate::panel::Layout = serde_json::from_str(&serde_json::to_string(&l).unwrap()).unwrap();
    assert_eq!(back.checks, l.checks);
}
