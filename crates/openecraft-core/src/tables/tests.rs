use super::*;
use crate::sample::demo_project;

fn nets(prj: &Project) -> (Netlist, Netlist) {
    (Netlist::build(prj, true), Netlist::build(prj, false))
}

fn table(prj: &Project, kind: TableKind) -> Vec<Row> {
    let (nl, open) = nets(prj);
    rows(prj, &nl, &open, kind)
}

#[test]
fn devices_are_renamed_described_and_get_parts() {
    let mut prj = demo_project();
    let rows = table(&prj, TableKind::Devices);
    let k1 = rows.iter().find(|r| r.key == "-K1").unwrap();
    assert_eq!(k1.cells[1], "Conveyor motor");
    let (_, open) = nets(&prj);
    assert!(set_cell(&mut prj, &open, TableKind::Devices, "-K1", 2, "Main contactor").unwrap());
    assert!(!set_cell(&mut prj, &open, TableKind::Devices, "-K1", 2, "Main contactor").unwrap());
    assert!(set_cell(&mut prj, &open, TableKind::Devices, "-K1", 3, "GEN.CT-18A-24DC").unwrap());
    assert_eq!(prj.devices["-K1"].parts[0].number, "GEN.CT-18A-24DC");
    assert!(set_cell(&mut prj, &open, TableKind::Devices, "-K1", 1, "Pump\\nmotor").unwrap());
    assert!(set_cell(&mut prj, &open, TableKind::Devices, "-K1", 0, "-K9").unwrap());
    assert!(prj.pages.iter().flat_map(|p| &p.symbols).all(|s| s.tag != "-K1"));
    let k9 = table(&prj, TableKind::Devices).into_iter().find(|r| r.key == "-K9").unwrap();
    assert_eq!((k9.cells[1].as_str(), k9.cells[2].as_str()), ("Pump\nmotor", "Main contactor"));
    // Existing tags and read only columns are refused.
    assert!(set_cell(&mut prj, &open, TableKind::Devices, "-K9", 0, "-Q1").unwrap_err().contains("already exists"));
    assert!(set_cell(&mut prj, &open, TableKind::Devices, "-K9", 4, "1").is_err());
}

#[test]
fn renaming_a_strip_keeps_its_data() {
    let mut prj = demo_project();
    prj.strips.entry("-X1".into()).or_default().spares = 3;
    let (_, open) = nets(&prj);
    set_cell(&mut prj, &open, TableKind::Devices, "-X1", 0, "-X5").unwrap();
    assert_eq!(prj.strips["-X5"].spares, 3);
}

#[test]
fn connections_change_all_their_wires() {
    let mut prj = demo_project();
    let rows = table(&prj, TableKind::Connections);
    let r = rows.iter().find(|r| r.cells[0].starts_with("-K1:14") || r.cells[1].contains("-K1:14")).unwrap().clone();
    let (_, open) = nets(&prj);
    assert!(set_cell(&mut prj, &open, TableKind::Connections, &r.key, 4, "OG").unwrap());
    assert!(set_cell(&mut prj, &open, TableKind::Connections, &r.key, 3, "W77").unwrap());
    let net = &open.nets[open.wire_net[&r.key.parse::<u64>().unwrap()]];
    for w in crate::wirenum::wires_of(&prj, net) {
        assert_eq!((w.color.as_str(), w.number.as_str(), w.number_fixed), ("OG", "W77", true));
    }
    assert!(set_cell(&mut prj, &open, TableKind::Connections, &r.key, 6, "-W99").unwrap_err().contains("Unknown cable"));
}

#[test]
fn released_pages_are_not_changed() {
    let mut prj = demo_project();
    let page = prj.pages.iter().position(|p| p.symbols.iter().any(|s| s.tag == "-K1")).unwrap();
    prj.pages[page].locked = true;
    let rows = table(&prj, TableKind::Devices);
    assert!(rows.iter().find(|r| r.key == "-K1").unwrap().locked);
    let (_, open) = nets(&prj);
    assert!(set_cell(&mut prj, &open, TableKind::Devices, "-K1", 2, "x").is_err());
    let t = table(&prj, TableKind::Terminals).into_iter().find(|r| r.locked);
    if let Some(t) = t {
        assert!(set_cell(&mut prj, &open, TableKind::Terminals, &t.key, 2, "x").is_err());
    }
}

#[test]
fn terminals_are_renumbered_and_moved() {
    let mut prj = demo_project();
    let rows = table(&prj, TableKind::Terminals);
    let t = rows.iter().find(|r| r.cells[0] == "-X1" && r.cells[1] == "1").unwrap();
    let (_, open) = nets(&prj);
    assert!(set_cell(&mut prj, &open, TableKind::Terminals, &t.key, 1, "11").unwrap());
    assert!(set_cell(&mut prj, &open, TableKind::Terminals, &t.key, 0, "-X2").unwrap());
    let moved = table(&prj, TableKind::Terminals).into_iter().find(|r| r.key == t.key).unwrap();
    assert_eq!((moved.cells[0].as_str(), moved.cells[1].as_str()), ("-X2", "11"));
}

#[test]
fn csv_round_trip_applies_only_changed_cells() {
    let mut prj = demo_project();
    let rows = table(&prj, TableKind::Devices);
    let csv = to_csv(TableKind::Devices, &rows);
    assert!(csv.starts_with("Key;Device tag;Function text;Description;Part;Pages\n"));
    let (nl, open) = nets(&prj);
    assert_eq!(apply_csv(&mut prj, &nl, &open, TableKind::Devices, &csv).unwrap(), (0, vec![]));
    // Changed in a spreadsheet: a description, a rename and a row that does not exist.
    let edited = csv.replacen("-K1;Conveyor motor;;", "-K1;Conveyor motor;Main contactor;", 1).replacen("\n-P1;", "\n-P1;-P7;", 1).replace("-P1;-P7;-P1;", "-P1;-P7;") + "-Z9;-Z9;;;;\n";
    let (n, problems) = apply_csv(&mut prj, &nl, &open, TableKind::Devices, &edited).unwrap();
    assert_eq!(problems, vec!["Row -Z9 not found".to_string()]);
    assert!(n >= 1);
    assert_eq!(prj.devices["-K1"].description, "Main contactor");
    assert!(apply_csv(&mut prj, &nl, &open, TableKind::Devices, "Tag;x\n").is_err());
}

#[test]
fn fill_and_find_replace_on_filtered_rows() {
    let mut prj = demo_project();
    let rows: Vec<Row> = table(&prj, TableKind::Devices).into_iter().filter(|r| matches(r, "-S")).collect();
    assert!(rows.len() >= 2 && rows.iter().all(|r| r.key.contains("-S") || r.cells.iter().any(|c| c.to_lowercase().contains("-s"))));
    let (_, open) = nets(&prj);
    let (n, problems) = replace(&mut prj, &open, TableKind::Devices, &rows, 2, "", "Door");
    assert_eq!((n, problems.len()), (rows.len(), 0));
    let rows = table(&prj, TableKind::Devices);
    let doors: Vec<&Row> = rows.iter().filter(|r| r.cells[2] == "Door").collect();
    let (n, _) = replace(&mut prj, &open, TableKind::Devices, &doors.into_iter().cloned().collect::<Vec<_>>(), 2, "Door", "Door device");
    assert!(n >= 2);
    assert!(prj.devices.values().any(|d| d.description == "Door device"));
}
