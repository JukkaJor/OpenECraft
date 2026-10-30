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
fn focusing_frames_the_whole_view_unless_zoomed_in() {
    let mut prj = Project::default();
    let item = PanelItem { id: 5, x: 100.0, y: 50.0, w: 40.0, h: 80.0, ..Default::default() };
    prj.layout.enclosures.push(Enclosure { name: "+A".into(), width: 800.0, height: 2000.0, items: vec![item], ..Default::default() });
    let mut ui = PanelUi::default();
    let full = view3d::default_cam(&prj, &ui.shown_3d(&prj, 0));
    ui.focus_item(&prj, 0, 5);
    assert_eq!(ui.cam.as_ref().map(|c| (c.target, c.dist)), Some((full.target, full.dist)), "Fit shows the whole enclosure");
    ui.cam.as_mut().unwrap().dist = full.dist / 3.0;
    ui.focus_item(&prj, 0, 5);
    let near = ui.cam.as_ref().unwrap();
    assert!(near.target[1] > 1800.0 && near.dist == full.dist / 3.0, "zoomed in, the camera turns to the item near the top");
    ui.cam.as_mut().unwrap().dist = full.dist * 2.0;
    ui.focus_item(&prj, 0, 5);
    assert_eq!(ui.cam.as_ref().unwrap().target, full.target, "zoomed out, everything stays in view");
}

#[test]
fn rooms_show_their_walls_and_cabinets() {
    let mut prj = Project::default();
    prj.layout.enclosures.push(Enclosure { name: "+A".into(), id: 7, ..Default::default() });
    let mut ui = PanelUi::default();
    ui.new_room(&mut prj);
    ui.new_room(&mut prj);
    assert_eq!(prj.layout.enclosures[2].name, "+ROOM2");
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

#[test]
fn face_summaries_count_openings_apart() {
    let item = |kind: ItemKind, opening: Opening| PanelItem { kind, opening, ..Default::default() };
    let rail = item(ItemKind::Rail, Opening::Cutout);
    let window = item(ItemKind::Cutout, Opening::Window);
    let door = item(ItemKind::Cutout, Opening::Door);
    let device = item(ItemKind::Device, Opening::Cutout);
    assert_eq!(face_summary(&[]), "empty");
    assert_eq!(face_summary(&[&window]), "1 window");
    assert_eq!(face_summary(&[&rail, &device, &device, &door]), "1 rail, 2 devices, 1 door");
}

#[test]
fn inner_walls_end_on_the_walls_of_the_room() {
    let room = panel::new_room(&mut Project::default(), "+ROOM1", 4030.0, 3000.0, 2800.0);
    let walls = panel::walls(&room);
    // The grid alone would leave 50 mm to the rear wall and 30 mm to the right one.
    assert_eq!(plan::inner_corner(&walls, None, p(2000.0, 40.0)), p(2000.0, 0.0));
    assert_eq!(plan::inner_corner(&walls, Some(p(2000.0, 1000.0)), p(4010.0, 1000.0)), p(4030.0, 1000.0));
    // Away from the walls the grid stays.
    assert_eq!(plan::inner_corner(&walls, Some(p(2000.0, 0.0)), p(2000.0, 1490.0)), p(2000.0, 1500.0));
}

#[test]
fn new_enclosures_and_rooms_take_free_names() {
    let mut prj = Project::default();
    let mut ui = PanelUi::default();
    ui.new_room(&mut prj);
    ui.new_enclosure_dialog(&prj, &PartsDb::default());
    assert_eq!(ui.new_enclosure.as_ref().map(|n| n.name.as_str()), Some("+CAB1"));
    ui.new_room(&mut prj);
    assert_eq!(prj.layout.enclosures[1].name, "+ROOM2");
}

#[test]
fn pipe_runs_and_the_floor_belong_to_rooms() {
    let mut prj = Project::default();
    let mut ui = PanelUi::default();
    // Not in a cabinet.
    let cab = panel::new_enclosure(&mut prj, panel::ENCLOSURE_PART, "+CAB1");
    prj.layout.enclosures.push(cab);
    ui.enclosure_index(&prj);
    ui.choose_tool(Tool::PipeRun);
    assert_eq!(ui.tool, Tool::Select);
    // In a room on any of its faces, the floor among them.
    ui.new_room(&mut prj);
    let ri = ui.enclosure_index(&prj).unwrap();
    assert!(prj.layout.enclosures[ri].faces().contains(&Face::Bottom));
    ui.front_face(Face::Bottom, false);
    ui.choose_tool(Tool::PipeRun);
    assert_eq!((ui.tool.clone(), ui.view, ui.face), (Tool::PipeRun, View::Front, Face::Bottom));
    // Corners at right angles, along the longer direction.
    assert_eq!(ortho(Some(p(0.0, 0.0)), p(500.0, 40.0)), p(500.0, 0.0));
    assert_eq!(ortho(Some(p(0.0, 0.0)), p(40.0, -500.0)), p(0.0, -500.0));
    assert_eq!(ortho(None, p(40.0, 50.0)), p(40.0, 50.0));
}

#[test]
fn a_device_from_the_schematic_is_shown_where_it_is_placed() {
    let prj = openecraft_core::sample::example("building_services").unwrap();
    let mut ui = PanelUi::default();
    assert!(!ui.focus_device(&prj, "=NOPE-X1", false), "not placed");
    // On the floor of a room: the floor plan with the device selected.
    let (ei, it) = panel::locate(&prj, "=WAT+TR-GP1").expect("booster pump placed");
    assert!(ui.focus_device(&prj, "=WAT+TR-GP1", false));
    assert_eq!((ui.enclosure, ui.view, ui.selection.iter().copied().collect::<Vec<_>>()), (ei, View::Plan, vec![it.id]));
    // In 3D the camera looks at it from close by.
    assert!(ui.focus_device(&prj, "=WAT+TR-GP1", true));
    assert_eq!(ui.view, View::ThreeD);
    let cam = ui.cam.expect("camera");
    let full = view3d::default_cam(&prj, &ui.shown_3d(&prj, ei));
    assert!(cam.dist < full.dist, "{} {}", cam.dist, full.dist);
    // On a mounting plate: the front view of its cabinet.
    let (ci, _) = panel::locate(&prj, "=CTL+CC1-A1").expect("PLC placed");
    assert!(ui.focus_device(&prj, "=CTL+CC1-A1", false));
    assert_eq!((ui.enclosure, ui.view, ui.face), (ci, View::Front, Face::Plate));
}
