use super::*;
use crate::index::Index;
use crate::reports::{Analysis, Severity};

#[test]
fn demo_is_consistent() {
    let prj = demo_project();
    let an = Analysis::build(&prj);
    let errors: Vec<_> = an.issues.iter().filter(|i| i.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let idx = Index::build(&prj);
    assert_eq!(idx.devices["-K1"].mains.len(), 1);
    assert_eq!(idx.devices["-K1"].auxs.len(), 4);
    // Potentials propagate through the interruption points.
    let lamp = prj.pages[2].symbols.iter().find(|s| s.tag == "-P1").unwrap();
    let net = an.netlist.pin_net[&(lamp.id, 1)];
    assert_eq!(an.netlist.nets[net].name, "0V");
    let warnings: Vec<_> = an.issues.iter().filter(|i| i.severity == Severity::Warning).map(|i| &i.message).collect();
    assert!(warnings.is_empty(), "{warnings:#?}");
    // Unused contacts of contactor + auxiliary block are reserves (2 NC of the block).
    let reserve = crate::parts::reserve_functions(&prj, None, "-K1");
    let pins: Vec<_> = reserve.iter().map(|f| f.pins.as_str()).collect();
    assert_eq!(pins, vec!["61 62", "71 72"]);
    // BOM: -X1 has four terminals.
    let bom = &an.reports[&crate::model::ReportKind::BillOfMaterials];
    let tb = bom.rows.iter().find(|r| r[3] == "GEN.TB-2.5").unwrap();
    assert_eq!(tb[1], "4");
}
