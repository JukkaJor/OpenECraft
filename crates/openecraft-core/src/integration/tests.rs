use super::*;
use crate::sample::demo_project;

fn setup() -> (Project, Analysis) {
    let prj = demo_project();
    let an = Analysis::build(&prj);
    (prj, an)
}

fn first_part(prj: &Project, an: &Analysis) -> String {
    bom(prj, an).first().expect("demo project has a BOM").part.clone()
}

#[test]
fn every_connector_has_an_id_of_its_own_and_exports() {
    let (prj, an) = setup();
    for (i, c) in CONNECTORS.iter().enumerate() {
        assert!(CONNECTORS[..i].iter().all(|o| o.id != c.id), "{}", c.id);
        assert!(!c.accepts.is_empty());
        let text = (c.export)(&prj, &an);
        assert!(!text.is_empty(), "{}", c.id);
        // Exports read back by the same connector change nothing.
        let p = import(&prj, &an, c, &text, &BTreeMap::new()).unwrap();
        assert!(p.changes.is_empty(), "{}: {:?}", c.id, p.changes);
        assert!(p.unmatched.is_empty(), "{}: {:?}", c.id, p.unmatched);
    }
    for s in System::ALL {
        assert!(CONNECTORS.iter().any(|c| c.system == s), "{s:?}");
    }
}

#[test]
fn field_codes_are_unique_and_found_by_alias() {
    for (i, f) in Field::ALL.iter().enumerate() {
        assert!(Field::ALL[..i].iter().all(|o| o.code() != f.code()));
        assert_eq!(Field::from_code(f.code()), Some(*f));
        assert_eq!(column_field(f.code(), &BTreeMap::new()), Some(*f));
    }
    let none = BTreeMap::new();
    assert_eq!(column_field("Material", &none), Some(Field::PartNumber));
    assert_eq!(column_field("Lager-Bestand", &none), Some(Field::Stock));
    assert_eq!(column_field("Part No.", &none), Some(Field::PartNumber));
    let mapping = BTreeMap::from([("erp_number".to_string(), "Matnr".to_string())]);
    assert_eq!(column_field("MATNR", &mapping), Some(Field::ErpNumber));
}

#[test]
fn numbers_of_other_systems() {
    assert_eq!(number("12,5"), Some(12.5));
    assert_eq!(number("1.234,5"), Some(1234.5));
    assert_eq!(number("1,234.5"), Some(1234.5));
    assert_eq!(number("3 pcs"), Some(3.0));
    assert_eq!(number("abc"), None);
    assert_eq!(fmt_number(2.50), "2.5");
    assert_eq!(fmt_number(3.0), "3");
}

#[test]
fn erp_import_takes_over_numbers_prices_and_stock() {
    let (mut prj, an) = setup();
    let part = first_part(&prj, &an);
    let c = connector("erp").unwrap();
    let text = format!("Material;Matnr;Preis;Bestand;Lieferzeit\n{part};4711;12,50;7;14\nNOPE.PART;1;1;1;1\n");
    let mapping = BTreeMap::from([("erp_number".to_string(), "Matnr".to_string())]);
    let p = import(&prj, &an, c, &text, &mapping).unwrap();
    assert_eq!(p.records, 2);
    assert_eq!(p.unmatched, vec!["NOPE.PART".to_string()]);
    let fields: Vec<Field> = p.changes.iter().map(|c| c.field).collect();
    assert!(fields.contains(&Field::ErpNumber) && fields.contains(&Field::Stock) && fields.contains(&Field::LeadTime), "{fields:?}");
    let n = apply(&mut prj, &an, c, &p.changes, false, "2026-10-24");
    assert_eq!(n, p.changes.len());
    let link = &prj.integration.items[&part];
    assert_eq!((link.erp_number.as_str(), link.stock, link.lead_time), ("4711", Some(7.0), Some(14)));
    assert_eq!(prj.parts.iter().find(|x| x.number == part).unwrap().price, 12.5);
    assert_eq!(prj.integration.log.last().unwrap().direction, Direction::Import);
    // Parts are found by their ERP number alone afterwards.
    let again = import(&prj, &an, c, "erp_number;stock\n4711;9\n", &BTreeMap::new()).unwrap();
    assert_eq!(again.changes.len(), 1);
    assert_eq!(again.changes[0].key, part);
}

