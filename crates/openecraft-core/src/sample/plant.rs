//! Level 6: pump station structured after IEC 81346: functions (`=SUP`, `=P1`,
//! `=P2`, `=PLC`) and locations (`+CAB1` cabinet, `+FLD` field), page sections, a
//! single-line diagram, a PLC with digital and analog cards, a sensor plug.

use super::*;
use crate::structure::Aspect;

const PHASES: [(f32, &str); 5] = [(24.0, "L1"), (32.0, "L2"), (40.0, "L3"), (48.0, "N"), (56.0, "PE")];

pub(super) fn project() -> Project {
    let mut prj = Project { pages: Vec::new(), ..Default::default() };
    prj.info = ProjectInfo {
        name: "Pump station".into(),
        description: "Two pumps with PLC control\nstructured after IEC 81346".into(),
        company: "OpenECraft".into(),
        creator: "OpenECraft".into(),
        date: "2026-10-27".into(),
        revision: "A".into(),
        ..Default::default()
    };
    prj.apply_preset(crate::potentials::Preset::Iec60204);
    prj.wire_colors = true;
    for (aspect, id, desc) in [
        (Aspect::Function, "SUP", "Infeed and supply"),
        (Aspect::Function, "P1", "Pump 1 (direct on line)"),
        (Aspect::Function, "P2", "Pump 2 (frequency converter)"),
        (Aspect::Function, "PLC", "Control"),
        (Aspect::Location, "CAB1", "Control cabinet"),
        (Aspect::Location, "FLD", "Field (pump pit)"),
    ] {
        prj.structures.define(aspect, id, desc);
    }
    let general = crate::sections::add(&mut prj, 0, "General");
    let infeed = crate::sections::add(&mut prj, 0, "Infeed");
    let pump1 = crate::sections::add(&mut prj, 0, "Pump 1");
    let pump2 = crate::sections::add(&mut prj, 0, "Pump 2");
    let plc = crate::sections::add(&mut prj, 0, "PLC");
    let reports = crate::sections::add(&mut prj, 0, "Reports");

    sec_page(&mut prj, general, "1", "Cover sheet", PageKind::Cover, "", "");
    single_line(&mut prj, general);
    supply(&mut prj, infeed);
    pump_dol(&mut prj, pump1);
    pump_vfd(&mut prj, pump2);
    plc_page(&mut prj, plc);
    for (n, desc, kind) in [
        ("1", "Table of contents", ReportKind::TableOfContents),
        ("2", "PLC assignment", ReportKind::PlcAssignment),
        ("3", "Plug diagram", ReportKind::PlugDiagram),
        ("4", "Terminal diagram", ReportKind::TerminalDiagram),
        ("5", "Device list", ReportKind::DeviceList),
        ("6", "Bill of materials", ReportKind::BillOfMaterials),
    ] {
        sec_page(&mut prj, reports, n, desc, PageKind::Report(kind.into()), "", "");
    }

    let db = PartsDb::default();
    assign_parts(
        &mut prj,
        &db,
        &[
            ("=SUP+CAB1-F1", &["GEN.MCB-1P-C6"][..]),
            ("=SUP+CAB1-T1", &["GEN.PS-24DC-5A"]),
            ("=SUP+FLD-BP1", &["GEN.PT-10BAR-420"]),
            ("=SUP+FLD-X10", &["GEN.CON-M12-4"]),
            ("=P1+CAB1-Q1", &["GEN.MPS-2.5-4A", "GEN.MPS-AUX-1NO1NC"]),
            ("=P1+CAB1-K1", &["GEN.CT-9A-24DC"]),
            ("=P1+CAB1-X1", &["GEN.TB-2.5", "GEN.TB-PE-2.5"]),
            ("=P1+CAB1-P1", &["GEN.LMP-RD-24"]),
            ("=P1+FLD-M1", &["GEN.MOT-0.75KW"]),
            ("=P1+FLD-BL1", &["GEN.FLOAT-SW-1NO"]),
            ("=P2+CAB1-F1", &["GEN.MCB-3P-C16"]),
            ("=P2+CAB1-T1", &["GEN.VFD-0.75KW"]),
            ("=P2+CAB1-X1", &["GEN.TB-2.5", "GEN.TB-PE-2.5"]),
            ("=P2+CAB1-P1", &["GEN.LMP-RD-24"]),
            ("=P2+FLD-M1", &["GEN.MOT-0.75KW"]),
            ("=P2+FLD-BL1", &["GEN.FLOAT-SW-1NO"]),
            ("=PLC+CAB1-A1", &["GEN.PLC-DI16"]),
            ("=PLC+CAB1-A2", &["GEN.PLC-DO16"]),
            ("=PLC+CAB1-A3", &["GEN.PLC-AI4"]),
        ],
    );
    for (card, start) in [("=PLC+CAB1-A1", 0), ("=PLC+CAB1-A2", 0), ("=PLC+CAB1-A3", 64)] {
        prj.plc_cards.insert(card.into(), PlcCard { start_byte: start, rack: "0".into(), ..Default::default() });
    }
    number_wires(&mut prj);
    prj
}

