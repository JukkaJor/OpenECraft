use super::*;
use openecraft_core::parts::vec3;

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
fn cable_cores_are_drawn_with_their_cable() {
    // The link cable between two batteries in the rack is drawn by the 3D routing; its
    // cores are not drawn again as wires over the rear of the rack.
    let prj = openecraft_core::sample::example("building_services").unwrap();
    let an = Analysis::checked(&prj);
    let rack = prj.layout.enclosures.iter().position(|e| e.name == "+RACK").unwrap();
    assert!(an.wiring.iter().any(|w| w.cable == "=ESS-W97" && w.in_enclosure(rack)));
    assert!(an.routing.item("=ESS-W97").is_some_and(|c| c.routed()));
    let mut ui = PanelUi::default();
    let shown = ui.shown_3d(&prj, rack);
    let scene = ui.scene_3d(&egui::Context::default(), &prj, &an, rack, &shown);
    let cores = scene.picks.iter().filter(|(_, _, t)| matches!(t, routing_ui::RouteRef::Wire(wi) if an.wiring[*wi].cable == "=ESS-W97")).count();
    assert_eq!(cores, 0);
    assert!(scene.picks.iter().any(|(_, _, t)| matches!(t, routing_ui::RouteRef::Item(key) if key == "=ESS-W97")));
    // The battery cable parts in front of the battery into its cores to + and -: each a
    // smooth curve out of the end of the cable into its connection point, coming in
    // against the direction it leaves it in; they are seen without the cable being
    // selected.
    let w71 = an.routing.item("=ESS-W71").unwrap();
    let end = w71.ends.iter().find(|e| e.tag == "=ESS+RACK-G1").unwrap();
    assert_eq!((end.cores.len(), end.exits.len()), (2, 2));
    let lines = ui.routing_lines_3d(&prj, &an, &[rack], &|_, q| q, None);
    let lines: Vec<_> = lines.iter().filter(|l| matches!(&l.target, routing_ui::RouteRef::Item(k) if k == "=ESS-W71")).collect();
    let (pos, (_, pose)) = (panel::positions(&prj.layout)[rack], shown[0]);
    let view = |q: [f32; 3]| pose.apply([q[0] - pos[0], prj.layout.enclosures[rack].height - (q[1] - pos[1]), q[2] - pos[2]]);
    for (core, exit) in end.cores.iter().zip(&end.exits) {
        let line = lines.iter().find(|l| l.points.last() == Some(core) && l.diameter < w71.diameter).unwrap_or_else(|| panic!("{core:?}"));
        let dirs: Vec<[f32; 3]> = line.points.windows(2).map(|s| vec3::norm(vec3::sub(s[1], s[0]))).collect();
        assert!(dirs.windows(2).all(|d| vec3::dot(d[0], d[1]) > 0.7), "{:?}", line.points);
        assert!(vec3::dot(dirs[dirs.len() - 1], vec3::norm(*exit)) < -0.9, "{:?}", line.points);
        let start = line.points[0];
        assert!(lines.iter().any(|l| l.diameter == w71.diameter && l.points.iter().any(|q| vec3::dist(*q, start) <= w71.diameter / 2.0 + 0.01)), "{start:?}");
        let at = view(*core);
        assert!(scene.wires.iter().any(|w| [w.line.a, w.line.b].iter().any(|q| (0..3).all(|k| (q[k] - at[k]).abs() < 0.5)) && w.line.a != w.line.b), "{core:?} not drawn");
    }
}

