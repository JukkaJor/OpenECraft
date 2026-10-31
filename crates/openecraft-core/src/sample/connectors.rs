//! Level 12: connectors – a sensor and a valve on M12 / M8, a fan and its thermostat on
//! GX aviation connectors, a buzzer on a D-sub and an operator panel on a ribbon cable
//! between two IDC connectors; the plugs window shows their pin-outs.

use super::*;
use crate::pinout::{self, Side};

pub(super) fn project() -> Project {
    let mut prj = Project { pages: Vec::new(), ..Default::default() };
    prj.info = ProjectInfo {
        name: "Connector handling".into(),
        description: "Circular, D-sub and IDC connectors\nwith ribbon cable".into(),
        company: "OpenECraft".into(),
        creator: "OpenECraft".into(),
        date: "2026-10-31".into(),
        revision: "A".into(),
        ..Default::default()
    };
    prj.apply_preset(crate::potentials::Preset::Iec60204);
    prj.wire_colors = true;
    add_page(&mut prj, "1", "Cover sheet", PageKind::Cover);
    sensors(&mut prj);
    fan(&mut prj);
    panel(&mut prj);
    add_page(&mut prj, "5", "Plug diagram", PageKind::Report(ReportKind::PlugDiagram.into()));
    add_page(&mut prj, "6", "Cable overview", PageKind::Report(ReportKind::CableOverview.into()));
    add_page(&mut prj, "7", "Cable diagram", PageKind::Report(ReportKind::CableDiagram.into()));
    add_page(&mut prj, "8", "Bill of materials", PageKind::Report(ReportKind::BillOfMaterials.into()));

    let db = PartsDb::default();
    assign_parts(
        &mut prj,
        &db,
        &[
            ("-K1", &["GEN.CT-9A-24DC", "GEN.AUX-1NO1NC"][..]),
            ("-K2", &["GEN.CT-9A-24DC", "GEN.AUX-1NO1NC"]),
            ("-B1", &["GEN.PROX-M12-PNP"]),
            ("-Y1", &["GEN.SV-24DC"]),
            ("-B2", &["GEN.TEMP-SW-60C"]),
            ("-M1", &["GEN.MOT-DC-24V-60W"]),
            ("-P3", &["GEN.HORN-24"]),
            ("-S1", &["GEN.SEL-2POS-1NO"]),
            ("-P1", &["GEN.LMP-GN-24"]),
            ("-P2", &["GEN.LMP-RD-24"]),
            // Each plug: the housing on the device (male) and the one on the cable (female).
            ("-X11", &["GEN.CON-M12-4M", "GEN.CON-M12-4"]),
            ("-X12", &["GEN.CON-M8-3M", "GEN.CON-M8-3F"]),
            ("-X13", &["GEN.CON-GX16-4M", "GEN.CON-GX16-4F"]),
            ("-X14", &["GEN.CON-GX12-3M", "GEN.CON-GX12-3F"]),
            ("-X15", &["GEN.CON-DSUB-9M", "GEN.CON-DSUB-9F"]),
            ("-X20", &["GEN.CON-BOX-10M", "GEN.CON-IDC-10F"]),
            ("-X21", &["GEN.CON-BOX-10M", "GEN.CON-IDC-10F"]),
        ],
    );
    for (tag, part, length, desc) in [
        ("-W11", "GEN.CBL-SENS-3X0.34", 5.0, "Proximity switch"),
        ("-W12", "GEN.CBL-SENS-3X0.34", 3.0, "Valve"),
        ("-W13", "GEN.CBL-4G1.5", 4.0, "Fan"),
        ("-W14", "GEN.CBL-SENS-3X0.34", 4.0, "Fan thermostat"),
        ("-W20", "GEN.CBL-FLAT-10", 0.6, "Operator panel"),
    ] {
        add_cable(&mut prj, &db, tag, part, length, desc);
    }
    // Field cables: the core at the device side of each plug pin.
    for (page, x, cable, core) in [
        ("2", 60.0, "-W11", "BN"),
        ("2", 80.0, "-W11", "BK"),
        ("2", 160.0, "-W12", "BN"),
        ("2", 180.0, "-W12", "BU"),
        ("3", 60.0, "-W14", "BN"),
        ("3", 80.0, "-W14", "BU"),
        ("3", 100.0, "-W13", "BN"),
        ("3", 120.0, "-W13", "BK"),
    ] {
        let pi = prj.pages.iter().position(|p| p.name == page).expect("page");
        if let Some(w) = prj.pages[pi].wires.iter_mut().find(|w| w.a == p(x, 104.0) || w.b == p(x, 104.0)) {
            w.cable = cable.into();
            w.core = core.into();
        }
    }
    // The ribbon cable: core n on pin n of both IDC sockets.
    pinout::connect_flat_cable(&mut prj, "-W20", "-X20", Side::Socket).expect("ribbon at -X20");
    pinout::connect_flat_cable(&mut prj, "-W20", "-X21", Side::Socket).expect("ribbon at -X21");
    number_wires(&mut prj);
    prj
}