/// A page of a section with its structure; the name starts with the section number.
fn sec_page(prj: &mut Project, section: u64, n: &str, desc: &str, kind: PageKind, function: &str, location: &str) -> usize {
    let name = format!("{}.{n}", crate::sections::full_number(prj, section));
    let i = add_page(prj, &name, desc, kind);
    let page = &mut prj.pages[i];
    page.section = section;
    page.function = function.into();
    page.location = location.into();
    i
}

/// Overview of the power distribution: planned with single-line symbols, engineered
/// on the multi-line pages (same device tags; the check compares both).
fn single_line(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "2", "Single-line diagram", PageKind::Overview, "", "");
    let mut g = Pg { prj, page };
    g.w(80.0, 60.0, 260.0, 60.0);
    g.text(80.0, 54.0, "3/N/PE AC 400 V 50 Hz", 2.5);
    g.sym("sl_mps", 100.0, 84.0, 0, "=P1+CAB1-Q1", "", "Pump 1").w(100.0, 60.0, 100.0, 76.0);
    g.sym("sl_contactor", 100.0, 116.0, 0, "=P1+CAB1-K1", "", "").w(100.0, 92.0, 100.0, 108.0);
    g.sym("sl_motor", 100.0, 148.0, 0, "=P1+FLD-M1", "", "").w(100.0, 124.0, 100.0, 140.0);
    g.sym("sl_cb", 200.0, 84.0, 0, "=P2+CAB1-F1", "", "Pump 2").w(200.0, 60.0, 200.0, 76.0);
    // The frequency converter has no single-line symbol: a note stands for it.
    g.sym("sl_motor", 200.0, 148.0, 0, "=P2+FLD-M1", "", "").w(200.0, 92.0, 200.0, 108.0).w(200.0, 128.0, 200.0, 140.0);
    g.text(196.0, 119.0, "=P2+CAB1-T1 frequency converter", 2.5);
}

/// Power rails continued from the infeed page.
fn rails_in(g: &mut Pg) {
    for (y, name) in PHASES {
        g.sym("intr", 20.0, y, 1, "", name, "").w(20.0, y, 400.0, y);
    }
}

fn dc_rails(g: &mut Pg) {
    g.sym("intr", 20.0, 24.0, 1, "", "+24V", "").w(20.0, 24.0, 400.0, 24.0);
    g.sym("intr", 20.0, 236.0, 1, "", "0V", "").w(20.0, 236.0, 400.0, 236.0);
}

fn supply(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "1", "Infeed and 24 V supply", PageKind::Schematic, "SUP", "CAB1");
    let mut g = Pg { prj, page };
    for ((y, name), key) in PHASES.into_iter().zip(["pot_l1", "pot_l2", "pot_l3", "pot_n", "pot_pe"]) {
        g.sym(key, 20.0, y, 3, "", "", "").w(24.0, y, 400.0, y).sym("intr", 400.0, y, 3, "", name, "");
    }
    g.sym("cb1", 60.0, 80.0, 0, "-F1", "", "").w(60.0, 24.0, 60.0, 76.0).w(60.0, 84.0, 60.0, 92.0);
    g.sym("psu", 60.0, 96.0, 0, "-T1", "", "24 V DC");
    g.w(68.0, 48.0, 68.0, 92.0).w(76.0, 56.0, 76.0, 92.0);
    g.path(&[(60.0, 112.0), (60.0, 128.0)]).sym("intr", 60.0, 128.0, 0, "", "+24V", "");
    g.path(&[(68.0, 112.0), (68.0, 120.0), (84.0, 120.0), (84.0, 128.0)]).sym("intr", 84.0, 128.0, 0, "", "0V", "");
    g.text(160.0, 100.0, "Structure identifiers (IEC 81346)", 3.5);
    g.text(160.0, 112.0, "=SUP infeed, =P1 / =P2 pumps, =PLC control; +CAB1 cabinet, +FLD field.", 2.5);
    g.text(160.0, 119.0, "Devices take the structure of their page: -F1 here is =SUP+CAB1-F1.", 2.5);
    g.text(160.0, 126.0, "Tags on a page show only what differs from the page (+FLD-M1).", 2.5);
}

