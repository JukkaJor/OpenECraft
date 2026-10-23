use super::*;
use crate::reports::Severity;
use crate::geom::{p, P};
use crate::model::{SymbolInst, Wire};

fn sym(id: u64, key: &str, pos: P, tag: &str) -> SymbolInst {
    let def = crate::symbols::get(key).unwrap();
    SymbolInst { id, sym: key.into(), pos, tag: tag.into(), pins: def.pins.iter().map(|x| x.label.to_string()).collect(), ..Default::default() }
}

fn wire(id: u64, a: P, b: P) -> Wire {
    Wire { id, a, b, ..Default::default() }
}

/// Air supply → 5/2 valve → double-acting cylinder, and a coil with a 24 V supply.
fn pneumatics() -> Project {
    let mut prj = Project { next_id: 100, ..Default::default() };
    let pg = &mut prj.pages[0];
    pg.symbols.push(sym(1, "fl_valve52", p(100.0, 100.0), "-QM1"));
    pg.symbols.push(sym(2, "fl_source", p(104.0, 120.0), "-GQ1"));
    pg.symbols.push(sym(3, "fl_cylinder", p(104.0, 60.0), "-MM1"));
    pg.symbols.push(sym(4, "coil", p(200.0, 100.0), "-MB1"));
    pg.symbols.push(SymbolInst { label: "+24V".into(), ..sym(5, "pot_24", p(200.0, 40.0), "") });
    // Supply to port 1, port 4 to cylinder port 1, port 2 to cylinder port 2.
    pg.wires.push(wire(10, p(104.0, 108.0), p(104.0, 124.0)));
    pg.wires.push(wire(11, p(104.0, 92.0), p(104.0, 64.0)));
    pg.wires.push(wire(12, p(108.0, 92.0), p(108.0, 80.0)));
    pg.wires.push(wire(13, p(108.0, 80.0), p(120.0, 80.0)));
    pg.wires.push(wire(14, p(120.0, 80.0), p(120.0, 64.0)));
    pg.wires.push(wire(15, p(200.0, 44.0), p(200.0, 96.0)));
    prj
}

#[test]
fn hoses_form_their_own_nets() {
    let prj = pneumatics();
    let nl = Netlist::build(&prj, false);
    let fluid: Vec<&Net> = nl.nets.iter().filter(|n| n.fluid && !n.wires.is_empty()).collect();
    assert_eq!(fluid.len(), 3);
    assert!(fluid.iter().all(|n| n.name.starts_with('F') && !n.mixed));
    let coil = &nl.nets[nl.pin_net[&(4, 0)]];
    assert!(!coil.fluid);
    assert_eq!(coil.name, "+24V");
    let hoses = hoses(&prj, &nl);
    assert_eq!(hoses.len(), 3);
    assert!(hoses.iter().any(|h| h.from == "-QM1:2" && h.to == "-MM1:2" || h.from == "-MM1:2" && h.to == "-QM1:2"));
    assert!(hoses.iter().all(|h| h.medium == Medium::CompressedAir && h.size == "8 mm" && h.color == "BU"));
    // Electrical connections leave the hoses out.
    let bridged = Netlist::build(&prj, true);
    let conns = crate::netlist::connections(&prj, &bridged, &nl);
    assert!(conns.iter().all(|c| !pin_designation(&prj, c.a).starts_with("-QM1")));
}

#[test]
fn hose_properties_come_from_the_wires() {
    let mut prj = pneumatics();
    let w = prj.pages[0].wires.iter_mut().find(|w| w.id == 13).unwrap();
    w.medium = Some(Medium::Vacuum);
    w.cross_section = "6 mm".into();
    w.length = 1.25;
    let nl = Netlist::build(&prj, false);
    let h = hoses(&prj, &nl).into_iter().find(|h| h.from.contains(":2") && h.to.contains(":2")).unwrap();
    assert_eq!((h.medium, h.size.as_str(), h.length), (Medium::Vacuum, "6 mm", 1.25));
    // Hoses without length are reported, the one with a length is not.
    let msgs: Vec<String> = problems(&prj, &nl).into_iter().map(|x| x.1).filter(|m| m.contains("without length")).collect();
    assert_eq!(msgs.len(), 2, "{msgs:?}");
}

#[test]
fn hoses_are_numbered_apart_from_the_wires() {
    let mut prj = pneumatics();
    let open = Netlist::build(&prj, false);
    let bridged = Netlist::build(&prj, true);
    crate::wirenum::renumber(&mut prj, &open, &bridged, false);
    let numbers = |prj: &Project, ids: &[u64]| -> Vec<String> { ids.iter().map(|id| prj.pages[0].wires.iter().find(|w| w.id == *id).unwrap().number.clone()).collect() };
    let mut hoses = numbers(&prj, &[10, 11, 12]);
    hoses.sort();
    assert_eq!(hoses, ["H1", "H2", "H3"]);
    // All segments of a hose carry its number.
    let h = numbers(&prj, &[12])[0].clone();
    assert_eq!(numbers(&prj, &[13, 14]), [h.clone(), h]);
    assert_eq!(numbers(&prj, &[15]), ["1"]);
}

