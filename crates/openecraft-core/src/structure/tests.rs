use super::*;
use crate::model::{PageKind, Project};

fn page(function: &str, location: &str) -> Page {
    let mut p = Page::new(1, "3", "", PageKind::Schematic);
    p.function = function.into();
    p.location = location.into();
    p
}

#[test]
fn tags_are_split_into_aspects() {
    let dt = Dt::parse("=A1+CAB1-K1");
    assert_eq!((dt.function.as_str(), dt.location.as_str(), dt.name.as_str()), ("A1", "CAB1", "-K1"));
    assert_eq!(dt.tag(), "=A1+CAB1-K1");
    // Either order, missing aspects, plain names
    assert_eq!(Dt::parse("+CAB1=A1-K1").tag(), "=A1+CAB1-K1");
    assert_eq!(Dt::parse("+CAB1-X1"), Dt { location: "CAB1".into(), name: "-X1".into(), ..Default::default() });
    assert_eq!(Dt::parse("-K1"), Dt { name: "-K1".into(), ..Default::default() });
    assert_eq!(Dt::parse("K1").name, "K1");
    assert_eq!(Dt::parse("=A1.2").function, "A1.2");
    assert_eq!(Dt::parse("=A1").structure(), "=A1");
}

#[test]
fn page_structure_completes_and_shortens_tags() {
    let pg = page("A1", "CAB1");
    assert_eq!(page_full_name(&pg), "=A1+CAB1/3");
    assert_eq!(page_full_name(&page("", "")), "3");
    assert_eq!(complete(&pg, "-K1"), "=A1+CAB1-K1");
    assert_eq!(complete(&pg, "+FIELD-S1"), "=A1+FIELD-S1");
    assert_eq!(complete(&pg, "=B2+X-K1"), "=B2+X-K1");
    assert_eq!(relative(&pg, "=A1+CAB1-K1"), "-K1");
    assert_eq!(relative(&pg, "=A1+FIELD-S1"), "+FIELD-S1");
    assert_eq!(relative(&pg, "=B2+CAB1-K1"), "=B2-K1");
    // Without page structure nothing changes.
    let plain = page("", "");
    assert_eq!(complete(&plain, "-K1"), "-K1");
    assert_eq!(relative(&plain, "=A1-K1"), "=A1-K1");
}

#[test]
fn aspects_are_cleaned() {
    assert_eq!(clean_aspect(" =A1 "), "A1");
    assert_eq!(clean_aspect("+CAB 1"), "CAB1");
    assert_eq!(Dt { location: "CAB1".into(), ..Default::default() }.structure(), "+CAB1");
}

#[test]
fn installation_site_and_product_are_aspects() {
    let dt = Dt::parse("=A1++HALL1+CAB1-U1-K1");
    assert_eq!(
        (dt.function.as_str(), dt.installation.as_str(), dt.location.as_str(), dt.product.as_str(), dt.name.as_str()),
        ("A1", "HALL1", "CAB1", "U1", "-K1")
    );
    assert_eq!(dt.tag(), "=A1++HALL1+CAB1-U1-K1");
    assert_eq!(Dt::parse("+CAB1++HALL1-K1").tag(), "++HALL1+CAB1-K1");
    assert_eq!(Dt::parse("-U1-A2-K1").product, "U1-A2");
    assert_eq!(Dt::parse("++HALL1").structure(), "++HALL1");
    assert_eq!(Dt::parse_structure("=A1-U1-A2").product, "U1-A2");
    assert_eq!(Dt::parse_structure("+CAB1-U1").tag(), "+CAB1-U1");

    let mut pg = page("A1", "CAB1");
    pg.installation = "HALL1".into();
    pg.product = "U1".into();
    assert_eq!(page_full_name(&pg), "=A1++HALL1+CAB1-U1/3");
    assert_eq!(complete(&pg, "-K1"), "=A1++HALL1+CAB1-U1-K1");
    assert_eq!(complete(&pg, "-U2-K1"), "=A1++HALL1+CAB1-U2-K1");
    assert_eq!(relative(&pg, "=A1++HALL1+CAB1-U1-K1"), "-K1");
    assert_eq!(relative(&pg, "=A1++HALL2+CAB1-U1-K1"), "++HALL2-K1");
    assert_eq!(clean_aspect("-U1"), "U1");
    assert_eq!(Aspect::from_key("++"), Some(Aspect::Installation));
    assert_eq!(Aspect::from_key("Product"), Some(Aspect::Product));
}

fn project() -> Project {
    let mut prj = Project::default();
    prj.pages[0].function = "A1".into();
    prj.pages[0].location = "CAB1".into();
    let id = prj.new_id();
    prj.pages[0].symbols.push(crate::model::SymbolInst { id, sym: "coil".into(), tag: "=A1+CAB1-K1".into(), ..Default::default() });
    let id = prj.new_id();
    prj.pages[0].symbols.push(crate::model::SymbolInst { id, sym: "coil".into(), tag: "=A2+CAB1-K1".into(), ..Default::default() });
    let id = prj.new_id();
    prj.cables.push(crate::model::Cable { id, tag: "=A1+CAB1-W1".into(), ..Default::default() });
    let id = prj.new_id();
    prj.pages[0].wires.push(crate::model::Wire { id, cable: "=A1+CAB1-W1".into(), core: "BN".into(), ..Default::default() });
    prj
}

