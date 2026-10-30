//! Heating and hot water: a heat pump charging a buffer tank through its circulating
//! pump, the buffer tank fed with cold water from the water supply and giving hot
//! water to the distribution of the house with a circulation return; enabled, set and
//! watched by the PLC through the EtherCAT terminals of the pump cabinet.

use super::*;
use crate::model::fluid::{HoseConfig, LineType, Medium};

/// Supply of the heat pump and the two circulating pumps through terminals -X1.
pub(super) fn power(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "1", "Power heat pump and circulating pumps", PageKind::Schematic, "HEAT", "CC2");
    let mut g = Pg { prj, page };
    super::power::rails_from(&mut g, "CC2");
    g.sym("cb3", 140.0, 128.0, 0, "-Q1", "", "Heat pump");
    g.w(140.0, 80.0, 140.0, 124.0).w(148.0, 88.0, 148.0, 124.0).w(156.0, 96.0, 156.0, 124.0);
    for (i, (key, label, from)) in [("term", "1", 0.0), ("term", "2", 0.0), ("term", "3", 0.0), ("term_n", "N", 104.0), ("term_pe", "PE", 112.0)].into_iter().enumerate() {
        let x = 140.0 + 8.0 * i as f32;
        if from > 0.0 {
            g.w(x, from, x, 156.0);
        } else {
            g.w(x, 132.0, x, 156.0);
        }
        g.sym(key, x, 160.0, 0, "-X1", label, "");
        g.core(x, 164.0, x, 196.0, "=HEAT-W1", ["BN", "BK", "GY", "BU", "GNYE"][i]);
        g.socket(x, 200.0, "+TR-EQ1", ["L1", "L2", "L3", "N", "PE"][i]);
    }
    g.text(140.0, 216.0, "Heat pump =HEAT+TR-EQ1", 2.5);
    // Circulating pumps, switched by the interface relays -K2 / -K3.
    g.sym("cb1", 240.0, 128.0, 0, "-F2", "", "Circulating pumps").w(240.0, 80.0, 240.0, 124.0).w(240.0, 132.0, 240.0, 152.0);
    g.path(&[(240.0, 140.0), (300.0, 140.0), (300.0, 152.0)]);
    for (k, x) in [240.0, 300.0].into_iter().enumerate() {
        let n = k + 1;
        g.sym("no", x, 156.0, 0, &format!("-K{}", n + 1), "", "").pins(&["13", "14"]).w(x, 160.0, x, 172.0);
        let labels = [(4 + k).to_string(), format!("N{n}"), format!("PE{n}")];
        for (i, key) in ["term", "term_n", "term_pe"].into_iter().enumerate() {
            let xi = x + 8.0 * i as f32;
            g.sym(key, xi, 176.0, 0, "-X1", &labels[i], "");
            if i > 0 {
                g.w(xi, [104.0, 112.0][i - 1], xi, 172.0);
            }
            g.core(xi, 180.0, xi, 196.0, &format!("=HEAT-W{}", n + 1), ["BN", "BU", "GNYE"][i]);
            g.socket(xi, 200.0, &format!("+TR-GP{n}"), ["L", "N", "PE"][i]);
        }
    }
    g.text(240.0, 216.0, "Circulating pumps -GP1 (heat pump) and -GP2 (hot water circulation)", 2.5);
}

/// The relays of the circulating pumps and of the heat pump's enable, its fault
/// contact and its 0 - 10 V setpoint.
pub(super) fn control(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "2", "Control heat pump", PageKind::Schematic, "HEAT", "CC2");
    let mut g = Pg { prj, page };
    g.dc_rails();
    for (x, ch, tag, text) in [(40.0, "DO4", "-K2", "Circulating pump heat pump"), (80.0, "DO5", "-K3", "Hot water circulation"), (120.0, "DO6", "-K1", "Heat pump enable")] {
        g.sym("plc_out", x, 120.0, 0, "=CTL-A12", ch, text).w(x, 124.0, x, 196.0);
        g.sym("coil", x, 200.0, 0, tag, "", "").w(x, 204.0, x, 236.0);
    }
    // Enable: the relay contact between SG1 and SG2 of the heat pump.
    g.sym("plug_f", 180.0, 108.0, 2, "+TR-EQ1", "SG1", "").w(180.0, 112.0, 180.0, 116.0);
    g.sym("no", 180.0, 120.0, 0, "-K1", "", "").pins(&["13", "14"]).w(180.0, 124.0, 180.0, 128.0);
    g.socket(180.0, 132.0, "+TR-EQ1", "SG2");
    // Fault contact of the heat pump to the input DI7.
    g.socket(240.0, 64.0, "+TR-EQ1", "ER1").w(240.0, 24.0, 240.0, 60.0);
    g.sym("plug_f", 260.0, 56.0, 2, "+TR-EQ1", "ER2", "").w(260.0, 60.0, 260.0, 80.0);
    g.sym("plc_in", 260.0, 84.0, 0, "=CTL-A11", "DI7", "Heat pump fault");
    // Flow temperature setpoint 0 - 10 V.
    g.sym("plc_ao", 320.0, 120.0, 0, "=CTL-A13", "AO1", "Setpoint flow temperature").w(320.0, 124.0, 320.0, 136.0);
    g.socket(320.0, 140.0, "+TR-EQ1", "AI+");
    g.sym("plug_f", 340.0, 140.0, 2, "+TR-EQ1", "AI-", "").w(340.0, 144.0, 340.0, 236.0);
}

