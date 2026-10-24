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
