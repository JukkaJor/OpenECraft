//! Level 5: electro-pneumatic press: the pneumatic circuit and the electrical control
//! share their devices (valves and solenoid coils, cylinders and position switches).

use super::*;
use crate::model::electrofluid::{DeviceGroup, GroupKind};

pub(super) fn project() -> Project {
    let mut prj = Project { pages: Vec::new(), ..Default::default() };
    prj.info = ProjectInfo {
        name: "Pneumatic press".into(),
        description: "Clamping and pressing cylinder\nwith electrical control".into(),
        company: "OpenECraft".into(),
        creator: "OpenECraft".into(),
        date: "2026-10-27".into(),
        revision: "A".into(),
        ..Default::default()
    };
    prj.apply_preset(crate::potentials::Preset::Iec60204);
    prj.wire_colors = true;
    prj.fluid.size = "6 mm".into();
    prj.fluid.color = "BU".into();
    prj.fluid.prefix = "H".into();
    prj.fluid.pressure = 6.0;
    add_page(&mut prj, "1", "Cover sheet", PageKind::Cover);
    pneumatics(&mut prj);
    supply(&mut prj);
    control(&mut prj);
    add_page(&mut prj, "5", "Electro-fluid cross reference", PageKind::Report(ReportKind::ElectroFluid.into()));
    add_page(&mut prj, "6", "Fluid netlist", PageKind::Report(ReportKind::FluidNetlist.into()));
    add_page(&mut prj, "7", "Hose list", PageKind::Report(ReportKind::HoseList.into()));
    add_page(&mut prj, "8", "Fluid device groups", PageKind::Report(ReportKind::FluidDeviceGroups.into()));
    add_page(&mut prj, "9", "Bill of materials", PageKind::Report(ReportKind::BillOfMaterials.into()));

    let db = PartsDb::default();
    assign_parts(
        &mut prj,
        &db,
        &[
            ("-AZ1", &["GEN.FR-G14"][..]),
            ("-QM1", &["GEN.VLV-52-24DC"]),
            ("-QM2", &["GEN.VLV-52-24DC"]),
            ("-RN1", &["GEN.THROTTLE-G18"]),
            ("-RN2", &["GEN.THROTTLE-G18"]),
            ("-RN3", &["GEN.THROTTLE-G18"]),
            ("-RN4", &["GEN.THROTTLE-G18"]),
            ("-MM1", &["GEN.CYL-32-100"]),
            ("-MM2", &["GEN.CYL-32-100"]),
            ("-F1", &["GEN.MCB-1P-C6"]),
            ("-T1", &["GEN.PS-24DC-5A"]),
            ("-S1", &["GEN.ESTOP-1NC"]),
            ("-S2", &["GEN.SEL-2POS-1NO"]),
            ("-S3", &["GEN.PB-GN-1NO"]),
            ("-BG1", &["GEN.LIMIT-1NO1NC"]),
            ("-BG2", &["GEN.LIMIT-1NC"]),
            ("-BG3", &["GEN.LIMIT-1NO1NC"]),
            ("-BP2", &["GEN.PRESS-SW-10BAR"]),
            ("-P1", &["GEN.LMP-GN-24"]),
            ("-P2", &["GEN.LMP-GN-24"]),
        ],
    );
    // Valve terminal -KH1 holds both valves; the coils -MB1 / -MB2 belong to the
    // valves -QM1 / -QM2 (synchronize links them by their number), the position
    // switches report the cylinder positions.
    prj.fluid.groups.push(DeviceGroup {
        tag: "-KH1".into(),
        kind: GroupKind::ValveTerminal,
        description: "Valve terminal press".into(),
        members: vec!["-QM1".into(), "-QM2".into()],
        slots: 4,
        part: String::new(),
    });
    crate::electrofluid::synchronize(&mut prj);
    prj.fluid.links.insert("-MM1".into(), vec!["-BG1".into()]);
    prj.fluid.links.insert("-MM2".into(), vec!["-BG2".into(), "-BG3".into()]);
    number_wires(&mut prj);
    prj
}