/// Temperatures of the buffer tank, the hot water and outside, the pressure of the hot
/// water, the high level of the storage tank.
pub(super) fn measurements(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "3", "Measurements heating and hot water", PageKind::Schematic, "HEAT", "CC2");
    let mut g = Pg { prj, page };
    g.dc_rails();
    for (x, tag, card, addr, text) in [
        (40.0, "+TR-BT1", "=CTL-A14", "AI1", "Buffer tank top\n0 - 100 °C"),
        (100.0, "+TR-BT2", "=CTL-A14", "AI2", "Buffer tank bottom\n0 - 100 °C"),
        (160.0, "+TR-BT3", "=CTL-A14", "AI3", "Hot water flow\n0 - 100 °C"),
        (220.0, "+TR-BP1", "=CTL-A14", "AI4", "Hot water pressure\n0 - 10 bar"),
        (280.0, "+TR-BT4", "=CTL+CC1-A4", "IW70", "Outside air\n-40 - 60 °C"),
    ] {
        g.sym("transmitter", x, 80.0, 0, tag, "", text).w(x, 24.0, x, 76.0);
        g.path(&[(x + 8.0, 76.0), (x + 8.0, 68.0), (x + 32.0, 68.0), (x + 32.0, 120.0)]);
        g.sym("plc_ai", x + 32.0, 124.0, 0, card, addr, text.split('\n').next().unwrap_or_default());
    }
    g.sym("level_no", 360.0, 60.0, 0, "=WAT+TR-BL3", "", "Storage tank full").w(360.0, 24.0, 360.0, 56.0).w(360.0, 64.0, 360.0, 80.0);
    g.sym("plc_in", 360.0, 84.0, 0, "=CTL-A11", "DI8", "Storage tank high level");
}

/// P&ID: heat pump and buffer tank, cold water from the distribution manifold of the
/// water supply, hot water to the house and its circulation.
pub(super) fn pid(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "4", "Heating and hot water P&ID", PageKind::Schematic, "HEAT", "TR");
    let mut g = Pg { prj, page };
    // Heating circuit: heat pump, buffer tank, circulating pump.
    g.sym("pid_heat_pump", 60.0, 120.0, 0, "-EQ1", "", "Heat pump 12 kW");
    g.sym("pid_instrument", 40.0, 92.0, 0, "-BT4", "TI 5", "");
    g.sym("pid_buffer", 160.0, 120.0, 0, "-CM1", "", "Buffer tank 500 l");
    g.w(68.0, 116.0, 152.0, 116.0);
    g.path(&[(152.0, 124.0), (136.0, 124.0), (136.0, 128.0), (120.0, 128.0)]);
    g.sym("pid_pump", 112.0, 128.0, 0, "-GP1", "", "").mirror();
    g.w(104.0, 124.0, 68.0, 124.0);
    let heating = g.prj.pages[page].wires.len();
    g.sym("pid_instrument", 184.0, 100.0, 0, "-BT1", "TIC 1", "");
    g.sym("pid_instrument", 184.0, 140.0, 0, "-BT2", "TI 2", "");
    // Hot water to the house.
    g.path(&[(160.0, 104.0), (160.0, 88.0), (220.0, 88.0)]);
    g.sym("pid_valve", 228.0, 88.0, 0, "-QM1", "", "");
    g.w(236.0, 88.0, 268.0, 88.0);
    g.sym("pid_connection", 276.0, 88.0, 0, "-UM1", "", "Hot water to the floors").mirror();
    g.sym("pid_instrument", 252.0, 72.0, 0, "-BT3", "TI 3", "");
    g.sym("pid_instrument", 204.0, 72.0, 0, "-BP1", "PI 4", "");
    // Cold water from the distribution manifold of the water supply (4.3).
    g.sym("pid_connection", 40.0, 160.0, 0, "=WAT-UM2", "", "Cold water").pins(&["OUT1"]);
    g.sym("pid_valve", 64.0, 160.0, 0, "-QM2", "", "");
    g.sym("pid_check_valve", 88.0, 160.0, 0, "-RM1", "", "");
    let cold = g.prj.pages[page].wires.len();
    g.w(48.0, 160.0, 56.0, 160.0).w(72.0, 160.0, 80.0, 160.0);
    g.path(&[(96.0, 160.0), (160.0, 160.0), (160.0, 136.0)]);
    // Circulation return into the cold water inlet of the buffer tank.
    g.sym("pid_connection", 236.0, 184.0, 0, "-UM2", "", "Circulation return").mirror();
    g.sym("pid_pump", 212.0, 184.0, 0, "-GP2", "", "Circulation").mirror();
    g.sym("pid_check_valve", 188.0, 180.0, 0, "-RM2", "", "");
    g.w(228.0, 184.0, 220.0, 184.0).w(204.0, 180.0, 196.0, 180.0);
    g.path(&[(180.0, 180.0), (140.0, 180.0), (140.0, 160.0)]);
    for (i, w) in g.prj.pages[page].wires.iter_mut().enumerate() {
        let (medium, size) = match i {
            _ if i < heating => (Medium::HeatingWater, "DN25"),
            _ if (cold..cold + 2).contains(&i) => (Medium::Water, "DN32"),
            _ => (Medium::HotWater, "DN32"),
        };
        w.medium = Some(medium);
        w.cross_section = size.into();
        w.hose = Some(HoseConfig { line: LineType::Pipe, ..Default::default() });
    }
    g.text(20.0, 200.0, "Heating water DN25 between heat pump and buffer tank; hot water DN32 to the floors with circulation; the PLC", 2.5);
    g.text(20.0, 207.0, "enables the heat pump (-K1), sets its flow temperature (0 - 10 V) and switches the circulating pumps (6.1 - 6.3).", 2.5);
}