#[test]
fn cables_run_on_through_the_cable_glands_into_the_room() {
    // The motor cable of pump 1 leaves its cabinet through the gland in the roof: the
    // stretch through the cabinet and the one through the room are one line through it.
    let prj = openecraft_core::sample::example("pump_room").unwrap();
    let an = Analysis::build(&prj);
    let room = prj.layout.enclosures.iter().position(|e| e.is_room()).unwrap();
    let w1 = an.routing.item("=P1-W1").unwrap();
    assert_eq!(w1.legs.len(), 2);
    let mut ui = PanelUi { show_wires: true, ..Default::default() };
    let shown = ui.shown_3d(&prj, room);
    let scene = ui.scene_3d(&egui::Context::default(), &prj, &an, room, &shown);
    let k = w1.legs[0].enclosure;
    let (pos, pose) = (panel::positions(&prj.layout)[k], shown.iter().find(|s| s.0 == k).unwrap().1);
    let gland = *w1.legs[0].points.last().unwrap();
    let g = pose.apply([gland[0] - pos[0], prj.layout.enclosures[k].height - (gland[1] - pos[1]), gland[2] - pos[2]]);
    // A piece of the cable crosses the roof just above the gland.
    let above = g[1] + 20.0;
    let through = scene.picks.iter().filter(|(_, _, t)| matches!(t, routing_ui::RouteRef::Item(key) if key == "=P1-W1")).any(|(a, b, _)| {
        let (lo, hi) = if a[1] < b[1] { (a, b) } else { (b, a) };
        if !(lo[1] <= above && hi[1] >= above) || hi[1] - lo[1] < 1e-3 {
            return false;
        }
        let t = (above - lo[1]) / (hi[1] - lo[1]);
        let q = [0, 1, 2].map(|i| lo[i] + (hi[i] - lo[i]) * t);
        (q[0] - g[0]).hypot(q[2] - g[2]) < 30.0
    });
    assert!(through, "no piece of the cable through the gland at {g:?}");
}

#[test]
fn wires_run_through_the_cable_passages_in_one_bend() {
    // Wires from +CAB1 to +CAB2 of the pump room pass the cable passage between them
    // bent round, not with a corner where their stretches in the two cabinets meet.
    let prj = openecraft_core::sample::example("pump_room").unwrap();
    let an = Analysis::build(&prj);
    let mut ui = PanelUi { show_wires: true, ..Default::default() };
    let shown = ui.shown_3d(&prj, 0);
    let scene = ui.scene_3d(&egui::Context::default(), &prj, &an, 0, &shown);
    let pos = panel::positions(&prj.layout);
    let mut checked = 0;
    for (wi, w) in an.wiring.iter().enumerate().filter(|(_, w)| w.legs.len() == 1 && w.cable.is_empty()) {
        let (k, pose) = shown.iter().copied().find(|s| s.0 == w.enclosure).unwrap();
        let q = w.legs[0].passage;
        let p = pose.apply([q[0] - pos[k][0], prj.layout.enclosures[k].height - (q[1] - pos[k][1]), q[2] - pos[k][2]]);
        let segs: Vec<_> = scene.picks.iter().filter(|(_, _, t)| matches!(t, routing_ui::RouteRef::Wire(i) if *i == wi)).collect();
        let near = |a: [f32; 3]| vec3::dist(a, p) < 40.0;
        for s in segs.windows(2).filter(|s| vec3::dist(s[0].1, s[1].0) < 0.01 && near(s[0].1)) {
            let (u, v) = (vec3::norm(vec3::sub(s[0].1, s[0].0)), vec3::norm(vec3::sub(s[1].1, s[1].0)));
            assert!(vec3::dot(u, v) > 0.6, "{} turns sharply at {:?}", w.number, s[0].1);
            checked += 1;
        }
    }
    assert!(checked > 0);
}

#[test]
fn routed_pipes_are_round_tubes_bent_round_their_corners() {
    let prj = openecraft_core::sample::example("pump_room").unwrap();
    let an = Analysis::build(&prj);
    let room = prj.layout.enclosures.iter().position(|e| e.is_room()).unwrap();
    let pipe = an.routing.items.iter().find(|i| i.kind == routing3d::RouteKind::Tube && i.routed() && i.points.len() > 3).expect("a pipe with corners");
    let mut ui = PanelUi { show_wires: true, ..Default::default() };
    let shown = ui.shown_3d(&prj, room);
    let scene = ui.scene_3d(&egui::Context::default(), &prj, &an, room, &shown);
    assert!(!scene.tubes.is_empty());
    // Its corners are arcs: more segments than the route has corners.
    let segments: Vec<_> = scene.picks.iter().filter(|(_, _, t)| matches!(t, routing_ui::RouteRef::Item(k) if *k == pipe.key)).collect();
    assert!(segments.len() > pipe.points.len(), "{} segments, {} points", segments.len(), pipe.points.len());
    assert!(scene.wires.iter().any(|w| w.line.tube && w.line.diameter == pipe.diameter));
}