#[test]
fn fluid_ports_on_electrical_nets_are_errors() {
    let mut prj = pneumatics();
    // Valve port 5 to the coil's A2.
    prj.pages[0].wires.push(wire(16, p(100.0, 108.0), p(200.0, 104.0)));
    let nl = Netlist::build(&prj, false);
    let mixed: Vec<&Net> = nl.nets.iter().filter(|n| n.mixed).collect();
    assert_eq!(mixed.len(), 1);
    assert!(mixed[0].name.starts_with('N'));
    assert!(problems(&prj, &nl).iter().any(|(sev, m, _)| *sev == Severity::Error && m.contains("-QM1:5")));
    assert!(hoses(&prj, &nl).iter().all(|h| !h.from.contains(":5") && !h.to.contains(":5")));
}

#[test]
fn media_codes_round_trip() {
    for m in Medium::ALL {
        assert_eq!(Medium::from_code(m.code()), Some(m));
    }
    assert_eq!(Medium::from_code("Hydraulic oil"), Some(Medium::HydraulicOil));
    assert_eq!(Medium::from_code("lava"), None);
}

fn wire_mut(prj: &mut Project, id: u64) -> &mut Wire {
    prj.pages[0].wires.iter_mut().find(|w| w.id == id).unwrap()
}

fn messages(prj: &Project) -> Vec<(Severity, String)> {
    let nl = Netlist::build(prj, false);
    problems(prj, &nl).into_iter().map(|(s, m, _)| (s, m)).collect()
}

#[test]
fn the_fluid_netlist_classifies_ports_and_lines() {
    let prj = pneumatics();
    let nl = Netlist::build(&prj, false);
    let nets = fluid_nets(&prj, &nl);
    assert_eq!(nets.len(), 3);
    let supply = nets.iter().find(|n| n.ports.iter().any(|p| p.designation == "-GQ1:1")).unwrap();
    assert_eq!(supply.line, LineKind::Pressure);
    let kinds: Vec<(&str, PortKind)> = supply.ports.iter().map(|p| (p.designation.as_str(), p.kind)).collect();
    assert!(kinds.contains(&("-GQ1:1", PortKind::Supply)) && kinds.contains(&("-QM1:1", PortKind::Pressure)), "{kinds:?}");
    let working: Vec<&FluidNet> = nets.iter().filter(|n| n.line == LineKind::Working).collect();
    assert_eq!(working.len(), 2);
    assert!(working.iter().all(|n| n.branches() == 0 && n.medium == Medium::CompressedAir));
    let def = crate::symbols::get("fl_valve42").unwrap();
    assert_eq!([port_kind(def, "T"), port_kind(def, "X"), port_kind(def, "A")], [PortKind::Return, PortKind::Pilot, PortKind::Working]);
}

#[test]
fn pressure_lines_on_the_exhaust_are_errors() {
    let mut prj = pneumatics();
    // Valve exhaust 3 onto the supply line.
    prj.pages[0].wires.push(wire(16, p(108.0, 108.0), p(104.0, 116.0)));
    let msgs = messages(&prj);
    assert!(msgs.iter().any(|(s, m)| *s == Severity::Error && m.contains("pressure line") && m.contains("-QM1:3")), "{msgs:?}");
    let nl = Netlist::build(&prj, false);
    let supply = fluid_nets(&prj, &nl).into_iter().find(|n| n.ports.iter().any(|p| p.designation == "-QM1:3")).unwrap();
    assert_eq!(supply.branches(), 1);
}

#[test]
fn open_exhausts_vent_to_the_atmosphere() {
    let mut prj = pneumatics();
    let check = |prj: &Project| {
        let nl = Netlist::build(prj, true);
        crate::reports::check(prj, &crate::index::Index::build(prj), &nl).into_iter().map(|i| i.message).collect::<Vec<_>>()
    };
    let msgs = messages(&prj);
    assert_eq!(msgs.iter().filter(|(s, m)| *s == Severity::Info && m.contains("open to the atmosphere")).count(), 2, "{msgs:?}");
    assert!(!check(&prj).iter().any(|m| m.contains("-QM1:3")));
    // Hydraulics: an open tank port is an open connection point.
    prj.fluid.medium = Medium::HydraulicOil;
    assert!(!messages(&prj).iter().any(|(_, m)| m.contains("open to the atmosphere")));
    assert!(check(&prj).iter().any(|m| m.contains("-QM1:3")));
}

