//! Level 2: start / stop circuit with a relay holding itself through its own contact.

use super::*;

pub(super) fn project() -> Project {
    let mut prj = Project { pages: Vec::new(), ..Default::default() };
    prj.info = ProjectInfo {
        name: "Self-holding relay".into(),
        description: "Start / stop circuit\nwith terminal strip".into(),
        company: "OpenECraft".into(),
        creator: "OpenECraft".into(),
        date: "2026-10-27".into(),
        revision: "A".into(),
        ..Default::default()
    };
    prj.apply_preset(crate::potentials::Preset::Iec60204);
    prj.wire_colors = true;
    prj.wire_numbering.scheme = NumberScheme::PageColumn;
    add_page(&mut prj, "1", "Cover sheet", PageKind::Cover);

    let page = add_page(&mut prj, "2", "Control circuit", PageKind::Schematic);
    let mut g = Pg { prj: &mut prj, page };
    g.sym("pot_24", 20.0, 24.0, 3, "", "+24V", "").w(24.0, 24.0, 400.0, 24.0);
    g.sym("pot_0v", 20.0, 236.0, 3, "", "0V", "").w(24.0, 236.0, 400.0, 236.0);
    g.sym("cb1", 60.0, 48.0, 0, "-F1", "", "Control voltage").w(60.0, 24.0, 60.0, 44.0);
    // The push buttons sit in the door: they are connected through terminal strip -X1.
    g.sym("term", 60.0, 72.0, 0, "-X1", "1", "").w(60.0, 52.0, 60.0, 68.0);
    g.sym("pb_nc", 60.0, 96.0, 0, "-S1", "", "Stop").w(60.0, 76.0, 60.0, 92.0);
    g.sym("pb_no", 60.0, 124.0, 0, "-S2", "", "Start").w(60.0, 100.0, 60.0, 120.0);
    g.sym("term", 60.0, 156.0, 0, "-X1", "2", "").w(60.0, 128.0, 60.0, 152.0);
    // Self-holding: NO contact 13/14 of -K1 in parallel to the start button.
    g.sym("no", 88.0, 124.0, 0, "-K1", "", "Self-holding").pins(&["13", "14"]);
    g.path(&[(60.0, 108.0), (88.0, 108.0), (88.0, 120.0)]);
    g.path(&[(88.0, 128.0), (88.0, 140.0), (60.0, 140.0)]);
    g.sym("coil", 60.0, 196.0, 0, "-K1", "", "Machine on").w(60.0, 160.0, 60.0, 192.0).w(60.0, 200.0, 60.0, 236.0);
    // Indicator lamps: green while -K1 is on, red while it is off.
    g.sym("no", 160.0, 124.0, 0, "-K1", "", "").pins(&["53", "54"]);
    g.w(160.0, 24.0, 160.0, 120.0).w(160.0, 128.0, 160.0, 164.0);
    g.sym("term", 160.0, 168.0, 0, "-X1", "3", "").w(160.0, 172.0, 160.0, 192.0);
    g.sym("lamp", 160.0, 196.0, 0, "-P1", "", "Machine on").w(160.0, 200.0, 160.0, 236.0);
    g.sym("nc", 220.0, 124.0, 0, "-K1", "", "").pins(&["61", "62"]);
    g.w(220.0, 24.0, 220.0, 120.0).w(220.0, 128.0, 220.0, 164.0);
    g.sym("term", 220.0, 168.0, 0, "-X1", "4", "").w(220.0, 172.0, 220.0, 192.0);
    g.sym("lamp", 220.0, 196.0, 0, "-P2", "", "Machine off").w(220.0, 200.0, 220.0, 236.0);
    g.text(270.0, 60.0, "Devices and cross references", 3.5);
    g.text(270.0, 72.0, "Coil and contacts of -K1 form one device: the contact image", 2.5);
    g.text(270.0, 78.0, "below the coil shows where each contact is used.", 2.5);
    g.text(270.0, 88.0, "Door devices connect through terminal strip -X1 (page 3).", 2.5);
    g.text(270.0, 98.0, "Wire numbers follow page and column (2.1.1, ...).", 2.5);

    add_page(&mut prj, "3", "Terminal diagram -X1", PageKind::Report(ReportKind::TerminalDiagram.into()));
    add_page(&mut prj, "4", "Connection list", PageKind::Report(ReportKind::ConnectionList.into()));
    add_page(&mut prj, "5", "Device list", PageKind::Report(ReportKind::DeviceList.into()));
    add_page(&mut prj, "6", "Bill of materials", PageKind::Report(ReportKind::BillOfMaterials.into()));
    let db = PartsDb::default();
    assign_parts(
        &mut prj,
        &db,
        &[
            ("-F1", &["GEN.MCB-1P-C6"][..]),
            ("-K1", &["GEN.CT-9A-24DC", "GEN.AUX-2NO2NC"]),
            ("-X1", &["GEN.TB-2.5"]),
            ("-S1", &["GEN.PB-RD-1NC"]),
            ("-S2", &["GEN.PB-GN-1NO"]),
            ("-P1", &["GEN.LMP-GN-24"]),
            ("-P2", &["GEN.LMP-RD-24"]),
        ],
    );
    number_wires(&mut prj);
    prj
}