fn add_cable(prj: &mut Project, db: &PartsDb, tag: &str, number: &str, length: f32, desc: &str) {
    let part = db.get(number).expect("cable part");
    let id = prj.new_id();
    prj.cables.push(Cable {
        id,
        tag: tag.into(),
        cable_type: part.type_number.clone(),
        cores: part.tech_value("Cores").unwrap_or_default().into(),
        cross_section: part.tech_value("Cross-section").unwrap_or_default().into(),
        length,
        part: part.number.clone(),
        description: desc.into(),
        ..Default::default()
    });
    crate::parts::ensure_in_project(prj, db, number);
}

/// +24V at y = 24 and 0V at y = 32 across the page.
fn rails(g: &mut Pg) {
    g.sym("pot_24", 20.0, 24.0, 3, "", "+24V", "").w(24.0, 24.0, 400.0, 24.0);
    g.sym("pot_0v", 20.0, 32.0, 3, "", "0V", "").w(24.0, 32.0, 400.0, 32.0);
}

/// Connection points of the symbol placed last.
fn ends(g: &Pg) -> Vec<(f32, f32)> {
    let s = g.prj.pages[g.page].symbols.last().expect("symbol");
    let def = g.prj.symbol(&s.sym).expect("symbol");
    (0..def.pins.len()).map(|i| def.pin_world(i, s.pos, s.rot, s.mirror)).map(|q| (q.x, q.y)).collect()
}

/// A device between two plug pins at x = `a` and `b` (pin sides up at y = 96, device
/// sides down at y = 104): from pin `a` down into the device at y = 140, from the
/// device back up to pin `b`.
fn loop_through(g: &mut Pg, key: &str, a: f32, b: f32, tag: &str, ftext: &str) {
    g.sym(key, a, 140.0, 0, tag, "", ftext);
    let e = ends(g);
    let (top, bottom) = (e[0], e[e.len() - 1]);
    g.w(a, 104.0, top.0, top.1);
    g.path(&[bottom, (a, 152.0), (b, 152.0), (b, 104.0)]);
}

/// Plug pins of a cabinet-side plug: pin side (male, the device housing) up.
fn cabinet_pins(g: &mut Pg, plug: &str, pins: &[(f32, &str)]) {
    for (x, label) in pins {
        g.sym("plug_pair", *x, 100.0, 2, plug, label, "");
    }
}

/// A coil or contact at y = 60 from a rail down to the pin side of a plug pin at y = 96.
fn from_rail(g: &mut Pg, key: &str, x: f32, rail: f32, tag: &str, pins: &[&str], ftext: &str) {
    g.sym(key, x, 60.0, 0, tag, "", ftext);
    if !pins.is_empty() {
        g.pins(pins);
    }
    let e = ends(g);
    g.w(x, rail, e[0].0, e[0].1).w(e[1].0, e[1].1, x, 96.0);
}

fn sensors(prj: &mut Project) {
    let page = add_page(prj, "2", "Proximity switch on M12, valve on M8", PageKind::Schematic);
    let mut g = Pg { prj, page };
    rails(&mut g);
    // -B1 on M12 -X11: BN (+24V) on pin 1, BK (switching output) on pin 4 to the coil of -K1.
    cabinet_pins(&mut g, "-X11", &[(60.0, "1"), (80.0, "4")]);
    g.w(60.0, 24.0, 60.0, 96.0);
    from_rail(&mut g, "coil", 80.0, 32.0, "-K1", &[], "Part present");
    loop_through(&mut g, "prox", 60.0, 80.0, "-B1", "Part present");
    // -Y1 on M8 -X12 (pins 1, 3, 4): switched by -K1 on pin 1, 0V on pin 3.
    cabinet_pins(&mut g, "-X12", &[(160.0, "1"), (180.0, "3")]);
    from_rail(&mut g, "no", 160.0, 24.0, "-K1", &["13", "14"], "");
    g.w(180.0, 32.0, 180.0, 96.0);
    loop_through(&mut g, "valve", 160.0, 180.0, "-Y1", "Ejector");
    g.text(240.0, 60.0, "Circular connectors", 3.5);
    g.text(240.0, 72.0, "-X11: M12 A-coded, 4 pins; -X12: M8, pins 1, 3 and 4.", 2.5);
    g.text(240.0, 78.0, "The field cables carry their core colours to the pins:", 2.5);
    g.text(240.0, 84.0, "Devices › Plugs shows them on the mating faces.", 2.5);
}

