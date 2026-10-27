//! Level 1: push buttons switch lamps between the +24V and 0V potentials.

use super::*;

pub(super) fn project() -> Project {
    let mut prj = Project { pages: Vec::new(), ..Default::default() };
    prj.info = ProjectInfo {
        name: "First circuit".into(),
        description: "Push buttons and lamps\non one schematic page".into(),
        company: "OpenECraft".into(),
        creator: "OpenECraft".into(),
        date: "2026-10-27".into(),
        revision: "A".into(),
        ..Default::default()
    };
    prj.apply_preset(crate::potentials::Preset::Iec60204);
    prj.wire_colors = true;
    add_page(&mut prj, "1", "Cover sheet", PageKind::Cover);

    let page = add_page(&mut prj, "2", "Lamp circuits", PageKind::Schematic);
    let mut g = Pg { prj: &mut prj, page };
    // Potential rails: every wire touching them carries +24V or 0V.
    g.sym("pot_24", 20.0, 24.0, 3, "", "+24V", "").w(24.0, 24.0, 400.0, 24.0);
    g.sym("pot_0v", 20.0, 236.0, 3, "", "0V", "").w(24.0, 236.0, 400.0, 236.0);
    // The fuse protects both circuits.
    g.sym("cb1", 60.0, 48.0, 0, "-F1", "", "Control voltage").w(60.0, 24.0, 60.0, 44.0);
    g.path(&[(60.0, 52.0), (60.0, 64.0), (140.0, 64.0)]);
    // Circuit 1: push button -S1 lights the green lamp -P1 while it is pressed.
    g.sym("pb_no", 60.0, 100.0, 0, "-S1", "", "Test lamp").w(60.0, 64.0, 60.0, 96.0);
    g.sym("lamp", 60.0, 180.0, 0, "-P1", "", "Lamp test").w(60.0, 104.0, 60.0, 176.0).w(60.0, 184.0, 60.0, 236.0);
    // Circuit 2: selector switch -S2 keeps the red lamp -P2 on.
    g.sym("sel", 140.0, 100.0, 0, "-S2", "", "Light on / off").w(140.0, 64.0, 140.0, 96.0);
    g.sym("lamp", 140.0, 180.0, 0, "-P2", "", "Light").w(140.0, 104.0, 140.0, 176.0).w(140.0, 184.0, 140.0, 236.0);
    g.text(220.0, 60.0, "First steps", 3.5);
    g.text(220.0, 72.0, "Symbols come from the Insert center; their tags (-S1, -P1) are assigned automatically.", 2.5);
    g.text(220.0, 80.0, "The rails carry the potentials +24V and 0V: the wires take their colour BU.", 2.5);
    g.text(220.0, 88.0, "Each device has a part: see the bill of materials on page 3.", 2.5);

    add_page(&mut prj, "3", "Bill of materials", PageKind::Report(ReportKind::BillOfMaterials.into()));
    let db = PartsDb::default();
    assign_parts(
        &mut prj,
        &db,
        &[
            ("-F1", &["GEN.MCB-1P-C6"][..]),
            ("-S1", &["GEN.PB-GN-1NO"]),
            ("-S2", &["GEN.SEL-2POS-1NO"]),
            ("-P1", &["GEN.LMP-GN-24"]),
            ("-P2", &["GEN.LMP-RD-24"]),
        ],
    );
    number_wires(&mut prj);
    prj
}
