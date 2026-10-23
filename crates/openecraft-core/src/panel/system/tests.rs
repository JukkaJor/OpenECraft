use super::*;
use crate::panel::{layout_material, new_enclosure};
use crate::sample::demo_project;

fn baying() -> Enclosure {
    let mut e = Enclosure::default();
    apply_system(&mut e, "baying", Some((800.0, 2000.0, 600.0)));
    e
}

#[test]
fn monobloc_enclosure_is_one_part() {
    let mut e = Enclosure::default();
    apply_system(&mut e, "compact", Some((600.0, 760.0, 210.0)));
    assert_eq!(e.part, "GEN.CE-600X760X210");
    let parts: Vec<_> = components(&e).into_iter().filter(|c| !c.part.is_empty()).collect();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].role, Role::Body);
    // Walls, door, plate still have their geometry.
    assert!(components(&e).iter().any(|c| c.role == Role::Wall(Face::Left) && !c.boxes.is_empty()));
    assert_eq!(e.wall(), 1.5);
}

#[test]
fn baying_system_components() {
    let mut e = baying();
    assert_eq!(e.build.plinth, 100.0);
    let cs = components(&e);
    let part = |r: Role| cs.iter().find(|c| c.role == r).map(|c| c.part.clone());
    assert_eq!(part(Role::Body).as_deref(), Some("GEN.TS-800X2000X600"));
    assert_eq!(part(Role::Wall(Face::Left)).as_deref(), Some("GEN.TS-SP-2000X600"));
    assert_eq!(part(Role::Wall(Face::Rear)).as_deref(), Some("GEN.TS-RW-800X2000"));
    assert_eq!(part(Role::Plinth).as_deref(), Some("GEN.PL-800X600-100"));
    // Frame: four posts and eight rails; plinth below the enclosure.
    assert_eq!(cs.iter().find(|c| c.role == Role::Body).unwrap().boxes.len(), 12);
    let plinth = cs.iter().find(|c| c.role == Role::Plinth).unwrap();
    assert!(plinth.boxes.iter().all(|b| b.0[1] >= e.height && b.1[1] <= e.height + 100.0));
    // Bayed to the right: no right side panel, one baying kit.
    e.build.right_open = true;
    let cs = components(&e);
    assert!(!cs.iter().any(|c| c.role == Role::Wall(Face::Right)));
    assert_eq!(cs.iter().find(|c| c.role == Role::BayingKit).map(|c| c.count), Some(1));
    // Every component part of the standard sizes is in the catalog.
    for c in cs.iter().filter(|c| !c.part.is_empty()) {
        assert!(crate::parts::catalog().iter().any(|p| p.number == c.part), "{}", c.part);
    }
}

#[test]
fn system_from_catalog_part() {
    let mut prj = demo_project();
    let e = new_enclosure(&mut prj, "GEN.TS-600X2000X500", "+CAB9");
    assert_eq!(e.build.system, "baying");
    assert_eq!((e.width, e.height, e.depth), (600.0, 2000.0, 500.0));
    assert_eq!(e.climate.material, Material::SheetSteel);
    let p = new_enclosure(&mut prj, "GEN.PC-300X400X180", "+B1");
    assert_eq!(p.climate.material, Material::Plastic);
    assert!(!p.build.left_open && p.build.plinth == 0.0);
}

#[test]
fn components_in_the_bill_of_materials() {
    let mut prj = demo_project();
    let mut e = baying();
    e.name = "+CAB2".into();
    e.build.gland_plate = true;
    e.build.left_open = true;
    prj.layout.enclosures.push(e);
    let bom = layout_material(&prj);
    let qty = |n: &str| bom.iter().find(|m| m.0 == n).map(|m| m.1);
    assert_eq!(qty("GEN.TS-800X2000X600"), Some(1.0));
    assert_eq!(qty("GEN.TS-SP-2000X600"), Some(1.0));
    assert_eq!(qty("GEN.GP-800X600"), Some(1.0));
    assert_eq!(qty(crate::parts::mechanics::BAYING_KIT), Some(1.0));
    // The plain enclosure of the demo keeps its part.
    assert_eq!(qty(crate::panel::ENCLOSURE_PART), Some(1.0));
}

#[test]
fn resizing_follows_the_part_number() {
    let mut e = baying();
    e.width = 1200.0;
    sync_part(&mut e);
    assert_eq!(e.part, "GEN.TS-1200X2000X600");
    apply_system(&mut e, "", None);
    assert!(e.system().is_none());
}
