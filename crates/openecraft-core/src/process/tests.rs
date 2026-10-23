use super::*;
use crate::geom::p;
use crate::model::SymbolInst;

fn inst(id: u64, key: &str, x: f32, tag: &str, label: &str) -> SymbolInst {
    let def = symbols::get(key).unwrap();
    SymbolInst { id, sym: key.into(), pos: p(x, 100.0), tag: tag.into(), label: label.into(), pins: def.pins.iter().map(|x| x.label.to_string()).collect(), ..Default::default() }
}

#[test]
fn instrument_labels_are_parsed() {
    assert_eq!(parse_label("TIC 101"), Some(("TIC".into(), "101".into())));
    assert_eq!(parse_label("pi-12"), Some(("PI".into(), "12".into())));
    assert_eq!(parse_label("LSH102A"), Some(("LSH".into(), "102A".into())));
    assert_eq!(parse_label("TIC"), None);
    assert_eq!(parse_label("101"), None);
    assert_eq!(variable("TIC"), "Temperature");
    assert_eq!(variable("FQ"), "Flow");
}

#[test]
fn loops_follow_the_engineering() {
    let mut prj = Project::default();
    prj.pct_loops.push(PctLoop { name: "101".into(), description: "Tank temperature".into(), range: "0 … 120 °C".into(), devices: vec!["-KF101".into()] });
    prj.pct_loops.push(PctLoop { name: "300".into(), ..Default::default() });
    let pg = &mut prj.pages[0];
    pg.symbols.push(inst(1, "pid_instrument", 60.0, "-BT101", "TIC 101"));
    pg.symbols.push(inst(2, "pid_instrument", 80.0, "-BL201", "LI 201"));
    pg.symbols.push(inst(3, "pid_instrument", 100.0, "-BP1", "pressure"));
    let idx = Index::build(&prj);
    let loops = loops(&prj, &idx);
    assert_eq!(loops.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["101", "201", "300"]);
    assert_eq!(loops[0].functions, ["TIC"]);
    assert_eq!(loops[0].devices, ["-BT101", "-KF101"]);
    assert_eq!(loops[0].status, Status::Planned);
    assert!(!loops[1].defined);
    let msgs: Vec<String> = problems(&prj, &idx).into_iter().map(|x| x.0).collect();
    assert!(msgs.iter().any(|m| m.contains("-BP1")), "{msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("201") && m.contains("not in the pre-planning")));
    assert!(msgs.iter().any(|m| m.contains("300") && m.contains("no instrument")));

    // The sensor is drawn in the schematic: the loop is in engineering, then done.
    prj.pages[0].symbols.push(inst(4, "coil", 200.0, "-BT101", ""));
    let idx = Index::build(&prj);
    let l = &crate::process::loops(&prj, &idx)[0];
    assert_eq!((l.status, l.missing.clone()), (Status::InProgress, vec!["-KF101".to_string()]));
    prj.pages[0].symbols.push(inst(5, "coil", 240.0, "-KF101", ""));
    let idx = Index::build(&prj);
    assert_eq!(crate::process::loops(&prj, &idx)[0].status, Status::Engineered);
}

#[test]
fn instruments_are_overviews_without_netlist_pins() {
    let mut prj = Project::default();
    prj.pages[0].symbols.push(inst(1, "pid_instrument", 60.0, "-BT101", "TIC 101"));
    let nl = crate::netlist::Netlist::build(&prj, true);
    assert!(nl.unconnected.is_empty());
    let idx = Index::build(&prj);
    assert_eq!(idx.devices["-BT101"].overviews.len(), 1);
}