/// Pump 1: motor protection switch, contactor, terminals, motor in the field.
fn pump_dol(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "1", "Power pump 1", PageKind::Schematic, "P1", "CAB1");
    let mut g = Pg { prj, page };
    rails_in(&mut g);
    g.sym("mps3", 60.0, 80.0, 0, "-Q1", "", "Pump 1");
    g.w(60.0, 24.0, 60.0, 76.0).w(68.0, 32.0, 68.0, 76.0).w(76.0, 40.0, 76.0, 76.0);
    for x in [60.0, 68.0, 76.0] {
        g.w(x, 84.0, x, 108.0);
    }
    g.sym("main3", 60.0, 112.0, 0, "-K1", "", "Pump 1");
    feeder_to_motor(&mut g, 84.0, 116.0);

    let page = sec_page(g.prj, section, "2", "Control pump 1", PageKind::Schematic, "P1", "CAB1");
    let mut g = Pg { prj: g.prj, page };
    dc_rails(&mut g);
    g.sym("no", 60.0, 60.0, 0, "-Q1", "", "").pins(&["13", "14"]).w(60.0, 24.0, 60.0, 56.0);
    g.sym("plc_in", 60.0, 84.0, 0, "=PLC-A1", "I0.0", "Pump 1 ready").w(60.0, 64.0, 60.0, 80.0);
    g.sym("no", 110.0, 60.0, 0, "-K1", "", "").pins(&["13", "14"]).w(110.0, 24.0, 110.0, 56.0);
    g.sym("plc_in", 110.0, 84.0, 0, "=PLC-A1", "I0.1", "Pump 1 running").w(110.0, 64.0, 110.0, 80.0);
    g.sym("level_no", 160.0, 60.0, 0, "+FLD-BL1", "", "Dry-run protection").w(160.0, 24.0, 160.0, 56.0);
    g.sym("plc_in", 160.0, 84.0, 0, "=PLC-A1", "I0.2", "Pump 1 water level").w(160.0, 64.0, 160.0, 80.0);
    g.sym("plc_out", 240.0, 120.0, 0, "=PLC-A2", "Q0.0", "Pump 1 on");
    g.sym("coil", 240.0, 200.0, 0, "-K1", "", "Pump 1").w(240.0, 124.0, 240.0, 196.0).w(240.0, 204.0, 240.0, 236.0);
    g.sym("plc_out", 300.0, 120.0, 0, "=PLC-A2", "Q0.1", "Pump 1 fault");
    g.sym("lamp", 300.0, 200.0, 0, "-P1", "", "Fault pump 1").w(300.0, 124.0, 300.0, 196.0).w(300.0, 204.0, 300.0, 236.0);
}