#[test]
fn overview_counts_defined_and_used_identifiers() {
    let mut prj = project();
    prj.structures.define(Aspect::Location, "+FIELD", "Field");
    let rows = overview(&prj);
    let row = |a: Aspect, id: &str| rows.iter().find(|r| r.aspect == a && r.id == id).cloned().unwrap();
    assert_eq!((row(Aspect::Function, "A1").pages, row(Aspect::Function, "A1").devices), (1, 2));
    assert_eq!(row(Aspect::Location, "CAB1").devices, 3);
    assert!(row(Aspect::Location, "FIELD").defined && !row(Aspect::Location, "CAB1").defined);
    assert_eq!(rows[0].aspect, Aspect::Function);

    // Undefined identifiers are only reported once the management is in use.
    assert_eq!(problems(&prj).len(), 3);
    assert!(problems(&prj)[0].contains("=A1"));
    assert_eq!(define_used(&mut prj), 3);
    assert!(problems(&prj).is_empty());
    assert_eq!(remove_unused(&mut prj), 1);
    assert!(prj.structures.entry(Aspect::Location, "FIELD").is_none());
    prj.structures.entries.clear();
    assert!(problems(&prj).is_empty());
}

#[test]
fn identifiers_are_renamed_in_the_whole_project() {
    let mut prj = project();
    prj.structures.define(Aspect::Function, "A1", "Pumps");
    prj.structures.set_work_area(WorkArea { name: "Pumps".into(), filter: Dt { function: "A1".into(), ..Default::default() } });
    assert_eq!(rename(&mut prj, Aspect::Function, "=A1", "P1"), Ok(3));
    assert_eq!(prj.pages[0].function, "P1");
    assert_eq!(prj.all_tags(), vec!["=A2+CAB1-K1".to_string(), "=P1+CAB1-K1".to_string()]);
    assert_eq!(prj.cables[0].tag, "=P1+CAB1-W1");
    assert_eq!(prj.pages[0].wires[0].cable, "=P1+CAB1-W1");
    assert_eq!(prj.structures.entry(Aspect::Function, "P1").unwrap().description, "Pumps");
    assert_eq!(prj.structures.work_areas[0].filter.function, "P1");
    // A tag that exists already stops the rename before anything changes.
    let before = prj.clone();
    assert!(rename(&mut prj, Aspect::Function, "A2", "P1").unwrap_err().contains("=P1+CAB1-K1"));
    assert_eq!(prj, before);
    assert!(rename(&mut prj, Aspect::Function, "A2", "").is_err());
}

#[test]
fn work_areas_filter_pages_and_devices() {
    assert!(matches("", "A1"));
    assert!(matches("A1", "A1") && matches("=A1", "A1.2") && matches("U1", "U1-A3"));
    assert!(!matches("A1", "A10") && !matches("A1", ""));
    assert!(matches("A*", "A10") && matches("*1", "CAB1") && matches("C*B*", "CAB1") && !matches("C*X", "CAB1"));
    let area = WorkArea { name: "Cabinet".into(), filter: Dt { location: "CAB1".into(), ..Default::default() } };
    assert!(area.shows_device("=A1+CAB1-K1") && !area.shows_device("=A1+FIELD-S1") && !area.shows_device("-K1"));
    let mut pg = page("A1", "FIELD");
    assert!(!area.shows_page(&pg));
    pg.kind = PageKind::Report(crate::model::ReportKind::DeviceList);
    assert!(area.shows_page(&pg));
}

#[test]
fn templates_set_aspects_and_add_identifiers() {
    let mut s = Structures::default();
    assert_eq!(s.page_aspects(), vec![Aspect::Function, Aspect::Location]);
    let t = template("Machine (example identifiers)").unwrap();
    assert_eq!(apply_template(&mut s, t), 7);
    assert_eq!(apply_template(&mut s, t), 0);
    assert_eq!(s.entry(Aspect::Location, "CAB1").unwrap().description, "Control cabinet");
    apply_template(&mut s, template("All aspects (with product)").unwrap());
    assert_eq!(s.page_aspects(), Aspect::ALL.to_vec());
    // User templates are files of the structure management.
    s.set_work_area(WorkArea { name: "Drives".into(), filter: Dt { function: "DRIVE".into(), ..Default::default() } });
    let file = Structures::from_json(&s.to_json()).unwrap();
    let mut other = Structures::default();
    other.define(Aspect::Location, "CAB1", "Main cabinet");
    assert_eq!(other.merge(&file), 6);
    assert_eq!(other.entry(Aspect::Location, "CAB1").unwrap().description, "Main cabinet");
    assert_eq!(other.work_areas.len(), 1);
    assert!(Structures::from_json("42").is_err());
}
