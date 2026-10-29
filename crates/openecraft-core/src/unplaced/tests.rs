use super::*;
use crate::sample::demo_project;

#[test]
fn plc_channels_without_symbol() {
    let prj = demo_project();
    let channels = plc_channels(&prj);
    let a1: Vec<&Unplaced> = channels.iter().filter(|c| c.tag == "-A1").collect();
    assert!(!a1.is_empty(), "the 16 inputs of -A1 are not all placed");
    assert!(a1.iter().all(|c| c.symbol == "plc_in"));
    assert!(!a1.iter().any(|c| c.label == "I0.0" || c.label == "I0.2"), "placed channels are left out");
    assert!(a1.iter().any(|c| c.label == "I0.3"));
    assert!(channels.iter().any(|c| c.tag == "-A2" && c.symbol == "plc_out"));
}

#[test]
fn part_functions_not_used_yet() {
    let prj = demo_project();
    let funcs = device_functions(&prj, None);
    // -K1 has an auxiliary contact block (2 NO + 2 NC) of which not all contacts are drawn.
    let k1: Vec<&Unplaced> = funcs.iter().filter(|f| f.tag == "-K1").collect();
    assert!(!k1.is_empty(), "{funcs:?}");
    assert!(k1.iter().all(|f| crate::symbols::builtin(f.symbol).is_some()));
    assert!(!funcs.iter().any(|f| f.tag == "-A1" || f.tag == "-X1"), "PLC points and terminals are listed elsewhere");
    assert_eq!(symbol_for(FnKind::No), Some("no"));
    assert_eq!(symbol_for(FnKind::Coil), Some("coil"));
}

#[test]
fn defined_potentials_without_a_connection_point_are_unplaced() {
    let mut prj = crate::sample::demo_project();
    for pg in &mut prj.pages {
        pg.symbols.retain(|s| s.label != "L2");
    }
    prj.potentials.retain(|p| p.name != "L2");
    let before = potentials(&prj).len();
    prj.potentials.push(crate::model::potentials::PotentialDef { name: "L2".into(), description: "Phase 2".into(), ..Default::default() });
    prj.potentials.push(crate::model::potentials::PotentialDef { name: "+48V".into(), ..Default::default() });
    let list = potentials(&prj);
    assert_eq!(list.len(), before + 2);
    let l2 = list.iter().find(|u| u.label == "L2").unwrap();
    assert_eq!((l2.symbol, l2.title.as_str()), ("pot_l2", "L2  Phase 2"));
    assert_eq!(list.iter().find(|u| u.label == "+48V").unwrap().symbol, "pot");
}

#[test]
fn parts_are_placed_with_the_symbol_of_their_main_function() {
    let parts = crate::parts::default_catalog();
    let get = |n: &str| parts.iter().find(|p| p.number == n).unwrap();
    let (sym, pins) = part_symbol(get("GEN.LMP-GN-24")).expect("lamp");
    assert_eq!(crate::parts::kind_of_symbol(sym), Some(FnKind::Signal));
    assert_eq!(pins, vec!["X1".to_string(), "X2".to_string()]);
    assert_eq!(part_symbol(&crate::parts::Part::default()), None);
}
