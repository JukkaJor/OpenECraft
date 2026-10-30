//! P&ID of the water supply: the water main through the remote controlled valve in the
//! house connection room into the storage tank of the technical room, two booster
//! pumps and the pressure vessel to the distribution manifold of the house.

use super::*;
use crate::model::fluid::{HoseConfig, LineType, Medium};

pub(super) fn pid(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "3", "Water supply P&ID", PageKind::Schematic, "WAT", "TR");
    let mut g = Pg { prj, page };
    // House connection room: main shut-off, water meter, check valve, motor valve.
    g.sym("pid_connection", 20.0, 100.0, 0, "+HR-UM1", "", "Water main");
    g.sym("pid_valve", 44.0, 100.0, 0, "+HR-QM1", "", "Main shut-off");
    g.sym("pid_flow_meter", 68.0, 100.0, 0, "+HR-BF1", "", "Water meter");
    g.sym("pid_check_valve", 92.0, 100.0, 0, "+HR-RM1", "", "");
    g.sym("pid_motor_valve", 116.0, 100.0, 0, "+HR-QM2", "", "Remote controlled");
    g.w(28.0, 100.0, 36.0, 100.0).w(52.0, 100.0, 60.0, 100.0).w(76.0, 100.0, 84.0, 100.0).w(100.0, 100.0, 108.0, 100.0);
    // Technical room: storage tank, the booster pumps in parallel, pressure vessel.
    g.path(&[(124.0, 100.0), (168.0, 100.0), (168.0, 84.0), (180.0, 84.0), (180.0, 104.0)]);
    g.sym("pid_vessel", 180.0, 120.0, 0, "-CM1", "", "Storage tank 1000 l");
    g.sym("pid_instrument", 156.0, 116.0, 0, "-BL1", "LIT 1", "");
    g.sym("pid_instrument", 156.0, 140.0, 0, "-BL2", "LSL 2", "");
    g.path(&[(180.0, 136.0), (180.0, 152.0), (188.0, 152.0)]);
    g.sym("pid_valve", 196.0, 152.0, 0, "-QM3", "", "");
    g.w(204.0, 152.0, 220.0, 152.0).w(220.0, 136.0, 220.0, 168.0);
    for (k, y) in [136.0, 168.0].into_iter().enumerate() {
        let n = k + 1;
        g.w(220.0, y, 228.0, y);
        g.sym("pid_pump", 236.0, y, 0, &format!("-GP{n}"), "", &format!("Booster pump {n}"));
        g.w(244.0, y - 4.0, 252.0, y - 4.0);
        g.sym("pid_check_valve", 260.0, y - 4.0, 0, &format!("-RM{}", n + 1), "", "");
        g.w(268.0, y - 4.0, 284.0, y - 4.0);
    }
    g.w(284.0, 132.0, 284.0, 164.0).w(284.0, 148.0, 316.0, 148.0);
    g.sym("pid_vessel", 316.0, 132.0, 0, "-CM2", "", "Pressure vessel 100 l");
    g.sym("pid_instrument", 296.0, 112.0, 0, "-BP1", "PIT 3", "");
    g.path(&[(316.0, 116.0), (316.0, 100.0), (332.0, 100.0)]);
    g.sym("pid_valve", 340.0, 100.0, 0, "-QM4", "", "");
    g.w(348.0, 100.0, 356.0, 100.0);
    g.sym("pid_connection", 364.0, 100.0, 0, "-UM2", "", "To the house").pins(&["IN"]).mirror();
    for w in &mut g.prj.pages[page].wires {
        w.medium = Some(Medium::Water);
        w.cross_section = "DN32".into();
        w.hose = Some(HoseConfig { line: LineType::Pipe, ..Default::default() });
    }
    g.text(20.0, 60.0, "House connection room +HR", 3.0).text(150.0, 60.0, "Technical room +TR", 3.0);
    g.text(20.0, 200.0, "The pumps -GP1 / -GP2 are driven by the motors -M1 / -M2 (4.1), the motor valve -QM2 by the actuator -MA1 (3.4);", 2.5);
    g.text(20.0, 207.0, "level -BL1, dry-run protection -BL2 and pressure -BP1 go to the PLC (3.3, 3.5). Pipes DN32, drinking water.", 2.5);
}
