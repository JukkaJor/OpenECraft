use super::*;

fn inst(id: u64, key: &str, x: f32, y: f32, tag: &str) -> SymbolInst {
    let def = crate::symbols::builtin(key).unwrap();
    SymbolInst { id, sym: key.into(), pos: p(x, y), tag: tag.into(), pins: def.pins.iter().map(|x| x.label.to_string()).collect(), ..Default::default() }
}

fn wire(id: u64, a: P, b: P) -> Wire {
    Wire { id, a, b, ..Default::default() }
}

/// Single-line diagram -Q1 → -K1 → -M1 on an overview page; the motor is shifted to
/// the right so its conductors need a step.
fn single_line_project() -> Project {
    let mut prj = Project { pages: vec![Page::new(1, "1", "Feeder", PageKind::Overview)], next_id: 100, ..Default::default() };
    let pg = &mut prj.pages[0];
    pg.symbols.push(inst(2, "sl_cb", 40.0, 40.0, "-Q1"));
    pg.symbols.push(inst(3, "sl_contactor", 40.0, 64.0, "-K1"));
    pg.symbols.push(inst(4, "sl_motor", 60.0, 96.0, "-M1"));
    pg.wires.push(wire(5, p(40.0, 48.0), p(40.0, 56.0)));
    pg.wires.push(wire(6, p(40.0, 72.0), p(40.0, 80.0)));
    pg.wires.push(wire(7, p(40.0, 80.0), p(60.0, 80.0)));
    pg.wires.push(wire(8, p(60.0, 80.0), p(60.0, 88.0)));
    prj
}

/// Net of a pin of the symbol of `key` with `tag`, found by its designation.
fn net(prj: &Project, nl: &Netlist, tag: &str, key: &str, pin: &str) -> usize {
    let s = prj.pages.iter().flat_map(|pg| &pg.symbols).find(|s| s.tag == tag && s.sym == key).unwrap();
    let i = s.pins.iter().position(|x| x == pin).unwrap();
    nl.pin_net[&(s.id, i)]
}

#[test]
fn every_single_line_symbol_maps_onto_a_power_symbol_and_back() {
    for (single, multi, extra) in MAPPING {
        assert_eq!(crate::symbols::builtin(single).unwrap().category, Category::SingleLine, "{single}");
        assert!(crate::symbols::builtin(multi).is_some(), "{multi}");
        assert!(extra.is_none_or(|e| crate::symbols::builtin(e).is_some()));
        assert_eq!(single_of(multi), Some(single));
        assert_eq!(multi_of(single), Some(multi));
    }
    assert_eq!(single_of("cb4"), Some("sl_cb"));
    assert_eq!(single_of("coil"), None);
}

#[test]
fn single_line_symbols_on_overview_pages_belong_to_the_devices() {
    let mut prj = single_line_project();
    prj.pages[0].symbols.push(inst(9, "cb3", 120.0, 40.0, "-Q9"));
    let idx = Index::build(&prj);
    assert_eq!(idx.devices["-Q1"].overviews.len(), 1);
    // Multi-line symbols on overview pages are drawings only.
    assert!(!idx.devices.contains_key("-Q9"));
    let statuses: Vec<Status> = devices(&prj, &idx).iter().map(|d| d.status).collect();
    assert_eq!(statuses, [Status::Planned; 3]);
    let links = single_line_links(&prj);
    let pairs: Vec<(&str, &str)> = links.iter().map(|(a, b, _)| (a.as_str(), b.as_str())).collect();
    assert_eq!(pairs, [("-Q1", "-K1"), ("-K1", "-M1")]);
}

#[test]
fn the_schematic_is_generated_from_the_single_line_diagram() {
    let mut prj = single_line_project();
    prj.pages[0].symbols[1].function_text = "Pump".into();
    let g = to_multiline(&mut prj, 0).unwrap();
    assert_eq!(g.page, 1);
    assert_eq!(prj.pages[1].kind, PageKind::Schematic);
    // Breaker, main contacts, motor and the contactor coil.
    assert_eq!(g.symbols, 4);
    assert!(g.skipped.is_empty());
    let idx = Index::build(&prj);
    assert_eq!(prj.pages[1].symbols.iter().find(|s| s.sym == "main3").unwrap().function_text, "Pump");
    assert_eq!(idx.devices["-K1"].mains.len(), 1);
    assert_eq!(idx.devices["-K1"].auxs.len(), 1);
    assert!(devices(&prj, &idx).iter().all(|d| d.status == Status::Engineered));

    let nl = Netlist::build(&prj, true);
    for (q, k) in [("2", "1"), ("4", "3"), ("6", "5")] {
        assert_eq!(net(&prj, &nl, "-Q1", "cb3", q), net(&prj, &nl, "-K1", "main3", k));
    }
    for (k, m) in [("2", "U1"), ("4", "V1"), ("6", "W1")] {
        assert_eq!(net(&prj, &nl, "-K1", "main3", k), net(&prj, &nl, "-M1", "motor3", m));
    }
    let phases: BTreeSet<usize> = ["2", "4", "6"].iter().map(|k| net(&prj, &nl, "-K1", "main3", k)).collect();
    assert_eq!(phases.len(), 3, "no short between the stepped conductors");
    assert!(problems(&prj, &idx, &nl).iter().all(|(s, ..)| *s == Severity::Info));

    // Engineered devices are not generated twice.
    let again = to_multiline(&mut prj, 0).unwrap_err();
    assert!(again.contains("already"), "{again}");
}

