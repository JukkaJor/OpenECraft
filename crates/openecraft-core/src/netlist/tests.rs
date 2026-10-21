use super::*;
use crate::geom::p;
use crate::model::{SymbolInst, Wire};

fn sym(id: u64, key: &str, pos: P, tag: &str, label: &str) -> SymbolInst {
    let def = symbols::get(key).unwrap();
    SymbolInst {
        id,
        sym: key.into(),
        pos,
        tag: tag.into(),
        label: label.into(),
        pins: def.pins.iter().map(|x| x.label.to_string()).collect(),
        ..Default::default()
    }
}

#[test]
fn wire_connects_pins_and_potential_names_net() {
    let mut prj = Project::default();
    let page = &mut prj.pages[0];
    page.symbols.push(sym(10, "pot_24", p(100.0, 40.0), "", "+24V"));
    page.symbols.push(sym(11, "coil", p(100.0, 100.0), "-K1", ""));
    // potential pin at (100,44), coil A1 at (100,96)
    page.wires.push(Wire { id: 20, a: p(100.0, 44.0), b: p(100.0, 96.0), ..Default::default() });
    let nl = Netlist::build(&prj, true);
    let n1 = nl.pin_net[&(11, 0)];
    assert_eq!(nl.nets[n1].name, "+24V");
    assert_eq!(nl.wire_net[&20], n1);
    // A2 is unconnected
    assert!(nl.unconnected.iter().any(|&(_, id, pin)| id == 11 && pin == 1));
}

#[test]
fn t_junction_connects() {
    let mut prj = Project::default();
    let page = &mut prj.pages[0];
    page.wires.push(Wire { id: 1, a: p(0.0, 0.0), b: p(40.0, 0.0), ..Default::default() });
    page.wires.push(Wire { id: 2, a: p(20.0, 0.0), b: p(20.0, 20.0), ..Default::default() });
    let nl = Netlist::build(&prj, true);
    assert_eq!(nl.wire_net[&1], nl.wire_net[&2]);
    assert_eq!(junctions(&prj.pages[0]).len(), 1);
}
