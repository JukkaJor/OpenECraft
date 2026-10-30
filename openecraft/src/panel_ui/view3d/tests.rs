use super::*;

#[test]
fn a_click_into_the_air_clears_the_selected_cable() {
    let prj = Project::default();
    let an = Analysis::build(&prj);
    let mut ui = PanelUi { selected_route: Some("=MD-W11".into()), selected_wire: Some(("-K1:A1".into(), "-K2:A1".into())), ..Default::default() };
    ui.selection.insert(7);
    assert!(!ui.pick_3d(&prj, &an, &Scene3d::default(), &[], &|_| None, Pos2::ZERO));
    assert!(ui.selected_route.is_none() && ui.selected_wire.is_none() && ui.selection.is_empty());
}

#[test]
fn selecting_a_device_or_wire_drops_the_selected_cable() {
    let mut ui = PanelUi { selected_route: Some("=MD-W11".into()), ..Default::default() };
    ui.select_item(7);
    assert!(ui.selected_route.is_none());
    ui.selected_route = Some("=MD-W11".into());
    ui.select_wire(("-K1:A1".into(), "-K2:A1".into()));
    assert!(ui.selected_route.is_none() && ui.selection.is_empty());
}
