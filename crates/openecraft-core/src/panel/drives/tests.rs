use super::*;
use crate::panel::{device_mesh, new_room};

/// A room with a machine `-GP1` driven by the motor `-M1`, both standing on the floor
/// at the given places (rear left corners on the floor plan).
fn room_with(machine: (&str, [f32; 2]), motor: (&str, [f32; 2]), drive: &str) -> Project {
    let mut prj = Project::default();
    let room = new_room(&mut prj, "+R1", 5000.0, 4000.0, 2800.0);
    prj.layout.enclosures.push(room);
    for (i, (tag, (part, at))) in [("-GP1", machine), ("-M1", motor)].into_iter().enumerate() {
        let p = part_by_number(&prj, part).unwrap();
        let (w, h, d) = (p.width, p.depth, p.height);
        let it = PanelItem { id: i as u64 + 1, kind: ItemKind::Device, tag: tag.into(), part: part.into(), x: at[0], y: at[1], w, h, d, face: Face::Bottom, ..Default::default() };
        prj.layout.enclosures[0].items.push(it);
    }
    let values = &mut prj.devices.entry("-GP1".into()).or_default().values;
    values.insert(DRIVEN_BY.into(), "-M1".into());
    if !drive.is_empty() {
        values.insert(DRIVE.into(), drive.into());
    }
    prj
}

fn items(prj: &Project) -> (&PanelItem, &PanelItem) {
    let e = &prj.layout.enclosures[0];
    (&e.items[0], &e.items[1])
}

/// The parts of the drive in floor coordinates.
fn parts_of(prj: &Project) -> (DriveKind, Mesh3d) {
    let (machine, motor) = items(prj);
    let (_, kind, mesh) = drive_mesh(prj, machine, motor).unwrap();
    (kind, mesh)
}

fn bounds_of(m: &Mesh3d, color: Option<[u8; 3]>) -> (V3, V3) {
    let tris = (0..m.triangles.len()).filter(|&i| color.is_none() || m.color(i) == color);
    let pts: Vec<V3> = tris.flat_map(|i| m.triangles[i].map(|v| m.vertices[v as usize])).collect();
    assert!(!pts.is_empty(), "{color:?}");
    let lo = [0, 1, 2].map(|k| pts.iter().map(|q| q[k]).fold(f32::INFINITY, f32::min));
    let hi = [0, 1, 2].map(|k| pts.iter().map(|q| q[k]).fold(f32::NEG_INFINITY, f32::max));
    (lo, hi)
}

/// Enclosed volume: positive for a closed mesh facing outwards.
fn volume(m: &Mesh3d) -> f32 {
    m.triangles.iter().map(|t| t.map(|i| m.vertices[i as usize])).map(|[a, b, c]| dot(a, cross(b, c)) / 6.0).sum()
}

#[test]
fn a_motor_in_line_turns_its_pump_through_a_coupling() {
    // The pump's shaft comes out of its front (plan y 1640, x 1160); the motor stands in
    // line in front of it with its own shaft at its front: turned end for end.
    let prj = room_with(("GEN.PUMP-CENT-DN25", [1000.0, 1000.0]), ("GEN.MOT-2.2KW", [1045.0, 1700.0]), "");
    let found = drives(&prj, &prj.layout.enclosures[0]);
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].machine, found[0].motor, found[0].kind, found[0].turned), (1, 2, DriveKind::Coupling, true));
    assert!(!found[0].triangles.is_empty());
    let (_, mesh) = parts_of(&prj);
    let (lo, hi) = bounds_of(&mesh, None);
    assert!((lo[1] - 1640.0).abs() < 0.5 && (hi[1] - 1700.0).abs() < 0.5, "{lo:?} {hi:?}");
    assert!(lo[0] > 1100.0 && hi[0] < 1220.0 && lo[2] > 50.0, "{lo:?} {hi:?}");
    let (slo, shi) = bounds_of(&mesh, Some(SPIDER));
    assert!(slo[1] > 1660.0 && shi[1] < 1680.0, "the spider in the middle: {slo:?} {shi:?}");
    // The motor's model turned with it: the thin shaft at its rear (towards the pump),
    // the wide fan cover at its front.
    let (_, motor) = items(&prj);
    assert_eq!(turned_motor(&prj, motor), Some((1, 1865.0)));
    let tris = device_mesh(&prj, motor).unwrap();
    let spread = |y: f32| {
        let xs: Vec<f32> = tris.iter().flatten().filter(|v| (v[1] - y).abs() < 5.0).map(|v| v[0]).collect();
        xs.iter().copied().fold(f32::NEG_INFINITY, f32::max) - xs.iter().copied().fold(f32::INFINITY, f32::min)
    };
    assert!(spread(1700.0) < 40.0 && spread(2030.0) > 100.0, "{} {}", spread(1700.0), spread(2030.0));
}

