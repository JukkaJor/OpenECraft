use super::*;
use crate::geom::{p, P};
use crate::model::SymbolInst;

fn sym(id: u64, key: &str, pos: P, tag: &str) -> SymbolInst {
    let def = crate::symbols::builtin(key).unwrap();
    SymbolInst { id, sym: key.into(), pos, tag: tag.into(), pins: def.pins.iter().map(|x| x.label.to_string()).collect(), ..Default::default() }
}

/// Valve -QM1 with its coil -MB1, cylinder -MM1, a second coil -MB2 and a relay -KF1.
fn project() -> Project {
    let mut prj = Project { next_id: 100, ..Default::default() };
    let pg = &mut prj.pages[0];
    pg.symbols.push(SymbolInst { function_text: "Clamp".into(), ..sym(1, "fl_valve52", p(100.0, 100.0), "-QM1") });
    pg.symbols.push(sym(2, "fl_cylinder", p(104.0, 60.0), "-MM1"));
    pg.symbols.push(sym(3, "coil", p(300.0, 100.0), "-MB1"));
    pg.symbols.push(sym(4, "coil", p(340.0, 100.0), "-MB2"));
    pg.symbols.push(sym(5, "coil", p(380.0, 100.0), "-KF1"));
    prj
}

fn messages(prj: &Project) -> Vec<(Severity, String)> {
    problems(prj, &Index::build(prj)).into_iter().map(|(s, m, _)| (s, m)).collect()
}

#[test]
fn tags_and_letters() {
    assert_eq!(letters("=A1+S1-MB12"), "MB");
    assert_eq!(letters("-QM3.1"), "QM");
    assert_eq!(with_letters("=A1-QM3", "MB").as_deref(), Some("=A1-MB3"));
    assert_eq!(with_letters("QM3", "MB"), None);
    assert_eq!(GroupKind::from_code("valve terminal"), Some(GroupKind::ValveTerminal));
}

#[test]
fn synchronize_links_valves_with_their_coils() {
    let mut prj = project();
    let before = messages(&prj);
    assert!(before.iter().any(|(s, m)| *s == Severity::Warning && m.starts_with("-QM1: solenoid valve without")), "{before:?}");
    assert!(before.iter().any(|(s, m)| *s == Severity::Info && m.starts_with("-MB1: solenoid coil not linked")));
    // Relays are no solenoids.
    assert!(!before.iter().any(|(_, m)| m.starts_with("-KF1")));
    let sync = synchronize(&mut prj);
    assert_eq!(sync.linked, [("-QM1".to_string(), "-MB1".to_string())]);
    assert_eq!(sync.texts, 1);
    assert_eq!(prj.pages[0].symbols[2].function_text, "Clamp");
    assert_eq!(linked(&prj, "-QM1"), ["-MB1"]);
    let after = messages(&prj);
    assert!(!after.iter().any(|(_, m)| m.starts_with("-QM1") || m.starts_with("-MB1")), "{after:?}");
    // Nothing more to do.
    assert_eq!(synchronize(&mut prj), Sync::default());
}

#[test]
fn linked_devices_show_cross_references_in_both_schematics() {
    let mut prj = project();
    synchronize(&mut prj);
    let idx = Index::build(&prj);
    let at_valve = cross_refs(&prj, &idx, 0, "-QM1");
    assert_eq!(at_valve.len(), 1);
    assert!(at_valve[0].0.starts_with("-MB1 /1."), "{at_valve:?}");
    assert_eq!(at_valve[0].1, Some(0));
    assert!(cross_refs(&prj, &idx, 0, "-MB1")[0].0.starts_with("-QM1 /1."));
    assert!(cross_refs(&prj, &idx, 0, "-KF1").is_empty());
    let e = entries(&prj, &idx);
    assert_eq!(e.len(), 1);
    assert_eq!((e[0].tag.as_str(), e[0].solenoids, e[0].electrical[0].0.as_str()), ("-QM1", 1, "-MB1"));
}

#[test]
fn wrong_links_are_reported() {
    let mut prj = project();
    prj.fluid.links.insert("-QM1".into(), vec!["-MB1".into(), "-MB2".into(), "-MM1".into(), "-BG9".into()]);
    prj.fluid.links.insert("-QM7".into(), vec!["-MB1".into()]);
    let msgs = messages(&prj);
    let has = |s: Severity, text: &str| msgs.iter().any(|(sev, m)| *sev == s && m.contains(text));
    assert!(has(Severity::Warning, "2 solenoid coils linked, the valve has 1"), "{msgs:?}");
    assert!(has(Severity::Error, "-MM1 is a fluid device"));
    assert!(has(Severity::Warning, "linked electrical device -BG9 is not in the schematic"));
    assert!(has(Severity::Warning, "-QM7: linked fluid device is not in the schematic"));
    assert!(has(Severity::Warning, "-MB1: linked to several fluid devices (-QM1, -QM7)"));
}

#[test]
fn device_groups_are_checked() {
    let mut prj = project();
    prj.fluid.groups.push(DeviceGroup { tag: "-KH1".into(), slots: 2, members: vec!["-QM1".into(), "-MM1".into(), "-QM9".into()], ..Default::default() });
    prj.fluid.groups.push(DeviceGroup { tag: "-KH2".into(), kind: GroupKind::Manifold, members: vec!["-QM1".into(), "-MB2".into()], ..Default::default() });
    let msgs = messages(&prj);
    let has = |s: Severity, text: &str| msgs.iter().any(|(sev, m)| *sev == s && m.contains(text));
    assert!(has(Severity::Error, "Device group -KH1: 3 devices on 2 places"), "{msgs:?}");
    assert!(has(Severity::Warning, "Device group -KH1: -MM1 is not a valve"));
    assert!(has(Severity::Warning, "Device group -KH1: -QM9 is not in the schematic"));
    assert!(has(Severity::Warning, "Device group -KH2: -MB2 is not a fluid device"));
    assert!(has(Severity::Error, "-QM1: member of several device groups (-KH1, -KH2)"));
    assert_eq!(group_of(&prj, "-MM1").map(|g| g.tag.as_str()), Some("-KH1"));
}

#[test]
fn renamed_devices_keep_their_groups_and_links() {
    let mut prj = project();
    synchronize(&mut prj);
    prj.fluid.groups.push(DeviceGroup { tag: "-KH1".into(), members: vec!["-QM1".into()], ..Default::default() });
    crate::ops::rename_device(&mut prj, "-QM1", "-QM5");
    crate::ops::rename_device(&mut prj, "-MB1", "-MB5");
    crate::ops::rename_device(&mut prj, "-KH1", "-KH3");
    assert_eq!(linked(&prj, "-QM5"), ["-MB5"]);
    assert!(!prj.fluid.links.contains_key("-QM1"));
    assert_eq!(prj.fluid.groups[0].tag, "-KH3");
    assert_eq!(prj.fluid.groups[0].members, ["-QM5"]);
}
