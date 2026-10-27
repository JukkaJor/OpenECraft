//! Level 3: star-delta starter, power and control circuit on separate pages.

use super::*;

pub(super) fn project() -> Project {
    let mut prj = Project { pages: Vec::new(), ..Default::default() };
    prj.info = ProjectInfo {
        name: "Star-delta starter".into(),
        description: "Fan motor 1.5 kW\nstar-delta start".into(),
        company: "OpenECraft".into(),
        creator: "OpenECraft".into(),
        date: "2026-10-27".into(),
        revision: "A".into(),
        ..Default::default()
    };
    prj.apply_preset(crate::potentials::Preset::Iec60204);
    prj.wire_colors = true;
    add_page(&mut prj, "1", "Cover sheet", PageKind::Cover);
    power(&mut prj);
    control(&mut prj);
    add_page(&mut prj, "4", "Terminal diagram -X1", PageKind::Report(ReportKind::TerminalDiagram.into()));
    add_page(&mut prj, "5", "Cable overview", PageKind::Report(ReportKind::CableOverview.into()));
    add_page(&mut prj, "6", "Cable diagram -W1", PageKind::Report(ReportKind::CableDiagram.into()));
    add_page(&mut prj, "7", "Device list", PageKind::Report(ReportKind::DeviceList.into()));
    add_page(&mut prj, "8", "Bill of materials", PageKind::Report(ReportKind::BillOfMaterials.into()));

    let db = PartsDb::default();
    assign_parts(
        &mut prj,
        &db,
        &[
            ("-Q0", &["GEN.MS-3P-40A"][..]),
            ("-Q1", &["GEN.MPS-4-6.3A", "GEN.MPS-AUX-1NO1NC"]),
            ("-K1", &["GEN.CT-18A-24DC"]),
            ("-K2", &["GEN.CT-9A-24DC", "GEN.AUX-1NO1NC"]),
            ("-K3", &["GEN.CT-9A-24DC", "GEN.AUX-1NO1NC"]),
            ("-K4", &["GEN.TMR-24DC-OND-2"]),
            ("-M1", &["GEN.MOT-1.5KW-PTC"]),
            ("-X1", &["GEN.TB-2.5", "GEN.TB-PE-2.5"]),
            ("-F1", &["GEN.MCB-1P-C6"]),
            ("-T1", &["GEN.PS-24DC-5A"]),
            ("-S1", &["GEN.PB-RD-1NC"]),
            ("-S2", &["GEN.PB-GN-1NO"]),
            ("-P1", &["GEN.LMP-GN-24"]),
            ("-P2", &["GEN.LMP-GN-24"]),
        ],
    );
    motor_cable(&mut prj, &db);
    number_wires(&mut prj);
    prj
}

