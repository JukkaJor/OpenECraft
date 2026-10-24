use super::*;
use crate::sample::demo_project;

fn nl(prj: &Project) -> Netlist {
    Netlist::build(prj, true)
}

#[test]
fn points_of_the_demo() {
    let prj = demo_project();
    assert_eq!(cards(&prj), vec!["-A1".to_string(), "-A2".to_string()]);
    let pts = points(&prj, &nl(&prj));
    let addrs: Vec<&str> = pts.iter().map(|p| p.address.as_str()).collect();
    assert_eq!(addrs, ["I0.0", "I0.1", "I0.2", "Q0.0", "Q0.1"]);
    assert!(pts[0].input && !pts[3].input);
    assert!(pts[0].connected.contains("-B1"), "{}", pts[0].connected);
    assert_eq!(address_key("%I10.7"), ("I".into(), 10, 7));
}

#[test]
fn readdressing_uses_the_card_start_byte() {
    let mut prj = demo_project();
    prj.plc_cards.insert("-A2".into(), PlcCard { start_byte: 4, ..Default::default() });
    assert_eq!(readdress(&mut prj, "-A2"), 2);
    let pts = points(&prj, &nl(&prj));
    let a2: Vec<&str> = pts.iter().filter(|p| p.card == "-A2").map(|p| p.address.as_str()).collect();
    assert_eq!(a2, ["Q4.0", "Q4.1"]);
    assert_eq!(readdress(&mut prj, "-A2"), 0, "nothing left to change");
}

#[test]
fn symbol_table_round_trip() {
    let mut prj = demo_project();
    let table = symbol_table(&prj, &nl(&prj));
    let lines: Vec<&str> = table.lines().collect();
    assert_eq!(lines[0], "Name;Data Type;Logical Address;Comment");
    assert_eq!(lines.len(), 6);
    assert!(lines.contains(&"Part_present;Bool;%I0.0;-A1 Part present"), "{table}");
    // Names stay unique ("Motor running" is not used twice here, but the rule holds).
    let names: HashSet<&str> = lines[1..].iter().map(|l| l.split(';').next().unwrap()).collect();
    assert_eq!(names.len(), 5);

    let import = "\"Name\",\"Data Type\",\"Logical Address\",\"Comment\"\nX,Bool,%I0.1,Reset button\nY,Bool,%Q0.1,\n";
    assert_eq!(import_symbol_table(&mut prj, import).unwrap(), 2);
    let pts = points(&prj, &nl(&prj));
    let text = |a: &str| pts.iter().find(|p| p.address == a).unwrap().function_text.clone();
    assert_eq!(text("I0.1"), "Reset button");
    assert_eq!(text("Q0.1"), "Y", "name when there is no comment");
    assert!(import_symbol_table(&mut prj, "Name;Comment\nx;y\n").is_err());
}

#[test]
fn addresses_used_on_two_cards_are_reported() {
    let mut prj = demo_project();
    let s = prj.pages.iter_mut().flat_map(|p| p.symbols.iter_mut()).find(|s| s.label == "I0.2").unwrap();
    s.tag = "-A3".into();
    s.label = "I0.0".into();
    let msgs: Vec<String> = crate::analysis::Analysis::checked(&prj).issues.into_iter().map(|i| i.message()).collect();
    assert!(msgs.iter().any(|m| m == "PLC address I0.0 used on several cards: -A1, -A3"), "{msgs:#?}");
}

#[test]
fn iec_global_variables_per_address_space() {
    let mut prj = demo_project();
    let gvl = global_variables(&prj, &nl(&prj));
    assert!(gvl.starts_with("VAR_GLOBAL\n"), "{gvl}");
    assert!(gvl.contains("    Part_present AT %IX0.0 : BOOL; (* -A1 Part present *)\n"), "{gvl}");
    assert!(gvl.contains("    Valve AT %QX0.0 : BOOL;"), "{gvl}");
    crate::plc_hw::auto_configure(&mut prj, false);
    let gvl = global_variables(&prj, &nl(&prj));
    assert!(gvl.starts_with("(* PLC1 *)\nVAR_GLOBAL\n") && gvl.ends_with("END_VAR\n\n"), "{gvl}");
    // Exported comments carry the card; importing them gives the plain text again.
    let table = symbol_table(&prj, &nl(&prj));
    assert_eq!(import_symbol_table(&mut prj, &table).unwrap(), 0);
}

#[test]
fn vendor_tag_formats() {
    let prj = crate::sample::demo_project();
    let nl = Netlist::build(&prj, true);
    let n = tags(&prj, &nl).len();
    assert!(n > 0);
    let tia = tia_tags(&prj, &nl);
    assert_eq!(tia[0][..5], ["Name", "Path", "Data Type", "Logical Address", "Comment"]);
    assert_eq!(tia.len(), n + 1);
    assert!(tia[1..].iter().all(|r| r[2] == "Bool" && (r[3].starts_with("%I") || r[3].starts_with("%Q"))), "{tia:?}");
    let rw = rockwell_tags(&prj, &nl);
    let lines: Vec<&str> = rw.lines().collect();
    assert_eq!(lines[3], "TYPE,SCOPE,NAME,DESCRIPTION,DATATYPE,SPECIFIER,ATTRIBUTES");
    assert_eq!(lines.len(), 4 + n);
    assert!(lines[4].starts_with("ALIAS,,\"") && lines[4].contains("\"Local:"), "{}", lines[4]);
    // The workbook for TIA Portal reads back.
    let ex = crate::exporters::builtin().into_iter().find(|e| e.name == "plc_tia").unwrap();
    let an = crate::analysis::Analysis::build(&prj);
    let bytes = (ex.run)(&prj, &an, &crate::params::Args(&serde_json::Map::new())).unwrap().into_bytes();
    let sheets = crate::xlsx::read(&bytes).unwrap();
    assert_eq!(sheets[0].0, "PLC Tags");
    assert_eq!(sheets[0].1.len(), n + 1);
}