#[test]
fn cables_are_picked_on_the_plate_like_wires() {
    let prj = openecraft_core::sample::demo_project();
    let an = Analysis::checked(&prj);
    let pos = panel::positions(&prj.layout);
    // The middle of a segment of a cable's route over the plate of a cabinet.
    let (ei, index, q) = an
        .routing
        .items
        .iter()
        .enumerate()
        .filter(|(_, it)| it.kind == routing3d::RouteKind::Cable && it.routed())
        .find_map(|(i, it)| {
            let ei = *it.enclosures.first()?;
            let pts = it.stretches_in_lineup(&prj.layout, ei).into_iter().find(|s| s.len() >= 2)?;
            let (a, b) = (pts[0], pts[1]);
            Some((ei, i, p((a[0] + b[0]) / 2.0 - pos[ei][0] - PLATE_MARGIN, (a[1] + b[1]) / 2.0 - pos[ei][1] - PLATE_MARGIN)))
        })
        .expect("a cable over a plate");
    let mut ui = PanelUi::default();
    assert!(ui.cables_at(&prj, &an, ei, q, 2.0).contains(&index));
    assert!(ui.cables_at(&prj, &an, ei, p(-5000.0, -5000.0), 2.0).is_empty());
    // Picked, it is shown as in the 3D view: no wire or item selected with it.
    ui.selection.insert(7);
    ui.selected_wire = Some(("-K1:A1".into(), "-K2:A1".into()));
    ui.select_route(&an, &an.routing.items[index].key);
    assert_eq!(ui.selected_route.as_deref(), Some(an.routing.items[index].key.as_str()));
    assert!(ui.selected_wire.is_none() && ui.selection.is_empty() && ui.status.contains(&an.routing.items[index].label));
}

#[test]
fn wires_entering_the_ducts_take_out_the_fingers_they_pass() {
    // Shown, the wires pass through the walls of the ducts: the fingers there are left out.
    let prj = openecraft_core::sample::demo_project();
    let an = Analysis::build(&prj);
    let faces = |show_wires: bool| {
        let mut ui = PanelUi { show_wires, ..Default::default() };
        let shown = ui.shown_3d(&prj, 0);
        ui.scene_3d(&egui::Context::default(), &prj, &an, 0, &shown).faces.len()
    };
    let (hidden, shown) = (faces(false), faces(true));
    assert!(shown < hidden && (hidden - shown) % 6 == 0, "{hidden} faces, {shown} with the wires");
}

#[test]
fn wires_running_together_lie_side_by_side() {
    // In the ducts of the demo cabinet many wires run the same way: no two straight runs
    // of different wires lie inside each other (but at their ends, where they leave their
    // devices over them).
    let prj = openecraft_core::sample::demo_project();
    let an = Analysis::build(&prj);
    let mut ui = PanelUi { show_wires: true, ..Default::default() };
    let shown = ui.shown_3d(&prj, 0);
    let scene = ui.scene_3d(&egui::Context::default(), &prj, &an, 0, &shown);
    let axis = |a: [f32; 3], b: [f32; 3]| {
        let along: Vec<usize> = (0..3).filter(|&k| (a[k] - b[k]).abs() > 0.01).collect();
        (along.len() == 1 && (a[along[0]] - b[along[0]]).abs() > 30.0).then(|| along[0])
    };
    // The pieces of each wire without the stretch at either end of each piece.
    type Segment = ([f32; 3], [f32; 3]);
    let mut pieces: Vec<(usize, Vec<Segment>)> = Vec::new();
    for (a, b, t) in &scene.picks {
        let routing_ui::RouteRef::Wire(wi) = t else { continue };
        match pieces.last_mut() {
            Some((w, segs)) if w == wi && segs.last().is_some_and(|s| s.1 == *a) => segs.push((*a, *b)),
            _ => pieces.push((*wi, vec![(*a, *b)])),
        }
    }
    let runs: Vec<(usize, usize, [f32; 3], [f32; 3])> = pieces
        .iter()
        .flat_map(|(wi, segs)| segs.iter().skip(1).take(segs.len().saturating_sub(2)).filter_map(move |(a, b)| axis(*a, *b).map(|k| (*wi, k, *a, *b))))
        .collect();
    let radius = |wi: usize| openecraft_core::harness::outer_diameter(&an.wiring[wi].cross_section) / 2.0;
    let mut together = 0;
    for (i, &(w1, k1, a1, b1)) in runs.iter().enumerate() {
        for &(w2, k2, a2, b2) in &runs[i + 1..] {
            if w1 == w2 || k1 != k2 {
                continue;
            }
            let overlap = a1[k1].max(b1[k1]).min(a2[k1].max(b2[k1])) - a1[k1].min(b1[k1]).max(a2[k1].min(b2[k1]));
            let apart = (0..3).filter(|&k| k != k1).map(|k| (a1[k] - a2[k]).powi(2)).sum::<f32>().sqrt();
            if overlap > 10.0 && apart < radius(w1) + radius(w2) + 2.0 {
                together += 1;
                assert!(apart >= radius(w1) + radius(w2) - 0.01, "{} and {} inside each other: {a1:?} {b1:?} / {a2:?} {b2:?}", an.wiring[w1].number, an.wiring[w2].number);
            }
        }
    }
    assert!(together > 5, "{together} runs side by side");
}

