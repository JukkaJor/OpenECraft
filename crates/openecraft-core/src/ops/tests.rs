use super::*;
use crate::geom::p;
use crate::symlib::Symbols;

#[test]
fn contact_numbering_and_tag_follow_coil() {
    let mut prj = Project::default();
    let mut ctx = PlaceCtx::default();
    let coil = prj.symbol("coil").unwrap();
    let s = new_symbol(&mut prj, 0, &ctx, &coil, p(40.0, 40.0), 0, false);
    assert_eq!(s.tag, "-K1");
    ctx.remember(&prj, &s);
    prj.pages[0].symbols.push(s);
    let no = prj.symbol("no").unwrap();
    let c1 = new_symbol(&mut prj, 0, &ctx, &no, p(80.0, 40.0), 0, false);
    assert_eq!(c1.tag, "-K1");
    assert_eq!(c1.pins, vec!["13", "14"]);
    prj.pages[0].symbols.push(c1);
    let nc = prj.symbol("nc").unwrap();
    let c2 = new_symbol(&mut prj, 0, &ctx, &nc, p(120.0, 40.0), 0, false);
    assert_eq!(c2.pins, vec!["21", "22"]);
    prj.pages[0].symbols.push(c2);
    // Time-delayed contacts: x7/x8 (NO) and x5/x6 (NC).
    let c3_def = prj.symbol("no_on").unwrap();
    let c3 = new_symbol(&mut prj, 0, &ctx, &c3_def, p(160.0, 40.0), 0, false);
    assert_eq!((c3.tag.as_str(), c3.pins.clone()), ("-K1", vec!["37".to_string(), "38".to_string()]));
    prj.pages[0].symbols.push(c3);
    let c4_def = prj.symbol("nc_off").unwrap();
    let c4 = new_symbol(&mut prj, 0, &ctx, &c4_def, p(200.0, 40.0), 0, false);
    assert_eq!(c4.pins, vec!["45", "46"]);
}

#[test]
fn plc_addresses_increment() {
    let mut prj = Project::default();
    let mut ctx = PlaceCtx::default();
    let di = prj.symbol("plc_in").unwrap();
    for expect in ["I0.0", "I0.1"] {
        let s = new_symbol(&mut prj, 0, &ctx, &di, p(40.0, 40.0), 0, false);
        assert_eq!(s.label, expect);
        ctx.remember(&prj, &s);
        prj.pages[0].symbols.push(s);
    }
}

#[test]
fn paste_renames_existing_devices() {
    let mut prj = Project::default();
    let ctx = PlaceCtx::default();
    let coil = prj.symbol("coil").unwrap();
    let s = new_symbol(&mut prj, 0, &ctx, &coil, p(40.0, 40.0), 0, false);
    let id = s.id;
    prj.pages[0].symbols.push(s);
    let clip = copy(&prj.pages[0], &BTreeSet::from([id]));
    let ids = paste(&mut prj, 0, &clip, p(80.0, 40.0));
    let new = prj.pages[0].symbols.iter().find(|s| ids.contains(&s.id)).unwrap();
    assert_eq!(new.tag, "-K2");
}

#[test]
fn rotate_single_symbol_keeps_origin() {
    let mut prj = Project::default();
    let ctx = PlaceCtx::default();
    let s_def = prj.symbol("no").unwrap();
    let s = new_symbol(&mut prj, 0, &ctx, &s_def, p(40.0, 40.0), 0, false);
    let id = s.id;
    prj.pages[0].symbols.push(s);
    rotate(&Symbols::default(), &mut prj.pages[0], &BTreeSet::from([id]));
    let s = &prj.pages[0].symbols[0];
    assert_eq!((s.pos, s.rot), (p(40.0, 40.0), 1));
}

/// Project with a second page in structure =A1+CAB1.
fn structured() -> Project {
    let mut prj = Project::default();
    let id = prj.new_id();
    let mut pg = Page::new(id, "2", "", PageKind::Schematic);
    pg.function = "A1".into();
    pg.location = "CAB1".into();
    prj.pages.push(pg);
    prj
}

#[test]
fn devices_get_the_page_structure() {
    let mut prj = structured();
    let ctx = PlaceCtx::default();
    let coil = prj.symbol("coil").unwrap();
    let a = new_symbol(&mut prj, 1, &ctx, &coil, p(40.0, 40.0), 0, false);
    assert_eq!(a.tag, "=A1+CAB1-K1");
    prj.pages[1].symbols.push(a);
    // Numbering is per structure.
    assert_eq!(new_symbol(&mut prj, 1, &ctx, &coil, p(80.0, 40.0), 0, false).tag, "=A1+CAB1-K2");
    assert_eq!(new_symbol(&mut prj, 0, &ctx, &coil, p(80.0, 40.0), 0, false).tag, "-K1");
    let term = prj.symbol("term").unwrap();
    assert_eq!(new_symbol(&mut prj, 1, &ctx, &term, p(80.0, 40.0), 0, false).tag, "=A1+CAB1-X1");
}