fn pneumatics(prj: &mut Project) {
    let page = add_page(prj, "2", "Pneumatic circuit", PageKind::Schematic);
    let mut g = Pg { prj, page };
    // Supply: compressed air, service unit (filter regulator) and pressure gauge.
    g.sym("fl_source", 60.0, 226.0, 2, "-GQ1", "", "Compressed air 6 bar").w(60.0, 222.0, 60.0, 208.0);
    g.sym("fl_frl", 60.0, 196.0, 3, "-AZ1", "", "");
    g.w(60.0, 184.0, 60.0, 170.0).w(60.0, 170.0, 60.0, 150.0);
    g.sym("fl_gauge", 44.0, 166.0, 0, "-BP1", "", "").w(44.0, 170.0, 60.0, 170.0);
    g.w(60.0, 150.0, 84.0, 150.0).w(84.0, 150.0, 184.0, 150.0);
    // Two axes: clamping cylinder -MM1 and pressing cylinder -MM2, each with its
    // 5/2 valve and two one-way flow control valves (exhaust throttling).
    for (i, x) in [80.0, 180.0].into_iter().enumerate() {
        let n = i + 1;
        g.w(x + 4.0, 150.0, x + 4.0, 128.0);
        let text = if n == 1 { "Clamp" } else { "Press" };
        g.sym("fl_valve52", x, 120.0, 0, &format!("-QM{n}"), "", text);
        g.sym("fl_throttle", x + 4.0, 80.0, 0, &format!("-RN{}", 2 * n - 1), "", "").w(x + 4.0, 112.0, x + 4.0, 88.0);
        g.path(&[(x + 4.0, 72.0), (x + 4.0, 60.0), (x, 60.0), (x, 44.0)]);
        g.sym("fl_throttle", x + 32.0, 80.0, 0, &format!("-RN{}", 2 * n), "", "");
        g.path(&[(x + 8.0, 112.0), (x + 8.0, 100.0), (x + 32.0, 100.0), (x + 32.0, 88.0)]);
        g.path(&[(x + 32.0, 72.0), (x + 32.0, 60.0), (x + 16.0, 60.0), (x + 16.0, 44.0)]);
        let text = if n == 1 { "Clamping cylinder" } else { "Pressing cylinder" };
        g.sym("fl_cylinder", x, 40.0, 0, &format!("-MM{n}"), "", text);
    }
    g.text(260.0, 60.0, "Fluid power", 3.5);
    g.text(260.0, 72.0, "Lines between fluid ports are hoses (H1, H2, ...) in nets of their own.", 2.5);
    g.text(260.0, 79.0, "The valves -QM1 / -QM2 sit on valve terminal -KH1; their solenoid coils", 2.5);
    g.text(260.0, 86.0, "-MB1 / -MB2 and the position switches -BG1 to -BG3 are on page 4.", 2.5);
}

fn supply(prj: &mut Project) {
    let page = add_page(prj, "3", "Power supply 24 V", PageKind::Schematic);
    let mut g = Pg { prj, page };
    for (y, key) in [(24.0, "pot_l1"), (32.0, "pot_n"), (40.0, "pot_pe")] {
        g.sym(key, 20.0, y, 3, "", "", "").w(24.0, y, 400.0, y);
    }
    g.sym("cb1", 60.0, 64.0, 0, "-F1", "", "").w(60.0, 24.0, 60.0, 60.0).w(60.0, 68.0, 60.0, 76.0);
    g.sym("psu", 60.0, 80.0, 0, "-T1", "", "24 V DC");
    g.w(68.0, 32.0, 68.0, 76.0).w(76.0, 40.0, 76.0, 76.0);
    g.path(&[(60.0, 96.0), (60.0, 112.0)]).sym("intr", 60.0, 112.0, 0, "", "+24V", "");
    g.path(&[(68.0, 96.0), (68.0, 104.0), (84.0, 104.0), (84.0, 112.0)]).sym("intr", 84.0, 112.0, 0, "", "0V", "");
}

fn control(prj: &mut Project) {
    let page = add_page(prj, "4", "Control press", PageKind::Schematic);
    let mut g = Pg { prj, page };
    g.sym("intr", 20.0, 24.0, 1, "", "+24V", "").w(20.0, 24.0, 400.0, 24.0);
    g.sym("intr", 20.0, 236.0, 1, "", "0V", "").w(20.0, 236.0, 400.0, 236.0);
    // The emergency stop switches off both valves.
    g.sym("estop", 40.0, 40.0, 0, "-S1", "", "Emergency stop").w(40.0, 24.0, 40.0, 36.0);
    g.w(40.0, 44.0, 40.0, 52.0).w(40.0, 52.0, 60.0, 52.0).w(60.0, 52.0, 120.0, 52.0);
    g.sym("sel", 60.0, 80.0, 0, "-S2", "", "Clamp").w(60.0, 52.0, 60.0, 76.0);
    g.sym("valve", 60.0, 200.0, 0, "-MB1", "", "").w(60.0, 84.0, 60.0, 196.0).w(60.0, 204.0, 60.0, 236.0);
    // Pressing only with the part clamped and until the bottom position.
    g.sym("limit", 120.0, 80.0, 0, "-BG1", "", "Part clamped").w(120.0, 52.0, 120.0, 76.0);
    g.sym("pb_no", 120.0, 112.0, 0, "-S3", "", "Press").w(120.0, 84.0, 120.0, 108.0);
    g.sym("limit_nc", 120.0, 144.0, 0, "-BG2", "", "Bottom position").w(120.0, 116.0, 120.0, 140.0);
    g.sym("valve", 120.0, 200.0, 0, "-MB2", "", "").w(120.0, 148.0, 120.0, 196.0).w(120.0, 204.0, 120.0, 236.0);
    // Signals.
    g.sym("pressure_no", 220.0, 80.0, 0, "-BP2", "", "Air pressure").w(220.0, 24.0, 220.0, 76.0);
    g.sym("lamp", 220.0, 200.0, 0, "-P1", "", "Air pressure OK").w(220.0, 84.0, 220.0, 196.0).w(220.0, 204.0, 220.0, 236.0);
    g.sym("limit", 290.0, 80.0, 0, "-BG3", "", "Top position").w(290.0, 24.0, 290.0, 76.0);
    g.sym("lamp", 290.0, 200.0, 0, "-P2", "", "Press open").w(290.0, 84.0, 290.0, 196.0).w(290.0, 204.0, 290.0, 236.0);
}