#[test]
fn wires_and_cables_laid_side_by_side_keep_in_front_of_the_plate() {
    // In the cabinets of the demo and of the building (their view: the plate in front of
    // z 0) every wire and cable drawn inside stays in front of the mounting plate with
    // its whole thickness.
    let demo = openecraft_core::sample::demo_project();
    let building = openecraft_core::sample::example("building_services").unwrap();
    for (prj, name) in [(&demo, "+CAB1"), (&building, "+CC1")] {
        let an = Analysis::build(prj);
        let ei = prj.layout.enclosures.iter().position(|e| e.name.ends_with(name)).unwrap();
        let e = &prj.layout.enclosures[ei];
        let mut ui = PanelUi { enclosure: ei, show_wires: true, ..Default::default() };
        let shown = ui.shown_3d(prj, ei);
        let scene = ui.scene_3d(&egui::Context::default(), prj, &an, ei, &shown);
        let inside = |q: &[f32; 3]| q[0] > 0.0 && q[0] < e.width && q[1] > 0.0 && q[1] < e.height && q[2] < e.depth;
        let mut checked = 0;
        for (a, b, t) in &scene.picks {
            let radius = match t {
                routing_ui::RouteRef::Wire(wi) if an.wiring[*wi].door == (None, None) => openecraft_core::harness::outer_diameter(&an.wiring[*wi].cross_section) / 2.0,
                routing_ui::RouteRef::Item(key) => an.routing.item(key).map_or(0.0, |i| i.diameter / 2.0),
                _ => continue,
            };
            for q in [a, b].into_iter().filter(|q| inside(q)) {
                assert!(q[2] >= PLATE_Z + radius - 0.01, "{name}: {q:?} behind the plate (a cable: {})", matches!(t, routing_ui::RouteRef::Item(_)));
                checked += 1;
            }
        }
        assert!(checked > 100, "{name}: {checked}");
    }
}