#[test]
fn paste_moves_devices_into_the_target_structure() {
    let mut prj = structured();
    let ctx = PlaceCtx::default();
    let s_def = prj.symbol("coil").unwrap();
    let s = new_symbol(&mut prj, 0, &ctx, &s_def, p(40.0, 40.0), 0, false);
    let id = s.id;
    prj.pages[0].symbols.push(s);
    let clip = copy(&prj.pages[0], &BTreeSet::from([id]));
    let ids = paste(&mut prj, 1, &clip, p(80.0, 40.0));
    let new = prj.pages[1].symbols.iter().find(|s| ids.contains(&s.id)).unwrap();
    assert_eq!(new.tag, "=A1+CAB1-K1");
}

#[test]
fn page_structure_change_renames_its_devices() {
    let mut prj = structured();
    let ctx = PlaceCtx::default();
    let s_def = prj.symbol("coil").unwrap();
    let s = new_symbol(&mut prj, 1, &ctx, &s_def, p(40.0, 40.0), 0, false);
    prj.pages[1].symbols.push(s);
    // A contact of the coil on the other page follows the device.
    let contact_def = prj.symbol("no").unwrap();
    let mut contact = new_symbol(&mut prj, 0, &ctx, &contact_def, p(80.0, 40.0), 0, false);
    contact.tag = "=A1+CAB1-K1".into();
    prj.pages[0].symbols.push(contact);
    prj.devices.insert("=A1+CAB1-K1".into(), DeviceData { description: "pump".into(), ..Default::default() });
    let dt = crate::structure::Dt::parse_structure;
    set_page_structure(&mut prj, 1, &dt("=A2+CAB1"), true);
    assert_eq!((prj.pages[1].function.as_str(), prj.pages[1].location.as_str()), ("A2", "CAB1"));
    assert_eq!(prj.all_tags(), vec!["=A2+CAB1-K1".to_string()]);
    assert_eq!(prj.devices["=A2+CAB1-K1"].description, "pump");
    // Without moving, the tags stay.
    set_page_structure(&mut prj, 1, &dt("=A3"), false);
    assert_eq!(prj.all_tags(), vec!["=A2+CAB1-K1".to_string()]);
    // Installation site and product move along as well.
    set_page_structure(&mut prj, 1, &dt("=A2++HALL1+CAB1-U1"), false);
    let s_def = prj.symbol("coil").unwrap();
    let mut s = new_symbol(&mut prj, 1, &ctx, &s_def, p(40.0, 80.0), 0, false);
    s.tag = "=A2++HALL1+CAB1-U1-K2".into();
    prj.pages[1].symbols.push(s);
    set_page_structure(&mut prj, 1, &dt("=A2++HALL2+CAB1-U3"), true);
    assert!(prj.all_tags().contains(&"=A2++HALL2+CAB1-U3-K2".to_string()), "{:?}", prj.all_tags());
}

#[test]
fn wires_get_free_cores_in_order() {
    let mut prj = Project::default();
    let id = prj.new_id();
    prj.cables.push(Cable { id, tag: "-W1".into(), cores: "BN BK GY GNYE".into(), ..Default::default() });
    for (id, x) in [(10, 60.0), (11, 40.0), (12, 50.0)] {
        prj.pages[0].wires.push(Wire { id, a: p(x, 10.0), b: p(x, 30.0), ..Default::default() });
    }
    // BK is already used by another wire.
    prj.pages[0].wires.push(Wire { id: 13, a: p(0.0, 0.0), b: p(0.0, 9.0), cable: "-W1".into(), core: "BK".into(), ..Default::default() });
    assert_eq!(assign_cores(&mut prj, 0, &BTreeSet::from([10, 11, 12]), "-W1"), 3);
    let core = |id: u64| prj.pages[0].wires.iter().find(|w| w.id == id).unwrap().core.clone();
    assert_eq!((core(11), core(12), core(10)), ("BN".into(), "GY".into(), "GNYE".into()));
    // No free core left, unknown cable.
    prj.pages[0].wires.push(Wire { id: 14, a: p(70.0, 10.0), b: p(70.0, 30.0), ..Default::default() });
    assert_eq!(assign_cores(&mut prj, 0, &BTreeSet::from([14]), "-W1"), 0);
    assert_eq!(assign_cores(&mut prj, 0, &BTreeSet::from([14]), "-W9"), 0);
}

