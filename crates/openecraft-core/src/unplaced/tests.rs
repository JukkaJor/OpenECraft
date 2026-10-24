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
