use super::*;

#[test]
fn contact_numbering_and_tag_follow_coil() {
    let mut prj = Project::default();
    let mut ctx = PlaceCtx::default();
    let coil = symbols::get("coil").unwrap();
    let s = new_symbol(&mut prj, &ctx, coil, p(40.0, 40.0), 0, false);
    assert_eq!(s.tag, "-K1");
    ctx.remember(&s);
    prj.pages[0].symbols.push(s);
    let no = symbols::get("no").unwrap();
    let c1 = new_symbol(&mut prj, &ctx, no, p(80.0, 40.0), 0, false);
    assert_eq!(c1.tag, "-K1");
    assert_eq!(c1.pins, vec!["13", "14"]);
    prj.pages[0].symbols.push(c1);
    let nc = symbols::get("nc").unwrap();
    let c2 = new_symbol(&mut prj, &ctx, nc, p(120.0, 40.0), 0, false);
    assert_eq!(c2.pins, vec!["21", "22"]);
}

#[test]
fn plc_addresses_increment() {
    let mut prj = Project::default();
    let mut ctx = PlaceCtx::default();
    let di = symbols::get("plc_in").unwrap();
    for expect in ["I0.0", "I0.1"] {
        let s = new_symbol(&mut prj, &ctx, di, p(40.0, 40.0), 0, false);
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
    let s = new_symbol(&mut prj, &ctx, coil, p(40.0, 40.0), 0, false);
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
    let s = new_symbol(&mut prj, &ctx, symbols::get("no").unwrap(), p(40.0, 40.0), 0, false);
    let id = s.id;
    prj.pages[0].symbols.push(s);
    rotate(&mut prj.pages[0], &BTreeSet::from([id]));
    let s = &prj.pages[0].symbols[0];
    assert_eq!((s.pos, s.rot), (p(40.0, 40.0), 1));
}