#[test]
fn wires_pass_in_front_of_the_busbars() {
    // In the main distribution of the building the wires from the circuit breakers to
    // the duct above them cross the busbar: in front of it, not through it.
    let prj = openecraft_core::sample::example("building_services").unwrap();
    let an = Analysis::build(&prj);
    let ei = prj.layout.enclosures.iter().position(|e| e.items.iter().any(|i| i.kind == ItemKind::Busbar)).expect("busbars");
    let e = &prj.layout.enclosures[ei];
    let mut ui = PanelUi { enclosure: ei, show_wires: true, ..Default::default() };
    let shown = ui.shown_3d(&prj, ei);
    let pose = shown.iter().find(|s| s.0 == ei).unwrap().1;
    let scene = ui.scene_3d(&egui::Context::default(), &prj, &an, ei, &shown);
    let busbars: Vec<([f32; 3], [f32; 3])> = e
        .items
        .iter()
        .filter(|i| i.kind == ItemKind::Busbar)
        .map(|i| {
            let (a, b) = panel::item_box(e, i);
            let (lo, hi) = (pose.apply([a[0], e.height - b[1], a[2]]), pose.apply([b[0], e.height - a[1], b[2]]));
            ([0, 1, 2].map(|k| lo[k].min(hi[k]) + 0.5), [0, 1, 2].map(|k| lo[k].max(hi[k]) - 0.5))
        })
        .collect();
    let mut crossing = 0;
    for (a, b, t) in &scene.picks {
        if !matches!(t, routing_ui::RouteRef::Wire(_)) {
            continue;
        }
        for bx in &busbars {
            if openecraft_core::parts::vec3::segment_in_box(*a, *b, *bx).is_some() {
                crossing += 1;
            }
        }
    }
    assert_eq!(crossing, 0, "wire segments through a busbar");
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

/// Shapes of one frame of the 3D view of the room of `prj`, and the smallest alpha of
/// a filled shape.
fn room_frame(ui: &mut PanelUi, prj: &mut Project, an: &openecraft_core::analysis::Analysis) -> (usize, u8) {
    let room = prj.layout.enclosures.iter().position(|e| e.is_room()).unwrap();
    (ui.enclosure, ui.view) = (room, View::ThreeD);
    let ctx = egui::Context::default();
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 800.0))), ..Default::default() };
    let mut out = ctx.run_ui(input, |ui_| ui.view_3d(ui_, prj, room, an));
    out.textures_delta.clear();
    let mut alpha = 255;
    for s in &out.shapes {
        if let egui::Shape::Path(p) = &s.shape {
            alpha = alpha.min(p.fill.a());
        }
    }
    (out.shapes.len(), alpha)
}

#[test]
fn walls_in_front_and_the_ceiling_can_be_left_out() {
    let mut prj = openecraft_core::sample::example("pump_room").unwrap();
    let an = openecraft_core::analysis::Analysis::build(&prj);
    let mut ui = PanelUi::default();
    let (all, alpha) = room_frame(&mut ui, &mut prj, &an);
    assert!(alpha < 200, "walls in front are see-through");
    ui.hide_ceiling = true;
    let (no_ceiling, _) = room_frame(&mut ui, &mut prj, &an);
    assert!(no_ceiling < all, "{no_ceiling} {all}");
    ui.hide_front_walls = true;
    let (open, _) = room_frame(&mut ui, &mut prj, &an);
    assert!(open < no_ceiling, "{open} {no_ceiling}");
    // Opaque walls in front.
    let mut ui = PanelUi { wall_opacity: 1.0, ..Default::default() };
    assert_eq!(room_frame(&mut ui, &mut prj, &an).1, 255);
}

#[test]
fn wires_take_their_own_colours() {
    let rgb = |c: [u8; 3]| Color32::from_rgb(c[0], c[1], c[2]);
    let blue = rgb(openecraft_core::potentials::color_rgb("BU").unwrap());
    assert_eq!(wire_look("BU", false, false, false), (blue, None, 1.5));
    let (green, yellow) = (rgb(openecraft_core::potentials::color_rgb("GN").unwrap()), rgb(openecraft_core::potentials::color_rgb("YE").unwrap()));
    assert_eq!(wire_look("gn/ye", false, false, false), (green, Some(yellow), 1.5), "two colours: a stripe");
    assert_eq!(wire_look("", false, false, false).0, WIRE_COLOR, "no colour code");
    assert_eq!(wire_look("BU", true, false, false).0, WIRE_HIGHLIGHT);
    assert_eq!(wire_look("BU", false, true, false).0, WIRE_DONE);
    assert_eq!(wire_look("BU", false, false, true).0, blue.gamma_multiply(0.3), "dimmed while another is selected");
}

