//! Demo project: direct-on-line motor starter with control circuit and PLC I/O.

use crate::geom::{p, P};
use crate::model::*;
use crate::parts::PartsDb;
use crate::symbols;

struct Pg<'a> {
    prj: &'a mut Project,
    page: usize,
}

impl Pg<'_> {
    #[allow(clippy::too_many_arguments)]
    fn sym(&mut self, key: &str, x: f32, y: f32, rot: u8, tag: &str, label: &str, ftext: &str) -> &mut Self {
        let def = symbols::get(key).expect("symbol");
        let id = self.prj.new_id();
        let s = SymbolInst {
            id,
            sym: key.into(),
            pos: p(x, y),
            rot,
            mirror: false,
            tag: tag.into(),
            label: if label.is_empty() { def.default_label.into() } else { label.into() },
            function_text: ftext.into(),
            pins: def.pins.iter().map(|x| x.label.to_string()).collect(),
        };
        self.prj.pages[self.page].symbols.push(s);
        self
    }

    fn pins(&mut self, pins: &[&str]) -> &mut Self {
        if let Some(s) = self.prj.pages[self.page].symbols.last_mut() {
            s.pins = pins.iter().map(|x| x.to_string()).collect();
        }
        self
    }

    fn w(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> &mut Self {
        let id = self.prj.new_id();
        self.prj.pages[self.page].wires.push(Wire { id, a: p(x1, y1), b: p(x2, y2), ..Default::default() });
        self
    }

    /// Wire through several points.
    fn path(&mut self, pts: &[(f32, f32)]) -> &mut Self {
        for s in pts.windows(2) {
            self.w(s[0].0, s[0].1, s[1].0, s[1].1);
        }
        self
    }

    fn text(&mut self, x: f32, y: f32, t: &str, size: f32) -> &mut Self {
        let id = self.prj.new_id();
        self.prj.pages[self.page].texts.push(TextItem { id, pos: P { x, y }, text: t.into(), size });
        self
    }
}

fn add_page(prj: &mut Project, name: &str, desc: &str, kind: PageKind) -> usize {
    let id = prj.new_id();
    prj.pages.push(Page::new(id, name, desc, kind));
    prj.pages.len() - 1
}

