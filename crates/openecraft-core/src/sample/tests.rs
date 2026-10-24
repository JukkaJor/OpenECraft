use super::*;
use crate::index::Index;
use crate::reports::{Analysis, Severity};

#[test]
fn demo_is_consistent() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
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
    // BOM: -X1 has three feed-through terminals and one PE terminal.
    let bom = &an.report(&prj, crate::model::ReportKind::BillOfMaterials);
    let qty = |part: &str| bom.rows.iter().find(|r| r[3] == part).map(|r| r[1].clone()).unwrap_or_default();
    assert_eq!((qty("GEN.TB-2.5").as_str(), qty("GEN.TB-PE-2.5").as_str()), ("3", "1"));
}
