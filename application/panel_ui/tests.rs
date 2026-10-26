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
fn rooms_show_their_walls_and_cabinets() {
    let mut prj = Project::default();
    prj.layout.enclosures.push(Enclosure { name: "+A".into(), id: 7, ..Default::default() });
    let mut ui = PanelUi::default();
    ui.new_room(&mut prj);
    ui.new_room(&mut prj);
    assert_eq!(prj.layout.enclosures[2].name, "=ROOM2");
    let room = 1;
    ui.enclosure = room;
    assert!(ui.enclosure_index(&prj).is_some() && ui.in_room);
    assert_eq!((ui.face, ui.view), (Face::Wall(0), View::Plan));
    assert!(ui.tool_allowed(&Tool::Rail) && ui.tool_allowed(&Tool::Duct) && ui.tool_allowed(&Tool::Walls));
    assert!(!ui.tool_allowed(&Tool::Busbar) && !ui.tool_allowed(&Tool::Cutout));
    panel::set_room(&mut prj, 0, Some(room), Some(SitePlace { x: 500.0, z: 200.0, angle: 0.0 })).unwrap();
    let shown = ui.shown_3d(&prj, room);
    assert_eq!(shown.iter().map(|s| s.0).collect::<Vec<_>>(), vec![room, 0]);
    assert_eq!(shown[1].1.at, [500.0, 0.0, 200.0], "in the room's coordinates");
    // Back to a cabinet: its faces again.
    ui.enclosure = 0;
    ui.enclosure_index(&prj);
    assert!(!ui.in_room && !ui.tool_allowed(&Tool::Walls));
    assert_eq!(ui.view, View::Front, "no floor plan of a cabinet");
}

#[test]
fn walls_are_drawn_at_right_angles_and_closed() {
    // Corners snap to the grid and follow the last one at a right angle.
    assert_eq!(plan::ortho_corner(None, p(1020.0, 1980.0)), p(1000.0, 2000.0));
    assert_eq!(plan::ortho_corner(Some(p(0.0, 0.0)), p(3990.0, 220.0)), p(4000.0, 0.0));
    assert_eq!(plan::ortho_corner(Some(p(4000.0, 0.0)), p(4100.0, 2980.0)), p(4000.0, 3000.0));
    // An L drawn without its last corner is closed at a right angle.
    let drawn = [p(0.0, 0.0), p(6000.0, 0.0), p(6000.0, 6000.0), p(3000.0, 6000.0), p(3000.0, 3000.0)];
    let closed = plan::closed_outline(&drawn).unwrap();
    assert_eq!(closed.len(), 6);
    assert!(closed.contains(&[0.0, 3000.0]));
    assert!(plan::closed_outline(&drawn[..2]).is_err(), "too few walls");
    // Finishing the drawing gives the room its outline.
    let mut prj = Project::default();
    let mut ui = PanelUi::default();
    ui.new_room(&mut prj);
    ui.wall_points = drawn.to_vec();
    ui.finish_walls(&mut prj, 0);
    assert_eq!(panel::walls(&prj.layout.enclosures[0]).len(), 6);
    assert!(ui.wall_points.is_empty() && ui.tool == Tool::Select);
}

#[test]
fn doors_and_windows_are_placed_in_rooms_only() {
    let mut prj = Project::default();
    prj.layout.enclosures.push(Enclosure { name: "+A".into(), ..Default::default() });
    let mut ui = PanelUi::default();
    ui.enclosure_index(&prj);
    assert!(!ui.tool_allowed(&Tool::Opening(Opening::Door)), "not in a cabinet");
    ui.new_room(&mut prj);
    ui.view = View::ThreeD;
    ui.choose_tool(Tool::Opening(Opening::Window));
    assert_eq!((ui.tool.clone(), ui.view), (Tool::Opening(Opening::Window), View::Plan), "placed in the floor plan");
    ui.front_face(Face::Wall(1), false);
    ui.choose_tool(Tool::Opening(Opening::Door));
    assert_eq!(ui.view, View::Front, "or in the view of a wall");
}

#[test]
fn inner_walls_are_drawn_as_a_chain() {
    let mut prj = Project::default();
    let mut ui = PanelUi::default();
    ui.new_room(&mut prj);
    ui.choose_tool(Tool::InnerWalls);
    assert_eq!((ui.tool.clone(), ui.view), (Tool::InnerWalls, View::Plan));
    let id = panel::add_inner_wall(&mut prj, 0, [2000.0, 0.0], [2000.0, 1500.0], 100.0).unwrap();
    // Its sides are in the walls menu's faces and take doors.
    let faces = prj.layout.enclosures[0].faces();
    assert!(faces.contains(&Face::Inner(id, 0)) && faces.contains(&Face::Inner(id, 1)));
    ui.front_face(Face::Inner(id, 1), false);
    ui.choose_tool(Tool::Opening(Opening::Door));
    assert_eq!(ui.view, View::Front, "placed in the view of the side");
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
