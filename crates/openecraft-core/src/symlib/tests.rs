use super::*;
use crate::index::Index;
use crate::model::{Page, PageKind, SymbolInst};
use crate::netlist::Netlist;
use crate::reports::Analysis;

fn box_symbol(key: &str) -> UserSymbol {
    UserSymbol {
        key: key.into(),
        name: "Test box".into(),
        letter: "A".into(),
        prims: vec![Prim::Poly { pts: vec![p(-4.0, -4.0), p(4.0, -4.0), p(4.0, 4.0), p(-4.0, 4.0)], closed: true, fill: false }],
        pins: vec![UserPin { pos: p(0.0, -8.0), label: "1".into() }, UserPin { pos: p(0.0, 8.0), label: "2".into() }],
        ..Default::default()
    }
}

#[test]
fn registered_symbols_are_found_and_updated() {
    let mut u = box_symbol("user:test_registry");
    register(std::slice::from_ref(&u));
    let def = symbols::get("user:test_registry").expect("registered");
    assert_eq!(def.category, Category::User);
    assert_eq!(def.pins.len(), 2);
    assert_eq!(def.library, "User");
    // Unchanged symbols keep their definition, changed ones get a new one.
    register(std::slice::from_ref(&u));
    assert!(std::ptr::eq(def, symbols::get("user:test_registry").unwrap()));
    u.pins.push(UserPin { pos: p(8.0, 8.0), label: "3".into() });
    register(std::slice::from_ref(&u));
    assert_eq!(symbols::get("user:test_registry").unwrap().pins.len(), 3);
    // Keys without the prefix never reach the user registry.
    assert!(lookup("coil").is_none());
}

#[test]
fn keys_are_unique_slugs() {
    let mut list = vec![];
    let k = new_key(&list, "Motor starter (3~)");
    assert_eq!(k, "user:motor_starter_3");
    list.push(UserSymbol { key: k.clone(), ..Default::default() });
    assert_eq!(new_key(&list, "Motor starter (3~)"), "user:motor_starter_3_2");
    assert_eq!(new_key(&list, "äöü"), "user:symbol");
}

#[test]
fn copy_of_builtin_keeps_function_and_pins() {
    let coil = symbols::get("coil").unwrap();
    let u = from_def(coil, &[]);
    assert!(u.key.starts_with(PREFIX));
    assert_eq!(u.func, coil.func);
    assert_eq!(u.pins.iter().map(|x| x.label.as_str()).collect::<Vec<_>>(), vec!["A1", "A2"]);
    assert_eq!(u.prims, coil.prims);
    assert!(problems(&u).is_empty());
}

#[test]
fn problems_are_reported() {
    let mut u = box_symbol("user:p");
    assert!(problems(&u).is_empty());
    u.pins[1].pos = p(1.0, 8.0);
    u.pins.push(UserPin { pos: p(0.0, -8.0), label: "3".into() });
    let msgs = problems(&u);
    assert!(msgs.iter().any(|m| m.contains("not on the")), "{msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("lies on another")), "{msgs:?}");
    u.pins.clear();
    assert!(problems(&u).iter().any(|m| m.contains("no connection points")));
}

#[test]
fn hit_test_prefers_pins_and_moves_elements() {
    let mut u = box_symbol("user:h");
    u.prims.push(Prim::Line { a: p(0.0, -8.0), b: p(0.0, -4.0), dashed: false });
    // The pin lies on the line: the pin wins.
    assert_eq!(hit(&u, p(0.1, -8.0), 0.5), Some(Hit::Pin(0)));
    assert_eq!(hit(&u, p(0.1, -6.0), 0.5), Some(Hit::Prim(1)));
    assert_eq!(hit(&u, p(4.2, 0.0), 0.5), Some(Hit::Prim(0)));
    assert_eq!(hit(&u, p(20.0, 20.0), 0.5), None);
    translate(&mut u.prims[1], p(2.0, 0.0));
    assert_eq!(u.prims[1], Prim::Line { a: p(2.0, -8.0), b: p(2.0, -4.0), dashed: false });
    let text = Prim::Text { pos: p(0.0, 0.0), text: "ABC".into(), size: 2.0, anchor: Anchor::Left };
    assert_eq!(prim_distance(&text, p(1.0, 0.0)), 0.0);
    assert!(prim_distance(&text, p(-3.0, 0.0)) > 2.9);
}

#[test]
fn removing_a_pin_fixes_bridges() {
    let mut u = box_symbol("user:b");
    u.pins.push(UserPin { pos: p(8.0, 8.0), label: "3".into() });
    u.bridges = vec![(0, 1), (1, 2)];
    remove_pin(&mut u, 0);
    assert_eq!(u.pins.len(), 2);
    assert_eq!(u.bridges, vec![(0, 1)]);
    assert_eq!(next_pin_label(&u), "4");
}

#[test]
fn merge_adds_and_replaces() {
    let mut list = vec![box_symbol("user:a")];
    let mut changed = box_symbol("user:a");
    changed.name = "Changed".into();
    let foreign = UserSymbol { key: "coil".into(), name: "Foreign".into(), ..Default::default() };
    let (added, replaced) = merge(&mut list, SymbolLibrary { symbols: vec![changed, foreign] });
    assert_eq!((added, replaced), (1, 1));
    assert_eq!(list[0].name, "Changed");
    // Keys of other programs are made user keys (no clash with built-in symbols).
    assert_eq!(list[1].key, "user:foreign");
}

#[test]
fn placed_user_symbols_work_like_built_in_ones() {
    let mut prj = Project { pages: vec![Page::new(1, "1", "", PageKind::Schematic)], next_id: 10, ..Default::default() };
    prj.symbols.push(box_symbol("user:test_placed"));
    prj.pages[0].symbols.push(SymbolInst {
        id: 2,
        sym: "user:test_placed".into(),
        pos: p(40.0, 40.0),
        tag: "-A1".into(),
        ..Default::default()
    });
    // Saved and loaded: the symbol travels with the project.
    let json = serde_json::to_string(&prj).unwrap();
    let mut back: Project = serde_json::from_str(&json).unwrap();
    back.normalize();
    assert_eq!(back.pages[0].symbols[0].pins, vec!["1", "2"]);
    let idx = Index::build(&back);
    assert_eq!(idx.devices["-A1"].mains.len(), 1);
    let nl = Netlist::build(&back, true);
    assert_eq!(nl.unconnected.len(), 2);
    let an = Analysis::build(&back);
    assert!(!an.issues.iter().any(|i| i.message.contains("Unknown symbol")), "{:?}", an.issues.iter().map(|i| &i.message).collect::<Vec<_>>());
}

#[test]
fn library_files_hold_one_or_many_symbols() {
    let u = UserSymbol { key: "u_lamp".into(), name: "Lamp".into(), ..Default::default() };
    let lib = SymbolLibrary { symbols: vec![u.clone()] };
    assert_eq!(SymbolLibrary::from_json(&serde_json::to_string(&lib).unwrap()).unwrap().symbols, lib.symbols);
    assert_eq!(SymbolLibrary::from_json(&serde_json::to_string(&u).unwrap()).unwrap().symbols, lib.symbols);
    assert!(SymbolLibrary::from_json("42").is_err());
}