/// Pump 2: circuit breaker, frequency converter, terminals, motor in the field.
fn pump_vfd(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "1", "Power pump 2", PageKind::Schematic, "P2", "CAB1");
    let mut g = Pg { prj, page };
    rails_in(&mut g);
    g.sym("cb3", 60.0, 80.0, 0, "-F1", "", "Pump 2");
    g.w(60.0, 24.0, 60.0, 76.0).w(68.0, 32.0, 68.0, 76.0).w(76.0, 40.0, 76.0, 76.0);
    for x in [60.0, 68.0, 76.0] {
        g.w(x, 84.0, x, 92.0);
    }
    g.sym("vfd", 60.0, 96.0, 0, "-T1", "", "Pump 2\nspeed");
    g.w(84.0, 56.0, 84.0, 92.0);
    feeder_to_motor(&mut g, 92.0, 112.0);

    let page = sec_page(g.prj, section, "2", "Control pump 2", PageKind::Schematic, "P2", "CAB1");
    let mut g = Pg { prj: g.prj, page };
    dc_rails(&mut g);
    // Relay output of the frequency converter: 14 ready, 12 fault.
    g.sym("co", 60.0, 60.0, 2, "-T1", "", "").pins(&["11", "12", "14"]).mirror().w(60.0, 24.0, 60.0, 56.0);
    g.sym("plc_in", 60.0, 84.0, 0, "=PLC-A1", "I0.3", "Pump 2 ready").w(60.0, 64.0, 60.0, 80.0);
    g.path(&[(64.0, 64.0), (64.0, 70.0), (100.0, 70.0), (100.0, 80.0)]);
    g.sym("plc_in", 100.0, 84.0, 0, "=PLC-A1", "I0.5", "Pump 2 converter fault");
    g.sym("level_no", 160.0, 60.0, 0, "+FLD-BL1", "", "Dry-run protection").w(160.0, 24.0, 160.0, 56.0);
    g.sym("plc_in", 160.0, 84.0, 0, "=PLC-A1", "I0.4", "Pump 2 water level").w(160.0, 64.0, 160.0, 80.0);
    // Run command: PLC output to the digital input DI1 of the converter.
    g.sym("plc_out", 240.0, 120.0, 0, "=PLC-A2", "Q0.2", "Pump 2 on");
    g.sym("plc_in", 240.0, 164.0, 0, "-T1", "DI1", "Run").w(240.0, 124.0, 240.0, 160.0);
    g.sym("plc_out", 300.0, 120.0, 0, "=PLC-A2", "Q0.3", "Pump 2 fault");
    g.sym("lamp", 300.0, 200.0, 0, "-P1", "", "Fault pump 2").w(300.0, 124.0, 300.0, 196.0).w(300.0, 204.0, 300.0, 236.0);
}

/// Outputs at y = `from` (x 60 / 68 / 76) through terminals -X1 to the motor +FLD-M1;
/// PE from the rail at x = `pe`.
fn feeder_to_motor(g: &mut Pg, pe: f32, from: f32) {
    for (i, x) in [60.0, 68.0, 76.0].into_iter().enumerate() {
        g.sym("term", x, 140.0, 0, "-X1", &(i + 1).to_string(), "");
        g.w(x, from, x, 136.0).w(x, 144.0, x, 168.0);
    }
    g.sym("term_pe", pe, 140.0, 0, "-X1", "PE", "").w(pe, 56.0, pe, 136.0);
    if pe == 84.0 {
        g.w(84.0, 144.0, 84.0, 168.0);
    } else {
        g.path(&[(pe, 144.0), (pe, 156.0), (84.0, 156.0), (84.0, 168.0)]);
    }
    g.sym("motor3", 60.0, 172.0, 0, "+FLD-M1", "", "Pump motor\n0.75 kW");
    for w in g.prj.pages[g.page].wires.iter_mut().filter(|w| [60.0, 68.0, 76.0].contains(&w.a.x) && w.a.y >= 84.0) {
        w.color = "BK".into();
        w.cross_section = "2.5 mm²".into();
    }
}

/// Pressure transmitter in the field on the analog input card, through plug -X10.
fn plc_page(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "1", "Analog inputs", PageKind::Schematic, "PLC", "CAB1");
    let mut g = Pg { prj, page };
    dc_rails(&mut g);
    g.sym("plug_pair", 60.0, 48.0, 0, "=SUP+FLD-X10", "1", "").w(60.0, 24.0, 60.0, 44.0).w(60.0, 52.0, 60.0, 76.0);
    g.sym("transmitter", 60.0, 80.0, 0, "=SUP+FLD-BP1", "", "Pressure line\n0 - 10 bar");
    g.path(&[(68.0, 76.0), (68.0, 68.0), (100.0, 68.0), (100.0, 88.0)]);
    g.sym("plug_pair", 100.0, 92.0, 0, "=SUP+FLD-X10", "2", "").w(100.0, 96.0, 100.0, 120.0);
    g.sym("plc_ai", 100.0, 124.0, 0, "-A3", "IW64", "Line pressure");
    g.text(160.0, 60.0, "PLC", 3.5);
    g.text(160.0, 72.0, "Cards =PLC+CAB1-A1 (DI), -A2 (DO) and -A3 (AI): their channels are placed on the pump", 2.5);
    g.text(160.0, 79.0, "pages; the PLC assignment (page 6.2) lists them with addresses and function texts.", 2.5);
}
