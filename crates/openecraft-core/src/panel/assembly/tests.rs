use super::*;
use crate::parts::{bundled, PartsDb};
use crate::panel::{device_mesh, device_points, device_size, new_enclosure, place_device};

/// A project with an enclosure and devices of the given parts.
fn project(devices: &[(&str, &[&str])]) -> Project {
    let mut prj = Project::default();
    let db = PartsDb { parts: bundled::default_parts() };
    for (tag, parts) in devices {
        for n in *parts {
            crate::parts::add_to_device(&mut prj, &db, tag, n, 1);
        }
    }
    let e = new_enclosure(&mut prj, "GEN.ENC-600X800", "+CAB1");
    prj.layout.enclosures.push(e);
    prj
}

fn placed(prj: &mut Project, tag: &str) -> PanelItem {
    let id = place_device(prj, 0, tag, crate::geom::p(100.0, 100.0)).unwrap();
    prj.layout.enclosures[0].item(id).unwrap().clone()
}

#[test]
fn side_contacts_stand_beside_the_main_part() {
    let mut prj = project(&[("-F1", &["HAG.CCA216D", "HAG.CZ001"]), ("-F2", &["GEN.MCB-1P-C6", "GEN.MCB-AUX-1CO"])]);
    let a = assembly(&prj, "-F1").unwrap();
    // The signal contact (9 mm) sits left of the RCCB (35 mm).
    assert_eq!((a.main.part.number.as_str(), a.main.at, a.w, a.d), ("HAG.CCA216D", 9.0, 44.0, 70.0));
    assert_eq!((a.sides[0].part.number.as_str(), a.sides[0].at), ("HAG.CZ001", 0.0));
    let size = device_size(&prj, "-F1").unwrap();
    assert_eq!((size.part.as_str(), size.w, size.h), ("HAG.CCA216D", 44.0, 83.0));
    // The auxiliary switch of the breaker on its right.
    let b = assembly(&prj, "-F2").unwrap();
    assert_eq!((b.main.at, b.sides[0].at, b.w), (0.0, 18.0, 27.0));

    let it = placed(&mut prj, "-F1");
    assert!(mounted(&prj, &it).is_some());
    let points = device_points(&prj, &it);
    let x = |pin: &str| points.iter().find(|p| p.pin == pin).unwrap_or_else(|| panic!("{pin}")).pos[0];
    assert!(x("13") < it.x + 9.0 && x("21") < it.x + 9.0, "contacts on the signal contact");
    assert!(x("1") > it.x + 9.0 && x("N'") < it.x + 44.0, "terminals on the RCCB");
    // The 3D model shows both parts: the signal contact's model fills the left end.
    let tris = device_mesh(&prj, &it).unwrap();
    let left = tris.iter().flatten().map(|v| v[0]).fold(f32::MAX, f32::min);
    let right = tris.iter().flatten().map(|v| v[0]).fold(f32::MIN, f32::max);
    assert!((left - it.x).abs() < 0.01 && (right - (it.x + 44.0)).abs() < 0.01, "{left} {right}");
}

#[test]
fn motor_protection_switches_take_their_auxiliary_switch_on_the_left() {
    // The demo's -Q1: the lateral auxiliary switch (9 mm) left of the switch (45 mm).
    let prj = crate::sample::demo_project();
    let a = assembly(&prj, "-Q1").unwrap();
    assert_eq!((a.main.part.number.as_str(), a.main.at, a.w, a.d), ("GEN.MPS-2.5-4A", 9.0, 54.0, 97.0));
    let (_, it) = crate::panel::locate(&prj, "-Q1").unwrap();
    assert_eq!(it.w, 54.0);
    let points = device_points(&prj, it);
    assert!(points.iter().filter(|p| p.pin == "13" || p.pin == "21").all(|p| p.pos[0] < it.x + 9.0));
}

#[test]
fn front_blocks_are_stacked_in_front() {
    let mut prj = project(&[("-K1", &["GEN.CT-9A-24DC", "GEN.AUX-2NO2NC"])]);
    crate::parts::add_to_device(&mut prj, &PartsDb::default(), "-K1", "GEN.AUX-1NO1NC", 1);
    let a = assembly(&prj, "-K1").unwrap();
    assert_eq!((a.w, a.d, a.front.len()), (45.0, 73.0 + 40.0 + 40.0, 2));
    assert_eq!((a.front[0].z, a.front[1].z, a.front[0].at), (73.0, 113.0, 0.0));
    let it = placed(&mut prj, "-K1");
    let a = assembly(&prj, "-K1").unwrap();
    let sub = a.piece_item(&it, &a.front[1]);
    assert_eq!((sub.y - it.y, sub.h, sub.z - it.z, sub.d), (17.0, 40.0, 113.0, 40.0));
    let deepest = device_mesh(&prj, &it).unwrap().iter().flatten().map(|v| v[2]).fold(f32::MIN, f32::max);
    assert!((deepest - (it.z + it.d)).abs() < 0.01);
}

#[test]
fn plain_devices_and_strips_are_drawn_as_before() {
    let mut prj = project(&[("-F1", &["HAG.CCA216D"]), ("-X1", &["GEN.TB-2.5"])]);
    let it = placed(&mut prj, "-F1");
    assert!(assembly(&prj, "-F1").unwrap().is_plain() && mounted(&prj, &it).is_none());
    assert!(assembly(&prj, "-X1").is_none());
    // A loose accessory (only the accessory on the device) is drawn as the main part.
    let prj = project(&[("-F3", &["HAG.CZ001"])]);
    assert_eq!(assembly(&prj, "-F3").unwrap().main.part.number, "HAG.CZ001");
}
