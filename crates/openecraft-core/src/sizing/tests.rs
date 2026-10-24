use super::*;
use crate::geom::p;
use crate::parts::{FunctionTemplate, PartRef};
use crate::analysis::Analysis;
use crate::issue::Severity;

fn sym(prj: &mut Project, key: &str, x: f32, y: f32, tag: &str, label: &str) {
    let def = prj.symbol(key).unwrap();
    let id = prj.new_id();
    prj.pages[0].symbols.push(SymbolInst {
        id,
        sym: key.into(),
        pos: p(x, y),
        tag: tag.into(),
        label: label.into(),
        pins: def.pins.iter().map(|x| x.label.to_string()).collect(),
        ..Default::default()
    });
}

fn wire(prj: &mut Project, x1: f32, y1: f32, x2: f32, y2: f32, cs: &str) -> u64 {
    let id = prj.new_id();
    prj.pages[0].wires.push(Wire { id, a: p(x1, y1), b: p(x2, y2), cross_section: cs.into(), ..Default::default() });
    id
}

fn assign(prj: &mut Project, tag: &str, number: &str, kind: FnKind, pins: &str, current: &str, voltage: &str) {
    let part = Part {
        number: number.into(),
        current: current.into(),
        voltage: voltage.into(),
        functions: vec![FunctionTemplate::new(kind, pins)],
        ..Default::default()
    };
    prj.devices.entry(tag.into()).or_default().parts.push(PartRef { number: number.into(), count: 1 });
    prj.parts.push(part);
}

/// L1 → -F1 (1/2) → -K1 main contact (1/2) → lamp -P1.
fn circuit(cs_feed: &str, cs_load: &str, breaker: &str) -> (Project, u64, u64) {
    let mut prj = Project::default();
    sym(&mut prj, "pot_l1", 100.0, 76.0, "", "L1");
    wire(&mut prj, 100.0, 80.0, 100.0, 96.0, "2.5 mm²");
    sym(&mut prj, "cb1", 100.0, 100.0, "-F1", "");
    let feed = wire(&mut prj, 100.0, 104.0, 100.0, 116.0, cs_feed);
    sym(&mut prj, "main1", 100.0, 120.0, "-K1", "");
    let load = wire(&mut prj, 100.0, 124.0, 100.0, 136.0, cs_load);
    sym(&mut prj, "lamp", 100.0, 140.0, "-P1", "");
    assign(&mut prj, "-F1", "ACME.MCB", FnKind::Breaker1, "1 2", breaker, "");
    (prj, feed, load)
}

#[test]
fn rated_currents_and_voltages_are_read_from_the_parts() {
    assert_eq!(parse_current("16 A"), Some(16.0));
    assert_eq!(parse_current("2.5 - 4 A"), Some(4.0));
    assert_eq!(parse_current("9 A (AC-3)"), Some(9.0));
    assert_eq!(parse_current("6,3A"), Some(6.3));
    assert_eq!(parse_current("0.75 kW"), None);
    assert_eq!(parse_voltage("400 V AC"), Some(400.0));
    assert_eq!(parse_voltage("230 V AC / 24 V DC"), Some(230.0));
    assert_eq!(parse_voltage("0.75 kW"), None);
}

#[test]
fn ampacity_table() {
    assert_eq!(ampacity(1.5), Some(13.5));
    assert_eq!(ampacity(2.5), Some(18.3));
    assert_eq!(ampacity(3.0), Some(18.3));
    assert!((ampacity(0.5).unwrap() - 7.6 * 0.5 / 0.75).abs() < 1e-4);
    assert_eq!(ampacity(50.0), Some(117.0));
    assert_eq!(ampacity(150.0), None);
}

