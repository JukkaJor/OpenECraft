//! Level 10: the room of level 9 with the pumps in it: the two pump motors standing on
//! the floor, fed from the line-up through cable glands in the roofs of the cabinets;
//! a P&ID of the water circuit (vessel, ball valves, pump, pressure gauge) whose DN25
//! pipes are routed in 3D through the room, along a pipe run past the valves on the
//! front wall.

use super::*;
use crate::geom::p;
use crate::model::fluid::{HoseConfig, LineType, Medium};
use crate::panel::{self, Face};

pub(super) const PUMP: &str = "GEN.PUMP-CENT-DN25";
pub(super) const VESSEL: &str = "GEN.TANK-200L";
pub(super) const VALVE: &str = "GEN.VLV-BALL-DN25";
pub(super) const GAUGE: &str = "GEN.GAUGE-0-10BAR";
const MOTOR: &str = "GEN.MOT-0.75KW";

pub(super) fn project() -> Project {
    let mut prj = super::room::project();
    prj.info.name = "Pump room".into();
    prj.info.description = "Pumps and pipes in the room\nof the line-up".into();
    prj.structures.define(crate::structure::Aspect::Location, "ER1", "Pump room");
    water_circuit(&mut prj);
    let db = PartsDb::default();
    assign_parts(&mut prj, &db, &[("=P1+ER1-CM1", &[VESSEL][..]), ("=P1+ER1-QM1", &[VALVE]), ("=P1+ER1-GP1", &[PUMP]), ("=P1+ER1-QM2", &[VALVE]), ("=P1+ER1-BP1", &[GAUGE])]);
    // The motors are foot-mounted: they stand on the floor beside their pumps.
    if let Some(m) = prj.parts.iter_mut().find(|m| m.number == MOTOR) {
        (m.width, m.height, m.depth) = (230.0, 240.0, 330.0);
        m.mounting = "Floor".into();
        m.mounting_data.floor_mount = true;
    }
    let r = prj.layout.enclosures.iter().position(|e| e.is_room()).expect("room");
    // Floor: walls 0 rear, 1 right, 2 front (right part), 3 and 4 the cut-out corner,
    // 5 left; the floor in plan coordinates. The vessel right of the pump, the motors
    // left of it, clear of the door swing in the front wall.
    for (what, x, z) in [("=P1+ER1-CM1", 4400.0, 2300.0), ("=P1+ER1-GP1", 3500.0, 2300.0), ("=P1+FLD-M1", 3240.0, 2300.0), ("=P2+FLD-M1", 3240.0, 1200.0)] {
        panel::mount_on_wall(&mut prj, r, what, Face::Bottom, p(x, z)).expect("pump room");
    }
    // Front wall (seen from inside its left end is at x 6000): the valves and the gauge
    // about 1.4 m above the floor beside the door, the pipe run past them in front.
    for (what, x, y) in [("=P1+ER1-QM1", 3300.0, 1500.0), ("=P1+ER1-QM2", 2500.0, 1500.0), ("=P1+ER1-BP1", 2000.0, 1500.0)] {
        panel::mount_on_wall(&mut prj, r, what, Face::Wall(2), p(x, y)).expect("pump room");
    }
    panel::add_pipe_run(&mut prj, r, Face::Wall(2), &[p(1700.0, 1560.0), p(3500.0, 1560.0)], Some(200.0), None, Some("PR1")).expect("pump room");
    // The motor cables leave the cabinets through glands in their roofs.
    panel::place_accessory(&mut prj, 0, "GEN.GLAND-M20", Face::Roof, p(680.0, 250.0));
    panel::place_accessory(&mut prj, 1, "GEN.GLAND-M20", Face::Roof, p(300.0, 250.0));
    number_wires(&mut prj);
    prj
}

/// P&ID of the water circuit of pump 1: vessel → ball valve → pump → ball valve and
/// pressure gauge → back to the vessel, DN25 water pipes.
fn water_circuit(prj: &mut Project) {
    // In the section of pump 1, after its power and control pages.
    let section = prj.pages.iter().find(|p| p.name == "3.1").map_or(0, |p| p.section);
    let page = add_page(prj, "3.3", "Water circuit", PageKind::Schematic);
    prj.pages[page].section = section;
    prj.pages[page].function = "P1".into();
    prj.pages[page].location = "ER1".into();
    let mut g = Pg { prj, page };
    g.sym("pid_vessel", 60.0, 100.0, 0, "-CM1", "", "Vessel");
    g.sym("pid_valve", 100.0, 140.0, 0, "-QM1", "", "Suction valve");
    g.sym("pid_pump", 140.0, 140.0, 0, "-GP1", "", "Pump 1");
    g.sym("pid_valve", 180.0, 136.0, 0, "-QM2", "", "Pressure valve");
    g.sym("fl_gauge", 200.0, 120.0, 0, "-BP1", "", "Pressure");
    g.path(&[(60.0, 116.0), (60.0, 140.0), (92.0, 140.0)]);
    g.w(108.0, 140.0, 132.0, 140.0);
    g.w(148.0, 136.0, 172.0, 136.0);
    g.w(188.0, 136.0, 200.0, 136.0).w(200.0, 124.0, 200.0, 136.0);
    g.path(&[(200.0, 136.0), (220.0, 136.0), (220.0, 70.0), (60.0, 70.0), (60.0, 84.0)]);
    for w in &mut g.prj.pages[page].wires {
        w.medium = Some(Medium::Water);
        w.cross_section = "DN25".into();
        w.hose = Some(HoseConfig { line: LineType::Pipe, ..Default::default() });
    }
}