#[test]
fn the_single_line_diagram_is_generated_from_the_schematic() {
    let mut prj = Project { pages: vec![Page::new(1, "1", "", PageKind::Schematic)], next_id: 100, ..Default::default() };
    let pg = &mut prj.pages[0];
    pg.symbols.push(inst(2, "cb3", 40.0, 40.0, "-Q1"));
    pg.symbols.push(inst(3, "main3", 40.0, 60.0, "-K1"));
    pg.symbols.push(inst(4, "coil", 120.0, 60.0, "-K1"));
    pg.symbols.push(inst(5, "motor3", 40.0, 80.0, "-M1"));
    pg.symbols.push(inst(6, "motor3", 80.0, 80.0, "-M2"));
    pg.symbols[2].function_text = "Pump".into();
    let mut id = 10;
    for x in [40.0, 48.0, 56.0] {
        for (a, b) in [(44.0, 56.0), (64.0, 76.0)] {
            pg.wires.push(wire(id, p(x, a), p(x, b)));
            id += 1;
        }
        // -M2 is fed by -Q1 as well.
        pg.wires.push(wire(id, p(x, 44.0), p(x + 40.0, 44.0)));
        pg.wires.push(wire(id + 1, p(x + 40.0, 44.0), p(x + 40.0, 76.0)));
        id += 2;
    }
    let g = to_singleline(&mut prj, 0).unwrap();
    assert_eq!(g.page, 1);
    assert_eq!(prj.pages[1].kind, PageKind::Overview);
    assert_eq!(g.symbols, 4);
    let sl = &prj.pages[1];
    let k1 = sl.symbols.iter().find(|s| s.tag == "-K1").unwrap();
    assert_eq!((k1.sym.as_str(), k1.function_text.as_str()), ("sl_contactor", "Pump"));

    let mut links: Vec<(String, String)> = single_line_links(&prj).into_iter().map(|(a, b, _)| if a < b { (a, b) } else { (b, a) }).collect();
    links.sort();
    let links: Vec<(&str, &str)> = links.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    // -K1 and -M2 hang on the same node below -Q1.
    assert_eq!(links, [("-K1", "-M1"), ("-K1", "-M2"), ("-K1", "-Q1"), ("-M2", "-Q1")]);
    let idx = Index::build(&prj);
    assert!(devices(&prj, &idx).iter().all(|d| d.status == Status::Engineered));
    assert!(problems(&prj, &idx, &Netlist::build(&prj, true)).is_empty());
    assert!(to_singleline(&mut prj, 0).is_err());
}

#[test]
fn differences_between_both_representations_are_reported() {
    let mut prj = single_line_project();
    to_multiline(&mut prj, 0).unwrap();
    // A fuse in the single-line diagram, but a breaker in the schematic.
    prj.pages[0].symbols[0].sym = "sl_fuse".into();
    // The motor is no longer connected in the schematic.
    let motor = prj.pages[1].symbols.iter().find(|s| s.tag == "-M1").unwrap().id;
    let def = crate::symbols::builtin("motor3").unwrap();
    let pins: Vec<P> = {
        let s = prj.pages[1].symbols.iter().find(|s| s.id == motor).unwrap();
        (0..def.pins.len()).map(|i| def.pin_world(i, s.pos, s.rot, s.mirror)).collect()
    };
    prj.pages[1].wires.retain(|w| !pins.iter().any(|q| q.same(w.a) || q.same(w.b)));
    prj.pages[0].symbols.push(inst(20, "sl_fuse", 100.0, 40.0, "-F7"));
    let idx = Index::build(&prj);
    let msgs: Vec<String> = problems(&prj, &idx, &Netlist::build(&prj, true)).into_iter().map(|(_, m, _)| m).collect();
    assert!(msgs.iter().any(|m| m.contains("-Q1") && m.contains("does not match")), "{msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("-K1 – -M1")), "{msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("-F7") && m.contains("planned")), "{msgs:?}");
}

#[test]
fn function_texts_are_synchronized() {
    let mut prj = single_line_project();
    to_multiline(&mut prj, 0).unwrap();
    prj.pages[0].symbols[0].function_text = "Infeed".into();
    let coil = prj.pages[1].symbols.iter_mut().find(|s| s.sym == "coil").unwrap();
    coil.function_text = "Pump".into();
    assert_eq!(synchronize(&mut prj), 2);
    assert_eq!(prj.pages[1].symbols.iter().find(|s| s.sym == "cb3").unwrap().function_text, "Infeed");
    assert_eq!(prj.pages[0].symbols[1].function_text, "Pump");
    assert_eq!(synchronize(&mut prj), 0);
}