pub fn demo_project() -> Project {
    let mut prj = Project { pages: Vec::new(), ..Default::default() };
    prj.info = ProjectInfo {
        name: "Demo conveyor".into(),
        description: "Direct-on-line motor starter\nwith PLC controlled valve".into(),
        company: "OpenECraft".into(),
        creator: "OpenECraft".into(),
        date: "2026-10-19".into(),
        revision: "A".into(),
    };

    add_page(&mut prj, "1", "Cover sheet", PageKind::Cover);

    // ------------------------------------------------------------------ power
    let page = add_page(&mut prj, "2", "Power supply conveyor motor", PageKind::Schematic);
    let mut g = Pg { prj: &mut prj, page };
    let rails = [(24.0, "pot_l1"), (32.0, "pot_l2"), (40.0, "pot_l3"), (48.0, "pot_n"), (56.0, "pot_pe")];
    for (y, key) in rails {
        g.sym(key, 20.0, y, 3, "", "", "").w(24.0, y, 400.0, y);
    }
    // Motor feeder
    g.sym("mps3", 60.0, 80.0, 0, "-Q1", "", "Motor protection");
    g.w(60.0, 24.0, 60.0, 76.0).w(68.0, 32.0, 68.0, 76.0).w(76.0, 40.0, 76.0, 76.0);
    g.sym("main3", 60.0, 112.0, 0, "-K1", "", "");
    g.w(60.0, 84.0, 60.0, 108.0).w(68.0, 84.0, 68.0, 108.0).w(76.0, 84.0, 76.0, 108.0);
    for (i, x) in [60.0, 68.0, 76.0].into_iter().enumerate() {
        g.sym("term", x, 140.0, 0, "-X1", &(i + 1).to_string(), "");
        g.w(x, 116.0, x, 136.0).w(x, 144.0, x, 152.0);
    }
    g.sym("term_pe", 84.0, 140.0, 0, "-X1", "PE", "");
    g.w(84.0, 56.0, 84.0, 136.0).w(84.0, 144.0, 84.0, 152.0);
    g.sym("motor3", 60.0, 156.0, 0, "-M1", "", "Conveyor\n0.75 kW");
    // 24 V power supply
    g.sym("cb1", 180.0, 64.0, 0, "-F1", "", "");
    g.w(180.0, 24.0, 180.0, 60.0).w(180.0, 68.0, 180.0, 76.0);
    g.sym("psu", 180.0, 80.0, 0, "-T1", "", "24 V DC");
    g.w(188.0, 48.0, 188.0, 76.0).w(196.0, 56.0, 196.0, 76.0);
    g.path(&[(180.0, 96.0), (180.0, 112.0)]).sym("intr", 180.0, 112.0, 0, "", "24V", "");
    g.path(&[(188.0, 96.0), (188.0, 104.0), (204.0, 104.0), (204.0, 112.0)]).sym("intr", 204.0, 112.0, 0, "", "0V", "");
    g.text(260.0, 90.0, "Supply 400 V / 50 Hz, TN-S", 3.0);

    // ------------------------------------------------------------------ control
    let page = add_page(&mut prj, "3", "Control circuit conveyor", PageKind::Schematic);
    let mut g = Pg { prj: &mut prj, page };
    g.sym("intr", 20.0, 24.0, 1, "", "24V", "").w(20.0, 24.0, 400.0, 24.0);
    g.sym("intr", 20.0, 236.0, 1, "", "0V", "").w(20.0, 236.0, 400.0, 236.0);
    g.sym("estop", 40.0, 40.0, 0, "-S1", "", "Emergency stop").w(40.0, 24.0, 40.0, 36.0);
    g.sym("pb_nc", 40.0, 56.0, 0, "-S2", "", "Stop").w(40.0, 44.0, 40.0, 52.0);
    g.sym("pb_no", 40.0, 76.0, 0, "-S3", "", "Start").w(40.0, 60.0, 40.0, 72.0);
    g.sym("no", 64.0, 76.0, 0, "-K1", "", "").pins(&["13", "14"]);
    g.path(&[(40.0, 64.0), (64.0, 64.0), (64.0, 72.0)]);
    g.path(&[(64.0, 80.0), (64.0, 88.0), (40.0, 88.0)]).w(40.0, 80.0, 40.0, 88.0);
    g.sym("ol_nc", 40.0, 104.0, 0, "-Q1", "", "").pins(&["21", "22"]);
    g.w(40.0, 88.0, 40.0, 100.0);
    g.sym("coil", 40.0, 128.0, 0, "-K1", "", "Conveyor motor");
    g.w(40.0, 108.0, 40.0, 124.0).w(40.0, 132.0, 40.0, 236.0);
    // Indicator
    g.sym("no", 120.0, 76.0, 0, "-K1", "", "").pins(&["53", "54"]);
    g.w(120.0, 24.0, 120.0, 72.0).w(120.0, 80.0, 120.0, 124.0);
    g.sym("lamp", 120.0, 128.0, 0, "-P1", "", "Motor running").w(120.0, 132.0, 120.0, 236.0);
    // Feedback to PLC
    g.sym("no", 200.0, 76.0, 0, "-K1", "", "").pins(&["83", "84"]);
    g.w(200.0, 24.0, 200.0, 72.0).w(200.0, 80.0, 200.0, 120.0).sym("intr", 200.0, 120.0, 0, "", "K1.FB", "");

    // ------------------------------------------------------------------ PLC
    let page = add_page(&mut prj, "4", "PLC inputs / outputs", PageKind::Schematic);
    let mut g = Pg { prj: &mut prj, page };
    g.sym("intr", 20.0, 24.0, 1, "", "24V", "").w(20.0, 24.0, 400.0, 24.0);
    g.sym("intr", 20.0, 236.0, 1, "", "0V", "").w(20.0, 236.0, 400.0, 236.0);
    g.sym("prox", 60.0, 60.0, 0, "-B1", "", "Part present").w(60.0, 24.0, 60.0, 56.0);
    g.w(60.0, 64.0, 60.0, 80.0).sym("plc_in", 60.0, 84.0, 0, "-A1", "I0.0", "Part present");
    g.sym("pb_no", 100.0, 60.0, 0, "-S4", "", "Reset").w(100.0, 24.0, 100.0, 56.0);
    g.w(100.0, 64.0, 100.0, 80.0).sym("plc_in", 100.0, 84.0, 0, "-A1", "I0.1", "Reset");
    g.sym("intr", 140.0, 72.0, 2, "", "K1.FB", "").w(140.0, 72.0, 140.0, 80.0);
    g.sym("plc_in", 140.0, 84.0, 0, "-A1", "I0.2", "Motor running");
    g.sym("plc_out", 240.0, 120.0, 0, "-A2", "Q0.0", "Valve").w(240.0, 124.0, 240.0, 156.0);
    g.sym("valve", 240.0, 160.0, 0, "-Y1", "", "Ejector valve").w(240.0, 164.0, 240.0, 236.0);
    g.sym("plc_out", 300.0, 120.0, 0, "-A2", "Q0.1", "Fault").w(300.0, 124.0, 300.0, 156.0);
    g.sym("lamp", 300.0, 160.0, 0, "-P2", "", "Fault").w(300.0, 164.0, 300.0, 236.0);
    g.text(220.0, 100.0, "Digital outputs -A2", 2.5);
    g.text(40.0, 112.0, "Digital inputs -A1", 2.5);

    add_page(&mut prj, "5", "Bill of materials", PageKind::Report(ReportKind::BillOfMaterials));
    add_page(&mut prj, "6", "Terminal diagram -X1", PageKind::Report(ReportKind::TerminalDiagram));
    add_page(&mut prj, "7", "Device list", PageKind::Report(ReportKind::DeviceList));

    let db = PartsDb::default();
    for (tag, parts) in [
        ("-Q1", &["GEN.MPS-2.5-4A", "GEN.MPS-AUX-1NO1NC"][..]),
        ("-K1", &["GEN.CT-9A-24DC", "GEN.AUX-2NO2NC"]),
        ("-M1", &["GEN.MOT-0.75KW"]),
        ("-X1", &["GEN.TB-2.5"]),
        ("-F1", &["GEN.MCB-1P-C6"]),
        ("-T1", &["GEN.PS-24DC-5A"]),
        ("-S1", &["GEN.ESTOP-1NC"]),
        ("-S2", &["GEN.PB-RD-1NC"]),
        ("-S3", &["GEN.PB-GN-1NO"]),
        ("-S4", &["GEN.PB-GN-1NO"]),
        ("-P1", &["GEN.LMP-GN-24"]),
        ("-P2", &["GEN.LMP-RD-24"]),
        ("-B1", &["GEN.PROX-M12-PNP"]),
        ("-Y1", &["GEN.SV-24DC"]),
        ("-A1", &["GEN.PLC-DI16"]),
        ("-A2", &["GEN.PLC-DO16"]),
    ] {
        let refs = parts.iter().map(|n| PartRef { number: n.to_string(), count: 1 }).collect();
        prj.devices.insert(tag.into(), DeviceData { parts: refs, ..Default::default() });
        for n in parts {
            crate::parts::ensure_in_project(&mut prj, &db, n);
        }
    }
    add_panel_layout(&mut prj, &db);
    prj
}