fn power(prj: &mut Project) {
    let page = add_page(prj, "2", "Power circuit fan motor", PageKind::Schematic);
    let mut g = Pg { prj, page };
    for (y, key) in [(24.0, "pot_l1"), (32.0, "pot_l2"), (40.0, "pot_l3"), (48.0, "pot_n"), (56.0, "pot_pe")] {
        g.sym(key, 20.0, y, 3, "", "", "").w(24.0, y, 400.0, y);
    }
    g.sym("disc3", 60.0, 72.0, 0, "-Q0", "", "Main switch");
    g.w(60.0, 24.0, 60.0, 68.0).w(68.0, 32.0, 68.0, 68.0).w(76.0, 40.0, 76.0, 68.0);
    g.sym("mps3", 60.0, 100.0, 0, "-Q1", "", "Motor protection");
    for x in [60.0, 68.0, 76.0] {
        g.w(x, 76.0, x, 96.0);
    }
    // Mains contactor -K1 feeds U1 V1 W1.
    g.sym("main3", 60.0, 136.0, 0, "-K1", "", "Mains");
    for x in [60.0, 68.0, 76.0] {
        g.w(x, 104.0, x, 132.0).w(x, 140.0, x, 160.0);
    }
    // Delta contactor -K3 connects L1 L2 L3 to W2 U2 V2.
    g.path(&[(60.0, 112.0), (140.0, 112.0), (140.0, 132.0)]);
    g.path(&[(68.0, 116.0), (148.0, 116.0), (148.0, 132.0)]);
    g.path(&[(76.0, 120.0), (156.0, 120.0), (156.0, 132.0)]);
    g.sym("main3", 140.0, 136.0, 0, "-K3", "", "Delta");
    for x in [140.0, 148.0, 156.0] {
        g.w(x, 140.0, x, 160.0);
    }
    // Star contactor -K2 shorts W2 U2 V2.
    g.path(&[(140.0, 148.0), (220.0, 148.0), (220.0, 180.0)]);
    g.path(&[(148.0, 152.0), (228.0, 152.0), (228.0, 180.0)]);
    g.path(&[(156.0, 156.0), (236.0, 156.0), (236.0, 180.0)]);
    g.sym("main3", 220.0, 184.0, 0, "-K2", "", "Star");
    g.path(&[(220.0, 188.0), (220.0, 196.0), (236.0, 196.0)]).w(228.0, 188.0, 228.0, 196.0).w(236.0, 188.0, 236.0, 196.0);
    g.text(244.0, 197.0, "Star point", 2.5);
    // Terminals and motor: -X1:1-3 = U1 V1 W1, -X1:4-6 = W2 U2 V2.
    for (i, x) in [60.0, 68.0, 76.0, 140.0, 148.0, 156.0].into_iter().enumerate() {
        g.sym("term", x, 164.0, 0, "-X1", &(i + 1).to_string(), "");
    }
    g.sym("term_pe", 84.0, 164.0, 0, "-X1", "PE", "").w(84.0, 56.0, 84.0, 160.0);
    for x in [60.0, 68.0, 76.0, 84.0] {
        g.w(x, 168.0, x, 192.0);
    }
    g.sym("motor3_6", 60.0, 196.0, 0, "-M1", "", "Fan\n1.5 kW");
    g.path(&[(140.0, 168.0), (140.0, 228.0), (76.0, 228.0), (76.0, 216.0)]);
    g.path(&[(148.0, 168.0), (148.0, 232.0), (60.0, 232.0), (60.0, 216.0)]);
    g.path(&[(156.0, 168.0), (156.0, 224.0), (68.0, 224.0), (68.0, 216.0)]);
    // The motor circuit is a power circuit (black, 1.5 mm²).
    for w in g.prj.pages[page].wires.iter_mut().filter(|w| w.a.y >= 76.0 && w.a.x < 260.0 && w.a.x != 84.0) {
        w.color = "BK".into();
        w.cross_section = "1.5 mm²".into();
    }
    // 24 V supply of the control circuit.
    g.sym("cb1", 300.0, 64.0, 0, "-F1", "", "").w(300.0, 24.0, 300.0, 60.0).w(300.0, 68.0, 300.0, 76.0);
    g.sym("psu", 300.0, 80.0, 0, "-T1", "", "24 V DC");
    g.w(308.0, 48.0, 308.0, 76.0).w(316.0, 56.0, 316.0, 76.0);
    g.path(&[(300.0, 96.0), (300.0, 112.0)]).sym("intr", 300.0, 112.0, 0, "", "+24V", "");
    g.path(&[(308.0, 96.0), (308.0, 104.0), (324.0, 104.0), (324.0, 112.0)]).sym("intr", 324.0, 112.0, 0, "", "0V", "");
    g.text(260.0, 140.0, "Starts in star, after the time of -K4 in delta.", 2.5);
    g.text(260.0, 147.0, "-X1:4-6 lead to W2 U2 V2 (cable -W1, 7 cores).", 2.5);
}

