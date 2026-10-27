//! Level 9: the cabinet line-up of level 8 in its electrical room +ER1: an L-shaped
//! floor plan, an inner wall, a door and a window, the line-up at its place in the
//! floor plan, a mounting rail with a socket outlet on a wall, an emergency light made
//! for direct wall mounting and a cable tray under the ceiling.

use super::*;
use crate::geom::p;
use crate::model::panel::{Opening, SitePlace};
use crate::panel::{self, Face, ItemKind};
use crate::parts::{MountingData, Part};

/// Wall-mounted emergency light (a project part: made for direct wall mounting).
pub(super) const LIGHT: &str = "PRJ.EMERGENCY-LIGHT";
pub(super) const TRAY: &str = "GEN.DUCT-100X80";

pub(super) fn project() -> Project {
    let mut prj = super::cabinets::project();
    prj.info.name = "Pump station electrical room".into();
    prj.info.description = "Line-up in its room\nwith wall-mounted equipment".into();
    prj.structures.define(crate::structure::Aspect::Location, "ER1", "Electrical room");
    socket_circuit(&mut prj);
    let db = PartsDb::default();
    for part in [TRAY, "GEN.SOCKET-DIN-16A", "GEN.MCB-1P-B10"] {
        crate::parts::ensure_in_project(&mut prj, &db, part);
    }
    assign_parts(&mut prj, &db, &[("=SUP+CAB1-F2", &["GEN.MCB-1P-B10"][..]), ("=SUP+ER1-X2", &["GEN.SOCKET-DIN-16A"])]);
    prj.parts.push(Part {
        number: LIGHT.into(),
        description: "Emergency light LED 3 W, 3 h, IP65, for wall mounting".into(),
        group: "Installation devices".into(),
        subgroup: "Emergency light".into(),
        width: 350.0,
        height: 120.0,
        depth: 60.0,
        price: 89.0,
        mounting_data: MountingData { wall_mount: true, ..Default::default() },
        ..Default::default()
    });
    panel::place_device(&mut prj, 0, "=SUP+CAB1-F2", p(420.0, 380.0 - 45.0));

    // L-shaped room, 3 m high: 6 m along the rear wall, the front left corner cut out.
    let mut room = panel::new_room(&mut prj, "+ER1", 6000.0, 4500.0, 3000.0);
    panel::set_outline(&mut room, &[[0.0, 0.0], [6000.0, 0.0], [6000.0, 4500.0], [2500.0, 4500.0], [2500.0, 3000.0], [0.0, 3000.0]]).expect("electrical room");
    prj.layout.enclosures.push(room);
    let r = prj.layout.enclosures.len() - 1;
    // A partition behind the line-up separates the battery corner on the right.
    panel::add_inner_wall(&mut prj, r, [4200.0, 0.0], [4200.0, 2000.0], 100.0).expect("electrical room");
    // Walls: 0 rear, 1 right, 2 front (right part), 3 and 4 the cut-out corner, 5 left.
    panel::add_opening(&mut prj, r, Opening::Door, Face::Wall(2), Some(600.0), None, None).expect("electrical room");
    panel::add_opening(&mut prj, r, Opening::Window, Face::Wall(1), None, None, None).expect("electrical room");
    // The line-up stands 100 mm in front of the rear wall, its fronts towards the room.
    panel::set_room(&mut prj, 0, Some(r), Some(SitePlace { x: 600.0, z: 100.0, angle: 0.0 })).expect("electrical room");
    // Left wall: a mounting rail 1.4 m above the floor with the socket outlet for
    // service, and the emergency light above it.
    panel::add_wall_profile(&mut prj, r, ItemKind::Rail, panel::RAIL_PART, Face::Wall(5), p(1200.0, 1600.0), p(1800.0, 1600.0)).expect("electrical room");
    panel::mount_on_wall(&mut prj, r, "=SUP+ER1-X2", Face::Wall(5), p(1300.0, 1600.0 - 45.0)).expect("electrical room");
    panel::mount_on_wall(&mut prj, r, LIGHT, Face::Wall(5), p(1300.0, 400.0)).expect("electrical room");
    // Cable tray under the ceiling from above the line-up to the right wall (the
    // motor cables lead on to the pump pit).
    panel::add_wall_profile(&mut prj, r, ItemKind::Duct, TRAY, Face::Roof, p(800.0, 900.0), p(5900.0, 900.0)).expect("electrical room");
    number_wires(&mut prj);
    prj
}

/// Service socket outlet in the room, fed by -F2 in +CAB1 (infeed page).
fn socket_circuit(prj: &mut Project) {
    let page = prj.pages.iter().position(|p| p.function == "SUP" && p.kind == PageKind::Schematic).expect("infeed page");
    let mut g = Pg { prj, page };
    g.sym("cb1", 110.0, 80.0, 0, "-F2", "", "Service socket").w(110.0, 24.0, 110.0, 76.0).w(110.0, 84.0, 110.0, 180.0);
    g.sym("socket", 110.0, 184.0, 0, "+ER1-X2", "", "Service socket");
    g.w(118.0, 48.0, 118.0, 180.0).w(126.0, 56.0, 126.0, 180.0);
    // Behind the 10 A breaker: 1.5 mm², black like the other power circuits.
    for w in g.prj.pages[page].wires.iter_mut().filter(|w| w.a.x == 110.0 && w.a.y == 84.0) {
        w.color = "BK".into();
        w.cross_section = "1.5 mm²".into();
    }
}
