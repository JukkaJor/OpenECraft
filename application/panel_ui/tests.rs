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
    assert_eq!(ui.profile_here_for(&Tool::Duct), Some((ItemKind::Duct, panel::DOOR_DUCT_PART)));
    ui.face = Face::Plate;
    assert!(ui.tool_allowed(&Tool::Rail) && ui.tool_allowed(&Tool::Duct));
    assert_eq!(ui.profile_here_for(&Tool::Duct), Some((ItemKind::Duct, panel::DUCT_PART)));
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
