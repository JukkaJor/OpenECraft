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

#[test]
fn devices_in_the_cabinet_are_chained_before_field_devices() {
    let prj = crate::sample::demo_project();
    let nl = Netlist::build(&prj, true);
    let open = Netlist::build(&prj, false);
    let pairs: Vec<(String, String)> = connections(&prj, &nl, &open).into_iter().map(|c| (pin_designation(&prj, c.a), pin_designation(&prj, c.b))).collect();
    // 0 V: the two door lamps are bridged in the cabinet, one wire leaves for the valve.
    assert!(pairs.contains(&("-P1:X2".into(), "-P2:X2".into())), "{pairs:?}");
    assert!(pairs.contains(&("-P2:X2".into(), "-Y1:A2".into())));
    assert!(!pairs.iter().any(|(a, b)| a == "-Y1:A2" || (b == "-Y1:A2" && a == "-P1:X2")));
}

/// Pseudo-random wires (straight and diagonal, sharing end points): the row / column
/// shortcut of `junctions` finds the same dots as testing every end against every wire.
#[test]
fn junctions_match_the_exhaustive_count() {
    let mut page = Page::new(1, "1", "", PageKind::Schematic);
    let mut seed = 12345u64;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 33) % 13) as f32 * 4.0
    };
    for id in 1..=120u64 {
        let a = p(next(), next());
        let b = match id % 3 {
            0 => p(a.x, next()),
            1 => p(next(), a.y),
            _ => p(next(), next()),
        };
        page.wires.push(Wire { id, a, b, ..Default::default() });
    }
    let mut expected: Vec<(i64, i64)> = wire_ends(&page)
        .values()
        .filter(|(q, n)| n + 2 * page.wires.iter().filter(|w| on_segment_interior(*q, w.a, w.b)).count() >= 3)
        .map(|(q, _)| q.key())
        .collect();
    let mut got: Vec<(i64, i64)> = junctions(&page).iter().map(|q| q.key()).collect();
    expected.sort();
    got.sort();
    assert!(expected.len() > 5, "{expected:?}");
    assert_eq!(got, expected);
}