fn fan(prj: &mut Project) {
    let page = add_page(prj, "3", "Fan on GX aviation connectors, buzzer on D-sub", PageKind::Schematic);
    let mut g = Pg { prj, page };
    rails(&mut g);
    // Thermostat -B2 on GX12 -X14 in series with the fan -M1 on GX16 -X13.
    cabinet_pins(&mut g, "-X14", &[(60.0, "1"), (80.0, "2")]);
    cabinet_pins(&mut g, "-X13", &[(100.0, "1"), (120.0, "2")]);
    g.w(60.0, 24.0, 60.0, 96.0);
    g.path(&[(80.0, 96.0), (80.0, 72.0), (100.0, 72.0), (100.0, 96.0)]);
    g.w(120.0, 32.0, 120.0, 96.0);
    loop_through(&mut g, "temp_nc", 60.0, 80.0, "-B2", "Fan thermostat");
    loop_through(&mut g, "motor_dc", 100.0, 120.0, "-M1", "Fan");
    // Buzzer -P3 on pins 2 and 3 of the D-sub -X15, switched by -K1.
    cabinet_pins(&mut g, "-X15", &[(200.0, "2"), (220.0, "3")]);
    from_rail(&mut g, "no", 200.0, 24.0, "-K1", &["53", "54"], "");
    g.w(220.0, 32.0, 220.0, 96.0);
    loop_through(&mut g, "horn", 200.0, 220.0, "-P3", "Part present");
    g.text(260.0, 60.0, "GX aviation connectors and D-sub", 3.5);
    g.text(260.0, 72.0, "-X13: GX16, 4 pins; -X14: GX12, 3 pins; -X15: D-sub 9.", 2.5);
    g.text(260.0, 78.0, "Unused pins are spares in the plug diagram and on the faces.", 2.5);
}

fn panel(prj: &mut Project) {
    let page = add_page(prj, "4", "Operator panel on ribbon cable", PageKind::Schematic);
    let mut g = Pg { prj, page };
    rails(&mut g);
    let xs = [60.0, 80.0, 100.0, 120.0, 140.0];
    // -X20: box header on the controller board, IDC socket on the ribbon cable (down).
    for (i, x) in xs.into_iter().enumerate() {
        g.sym("plug_pair", x, 100.0, 2, "-X20", &(i + 1).to_string(), "");
    }
    g.w(60.0, 24.0, 60.0, 96.0).w(80.0, 32.0, 80.0, 96.0);
    from_rail(&mut g, "coil", 100.0, 32.0, "-K2", &[], "Automatic mode");
    from_rail(&mut g, "no", 120.0, 24.0, "-K2", &["13", "14"], "");
    from_rail(&mut g, "nc", 140.0, 24.0, "-K2", &["61", "62"], "");
    // The ribbon cable -W20 between the IDC sockets of -X20 and -X21.
    for (i, x) in xs.into_iter().enumerate() {
        g.w(x, 104.0, x, 136.0);
        g.sym("plug_pair", x, 140.0, 0, "-X21", &(i + 1).to_string(), "");
    }
    // Operator panel: +24V and 0V come in on pins 1 and 2.
    g.path(&[(60.0, 144.0), (60.0, 224.0), (100.0, 224.0)]);
    g.path(&[(80.0, 144.0), (80.0, 232.0), (140.0, 232.0)]);
    g.sym("sel", 100.0, 196.0, 0, "-S1", "", "Automatic");
    let e = ends(&g);
    g.w(100.0, 144.0, e[0].0, e[0].1).w(e[1].0, e[1].1, 100.0, 224.0);
    for (x, tag, ftext) in [(120.0, "-P1", "Automatic"), (140.0, "-P2", "Manual")] {
        g.sym("lamp", x, 196.0, 0, tag, "", ftext);
        let e = ends(&g);
        g.w(x, 144.0, e[0].0, e[0].1).w(e[1].0, e[1].1, x, 232.0);
    }
    g.text(200.0, 60.0, "Ribbon cable and IDC connectors", 3.5);
    g.text(200.0, 72.0, "-W20: ribbon cable 10 cores, pitch 1.27 mm, core 1 marked red.", 2.5);
    g.text(200.0, 78.0, "-X20 / -X21: IDC sockets 2 x 5 on box headers; core n lands on pin n.", 2.5);
    g.text(200.0, 84.0, "Pins 6 - 10 are spare. The check reports cores on the wrong pin.", 2.5);
    g.text(200.0, 96.0, "Controller board", 2.5);
    g.text(200.0, 160.0, "Operator panel board", 2.5);
}