/// Enclosure +CAB1: cable duct frame, three mounting rails and the cabinet devices.
fn add_panel_layout(prj: &mut Project, db: &PartsDb) {
    use crate::panel::{self, ItemKind};
    for part in [panel::ENCLOSURE_PART, panel::RAIL_PART, panel::DUCT_PART] {
        crate::parts::ensure_in_project(prj, db, part);
    }
    let enc = panel::new_enclosure(prj, panel::ENCLOSURE_PART, "+CAB1");
    let enc_id = enc.id;
    prj.layout.enclosures.push(enc);
    // Plate 550 x 750 mm: duct frame plus two horizontal ducts between the rails.
    let ducts = [
        ((0.0, 20.0), (550.0, 20.0)),
        ((0.0, 730.0), (550.0, 730.0)),
        ((20.0, 40.0), (20.0, 710.0)),
        ((530.0, 40.0), (530.0, 710.0)),
        ((40.0, 270.0), (510.0, 270.0)),
        ((40.0, 500.0), (510.0, 500.0)),
    ];
    for (a, b) in ducts {
        let item = panel::new_profile(prj, ItemKind::Duct, panel::DUCT_PART, p(a.0, a.1), p(b.0, b.1));
        prj.layout.enclosures[0].items.push(item);
    }
    for y in [145.0, 385.0, 615.0] {
        let item = panel::new_profile(prj, ItemKind::Rail, panel::RAIL_PART, p(60.0, y), p(490.0, y));
        prj.layout.enclosures[0].items.push(item);
    }
    for (tag, x, rail_y) in [
        ("-Q1", 70.0, 145.0),
        ("-T1", 140.0, 145.0),
        ("-K1", 70.0, 385.0),
        ("-F1", 135.0, 385.0),
        ("-A1", 180.0, 385.0),
        ("-A2", 215.0, 385.0),
        ("-X1", 70.0, 615.0),
    ] {
        let h = panel::device_size(prj, tag).map(|d| d.h).unwrap_or(40.0);
        panel::place_device(prj, 0, tag, p(x, rail_y - h / 2.0));
    }
    let id = prj.new_id();
    let at = prj.pages.len();
    prj.pages.push(Page::new(id, &(at + 1).to_string(), "Mounting layout +CAB1", PageKind::Mounting(enc_id)));
}

#[cfg(test)]
mod tests;