#[test]
fn changes_on_both_sides_are_conflicts() {
    let (mut prj, an) = setup();
    let part = first_part(&prj, &an);
    let c = connector("erp").unwrap();
    export(&mut prj, &an, c, "2026-10-01");
    // Changed in the project after the export …
    part_mut(&mut prj, &part).unwrap().description = "Local text".into();
    // … unchanged in the ERP system: the import keeps the project's value.
    let exported = (c.export)(&demo_project(), &an);
    assert!(import(&prj, &an, c, &exported, &BTreeMap::new()).unwrap().changes.is_empty());
    // … and changed there as well: a conflict.
    let text = format!("part_number;description\n{part};ERP text\n");
    let p = import(&prj, &an, c, &text, &BTreeMap::new()).unwrap();
    assert_eq!(p.conflicts(), 1);
    assert_eq!(apply(&mut prj, &an, c, &p.changes, false, ""), 0);
    assert_eq!(prj.integration.log.last().unwrap().conflicts, 1);
    assert_eq!(apply(&mut prj, &an, c, &p.changes, true, ""), 1);
    assert_eq!(prj.parts.iter().find(|x| x.number == part).unwrap().description, "ERP text");
}

#[test]
fn plm_lifecycle_and_obsolete_parts_warning() {
    let (mut prj, an) = setup();
    let c = connector("plm").unwrap();
    let used = prj.devices.iter().find_map(|(_, d)| d.parts.first().map(|r| r.number.clone())).unwrap();
    let text = format!(r#"{{"items": [{{"part_number": "{used}", "item_id": "PLM-1", "revision": "B", "lifecycle": "Obsolete"}}]}}"#);
    let p = import(&prj, &an, c, &text, &BTreeMap::new()).unwrap();
    assert_eq!(p.changes.len(), 3);
    apply(&mut prj, &an, c, &p.changes, false, "");
    let issues = problems(&prj);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains(&used));
    assert!(Analysis::build(&prj).issues.iter().any(|i| i.message == issues[0].message));
}

#[test]
fn pdm_documents_and_project() {
    let (mut prj, an) = setup();
    let c = connector("pdm").unwrap();
    let page = prj.pages[0].name.clone();
    let mut v: Value = serde_json::from_str(&(c.export)(&prj, &an)).unwrap();
    v["project"]["document_number"] = "DOC-100".into();
    v["documents"][0]["state"] = "Released".into();
    let p = import(&prj, &an, c, &v.to_string(), &BTreeMap::new()).unwrap();
    assert_eq!(p.changes.len(), 2, "{:?}", p.changes);
    apply(&mut prj, &an, c, &p.changes, false, "");
    assert_eq!(prj.integration.project.number, "DOC-100");
    assert_eq!(prj.integration.documents[&page].state, "Released");
}

#[test]
fn ecad_assigns_parts_to_devices() {
    let (mut prj, an) = setup();
    let c = connector("ecad").unwrap();
    let tag = prj.devices.keys().next().unwrap().clone();
    let text = format!("refdes;part;description\n{tag};GEN.NEW-PART;From PCB\n");
    let p = import(&prj, &an, c, &text, &BTreeMap::new()).unwrap();
    assert_eq!(p.changes.len(), 2);
    apply(&mut prj, &an, c, &p.changes, false, "");
    assert_eq!(prj.devices[&tag].parts[0].number, "GEN.NEW-PART");
    assert_eq!(prj.devices[&tag].description, "From PCB");
}

