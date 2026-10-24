use super::*;
use crate::sample::demo_project;

fn row<'a>(t: &'a Table, first: &str) -> Vec<&'a Vec<String>> {
    t.rows.iter().filter(|r| r[0] == first).collect()
}

#[test]
fn cable_reports_of_the_demo() {
    let prj = demo_project();
    let an = Analysis::checked(&prj);
    let overview = &an.report(&prj, ReportKind::CableOverview).unwrap();
    let w1 = row(overview, "-W1");
    assert_eq!(w1.len(), 1);
    assert_eq!(w1[0][2], "4/4");
    assert_eq!((w1[0][4].as_str(), w1[0][5].as_str(), w1[0][6].as_str()), ("15", "-X1", "-M1"));

    let diagram = &an.report(&prj, ReportKind::CableDiagram).unwrap();
    let cores: Vec<(&str, &str, &str)> = row(diagram, "-W1").iter().map(|r| (r[1].as_str(), r[2].as_str(), r[3].as_str())).collect();
    assert_eq!(cores, vec![("BN", "-X1:1", "-M1:U1"), ("BK", "-X1:2", "-M1:V1"), ("GY", "-X1:3", "-M1:W1"), ("GNYE", "-X1:PE", "-M1:PE")]);
    assert!(row(diagram, "-W1").iter().all(|r| !r[5].is_empty()), "wire numbers");

    // Connection list: wire numbers and cable cores, terminal sides apart.
    let conns = &an.report(&prj, ReportKind::ConnectionList).unwrap();
    let motor = conns.rows.iter().find(|r| r[3] == "-M1:U1").unwrap();
    assert_eq!((motor[2].as_str(), motor[7].as_str(), motor[8].as_str()), ("-X1:1", "-W1", "BN"));
    assert!(conns.rows.iter().all(|r| !r[0].is_empty()), "every connection is numbered");
    assert!(conns.rows.iter().all(|r| r[2] != r[3]), "no connection from a terminal to itself");

    // The cable is in the bill of materials by length.
    let bom = &an.report(&prj, ReportKind::BillOfMaterials).unwrap();
    let cable = bom.rows.iter().find(|r| r[3] == "GEN.CBL-4G1.5").unwrap();
    assert_eq!((cable[1].as_str(), cable[2].as_str(), cable[7].as_str()), ("15", "m", "-W1"));
}

fn messages(prj: &Project) -> Vec<String> {
    Analysis::checked(prj).issues.into_iter().map(|i| i.message).collect()
}

fn find<'a>(msgs: &'a [String], part: &str) -> Option<&'a String> {
    msgs.iter().find(|m| m.contains(part))
}

#[test]
fn cable_assignments_are_checked() {
    let mut prj = demo_project();
    let page = prj.pages.iter().position(|p| p.name == "2").unwrap();
    let cabled: Vec<usize> = prj.pages[page].wires.iter().enumerate().filter(|(_, w)| w.cable == "-W1").map(|(i, _)| i).collect();
    // Same core on two connections, a core the cable does not have, an unknown cable.
    prj.pages[page].wires[cabled[1]].core = "BN".into();
    prj.pages[page].wires[cabled[2]].core = "RD".into();
    prj.pages[page].wires[cabled[3]].cable = "-W7".into();
    let msgs = messages(&prj);
    assert!(find(&msgs, "-W1: core BN used for 2 connections").is_some(), "{msgs:#?}");
    assert!(find(&msgs, "core RD is not a core of the cable").is_some(), "{msgs:#?}");
    assert!(find(&msgs, "unknown cable -W7").is_some(), "{msgs:#?}");
    prj.cables.push(prj.cables[0].clone());
    assert!(find(&messages(&prj), "Cable -W1 defined 2 times").is_some());
}

#[test]
fn wire_numbers_and_tags_are_checked() {
    let mut prj = demo_project();
    let open = Netlist::build(&prj, false);
    let bridged = Netlist::build(&prj, true);
    let numbered: Vec<&crate::netlist::Net> = open.nets.iter().filter(|n| n.wires.len() > 1 && !n.pins.is_empty()).collect();
    // Two numbers in one connection
    let (pi, id) = numbered[0].wires[0];
    prj.pages[pi].wires.iter_mut().find(|w| w.id == id).unwrap().number = "X99".into();
    assert!(find(&messages(&prj), "Different wire numbers in one connection").is_some());
    // The same number on two potentials
    let pot = |n: &crate::netlist::Net| {
        let p = n.pins[0];
        bridged.pin_net[&(prj.pages[p.page].symbols[p.sym].id, p.pin)]
    };
    let other = open.nets.iter().find(|n| !n.wires.is_empty() && !n.pins.is_empty() && pot(n) != pot(numbered[1])).unwrap().clone();
    let number = crate::wirenum::number_of(&prj, numbered[1]);
    crate::wirenum::set(&mut prj, &other, &number, true);
    assert!(find(&messages(&prj), &format!("Wire number {number} is used for 2 different potentials")).is_some());
    // A tag without device name
    prj.pages[2].symbols[0].tag = "=A1".into();
    assert!(find(&messages(&prj), "=A1: device tag without device name").is_some());
}

#[test]
fn mounting_pages_without_enclosure_are_reported() {
    let mut prj = demo_project();
    assert!(find(&messages(&prj), "no longer exists").is_none());
    prj.layout.enclosures.clear();
    let msgs = messages(&prj);
    let page = prj.pages.iter().find(|p| matches!(p.kind, PageKind::Mounting(_))).unwrap();
    assert_eq!(find(&msgs, "no longer exists").unwrap(), &format!("Page {}: its enclosure no longer exists", page.name));
}

