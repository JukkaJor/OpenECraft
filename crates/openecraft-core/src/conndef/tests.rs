use super::*;
use crate::geom::p;

/// Two connections: wires 10+11 (with a corner) and wire 12.
fn project() -> Project {
    let mut prj = Project::default();
    let pg = &mut prj.pages[0];
    pg.wires.push(Wire { id: 10, a: p(20.0, 20.0), b: p(60.0, 20.0), ..Default::default() });
    pg.wires.push(Wire { id: 11, a: p(60.0, 20.0), b: p(60.0, 60.0), ..Default::default() });
    pg.wires.push(Wire { id: 12, a: p(100.0, 20.0), b: p(140.0, 20.0), ..Default::default() });
    prj.cables.push(Cable { id: 50, tag: "-W1".into(), cores: "1 2 3".into(), ..Default::default() });
    prj.fix_next_id();
    prj
}

#[test]
fn applies_to_the_whole_connection() {
    let mut prj = project();
    prj.pages[0].conn_defs.push(ConnDef { id: 1, pos: p(40.0, 20.0), number: "W7".into(), color: "BU".into(), cross_section: "1.5 mm²".into(), cable: "-W1".into(), core: "2".into(), ..Default::default() });
    let open = Netlist::build(&prj, false);
    assert!(problems(&prj, &open).iter().any(|i| i.severity == Severity::Info));
    assert_eq!(apply(&mut prj, &open, 0, 1), 2);
    let w = &prj.pages[0].wires;
    assert_eq!((w[1].number.as_str(), w[1].color.as_str(), w[1].number_fixed), ("W7", "BU", true));
    // The cable core goes to the wire under the point only.
    assert_eq!((w[0].cable.as_str(), w[0].core.as_str()), ("-W1", "2"));
    assert!(w[1].cable.is_empty());
    // The other connection is untouched.
    assert!(w[2].number.is_empty() && w[2].color.is_empty());
    let open = Netlist::build(&prj, false);
    assert!(problems(&prj, &open).is_empty());
}

#[test]
fn adopt_takes_the_current_values() {
    let mut prj = project();
    prj.pages[0].wires[1].color = "RD".into();
    prj.pages[0].wires[0].number = "5".into();
    let open = Netlist::build(&prj, false);
    let cd = adopt(&prj, &open, 0, p(60.0, 40.0));
    assert_eq!((cd.number.as_str(), cd.color.as_str()), ("5", "RD"));
}

#[test]
fn problems_beside_wire_and_contradicting() {
    let mut prj = project();
    prj.pages[0].conn_defs.push(ConnDef { id: 1, pos: p(40.0, 30.0), ..Default::default() });
    prj.pages[0].conn_defs.push(ConnDef { id: 2, pos: p(40.0, 20.0), color: "BU".into(), ..Default::default() });
    prj.pages[0].conn_defs.push(ConnDef { id: 3, pos: p(60.0, 40.0), color: "BK".into(), ..Default::default() });
    assert_eq!(apply_all(&mut prj), 2);
    let open = Netlist::build(&prj, false);
    let issues = problems(&prj, &open);
    assert!(issues.iter().any(|i| i.target == Some((0, 1)) && i.severity == Severity::Warning));
    assert!(issues.iter().any(|i| i.target == Some((0, 3)) && i.severity == Severity::Warning));
}

#[test]
fn label_lists_the_values() {
    let cd = ConnDef { number: "12".into(), cross_section: "2.5 mm²".into(), cable: "-W1".into(), core: "BN".into(), ..Default::default() };
    assert_eq!(label(&cd), "12  2.5 mm²  -W1:BN");
}