#[test]
fn wires_in_3d_have_their_colour_and_size() {
    let mut prj = openecraft_core::sample::example("cabinet_lineup").unwrap();
    let an = openecraft_core::analysis::Analysis::build(&prj);
    let colors: std::collections::HashSet<Color32> = an.wiring.iter().filter_map(|w| openecraft_core::potentials::color_rgb(&w.color)).map(|c| Color32::from_rgb(c[0], c[1], c[2])).collect();
    assert!(colors.len() > 1, "the example has wires of several colours");
    let e = an.wiring[0].enclosure;
    // Lines of one frame of the 3D view: their colours and the widest.
    let frame = |ui: &mut PanelUi, prj: &mut Project| {
        let ctx = egui::Context::default();
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 800.0))), ..Default::default() };
        let mut out = ctx.run_ui(input, |ui_| ui.view_3d(ui_, prj, e, &an));
        out.textures_delta.clear();
        let mut seen = std::collections::HashSet::new();
        let mut widest = 0.0f32;
        for s in &out.shapes {
            if let egui::Shape::LineSegment { stroke, .. } = &s.shape {
                seen.insert(stroke.color);
                widest = widest.max(stroke.width);
            }
        }
        (seen, widest)
    };
    let mut ui = PanelUi { enclosure: e, view: View::ThreeD, ..Default::default() };
    let (seen, far) = frame(&mut ui, &mut prj);
    assert!(colors.iter().filter(|c| seen.contains(c)).count() > 1, "drawn in their colours");
    // Closer, the wires are thicker on the screen.
    let cam = ui.cam.as_mut().unwrap();
    cam.dist *= 0.2;
    let (_, near) = frame(&mut ui, &mut prj);
    assert!(near > far, "{near} {far}");
}

/// Draws one frame of the 3D view of the room of `prj`; the shapes it painted.
fn room_shapes(ui: &mut PanelUi, prj: &mut Project, an: &openecraft_core::analysis::Analysis, yaw: f32) -> Vec<egui::Shape> {
    let room = prj.layout.enclosures.iter().position(|e| e.is_room()).unwrap();
    (ui.enclosure, ui.view) = (room, View::ThreeD);
    if let Some(cam) = ui.cam.as_mut() {
        cam.yaw = yaw;
    }
    let ctx = egui::Context::default();
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 800.0))), ..Default::default() };
    let mut out = ctx.run_ui(input, |ui_| ui.view_3d(ui_, prj, room, an));
    out.textures_delta.clear();
    out.shapes.into_iter().map(|s| s.shape).collect()
}

#[test]
fn the_3d_scene_is_built_again_only_when_what_it_shows_changes() {
    let mut prj = openecraft_core::sample::example("pump_room").unwrap();
    let an = openecraft_core::analysis::Analysis::build(&prj);
    let mut ui = PanelUi::default();
    let id = |ui: &PanelUi| ui.scene.as_ref().map(|s| s.id);
    let first = room_shapes(&mut ui, &mut prj, &an, 0.5);
    let built = id(&ui);
    assert!(built.is_some());
    // Turning the camera only projects the scene again.
    let turned = room_shapes(&mut ui, &mut prj, &an, 1.5);
    assert_eq!(id(&ui), built);
    assert_ne!(first.len(), 0);
    assert_ne!(format!("{first:?}"), format!("{turned:?}"));
    // A selection, the ceiling, a moved item and a new analysis build it again.
    let room = ui.enclosure;
    let item = prj.layout.enclosures[room].items[0].id;
    ui.selection.insert(item);
    room_shapes(&mut ui, &mut prj, &an, 1.5);
    let selected = id(&ui);
    assert_ne!(selected, built);
    ui.hide_ceiling = true;
    room_shapes(&mut ui, &mut prj, &an, 1.5);
    assert_ne!(id(&ui), selected);
    let hidden = id(&ui);
    prj.layout.enclosures[room].items[0].x += 10.0;
    room_shapes(&mut ui, &mut prj, &an, 1.5);
    assert_ne!(id(&ui), hidden);
    let moved = id(&ui);
    room_shapes(&mut ui, &mut prj, &an, 1.5);
    assert_eq!(id(&ui), moved);
    let an = openecraft_core::analysis::Analysis::build(&prj);
    room_shapes(&mut ui, &mut prj, &an, 1.5);
    assert_ne!(id(&ui), moved);
}

