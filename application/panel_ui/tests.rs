use super::*;

#[test]
fn fill_colours_follow_the_limit() {
    let green = fill_color(10.0, 70.0);
    assert_eq!(fill_color(56.0, 70.0), green, "up to 80 % of the limit");
    let orange = fill_color(60.0, 70.0);
    assert_ne!(orange, green);
    assert_eq!(fill_color(70.0, 70.0), orange, "the limit itself is still allowed");
    let red = fill_color(70.5, 70.0);
    assert!(red != orange && red != green);
}

#[test]
fn door_sides_have_their_own_tools() {
    let mut ui = PanelUi { face: Face::Door, ..Default::default() };
    assert!(!ui.tool_allowed(&Tool::Duct) && ui.tool_allowed(&Tool::Cutout));
    ui.door_inside = true;
    assert!(ui.tool_allowed(&Tool::Duct) && !ui.tool_allowed(&Tool::Cutout));
    assert!(!ui.tool_allowed(&Tool::Rail) && !ui.tool_allowed(&Tool::Busbar));
    assert_eq!(ui.profile_here_for(&Tool::Duct), Some((ItemKind::Duct, panel::DOOR_DUCT_PART.into())));
    ui.face = Face::Plate;
    assert!(ui.tool_allowed(&Tool::Rail) && ui.tool_allowed(&Tool::Duct));
    assert_eq!(ui.profile_here_for(&Tool::Duct), Some((ItemKind::Duct, panel::DUCT_PART.into())));
}

#[test]
fn chosen_profile_parts_are_kept_per_kind_and_door() {
    let mut ui = PanelUi::default();
    ui.choose_profile_part(ItemKind::Duct, "GEN.DUCT-60X80");
    ui.choose_profile_part(ItemKind::Rail, "GEN.RAIL-TS35X15-1M");
    assert_eq!(ui.profile_here_for(&Tool::Duct), Some((ItemKind::Duct, "GEN.DUCT-60X80".into())));
    assert_eq!(ui.profile_here_for(&Tool::Rail), Some((ItemKind::Rail, "GEN.RAIL-TS35X15-1M".into())));
    assert_eq!(ui.profile_here_for(&Tool::Busbar), Some((ItemKind::Busbar, panel::BUSBAR_PART.into())));
    ui.face = Face::Door;
    assert_eq!(ui.profile_here_for(&Tool::Duct), Some((ItemKind::Duct, panel::DOOR_DUCT_PART.into())), "the door keeps its own choice");
}

#[test]
fn ducts_behind_the_door_are_hidden_from_the_front() {
    let duct = PanelItem { kind: ItemKind::Duct, face: Face::Door, ..Default::default() };
    let device = PanelItem { kind: ItemKind::Device, face: Face::Door, ..Default::default() };
    let plate_duct = PanelItem { kind: ItemKind::Duct, ..Default::default() };
    assert!(hidden_behind_door(false, &duct));
    assert!(!hidden_behind_door(true, &duct));
    assert!(!hidden_behind_door(false, &device) && !hidden_behind_door(false, &plate_duct));
}

#[test]
fn the_inside_of_the_door_is_mirrored() {
    let e = Enclosure { width: 600.0, ..Default::default() };
    let mut ui = PanelUi::default();
    assert_eq!(ui.mirror(&e), None);
    ui.face = Face::Door;
    assert_eq!(ui.mirror(&e), None);
    ui.door_inside = true;
    assert_eq!(ui.mirror(&e), Some(600.0));
}

#[test]
fn the_3d_view_shows_all_enclosures_at_their_places() {
    let mut prj = Project::default();
    for name in ["+A", "+B"] {
        prj.layout.enclosures.push(Enclosure { name: name.into(), ..Default::default() });
    }
    panel::set_site(&mut prj, 1, Some(SitePlace { x: 3000.0, z: 1000.0, angle: 90.0 }));
    let mut ui = PanelUi::default();
    assert_eq!(ui.shown_3d(&prj, 0).iter().map(|s| s.0).collect::<Vec<_>>(), vec![0]);
    ui.show_site = true;
    let shown = ui.shown_3d(&prj, 0);
    assert_eq!(shown.len(), 2);
    assert_eq!(shown[1].1.at, [3000.0, 0.0, 1000.0]);
    let cam = view3d::default_cam(&prj, &shown);
    assert!(cam.target[0] > 600.0 && cam.target[2] > 0.0, "the camera looks at both");
}

#[test]
fn ducts_in_front_of_devices_are_painted_over_them() {
    // Seen from the left, the vertical duct at the left edge of the demo's mounting
    // plate is in front of the devices; it was always painted behind them.
    let prj = openecraft_core::sample::demo_project();
    let e = &prj.layout.enclosures[0];
    let shown: Vec<&PanelItem> = e.items.iter().filter(|i| i.face == Face::Plate && matches!(i.kind, ItemKind::Duct | ItemKind::Device)).collect();
    let boxes: Vec<_> = shown.iter().map(|i| panel::item_box(e, i)).collect();
    let duct = (0..shown.len()).filter(|&i| shown[i].kind == ItemKind::Duct && shown[i].h > shown[i].w).min_by(|&a, &b| shown[a].x.total_cmp(&shown[b].x)).unwrap();
    let eye = [-3000.0, e.height / 2.0, e.depth + 500.0];
    let order = openecraft_core::parts::vec3::paint_order(&boxes, eye);
    let rank = |i: usize| order.iter().position(|&k| k == i).unwrap();
    let devices: Vec<usize> = (0..shown.len()).filter(|&i| shown[i].kind == ItemKind::Device).collect();
    assert!(!devices.is_empty());
    for d in devices {
        assert!(rank(d) < rank(duct), "{} before the duct", shown[d].tag);
    }
}

#[test]
fn wires_routed_through_ducts_lie_inside_the_duct_boxes() {
    // The 3D view draws the pieces of a wire inside a duct box over that duct; the
    // stretches between the first and the last point of a route run in the ducts.
    let prj = openecraft_core::sample::demo_project();
    let an = Analysis::build(&prj);
    let e = &prj.layout.enclosures[0];
    let ducts: Vec<_> = e.items.iter().filter(|i| i.kind == ItemKind::Duct && i.face == Face::Plate).map(|i| panel::item_box(e, i)).collect();
    let z = panel::DUCT_PROFILE.1 * 0.5;
    let mut checked = 0;
    for w in an.wiring.iter().filter(|w| w.enclosure == 0 && w.route.via_ducts) {
        let pts: Vec<[f32; 3]> = w.route.points.iter().map(|q| [PLATE_MARGIN + q.x, PLATE_MARGIN + q.y, PLATE_Z + z]).collect();
        for q in pts.iter().skip(1).take(pts.len().saturating_sub(2)) {
            assert!(ducts.iter().any(|(lo, hi)| (0..3).all(|k| q[k] >= lo[k] - 0.5 && q[k] <= hi[k] + 0.5)), "{} at {q:?}", w.number);
            checked += 1;
        }
    }
    assert!(checked > 0);
}
