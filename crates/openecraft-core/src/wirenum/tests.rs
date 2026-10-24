use super::*;
use crate::sample::demo_project;
use crate::symbols::{Func};

fn nets(prj: &Project) -> (Netlist, Netlist) {
    (Netlist::build(prj, false), Netlist::build(prj, true))
}

fn numbered(open: &Netlist) -> Vec<&Net> {
    open.nets.iter().filter(|n| !n.wires.is_empty() && !n.pins.is_empty()).collect()
}

/// Number of the connection at a pin of the first symbol matching `pred`.
fn number_at(prj: &Project, pred: impl Fn(&SymbolInst) -> bool, pin: usize) -> String {
    let open = Netlist::build(prj, false);
    let s = prj.pages.iter().flat_map(|p| p.symbols.iter()).find(|s| pred(s)).expect("symbol");
    number_of(prj, &open.nets[open.pin_net[&(s.id, pin)]])
}

fn is_terminal(s: &SymbolInst, label: &str) -> bool {
    crate::symbols::builtin(&s.sym).map(|d| d.func) == Some(Func::Terminal) && s.label == label
}

#[test]
fn sequential_numbers_are_unique_per_connection() {
    let mut prj = demo_project();
    let (open, bridged) = nets(&prj);
    let n = renumber(&mut prj, &open, &bridged, false);
    assert_eq!(n, numbered(&open).len());
    let mut seen = HashSet::new();
    for net in numbered(&open) {
        let numbers = numbers_of(&prj, net);
        assert_eq!(numbers.len(), 1, "one number for all wires of a connection");
        assert!(seen.insert(numbers[0].clone()), "{} used twice", numbers[0]);
    }
    assert!(seen.contains("1"));
    // Both sides of a terminal are separate connections.
    let top = number_at(&prj, |s| is_terminal(s, "1"), 0);
    let bottom = number_at(&prj, |s| is_terminal(s, "1"), 1);
    assert!(!top.is_empty() && top != bottom);
}

#[test]
fn page_column_and_potential_schemes() {
    let mut prj = demo_project();
    let (open, bridged) = nets(&prj);
    prj.wire_numbering.scheme = NumberScheme::PageColumn;
    renumber(&mut prj, &open, &bridged, false);
    let first = number_at(&prj, |s| s.tag == "-Q1", 0);
    assert!(first.starts_with("2.") && first.ends_with(".1"), "{first}");

    prj.wire_numbering = WireNumbering { scheme: NumberScheme::Potential, prefix: "W".into(), start: 100 };
    renumber(&mut prj, &open, &bridged, false);
    // -Q1 is fed from the L1 rail; the motor side of the terminal is not a named potential.
    assert_eq!(number_at(&prj, |s| s.tag == "-Q1", 0), "L1");
    assert!(number_at(&prj, |s| is_terminal(s, "1"), 1).starts_with('W'));
}

#[test]
fn manual_numbers_are_kept_unless_overwritten() {
    let mut prj = demo_project();
    let (open, bridged) = nets(&prj);
    let target = &open.nets[open.pin_net[&(prj.pages[1].symbols.iter().find(|s| s.tag == "-M1").unwrap().id, 0)]];
    set(&mut prj, target, "MOTOR-U", true);
    renumber(&mut prj, &open, &bridged, false);
    assert_eq!(number_of(&prj, target), "MOTOR-U");
    let automatic = prj.pages.iter().flat_map(|p| &p.wires).filter(|w| !w.number.is_empty() && !w.number_fixed).count();
    assert_eq!(clear(&mut prj, false), automatic);
    assert_eq!(number_of(&prj, target), "MOTOR-U");
    renumber(&mut prj, &open, &bridged, true);
    assert_ne!(number_of(&prj, target), "MOTOR-U");
    assert!(clear(&mut prj, true) > 0);
    assert!(prj.pages.iter().flat_map(|p| &p.wires).all(|w| w.number.is_empty()));
}

#[test]
fn numbers_are_labelled_on_the_longest_wire() {
    let prj = demo_project();
    let open = Netlist::build(&prj, false);
    let net = numbered(&open).into_iter().find(|n| n.wires.len() > 1).unwrap();
    let pi = net.wires[0].0;
    let w = label_wire(&prj.pages[pi], pi, net).unwrap();
    let len = |w: &Wire| (w.b - w.a).len();
    assert!(net.wires.iter().filter(|x| x.0 == pi).all(|(_, id)| len(prj.pages[pi].wires.iter().find(|x| x.id == *id).unwrap()) <= len(w)));
}