#[test]
fn a_motor_in_line_behind_the_pump_or_far_away_drives_nothing() {
    // Behind the pump (its shaft points the other way), in another room, or not named.
    let behind = room_with(("GEN.PUMP-CENT-DN25", [1000.0, 1000.0]), ("GEN.MOT-2.2KW", [1045.0, 600.0]), "");
    assert!(drives(&behind, &behind.layout.enclosures[0]).is_empty());
    let mut elsewhere = room_with(("GEN.PUMP-CENT-DN25", [1000.0, 1000.0]), ("GEN.MOT-2.2KW", [1045.0, 1700.0]), "");
    elsewhere.devices.get_mut("-GP1").unwrap().values.insert(DRIVEN_BY.into(), "-M2".into());
    assert!(drives(&elsewhere, &elsewhere.layout.enclosures[0]).is_empty());
    assert_eq!(turned_motor(&elsewhere, items(&elsewhere).1), None);
}

#[test]
fn a_motor_beside_its_compressor_drives_it_through_v_belts_or_gears() {
    // The crankshaft comes out of the compressor's front at x 1715 (plan y 1500); the
    // motor stands right of it, its shaft at its front (y 1480) as well.
    let compressor = ("GEN.COMP-PISTON-450", [1000.0, 1000.0]);
    let motor = ("GEN.MOT-4KW", [2200.0, 1100.0]);
    for drive in ["", "Belt", "Coupling"] {
        let prj = room_with(compressor, motor, drive);
        let found = drives(&prj, &prj.layout.enclosures[0]);
        assert_eq!((found.len(), found[0].kind, found[0].turned), (1, DriveKind::Belt, false), "{drive}");
    }
    let (_, mesh) = parts_of(&room_with(compressor, motor, ""));
    // Pulleys and belts in front of both, the belts around both pulleys.
    let (plo, _) = bounds_of(&mesh, Some(PULLEY));
    let (blo, bhi) = bounds_of(&mesh, Some(BELT));
    assert!(plo[1] > 1500.0 && blo[1] > 1500.0, "{plo:?} {blo:?}");
    assert!(blo[0] < 1715.0 - 100.0 && bhi[0] > 2340.0 + 30.0, "{blo:?} {bhi:?}");
    assert!(blo[2] > 0.0 && bhi[2] > 600.0, "{blo:?} {bhi:?}");
    // A gear case instead.
    let prj = room_with(compressor, motor, "Gear");
    let (kind, mesh) = parts_of(&prj);
    assert_eq!(kind, DriveKind::Gear);
    let (clo, chi) = bounds_of(&mesh, Some(CASE));
    assert!(clo[1] > 1500.0 && clo[0] < 1715.0 && chi[0] > 2340.0 && clo[2] > 0.0, "{clo:?} {chi:?}");
    assert!(mesh.colors.iter().all(|c| *c != BELT));
}

#[test]
fn a_raised_motor_stands_on_a_riser_and_a_long_gap_takes_a_gear_unit() {
    let mut prj = room_with(("GEN.PUMP-CENT-DN25", [1000.0, 1000.0]), ("GEN.MOT-2.2KW", [1045.0, 1900.0]), "Gear");
    prj.layout.enclosures[0].items[1].z = 60.0;
    let (kind, mesh) = parts_of(&prj);
    assert_eq!(kind, DriveKind::Gear);
    let (clo, chi) = bounds_of(&mesh, Some(CASE));
    assert!(clo[2] == 0.0 && clo[1] > 1640.0 && chi[1] < 1900.0, "{clo:?} {chi:?}");
    let (rlo, rhi) = bounds_of(&mesh, Some(HUB));
    assert!(rlo[2] == 0.0 && rhi[1] > 2100.0, "the riser under the motor: {rlo:?} {rhi:?}");
}

#[test]
fn belts_and_gear_cases_face_outwards() {
    let (one, two) = (([0.0, 200.0], 60.0), ([500.0, 400.0], 150.0));
    let mut belt = Mesh3d::default();
    ribbon(&mut belt, 1, one, two, (-5.0, 3.0), (0.0, 13.0), BELT);
    assert!(volume(&belt) > 0.0);
    let mut case = Mesh3d::default();
    prism(&mut case, 0, one, two, (100.0, 30.0), CASE);
    assert!(volume(&case) > 0.0);
    // The outline touches both circles on their far sides.
    let ring = outline(one, two, 12);
    assert!(ring.iter().any(|q| (q[0] + 60.0).abs() < 1.0) && ring.iter().any(|q| (q[0] - 650.0).abs() < 1.0));
}

#[test]
fn the_drive_properties_are_defined() {
    let prj = Project::default();
    for key in [DRIVEN_BY, DRIVE] {
        assert!(crate::properties::def(&prj, key).is_some(), "{key}");
    }
}