#[test]
fn the_graphics_card_draws_the_scene_and_egui_the_rest() {
    let mut prj = openecraft_core::sample::example("pump_room").unwrap();
    let an = openecraft_core::analysis::Analysis::build(&prj);
    let mut painted = PanelUi::default();
    let mut gpu = PanelUi { gpu_3d: true, ..Default::default() };
    let shapes = room_shapes(&mut gpu, &mut prj, &an, 0.5);
    let callbacks = shapes.iter().filter(|s| matches!(s, egui::Shape::Callback(_))).count();
    assert_eq!(callbacks, 1);
    // No faces painted by egui: the background, labels, outlines and the hint only.
    let all = room_shapes(&mut painted, &mut prj, &an, 0.5);
    assert!(shapes.len() * 4 < all.len(), "{} {}", shapes.len(), all.len());
    assert!(!all.iter().any(|s| matches!(s, egui::Shape::Callback(_))));
}

#[test]
fn clicking_in_the_3d_view_picks_from_the_scene_with_the_graphics_card_too() {
    let mut prj = openecraft_core::sample::example("pump_room").unwrap();
    let an = openecraft_core::analysis::Analysis::build(&prj);
    for gpu_3d in [false, true] {
        let mut ui = PanelUi { gpu_3d, ..Default::default() };
        room_shapes(&mut ui, &mut prj, &an, 0.5);
        let room = ui.enclosure;
        let scene = ui.scene.clone().unwrap();
        let cam = ui.cam.unwrap();
        let (eye, right, up, fwd) = cam.basis();
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 800.0));
        let camera = gpu3d::Camera { eye, right, up, fwd, f: rect.height() * 1.3 };
        let prims = view3d::project(&scene, &camera, rect, ui.wall_opacity, false);
        // The centre of a device face that nothing picked is drawn over, clicked.
        let centre = |pts: &[egui::Pos2]| (pts.iter().fold(egui::Vec2::ZERO, |a, q| a + q.to_vec2()) / pts.len() as f32).to_pos2();
        let top = |q: egui::Pos2| prims.iter().rev().find_map(|p| match p {
            view3d::Prim::Face { pts, n, item: Some(item), .. } if super::view3d::point_in_convex(q, &pts[..*n as usize]) => Some(item.1),
            _ => None,
        });
        let target = prims.iter().rev().find_map(|p| match p {
            view3d::Prim::Face { pts, n, item: Some((k, id)), .. } if prj.layout.enclosures[*k].item(*id).is_some_and(|i| i.kind == ItemKind::Device) => {
                let at = centre(&pts[..*n as usize]);
                (top(at) == Some(*id) && rect.contains(at)).then_some((at, *id))
            }
            _ => None,
        });
        let Some((at, id)) = target else { panic!("no device face") };
        let ctx = egui::Context::default();
        for (t, pressed) in [(0.0, None), (0.1, Some(true)), (0.15, Some(false))] {
            let mut events = vec![egui::Event::PointerMoved(at)];
            events.extend(pressed.map(|pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() }));
            let input = egui::RawInput { screen_rect: Some(rect), time: Some(t), events, ..Default::default() };
            ctx.run_ui(input, |ui_| ui.view_3d(ui_, &mut prj, room, &an)).textures_delta.clear();
        }
        assert!(ui.selection.contains(&id), "gpu {gpu_3d}");
    }
}

#[test]
fn a_selected_cable_shows_its_properties() {
    // As a selected wire: the panel shows the cable with its route and the devices at
    // its ends, not the properties of the enclosure.
    let mut prj = openecraft_core::sample::demo_project();
    let an = Analysis::checked(&prj);
    let cable = an.routing.items.iter().find(|i| i.kind == routing3d::RouteKind::Cable && i.routed()).expect("a routed cable");
    let device = cable.ends.iter().filter_map(|e| prj.layout.enclosures.get(e.enclosure?)?.item(e.item?)).find(|i| !i.tag.is_empty()).expect("a device at an end").tag.clone();
    // Its cores with the connection points they join.
    let core = an.connections.iter().find(|c| c.cable == cable.key).expect("a core of the cable");
    let points = [core.a, core.b].map(|pin| openecraft_core::netlist::pin_designation(&prj, pin));
    let mut ui = PanelUi { selected_route: Some(cable.key.clone()), ..Default::default() };
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |c| {
        egui::CentralPanel::default().show(c, |u| ui.properties(u, &mut prj, &an, &PartsDb::default()));
    });
    fn texts(s: &egui::Shape, out: &mut Vec<String>) {
        match s {
            egui::Shape::Text(t) => out.push(t.galley.job.text.clone()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
            _ => {}
        }
    }
    out.textures_delta.clear();
    let mut shown = Vec::new();
    out.shapes.iter().for_each(|c| texts(&c.shape, &mut shown));
    for want in [cable.label.as_str(), tr("Free air"), tr("Connected devices"), device.as_str(), tr("Cores"), points[0].as_str(), points[1].as_str()] {
        assert!(shown.iter().any(|t| t.contains(want)), "{want} in {shown:?}");
    }
    assert!(!shown.iter().any(|t| t == tr("Enclosure")), "{shown:?}");
}