#[test]
fn wires_behind_a_breaker_and_its_contacts_must_carry_its_current() {
    let (prj, _, load) = circuit("2.5 mm²", "1.5 mm²", "16 A");
    let nl = Netlist::build(&prj, true);
    let open = Netlist::build(&prj, false);
    let o = overloads(&prj, &nl, &open);
    assert_eq!(o.len(), 1, "{o:?}");
    assert_eq!((o[0].device.as_str(), o[0].rated, o[0].ampacity, o[0].target.1), ("-F1", 16.0, 13.5, load));
    let an = Analysis::checked(&prj);
    assert!(an.issues.iter().any(|i| i.severity == Severity::Error && i.message() == "Cross-section 1.5 mm² carries 13.5 A, less than the 16 A of -F1"));
    // A smaller breaker protects them.
    let (prj, _, _) = circuit("2.5 mm²", "1.5 mm²", "10 A");
    assert!(overloads(&prj, &Netlist::build(&prj, true), &Netlist::build(&prj, false)).is_empty());
}

#[test]
fn the_supply_side_and_other_protective_devices_end_the_protected_circuit() {
    let (mut prj, _, _) = circuit("2.5 mm²", "2.5 mm²", "16 A");
    // The feed of the breaker (line side) is thin but not protected by it.
    prj.pages[0].wires[0].cross_section = "0.75 mm²".into();
    // A second, smaller breaker behind the lamp branch: its load side is its own.
    sym(&mut prj, "cb1", 140.0, 120.0, "-F2", "");
    wire(&mut prj, 100.0, 110.0, 140.0, 110.0, "2.5 mm²");
    wire(&mut prj, 140.0, 110.0, 140.0, 116.0, "2.5 mm²");
    wire(&mut prj, 140.0, 124.0, 140.0, 136.0, "0.75 mm²");
    assign(&mut prj, "-F2", "ACME.MCB2", FnKind::Breaker1, "1 2", "6 A", "");
    let o = overloads(&prj, &Netlist::build(&prj, true), &Netlist::build(&prj, false));
    assert!(o.is_empty(), "{o:?}");
}

#[test]
fn voltage_drop_of_a_motor_cable() {
    let mut prj = Project::new_default();
    for (i, x) in [100.0, 108.0, 116.0].into_iter().enumerate() {
        sym(&mut prj, "term", x, 100.0, "-X1", &(i + 1).to_string());
        wire(&mut prj, x, 104.0, x, 196.0, "");
        prj.pages[0].wires.last_mut().unwrap().cable = "-W1".into();
        prj.pages[0].wires.last_mut().unwrap().core = ["BN", "BK", "GY"][i].into();
    }
    sym(&mut prj, "motor3", 100.0, 200.0, "-M1", "");
    assign(&mut prj, "-M1", "ACME.MOT", FnKind::Motor3, "U1 V1 W1 PE", "1.9 A", "400 V");
    prj.cables.push(Cable { id: 900, tag: "-W1".into(), cores: "BN BK GY GNYE".into(), cross_section: "1.5 mm²".into(), length: 50.0, ..Default::default() });
    let an = Analysis::checked(&prj);
    let d = cable_drops(&prj, &an.connections);
    assert_eq!(d.len(), 1);
    let d = &d[0];
    assert_eq!((d.load.as_str(), d.current, d.voltage, d.three_phase), ("-M1", Some(1.9), Some(400.0), true));
    let du = 3f32.sqrt() * 1.9 * 50.0 * 0.0225 / 1.5;
    let (v, pct) = d.drop.unwrap();
    assert!((v - du).abs() < 1e-3 && (pct - du / 4.0).abs() < 1e-3);
    assert_eq!(d.ampacity, Some(13.5));
    assert!(!an.issues.iter().any(|i| i.message().contains("voltage drop")));
    assert_eq!(an.report(&prj, ReportKind::CableSizing).unwrap().rows.len(), 1);

    // 500 m of 1.5 mm² is too much.
    prj.cables[0].length = 500.0;
    let an = Analysis::checked(&prj);
    assert!(an.issues.iter().any(|i| i.severity == Severity::Warning && i.message().starts_with("-W1: voltage drop")));
}
