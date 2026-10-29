use super::*;
use crate::panel::{apply_system, components, new_enclosure, validate, Role, ENCLOSURE_PART};
use crate::parts::FnKind;
use crate::testkit::Kit;

/// A 42 U rack +RACK1 (600 x 2000 x 800) with the switch -KF1 (1 U, 250 mm deep) and
/// the contactor -QA1 (no 19-inch part) in the schematic. Returns the project and
/// the rack.
fn rack() -> (Project, usize) {
    let mut k = Kit::new();
    k.sym("cb1", 100.0, 60.0, "-KF1", "");
    k.sym("cb1", 140.0, 60.0, "-QA1", "");
    k.part("-KF1", FnKind::Generic, "1 2", |p| (p.width, p.height, p.depth) = (RACK_WIDTH, RACK_UNIT, 250.0));
    k.part("-QA1", FnKind::Generic, "1 2", |p| (p.width, p.height, p.depth) = (45.0, 80.0, 80.0));
    let mut prj = k.prj;
    let mut e = new_enclosure(&mut prj, ENCLOSURE_PART, "+RACK1");
    apply_system(&mut e, RACK_SYSTEM, Some((600.0, 2000.0, 800.0)));
    prj.layout.enclosures.push(e);
    (prj, 0)
}

#[test]
fn a_rack_has_rack_units_and_rails_instead_of_a_plate() {
    let (prj, r) = rack();
    let e = &prj.layout.enclosures[r];
    assert!(is_rack(e));
    assert_eq!(rack_units(e), 42);
    assert_eq!(e.part, "GEN.RK-600X2000X800");
    // The units are centred in the height, the panels in the width.
    let (top, bottom) = (unit_top(e, 42, 1) + PLATE_MARGIN, unit_top(e, 1, 1) + RACK_UNIT + PLATE_MARGIN);
    assert!((top - (2000.0 - bottom)).abs() < 0.01 && (bottom - top - 42.0 * RACK_UNIT).abs() < 0.01, "{top} {bottom}");
    assert!((panel_left(e) + PLATE_MARGIN - (600.0 - RACK_WIDTH) / 2.0).abs() < 0.01);
    let parts = components(e);
    assert!(parts.iter().all(|c| c.role != Role::MountingPlate));
    let rails = parts.iter().find(|c| c.role == Role::RackRails).expect("rails");
    assert_eq!((rails.part.as_str(), rails.count, rails.boxes.len()), ("GEN.RK-RAIL-42U", 2, 4));
    // Other enclosures have no units.
    let mut plain = e.clone();
    plain.build.system = "baying".into();
    assert_eq!(rack_units(&plain), 0);
}

#[test]
fn parts_are_mounted_on_whole_rack_units() {
    let (mut prj, r) = rack();
    // Without a unit in the highest free ones, from the top down.
    let patch = mount_in_rack(&mut prj, r, "GEN.RK-PATCH-24P", None).unwrap();
    let switch = mount_in_rack(&mut prj, r, "-KF1", None).unwrap();
    let e = &prj.layout.enclosures[r];
    assert_eq!(slot(e, e.item(patch).unwrap()), Some((42, 1)));
    assert_eq!(slot(e, e.item(switch).unwrap()), Some((41, 1)));
    // The front panel on the front rails.
    let sw = e.item(switch).unwrap();
    assert!((PLATE_Z + sw.z + sw.d - (800.0 - RACK_FRONT)).abs() < 0.01);
    assert_eq!(sw.fastening, Fastening::Clip);
    // A server of 2 U at unit 10; not where it overlaps, not beyond the top.
    let server = mount_in_rack(&mut prj, r, "GEN.RK-SERVER-2U", Some(10)).unwrap();
    assert_eq!(slot(&prj.layout.enclosures[r], prj.layout.enclosures[r].item(server).unwrap()), Some((10, 2)));
    let n = prj.layout.enclosures[r].items.len();
    assert!(mount_in_rack(&mut prj, r, "GEN.RK-UPS-2U", Some(11)).unwrap_err().contains("taken"));
    assert!(mount_in_rack(&mut prj, r, "GEN.RK-UPS-2U", Some(42)).unwrap_err().contains("outside"));
    assert!(mount_in_rack(&mut prj, r, "-QA1", None).unwrap_err().contains("19-inch"));
    assert!(mount_in_rack(&mut prj, r, "GEN.NOTHING", None).is_err());
    assert_eq!(prj.layout.enclosures[r].items.len(), n, "refused parts are not left behind");
    assert_eq!(occupied(&prj.layout.enclosures[r]).iter().map(|o| o.0).collect::<Vec<_>>(), [42, 41, 10]);
    assert_eq!(free_units(&prj.layout.enclosures[r], 2, 0), Some(39));
    // Neighbours on adjacent units do not collide.
    assert!(validate(&prj, &prj.layout.enclosures[r]).is_empty(), "{:?}", validate(&prj, &prj.layout.enclosures[r]));
    // A cabinet is no rack.
    let cab = new_enclosure(&mut prj, ENCLOSURE_PART, "+CAB1");
    prj.layout.enclosures.push(cab);
    assert!(mount_in_rack(&mut prj, 1, "GEN.RK-PATCH-24P", None).is_err());
}

#[test]
fn moved_19_inch_parts_snap_onto_units_and_others_are_reported() {
    let (mut prj, r) = rack();
    let id = mount_in_rack(&mut prj, r, "-KF1", Some(20)).unwrap();
    let e = &prj.layout.enclosures[r];
    let mut it = e.item(id).unwrap().clone();
    // Dragged 2.6 units down and to the side: back onto whole units, centred.
    it.y += 2.6 * RACK_UNIT;
    it.x += 37.0;
    crate::panel::snap_to_rail(e, &mut it);
    assert_eq!(slot(e, &it), Some((17, 1)));
    assert!((it.x - panel_left(e)).abs() < 0.01);
    // Not below unit 1.
    it.y += 100.0 * RACK_UNIT;
    crate::panel::snap_to_rail(e, &mut it);
    assert_eq!(slot(e, &it), Some((1, 1)));
    // A contactor placed on the mounting level is no 19-inch part.
    let qa = crate::panel::place_device_on(&mut prj, r, "-QA1", crate::geom::p(100.0, 100.0), Face::Plate).unwrap();
    let findings = validate(&prj, &prj.layout.enclosures[r]);
    assert!(findings.iter().any(|f| f.items == vec![qa] && f.kind == CheckKind::Mounting && f.severity == Severity::Info), "{findings:?}");
}

#[test]
fn the_rack_elevation_lists_the_units_from_the_top() {
    let (mut prj, r) = rack();
    mount_in_rack(&mut prj, r, "GEN.RK-SERVER-2U", Some(10)).unwrap();
    mount_in_rack(&mut prj, r, "-KF1", None).unwrap();
    let an = crate::analysis::Analysis::with_builtin(&prj);
    let t = crate::reports::table(&prj, &an, crate::model::ReportKind::RackElevation);
    let rows: Vec<(&str, &str, &str)> = t.rows.iter().map(|r| (r[1].as_str(), r[2].as_str(), r[3].as_str())).collect();
    assert_eq!(rows, [("42", "-KF1", "TEST.KF1-0"), ("11 – 10", "", "GEN.RK-SERVER-2U"), ("39 / 42 U", "", "")]);
}