fn control(prj: &mut Project) {
    let page = add_page(prj, "3", "Control circuit fan motor", PageKind::Schematic);
    let mut g = Pg { prj, page };
    g.sym("intr", 20.0, 24.0, 1, "", "+24V", "").w(20.0, 24.0, 400.0, 24.0);
    g.sym("intr", 20.0, 236.0, 1, "", "0V", "").w(20.0, 236.0, 400.0, 236.0);
    g.sym("ol_nc", 40.0, 40.0, 0, "-Q1", "", "").pins(&["21", "22"]).w(40.0, 24.0, 40.0, 36.0);
    g.sym("pb_nc", 40.0, 64.0, 0, "-S1", "", "Stop").w(40.0, 44.0, 40.0, 60.0);
    g.sym("pb_no", 40.0, 88.0, 0, "-S2", "", "Start").w(40.0, 68.0, 40.0, 84.0);
    g.sym("no", 64.0, 88.0, 0, "-K1", "", "").pins(&["13", "14"]);
    g.path(&[(40.0, 76.0), (64.0, 76.0), (64.0, 84.0)]).w(64.0, 92.0, 64.0, 100.0);
    g.w(40.0, 92.0, 40.0, 100.0);
    g.w(40.0, 100.0, 100.0, 100.0).w(100.0, 100.0, 140.0, 100.0).w(140.0, 100.0, 200.0, 100.0);
    g.sym("coil", 40.0, 200.0, 0, "-K1", "", "Mains").w(40.0, 100.0, 40.0, 196.0).w(40.0, 204.0, 40.0, 236.0);
    g.sym("coil_on", 100.0, 200.0, 0, "-K4", "", "Star time").w(100.0, 100.0, 100.0, 196.0).w(100.0, 204.0, 100.0, 236.0);
    // Star: until the time of -K4 has run, locked against delta.
    g.sym("nc_on", 140.0, 128.0, 0, "-K4", "", "").pins(&["25", "26"]).w(140.0, 100.0, 140.0, 124.0);
    g.sym("nc", 140.0, 160.0, 0, "-K3", "", "").pins(&["61", "62"]).w(140.0, 132.0, 140.0, 156.0);
    g.sym("coil", 140.0, 200.0, 0, "-K2", "", "Star").w(140.0, 164.0, 140.0, 196.0).w(140.0, 204.0, 140.0, 236.0);
    // Delta: after the time, locked against star.
    g.sym("no_on", 200.0, 128.0, 0, "-K4", "", "").pins(&["17", "18"]).w(200.0, 100.0, 200.0, 124.0);
    g.sym("nc", 200.0, 160.0, 0, "-K2", "", "").pins(&["61", "62"]).w(200.0, 132.0, 200.0, 156.0);
    g.sym("coil", 200.0, 200.0, 0, "-K3", "", "Delta").w(200.0, 164.0, 200.0, 196.0).w(200.0, 204.0, 200.0, 236.0);
    // Indicator lamps.
    g.sym("no", 270.0, 128.0, 0, "-K2", "", "").pins(&["13", "14"]).w(270.0, 24.0, 270.0, 124.0);
    g.sym("lamp", 270.0, 200.0, 0, "-P1", "", "Star").w(270.0, 132.0, 270.0, 196.0).w(270.0, 204.0, 270.0, 236.0);
    g.sym("no", 330.0, 128.0, 0, "-K3", "", "").pins(&["13", "14"]).w(330.0, 24.0, 330.0, 124.0);
    g.sym("lamp", 330.0, 200.0, 0, "-P2", "", "Delta").w(330.0, 132.0, 330.0, 196.0).w(330.0, 204.0, 330.0, 236.0);
}

/// Cable -W1 from -X1 to the motor: cores 1-6 and GNYE.
fn motor_cable(prj: &mut Project, db: &PartsDb) {
    let part = db.get("GEN.CBL-7G1.0").expect("cable part");
    let id = prj.new_id();
    prj.cables.push(Cable {
        id,
        tag: "-W1".into(),
        cable_type: part.type_number.clone(),
        cores: part.tech_value("Cores").unwrap_or_default().into(),
        cross_section: part.tech_value("Cross-section").unwrap_or_default().into(),
        length: 12.0,
        part: part.number.clone(),
        description: "Fan motor".into(),
        ..Default::default()
    });
    crate::parts::ensure_in_project(prj, db, "GEN.CBL-7G1.0");
    let page = prj.pages.iter().position(|p| p.name == "2").expect("page 2");
    // The wire leaving each terminal towards the motor carries the core.
    let cores = [(60.0, "1"), (68.0, "2"), (76.0, "3"), (140.0, "4"), (148.0, "5"), (156.0, "6"), (84.0, "GNYE")];
    for w in prj.pages[page].wires.iter_mut().filter(|w| w.a.y == 168.0) {
        if let Some((_, core)) = cores.iter().find(|c| c.0 == w.a.x) {
            w.cable = "-W1".into();
            w.core = core.to_string();
        }
    }
}
