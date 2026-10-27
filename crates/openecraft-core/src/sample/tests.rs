use super::*;
use crate::index::Index;
use crate::analysis::Analysis;
use crate::issue::Severity;

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
    let warnings: Vec<_> = an.issues.iter().filter(|i| i.severity == Severity::Warning).map(|i| i.message()).collect();
    assert!(warnings.is_empty(), "{warnings:#?}");
    // Unused contacts of contactor + auxiliary block are reserves (2 NC of the block).
    let reserve = crate::parts::reserve_functions(&prj, None, "-K1");
    let pins: Vec<_> = reserve.iter().map(|f| f.pins.as_str()).collect();
    assert_eq!(pins, vec!["61 62", "71 72"]);
    // BOM: -X1 has three feed-through terminals and one PE terminal.
    let bom = &an.report(&prj, crate::model::ReportKind::BillOfMaterials).unwrap();
    let qty = |part: &str| bom.rows.iter().find(|r| r[3] == part).map(|r| r[1].clone()).unwrap_or_default();
    assert_eq!((qty("GEN.TB-2.5").as_str(), qty("GEN.TB-PE-2.5").as_str()), ("3", "1"));
}

fn checked(key: &str) -> (Project, Analysis) {
    let prj = example(key).unwrap_or_else(|| panic!("example {key}"));
    let an = Analysis::checked(&prj);
    (prj, an)
}

#[test]
fn examples_are_consistent() {
    let mut keys: Vec<&str> = EXAMPLES.iter().map(|e| e.key).collect();
    keys.dedup();
    assert_eq!(keys.len(), EXAMPLES.len());
    assert!(EXAMPLES.windows(2).all(|w| w[0].level < w[1].level), "levels rise");
    for e in &EXAMPLES {
        let (prj, an) = checked(e.key);
        let problems: Vec<String> =
            an.issues.iter().filter(|i| i.severity <= Severity::Warning).map(|i| format!("{:?}: {}", i.severity, i.message())).collect();
        assert!(problems.is_empty(), "{}: {problems:#?}", e.key);
        // Every device with a function in the schematic has a part (fluid ports and
        // the solenoid coils of the valves aside).
        let idx = Index::build(&prj);
        let missing: Vec<&String> = idx
            .devices
            .keys()
            .filter(|t| !prj.devices.get(*t).is_some_and(|d| !d.parts.is_empty()))
            .filter(|t| !["-BP1", "-GQ1", "-MB1", "-MB2"].contains(&t.as_str()))
            .collect();
        assert!(missing.is_empty(), "{}: devices without part {missing:?}", e.key);
        // Saved and opened again unchanged.
        let json = serde_json::to_string(&prj).unwrap();
        let back: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), json, "{}", e.key);
    }
}

#[test]
fn lamp_circuit_takes_the_potential_colours() {
    let (prj, an) = checked("lamp_circuit");
    assert!(prj.wire_colors);
    let lamp = prj.pages[1].symbols.iter().find(|s| s.tag == "-P1").unwrap();
    assert_eq!(an.netlist.nets[an.netlist.pin_net[&(lamp.id, 1)]].name, "0V");
    let bom = an.report(&prj, ReportKind::BillOfMaterials).unwrap();
    for part in ["GEN.MCB-1P-C6", "GEN.PB-GN-1NO", "GEN.SEL-2POS-1NO", "GEN.LMP-GN-24", "GEN.LMP-RD-24"] {
        assert!(bom.rows.iter().any(|r| r[3] == part), "{part}");
    }
}

#[test]
fn relay_holds_itself_through_the_strip() {
    let (prj, an) = checked("self_holding_relay");
    let idx = Index::build(&prj);
    assert_eq!((idx.devices["-K1"].mains.len(), idx.devices["-K1"].auxs.len()), (1, 3));
    let strip = an.report(&prj, ReportKind::TerminalDiagram).unwrap();
    assert_eq!(strip.rows.iter().filter(|r| r[0] == "-X1" && r[1].parse::<u32>().is_ok()).count(), 4);
    // Wire numbers after page and column.
    assert!(prj.pages[1].wires.iter().any(|w| w.number.starts_with("2.")), "page / column numbers");
}

#[test]
fn star_delta_cable_uses_all_cores() {
    let (prj, an) = checked("star_delta_starter");
    let mut cores: Vec<&str> = prj.pages.iter().flat_map(|p| &p.wires).filter(|w| w.cable == "-W1").map(|w| w.core.as_str()).collect();
    cores.sort();
    assert_eq!(cores, ["1", "2", "3", "4", "5", "6", "GNYE"]);
    let idx = Index::build(&prj);
    for k in ["-K1", "-K2", "-K3", "-K4"] {
        assert_eq!(idx.devices[k].mains.len(), 1, "{k}");
    }
    assert!(an.report(&prj, ReportKind::CableOverview).unwrap().rows.iter().any(|r| r.contains(&"-W1".to_string())));
}

#[test]
fn press_links_fluid_and_electrical_devices() {
    let (prj, an) = checked("pneumatic_press");
    assert_eq!(crate::electrofluid::linked(&prj, "-QM1"), ["-MB1".to_string()]);
    assert_eq!(crate::electrofluid::linked(&prj, "-QM2"), ["-MB2".to_string()]);
    assert_eq!(prj.fluid.groups[0].members, ["-QM1", "-QM2"]);
    assert!(!an.report(&prj, ReportKind::HoseList).unwrap().rows.is_empty());
}

#[test]
fn pump_station_is_structured() {
    let (prj, an) = checked("pump_station");
    let idx = Index::build(&prj);
    for tag in ["=SUP+CAB1-T1", "=P1+CAB1-K1", "=P1+FLD-M1", "=P2+CAB1-T1", "=PLC+CAB1-A3", "=SUP+FLD-X10"] {
        assert!(idx.devices.contains_key(tag), "{tag}");
    }
    assert_eq!(prj.sections.len(), 6);
    assert!(prj.pages.iter().all(|p| p.name.contains('.')), "page names carry the section numbers");
    let plc = an.report(&prj, ReportKind::PlcAssignment).unwrap();
    assert!(plc.rows.iter().any(|r| r.contains(&"IW64".to_string())), "{plc:?}");
    // Planned in the single-line diagram and engineered in the schematic.
    let devices = crate::singleline::devices(&prj, &idx);
    assert!(devices.iter().filter(|d| d.status == crate::singleline::Status::Engineered).count() >= 5);
}

#[test]
fn workflow_marks_the_change_since_the_release() {
    use crate::revision::{pending, status, Status};
    let (prj, _) = checked("engineering_workflow");
    assert_eq!(status(&prj, 0), Status::Released);
    assert_eq!(status(&prj, 1), Status::InReview);
    let changed: Vec<String> = pending(&prj).into_iter().map(|c| c.name).collect();
    assert_eq!(changed, ["3"]);
    assert!(prj.pages.iter().filter(|p| p.name != "3").all(|p| p.locked));
    assert_eq!(prj.review.requests[0].number, "CR-001");
    assert_eq!(prj.baselines.len(), 1);
}
