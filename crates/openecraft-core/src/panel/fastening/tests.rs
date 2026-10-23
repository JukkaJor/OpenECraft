use super::*;
use crate::panel::{Enclosure, PanelItem};
use crate::production::{holes_on, item_holes, nc_csv, nc_dxf};
use crate::sample::demo_project;

fn rail(face: Face, fastening: Fastening) -> PanelItem {
    PanelItem { id: 1, kind: ItemKind::Rail, x: 0.0, y: 100.0, w: 300.0, h: 35.0, d: 7.5, face, fastening, ..Default::default() }
}

#[test]
fn automatic_fastening_by_kind_and_face() {
    let plate = rail(Face::Plate, Fastening::Auto);
    assert_eq!(Fastening::resolve(&plate, None), Fastening::Tapped);
    assert_eq!(Fastening::resolve(&rail(Face::Left, Fastening::Auto), None), Fastening::Through);
    let door_duct = PanelItem { kind: ItemKind::Duct, face: Face::Door, ..plate.clone() };
    assert_eq!(Fastening::resolve(&door_duct, None), Fastening::Adhesive);
    let on_rail = PanelItem { kind: ItemKind::Device, z: 7.5, ..plate.clone() };
    assert_eq!(Fastening::resolve(&on_rail, None), Fastening::Clip);
    // The item's own choice wins, then the part data.
    assert_eq!(Fastening::resolve(&rail(Face::Plate, Fastening::RivetNut), None), Fastening::RivetNut);
    let part = crate::parts::Part { tech_data: "Fastening: Rivet nut".into(), ..Default::default() };
    assert_eq!(Fastening::resolve(&plate, Some(&part)), Fastening::RivetNut);
    assert_eq!(Fastening::from_key("weld-stud"), None);
    assert_eq!(Fastening::from_key("stud"), Some(Fastening::Stud));
}

#[test]
fn holes_follow_the_fastening() {
    let tapped = item_holes(&rail(Face::Plate, Fastening::Auto), &[]);
    assert!(!tapped.is_empty());
    assert!(tapped.iter().all(|h| h.thread == "M5" && (h.diameter - 4.2).abs() < 0.01));
    let through = item_holes(&rail(Face::Plate, Fastening::Through), &[]);
    assert_eq!(through.len(), tapped.len());
    assert!(through.iter().all(|h| h.thread.is_empty() && h.thread_size == "M5" && (h.diameter - 5.5).abs() < 0.01));
    let rivet = item_holes(&rail(Face::Plate, Fastening::RivetNut), &[]);
    assert!(rivet.iter().all(|h| (h.diameter - 7.0).abs() < 0.01));
    assert!(item_holes(&rail(Face::Plate, Fastening::Adhesive), &[]).is_empty());
    assert!(item_holes(&rail(Face::Plate, Fastening::Clip), &[]).is_empty());
}

#[test]
fn hardware_per_fixing_point() {
    assert_eq!(Fastening::Tapped.hardware("M5"), vec![("GEN.SCREW-M5X12".to_string(), 1)]);
    let through = Fastening::Through.hardware("M4");
    assert!(through.contains(&("GEN.NUT-M4".to_string(), 1)) && through.contains(&("GEN.WASHER-M4".to_string(), 2)));
    assert!(Fastening::Clip.hardware("M5").is_empty());
    assert!(Fastening::Tapped.hardware("Ø4.5").is_empty());
}

#[test]
fn weld_studs_in_the_nc_data() {
    let mut prj = demo_project();
    let mut e = Enclosure { name: "+S1".into(), ..Default::default() };
    e.items.push(rail(Face::Rear, Fastening::Stud));
    prj.layout.enclosures = vec![e];
    let studs = holes_on(&prj.layout.enclosures[0], &prj.parts, Face::Rear);
    assert!(!studs.is_empty() && studs.iter().all(|h| h.fastening == Fastening::Stud));
    let csv = nc_csv(&prj, 0, Face::Rear);
    assert!(csv.contains("Weld stud") && csv.contains(";M5;"), "{csv}");
    let dxf = nc_dxf(&prj, 0, Face::Rear);
    assert!(dxf.contains("\nREAR\n") && dxf.contains("STUD_M5") && !dxf.contains("HOLE_D"));
}

#[test]
fn hardware_in_the_bill_of_materials() {
    let mut prj = demo_project();
    let material = fastening_material(&prj);
    // Rails and ducts on the plate are screwed into tapped holes.
    let screws = material.iter().find(|m| m.0 == "GEN.SCREW-M5X12").map(|m| m.1).unwrap_or(0);
    assert!(screws > 0, "{material:?}");
    let bom = crate::panel::layout_material(&prj);
    assert!(bom.iter().any(|m| m.0 == "GEN.SCREW-M4X10"));
    prj.layout.hardware = false;
    assert!(!crate::panel::layout_material(&prj).iter().any(|m| m.0.starts_with("GEN.SCREW")));
}
