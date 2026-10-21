use super::*;

#[test]
fn contact_numbering_and_tag_follow_coil() {
    let mut prj = Project::default();
    let mut ctx = PlaceCtx::default();
    let coil = symbols::get("coil").unwrap();
    let s = new_symbol(&mut prj, 0, &ctx, coil, p(40.0, 40.0), 0, false);
    assert_eq!(s.tag, "-K1");
    ctx.remember(&s);
    prj.pages[0].symbols.push(s);
    let no = symbols::get("no").unwrap();
    let c1 = new_symbol(&mut prj, 0, &ctx, no, p(80.0, 40.0), 0, false);
    assert_eq!(c1.tag, "-K1");
    assert_eq!(c1.pins, vec!["13", "14"]);
    prj.pages[0].symbols.push(c1);
    let nc = symbols::get("nc").unwrap();
    let c2 = new_symbol(&mut prj, 0, &ctx, nc, p(120.0, 40.0), 0, false);
    assert_eq!(c2.pins, vec!["21", "22"]);
}

#[test]
fn plc_addresses_increment() {
    let mut prj = Project::default();
    let mut ctx = PlaceCtx::default();
    let di = symbols::get("plc_in").unwrap();
    for expect in ["I0.0", "I0.1"] {
        let s = new_symbol(&mut prj, 0, &ctx, di, p(40.0, 40.0), 0, false);
        assert_eq!(s.label, expect);
        ctx.remember(&s);
        prj.pages[0].symbols.push(s);
    }
}

#[test]
fn paste_renames_existing_devices() {
    let mut prj = Project::default();
    let ctx = PlaceCtx::default();
    let coil = symbols::get("coil").unwrap();
    let s = new_symbol(&mut prj, 0, &ctx, coil, p(40.0, 40.0), 0, false);
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
    let s = new_symbol(&mut prj, 0, &ctx, symbols::get("no").unwrap(), p(40.0, 40.0), 0, false);
    let id = s.id;
    prj.pages[0].symbols.push(s);
    rotate(&mut prj.pages[0], &BTreeSet::from([id]));
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
    let coil = symbols::get("coil").unwrap();
    let a = new_symbol(&mut prj, 1, &ctx, coil, p(40.0, 40.0), 0, false);
    assert_eq!(a.tag, "=A1+CAB1-K1");
    prj.pages[1].symbols.push(a);
    // Numbering is per structure.
    assert_eq!(new_symbol(&mut prj, 1, &ctx, coil, p(80.0, 40.0), 0, false).tag, "=A1+CAB1-K2");
    assert_eq!(new_symbol(&mut prj, 0, &ctx, coil, p(80.0, 40.0), 0, false).tag, "-K1");
    let term = symbols::get("term").unwrap();
    assert_eq!(new_symbol(&mut prj, 1, &ctx, term, p(80.0, 40.0), 0, false).tag, "=A1+CAB1-X1");
}

#[test]
fn paste_moves_devices_into_the_target_structure() {
    let mut prj = structured();
    let ctx = PlaceCtx::default();
    let s = new_symbol(&mut prj, 0, &ctx, symbols::get("coil").unwrap(), p(40.0, 40.0), 0, false);
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
    let s = new_symbol(&mut prj, 1, &ctx, symbols::get("coil").unwrap(), p(40.0, 40.0), 0, false);
    prj.pages[1].symbols.push(s);
    // A contact of the coil on the other page follows the device.
    let mut contact = new_symbol(&mut prj, 0, &ctx, symbols::get("no").unwrap(), p(80.0, 40.0), 0, false);
    contact.tag = "=A1+CAB1-K1".into();
    prj.pages[0].symbols.push(contact);
    prj.devices.insert("=A1+CAB1-K1".into(), DeviceData { description: "pump".into(), ..Default::default() });
    set_page_structure(&mut prj, 1, "=A2", "+CAB1", true);
    assert_eq!((prj.pages[1].function.as_str(), prj.pages[1].location.as_str()), ("A2", "CAB1"));
    assert_eq!(prj.all_tags(), vec!["=A2+CAB1-K1".to_string()]);
    assert_eq!(prj.devices["=A2+CAB1-K1"].description, "pump");
    // Without moving, the tags stay.
    set_page_structure(&mut prj, 1, "A3", "", false);
    assert_eq!(prj.all_tags(), vec!["=A2+CAB1-K1".to_string()]);
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