#[test]
fn wires_are_laid_out_on_another_thread_with_their_progress_shown() {
    let prj = openecraft_core::sample::demo_project();
    let an = Analysis::build(&prj);
    let ctx = egui::Context::default();
    let mut ui = PanelUi { show_wires: true, ..Default::default() };
    ui.layouts.wait = 0;
    let shown = ui.shown_3d(&prj, 0);
    let first = ui.scene_3d(&ctx, &prj, &an, 0, &shown);
    // With threads the scene does not wait for the layout: its wires are shown as routed
    // until it is done (without threads it is done at once).
    if let Some(key) = first.waiting {
        assert!(ui.layouts.running(key) && ui.layouts.progress().is_some_and(|p| (0.0..=1.0).contains(&p)));
        while ui.layouts.progress().is_some() {
            ui.layouts.collect();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    let laid = ui.scene_3d(&ctx, &prj, &an, 0, &shown);
    assert!(laid.waiting.is_none() && ui.layouts.progress().is_none());
    assert!(first.waiting.is_none() || laid.id != first.id);
    assert_eq!(ui.scene_3d(&ctx, &prj, &an, 0, &shown).id, laid.id, "kept");
    // A selection draws it again with the same layout, none worked out again.
    let wire = an.wiring.iter().position(|w| w.in_enclosure(0)).unwrap();
    ui.selected_wire = Some((an.wiring[wire].from.clone(), an.wiring[wire].to.clone()));
    let selected = ui.scene_3d(&ctx, &prj, &an, 0, &shown);
    assert!(selected.id != laid.id && selected.waiting.is_none() && ui.layouts.progress().is_none());
    // During a gesture none is worked out: shown as routed.
    let mut moved = prj.clone();
    let device = moved.layout.enclosures[0].items.iter().position(|i| i.kind == ItemKind::Device).unwrap();
    moved.layout.enclosures[0].items[device].x += 30.0;
    ui.drag = Some(MoveDrag { start: openecraft_core::geom::p(0.0, 0.0), originals: Vec::new() });
    let dragged = ui.scene_3d(&ctx, &moved, &an, 0, &shown);
    assert!(dragged.waiting.is_some() && ui.layouts.progress().is_none());
}

#[test]
fn cores_part_from_their_cable_without_lying_in_each_other() {
    // The five cores of the main cable to the AC distribution part in front of its
    // terminals: each from its own place in the cable, none inside another where they
    // start, thinner than half the cable.
    let prj = openecraft_core::sample::example("building_services").unwrap();
    let an = Analysis::build(&prj);
    let acd = prj.layout.enclosures.iter().position(|e| e.name == "+ACD").unwrap();
    let w12 = an.routing.item("=MD-W12").unwrap();
    let ui = PanelUi::default();
    let lines = ui.routing_lines_3d(&prj, &an, &[acd], &|_, q| q, None);
    let at = w12.ends.iter().find(|e| e.tag == "=ESS+ACD-X0").unwrap();
    let cores: Vec<_> = lines.iter().filter(|l| l.as_is && at.cores.iter().any(|q| l.points.last() == Some(q))).collect();
    assert_eq!(cores.len(), 5);
    for (i, a) in cores.iter().enumerate() {
        assert!(a.diameter < w12.diameter * 0.5);
        for b in &cores[i + 1..] {
            assert!(vec3::dist(a.points[0], b.points[0]) >= a.diameter - 0.01, "{:?} {:?}", a.points[0], b.points[0]);
        }
    }
}