#[test]
fn mcad_moves_devices() {
    let (mut prj, an) = setup();
    let c = connector("mcad").unwrap();
    let text = (c.export)(&prj, &an);
    let rows = openecraft_parts::parse_csv(&text);
    let (tag, x) = (rows[1][1].clone(), number(&rows[1][4]).unwrap());
    let moved = format!("tag;x\n{tag};{}\n", x + 25.0);
    let p = import(&prj, &an, c, &moved, &BTreeMap::new()).unwrap();
    assert_eq!(p.changes.len(), 1);
    apply(&mut prj, &an, c, &p.changes, false, "");
    let (e, i) = placement(&prj, &tag).unwrap();
    assert_eq!(prj.layout.enclosures[e].items[i].x, x + 25.0);
}

#[test]
fn procurement_rounds_to_packs_and_reads_confirmations() {
    let (mut prj, an) = setup();
    let part = first_part(&prj, &an);
    part_mut(&mut prj, &part).unwrap().pack_quantity = 50;
    let c = connector("procurement").unwrap();
    let text = (c.export)(&prj, &an);
    let rows = openecraft_parts::parse_csv(&text);
    let col = |f: Field| rows[0].iter().position(|h| h == f.code()).unwrap();
    let row = rows.iter().find(|r| r[col(Field::PartNumber)] == part).unwrap();
    assert_eq!(number(&row[col(Field::ToOrder)]).unwrap() % 50.0, 0.0);
    let confirm = format!("Artikel;Bestellstatus;Liefertermin\n{part};confirmed;2026-11-02\n");
    let p = import(&prj, &an, c, &confirm, &BTreeMap::new()).unwrap();
    apply(&mut prj, &an, c, &p.changes, false, "");
    assert_eq!(prj.integration.items[&part].order_status, "confirmed");
    assert_eq!(prj.integration.items[&part].delivery, "2026-11-02");
}

#[test]
fn wiring_machine_feedback_sets_wiring_marks() {
    let (mut prj, an) = setup();
    assert!(!an.wiring.is_empty());
    let c = connector("wiring_machine").unwrap();
    let text = (c.export)(&prj, &an);
    assert!(text.lines().count() == an.wiring.len() + 1);
    let (a, b) = (an.wiring[0].from.clone(), an.wiring[1].from.clone());
    let (a_to, b_to) = (an.wiring[0].to.clone(), an.wiring[1].to.clone());
    let feedback = format!("from;to;result;comment\n{a};{a_to};OK;\n{b};{b_to};NOK;Crimp height\n");
    let p = import(&prj, &an, c, &feedback, &BTreeMap::new()).unwrap();
    apply(&mut prj, &an, c, &p.changes, false, "");
    assert_eq!(wiring::status(&prj, &an.wiring[0]), wiring::Status::Done);
    assert_eq!(wiring::status(&prj, &an.wiring[1]), wiring::Status::Problem);
    assert_eq!(wiring::mark_of(&prj, &an.wiring[1]).unwrap().note, "Crimp height");
}

#[test]
fn exchange_file_is_read_by_the_connectors() {
    let (prj, an) = setup();
    let text = exchange(&prj, &an);
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["format"], "openecraft.exchange/1");
    for c in CONNECTORS {
        let p = import(&prj, &an, c, &text, &BTreeMap::new()).unwrap();
        assert!(p.changes.is_empty(), "{}: {:?}", c.id, p.changes);
    }
}

#[test]
fn bad_files_are_reported() {
    let (prj, an) = setup();
    let c = connector("erp").unwrap();
    assert!(import(&prj, &an, c, "foo;bar\n1;2\n", &BTreeMap::new()).is_err());
    assert!(import(&prj, &an, c, "{ nope", &BTreeMap::new()).is_err());
}

#[test]
fn integration_data_survives_saving() {
    let (mut prj, an) = setup();
    let c = connector("erp").unwrap();
    export(&mut prj, &an, c, "2026-10-24");
    prj.integration.profiles.push(Profile { name: "SAP".into(), connector: "erp".into(), mapping: BTreeMap::from([("erp_number".into(), "MATNR".into())]) });
    let back = Project::from_json(&prj.to_json()).unwrap();
    assert_eq!(back.integration, prj.integration);
    assert!(Project::default().integration.is_empty());
}
