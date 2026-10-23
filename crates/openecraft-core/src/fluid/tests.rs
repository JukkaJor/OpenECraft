use super::*;
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
    let msgs: Vec<String> = problems(&prj, &nl).into_iter().map(|x| x.0).collect();
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
    assert!(problems(&prj, &nl).iter().any(|(m, error, _)| *error && m.contains("-QM1:5")));
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