#[test]
fn media_and_sizes_must_agree() {
    let mut prj = pneumatics();
    wire_mut(&mut prj, 12).medium = Some(Medium::Vacuum);
    wire_mut(&mut prj, 13).medium = Some(Medium::Water);
    wire_mut(&mut prj, 13).cross_section = "6 mm".into();
    wire_mut(&mut prj, 14).cross_section = "8 mm".into();
    let msgs = messages(&prj);
    assert!(msgs.iter().any(|(s, m)| *s == Severity::Warning && m.contains("different media on its wires")), "{msgs:?}");
    assert!(msgs.iter().any(|(_, m)| m.contains("different nominal sizes")));
    // The valve and the cylinder see vacuum on one port and compressed air on the others.
    assert!(msgs.iter().any(|(_, m)| m.starts_with("-QM1: ports with different media")));
    assert!(msgs.iter().any(|(_, m)| m.starts_with("-MM1: ports with different media")));
}

#[test]
fn hose_ratings_are_checked_against_the_operating_pressure() {
    let mut prj = pneumatics();
    prj.fluid.pressure = 6.0;
    wire_mut(&mut prj, 11).hose = Some(HoseConfig { pressure: 4.0, ..Default::default() });
    let rated = |prj: &Project| messages(prj).into_iter().filter(|(s, m)| *s == Severity::Error && m.contains("rated for 4 bar")).count();
    assert_eq!(rated(&prj), 1);
    prj.fluid.pressure = 0.0;
    assert_eq!(rated(&prj), 0);
}

#[test]
fn tubes_are_routed_in_the_panel_layout() {
    use crate::panel::{Enclosure, ItemKind, PanelItem};
    let mut prj = pneumatics();
    let item = |id, tag: &str, x| PanelItem { id, kind: ItemKind::Device, tag: tag.into(), x, y: 100.0, w: 40.0, h: 60.0, d: 50.0, ..Default::default() };
    prj.layout.enclosures.push(Enclosure { id: 50, items: vec![item(51, "-QM1", 100.0), item(52, "-MM1", 300.0)], ..Default::default() });
    let nl = Netlist::build(&prj, false);
    let hs = hoses(&prj, &nl);
    let to_cylinder: Vec<&Hose> = hs.iter().filter(|h| h.from.starts_with("-QM1") && h.to.starts_with("-MM1") || h.to.starts_with("-QM1") && h.from.starts_with("-MM1")).collect();
    assert_eq!(to_cylinder.len(), 2);
    let r = to_cylinder[0].routed.as_ref().expect("routed");
    // 200 mm across the plate, 2 × 50 mm to the plate and 100 mm reserve.
    assert!((r.length - 0.4).abs() < 1e-4, "{r:?}");
    assert_eq!(to_cylinder[0].effective_length(), r.length);
    // The supply is not placed: not routed, reported without length.
    assert!(hs.iter().any(|h| h.from.starts_with("-GQ1") || h.to.starts_with("-GQ1")));
    assert!(hs.iter().filter(|h| h.routed.is_none()).all(|h| h.effective_length() == 0.0));
    // A set length shorter than the route is a warning.
    wire_mut(&mut prj, 11).length = 0.3;
    assert!(messages(&prj).iter().any(|(s, m)| *s == Severity::Warning && m.contains("shorter than the routed 0.40 m")));
}

#[test]
fn tubing_adds_up_cut_lengths_and_fittings() {
    let mut prj = pneumatics();
    prj.fluid.allowance = 10.0;
    for w in &mut prj.pages[0].wires {
        w.length = 1.0;
        w.hose = Some(HoseConfig { line: LineType::Tube, part: "PUN-8".into(), fitting_a: "QS-8".into(), fitting_b: "QSL-8".into(), ..Default::default() });
    }
    let nl = Netlist::build(&prj, false);
    let hs = hoses(&prj, &nl);
    assert!((hs[0].cut_length(prj.fluid.allowance) - 1.02).abs() < 1e-5);
    let t = tubing(&prj, &hs);
    assert_eq!(t.len(), 1);
    assert_eq!((t[0].line, t[0].part.as_str(), t[0].pieces), (LineType::Tube, "PUN-8", 3));
    assert!((t[0].length - 3.06).abs() < 1e-4);
    assert_eq!(t[0].fittings.get("QS-8"), Some(&3));
    assert_eq!(t[0].fittings.get("QSL-8"), Some(&3));
    // The configuration is stored with the project.
    let back = Project::from_json(&prj.to_json()).unwrap();
    assert_eq!(back.pages[0].wires[0].hose, prj.pages[0].wires[0].hose);
    assert_eq!(LineType::from_code("pipe"), Some(LineType::Pipe));
}
