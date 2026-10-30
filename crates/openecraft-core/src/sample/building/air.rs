//! Compressed air: the compressor with its external air receiver and refrigeration
//! dryer, a distribution line with four outlets, each with its own treatment (oiled
//! air for tools, dry air through a membrane dryer, clean air), switched and watched
//! from the pump cabinet.

use super::*;
use crate::model::fluid::{HoseConfig, LineType, Medium};

/// The outlets: their treatment from the ball valve down to the coupling (symbol,
/// tag, function text), the use of the air.
pub(super) const OUTLETS: [(&[(&str, &str)], &str); 4] = [
    (&[("fl_shutoff", "-QM1"), ("fl_regulator", "-AZ1"), ("fl_lubricator", "-HQ3"), ("fl_coupling", "-XM1")], "Tools (oiled air)"),
    (&[("fl_shutoff", "-QM2"), ("fl_dryer", "-HQ2"), ("fl_regulator", "-AZ2"), ("fl_coupling", "-XM2")], "Paint shop (dry air)"),
    (&[("fl_shutoff", "-QM3"), ("fl_regulator", "-AZ3"), ("fl_coupling", "-XM3")], "Blow-off (clean air)"),
    (&[("fl_shutoff", "-QM4"), ("fl_regulator", "-AZ4"), ("fl_lubricator", "-HQ4"), ("fl_coupling", "-XM4")], "Workshop (oiled air)"),
];

/// The compressor motor in the pump cabinet: motor protection, contactor, terminals.
pub(super) fn power(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "1", "Power compressor", PageKind::Schematic, "AIR", "CC2");
    let mut g = Pg { prj, page };
    super::power::rails_from(&mut g, "CC2");
    g.sym("mps3", 140.0, 140.0, 0, "-Q1", "", "Compressor\n7 - 10 A");
    g.w(140.0, 80.0, 140.0, 136.0).w(148.0, 88.0, 148.0, 136.0).w(156.0, 96.0, 156.0, 136.0);
    for i in 0..3 {
        let x = 140.0 + 8.0 * i as f32;
        g.w(x, 144.0, x, 164.0);
    }
    g.sym("main3", 140.0, 168.0, 0, "-K1", "", "Compressor");
    for (i, core) in ["BN", "BK", "GY"].into_iter().enumerate() {
        let x = 140.0 + 8.0 * i as f32;
        g.w(x, 172.0, x, 188.0).sym("term", x, 192.0, 0, "-X1", &(i + 1).to_string(), "");
        g.core(x, 196.0, x, 216.0, "=AIR-W1", core);
    }
    g.sym("term_pe", 164.0, 192.0, 0, "-X1", "PE", "").w(164.0, 112.0, 164.0, 188.0);
    g.core(164.0, 196.0, 164.0, 216.0, "=AIR-W1", "GNYE");
    g.sym("motor3", 140.0, 220.0, 0, "+TR-M1", "", "Compressor motor\n4 kW");
}

/// Ready and running to the EtherCAT inputs, the contactor from an output, the pressure
/// of the air receiver to the PLC.
pub(super) fn control(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "2", "Control compressor", PageKind::Schematic, "AIR", "CC2");
    let mut g = Pg { prj, page };
    g.dc_rails();
    for (x, tag, ch, text) in [(40.0, "-Q1", "DI5", "Compressor ready"), (80.0, "-K1", "DI6", "Compressor running")] {
        g.sym("no", x, 60.0, 0, tag, "", "").pins(&["13", "14"]).w(x, 24.0, x, 56.0).w(x, 64.0, x, 80.0);
        g.sym("plc_in", x, 84.0, 0, "=CTL-A11", ch, text);
    }
    g.sym("plc_out", 160.0, 120.0, 0, "=CTL-A12", "DO3", "Compressor on").w(160.0, 124.0, 160.0, 196.0);
    g.sym("coil", 160.0, 200.0, 0, "-K1", "", "Compressor").w(160.0, 204.0, 160.0, 236.0);
    g.sym("transmitter", 240.0, 80.0, 0, "+TR-BP1", "", "Air receiver\n0 - 16 bar").w(240.0, 24.0, 240.0, 76.0);
    g.path(&[(248.0, 76.0), (248.0, 68.0), (280.0, 68.0), (280.0, 120.0)]);
    g.sym("plc_ai", 280.0, 124.0, 0, "=CTL+CC1-A4", "IW68", "Air pressure");
}

/// P&ID: intake filter, compressor, receiver, refrigeration dryer, the distribution
/// line and its outlets.
pub(super) fn pid(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "3", "Compressed air P&ID", PageKind::Schematic, "AIR", "TR");
    let mut g = Pg { prj, page };
    g.sym("fl_silencer", 40.0, 224.0, 0, "-RN1", "", "Intake filter").pins(&["1"]).w(40.0, 208.0, 40.0, 220.0);
    g.sym("fl_compressor", 40.0, 200.0, 0, "-GQ1", "", "Compressor 450 l/min");
    g.w(40.0, 192.0, 40.0, 176.0);
    g.sym("fl_receiver", 40.0, 168.0, 0, "-CM1", "", "Air receiver 270 l");
    g.sym("pid_instrument", 64.0, 160.0, 0, "-BP1", "PT 1", "");
    g.w(40.0, 160.0, 40.0, 140.0);
    g.sym("fl_dryer", 40.0, 136.0, 2, "-HQ1", "", "Refrigeration dryer");
    // The distribution line along the wall, the outlets hanging from it.
    g.w(40.0, 132.0, 40.0, 60.0).w(40.0, 60.0, 340.0, 60.0);
    for (k, (devices, text)) in OUTLETS.iter().enumerate() {
        let x = 100.0 + 80.0 * k as f32;
        g.w(x, 60.0, x, 76.0);
        for (i, (key, tag)) in devices.iter().enumerate() {
            let y = 80.0 + 20.0 * i as f32;
            let last = i + 1 == devices.len();
            let ftext = if last { *text } else { "" };
            // Pressure regulators drawn for the flow downwards.
            let rot = if *key == "fl_regulator" { 2 } else { 0 };
            g.sym(key, x, y, rot, tag, "", ftext);
            if !last {
                g.w(x, y + 4.0, x, y + 16.0);
            }
        }
    }
    for w in &mut g.prj.pages[page].wires {
        w.medium = Some(Medium::CompressedAir);
        w.cross_section = "DN20".into();
        w.hose = Some(HoseConfig { line: LineType::Pipe, ..Default::default() });
    }
    g.text(120.0, 40.0, "Compressed air 10 bar: outlets with lubricator (oiled air), membrane dryer (dry air) and filter regulator", 2.5);
}