#[test]
fn paste_drops_numbers_and_cores() {
    let mut prj = Project::default();
    prj.pages[0].wires.push(Wire { id: 5, a: p(0.0, 0.0), b: p(0.0, 20.0), number: "7".into(), cable: "-W1".into(), core: "BN".into(), ..Default::default() });
    prj.next_id = 6;
    let clip = copy(&prj.pages[0], &BTreeSet::from([5]));
    let ids = paste(&mut prj, 0, &clip, p(40.0, 0.0));
    let w = prj.pages[0].wires.iter().find(|w| ids.contains(&w.id)).unwrap();
    assert!(w.number.is_empty() && w.cable.is_empty() && w.core.is_empty());
}

#[test]
fn rename_device_updates_the_panel_layout() {
    let mut prj = crate::sample::demo_project();
    rename_device(&mut prj, "-K1", "-K10");
    assert!(prj.layout.find_device("-K10").is_some());
    assert!(prj.layout.find_device("-K1").is_none());
}

#[test]
fn add_page_numbers_after_the_highest_page() {
    let mut prj = Project::default();
    prj.pages[0].name = "7".into();
    let at = add_page(&mut prj, PageKind::Report(ReportKind::DeviceList.into()), 0);
    assert_eq!(at, 0);
    assert_eq!(prj.pages[0].name, "8");
    assert_eq!(prj.pages[0].kind, PageKind::Report(ReportKind::DeviceList.into()));
    assert_eq!(add_page(&mut prj, PageKind::Schematic, 99), 2);
}

#[test]
fn released_and_last_pages_are_not_deleted() {
    let mut prj = Project::default();
    assert!(delete_page(&mut prj, 0).is_err(), "last page");
    add_page(&mut prj, PageKind::Schematic, 1);
    prj.pages[1].locked = true;
    assert_eq!(delete_page(&mut prj, 1).unwrap_err(), "Released pages cannot be deleted");
    assert_eq!(delete_page(&mut prj, 0).unwrap().name, "1");
    assert_eq!(prj.pages.len(), 1);
}

#[test]
fn cables_keep_their_wires_consistent() {
    let mut prj = Project::default();
    let tag = add_cable(&mut prj, "");
    assert_eq!(tag, "-W1");
    assert_eq!(add_cable(&mut prj, ""), "-W2");
    prj.pages[0].wires.push(Wire { id: 50, a: p(0.0, 0.0), b: p(0.0, 8.0), cable: "-W1".into(), core: "BN".into(), ..Default::default() });
    prj.cables[0].tag = "-W10".into();
    rename_cable(&mut prj, "-W1", "-W10");
    assert_eq!(prj.pages[0].wires[0].cable, "-W10");
    assert_eq!(remove_cable(&mut prj, 0).tag, "-W10");
    let w = &prj.pages[0].wires[0];
    assert!(w.cable.is_empty() && w.core.is_empty());
    assert_eq!(prj.cables.len(), 1);

    let mut part = crate::parts::Part { number: "GEN.CABLE".into(), type_number: "H05VV-F".into(), ..Default::default() };
    part.tech_data = "Cores: BK BU GNYE
Cross-section: 0.75 mm²".into();
    apply_cable_part(&mut prj.cables[0], &part);
    let c = &prj.cables[0];
    assert_eq!((c.part.as_str(), c.cable_type.as_str()), ("GEN.CABLE", "H05VV-F"));
    assert_eq!((c.cores.as_str(), c.cross_section.as_str()), ("BK BU GNYE", "0.75 mm²"));
}

#[test]
fn pages_move_to_another_position() {
    let mut prj = crate::sample::demo_project();
    let names: Vec<String> = prj.pages.iter().map(|p| p.name.clone()).collect();
    assert_eq!(move_page(&mut prj, 0, 2), 2);
    assert_eq!(prj.pages[2].name, names[0]);
    assert_eq!(prj.pages[0].name, names[1]);
    assert_eq!(move_page(&mut prj, 2, 0), 0);
    assert_eq!(prj.pages.iter().map(|p| p.name.clone()).collect::<Vec<_>>(), names);
    assert_eq!(move_page(&mut prj, 99, 0), 99, "out of range: unchanged");
}

#[test]
fn rotating_a_selection_turns_its_texts() {
    let mut prj = crate::model::Project::default();
    let id = prj.new_id();
    prj.pages[0].texts.push(crate::model::TextItem { id, pos: crate::geom::p(100.0, 100.0), text: "T".into(), ..Default::default() });
    let symbols = prj.symbol_table();
    let sel: std::collections::BTreeSet<u64> = [id].into_iter().collect();
    rotate(&symbols, &mut prj.pages[0], &sel);
    assert_eq!(prj.pages[0].texts[0].angle, 270.0, "clockwise like the symbols");
    rotate(&symbols, &mut prj.pages[0], &sel);
    rotate(&symbols, &mut prj.pages[0], &sel);
    rotate(&symbols, &mut prj.pages[0], &sel);
    assert_eq!(prj.pages[0].texts[0].angle, 0.0);
}
