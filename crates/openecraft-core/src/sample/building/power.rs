//! Supply, main distribution and the supply and control of the cabinets.

use super::*;

/// Overview of the power flow: house connection, busbar, the three NH feeders with
/// their meters, the inverters with PV generators and batteries.
pub(super) fn single_line(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "3", "Single-line diagram", PageKind::Overview, "", "");
    let mut g = Pg { prj, page };
    g.text(48.0, 30.0, "3/PEN AC 400 V 50 Hz, TN-C-S", 2.5).text(48.0, 36.0, "-W1 NAYY-J 4 x 150 from the utility", 2.5);
    g.sym("sl_fuse", 40.0, 52.0, 0, "=GRID+HR-F1", "", "House connection box");
    g.path(&[(40.0, 60.0), (40.0, 80.0), (240.0, 80.0)]);
    g.text(250.0, 81.0, "Busbar =MD+MD", 2.5);
    for (x, q, p, to) in [(80.0, "=MD+MD-Q1", "=MD+MD-P1", "=CTL+CC1 automation"), (160.0, "=MD+MD-Q2", "=MD+MD-P2", ""), (240.0, "=MD+MD-Q3", "=MD+MD-P3", "=WAT+CC2 pumps")] {
        g.sym("sl_fuse_switch", x, 100.0, 0, q, "", "").w(x, 80.0, x, 92.0).w(x, 108.0, x, 116.0);
        g.sym("sl_meter", x, 124.0, 0, p, "", "");
        if !to.is_empty() {
            g.text(x - 4.0, 140.0, to, 2.5);
        }
    }
    // Inverter / chargers on the feeder to the battery storage, the Inverter RS with
    // their PV generators; all of them on the 48 V battery.
    g.path(&[(160.0, 132.0), (160.0, 140.0)]).w(140.0, 140.0, 180.0, 140.0);
    for (x, tag) in [(140.0, "=ESS+TR-T1"), (160.0, "=ESS+TR-T2"), (180.0, "=ESS+TR-T3"), (280.0, "=PV+TR-T1"), (320.0, "=PV+TR-T2")] {
        g.sym("sl_inverter", x, 160.0, 0, tag, "", "").w(x, 168.0, x, 184.0);
        if x < 200.0 {
            g.w(x, 140.0, x, 152.0);
        }
    }
    for (x, tag) in [(280.0, "=PV+ROOF-G1"), (320.0, "=PV+ROOF-G2")] {
        g.sym("sl_pv", x, 136.0, 0, tag, "", "").w(x, 144.0, x, 152.0);
    }
    g.w(140.0, 184.0, 320.0, 184.0).w(220.0, 184.0, 220.0, 196.0);
    g.sym("sl_battery", 220.0, 204.0, 0, "=ESS+RACK-G1", "", "4 x Pylontech US5000");
    g.text(230.0, 190.0, "48 V DC", 2.5);
}

/// The utility cable from the street cabinet to the house connection box, the main
/// cable to the busbar of the main distribution, feeders Q1 and Q2 with their meters.
pub(super) fn house_connection(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "1", "House connection and busbar", PageKind::Schematic, "GRID", "HR");
    let mut g = Pg { prj, page };
    for (x, key, label) in [(60.0, "pot_l1", "L1"), (68.0, "pot_l2", "L2"), (76.0, "pot_l3", "L3"), (84.0, "pot", "PEN")] {
        g.sym(key, x, 20.0, 0, "", label, "");
    }
    // Terminals in the cable distribution cabinet of the utility.
    for (i, x) in [60.0, 68.0, 76.0].into_iter().enumerate() {
        g.sym("term", x, 32.0, 0, "+KVS-X1", &(i + 1).to_string(), "").w(x, 24.0, x, 28.0);
    }
    g.sym("term_pe", 84.0, 32.0, 0, "+KVS-X1", "PEN", "").w(84.0, 24.0, 84.0, 28.0);
    for (x, core) in [(60.0, "BN"), (68.0, "BK"), (76.0, "GY"), (84.0, "GNYE")] {
        g.core(x, 36.0, x, 68.0, "=GRID-W1", core);
    }
    g.text(96.0, 34.0, "Cable distribution cabinet of the utility", 2.5).text(96.0, 50.0, "-W1 NAYY-J 4 x 150 SE, through the building entry", 2.5);
    g.sym("fuse3", 60.0, 72.0, 0, "-F1", "", "House connection box\nNH2 3 x 250 A gG");
    g.sym("term_pe", 84.0, 72.0, 0, "-X0", "PEN", "");
    for (x, core) in [(60.0, "BN"), (68.0, "BK"), (76.0, "GY"), (84.0, "GNYE")] {
        g.core(x, 76.0, x, 108.0, "=GRID-W2", core);
    }
    g.text(96.0, 92.0, "-W2 NYY-J 4 x 150 to the main distribution", 2.5);
    // Main distribution: feed-in terminals, PEN terminal split into N and PE.
    for (i, x) in [60.0, 68.0, 76.0].into_iter().enumerate() {
        g.sym("term", x, 112.0, 0, "=MD+MD-X0", &format!("L{}", i + 1), "");
    }
    g.sym("term_pe", 84.0, 112.0, 0, "=MD+MD-X1", "PEN", "");
    rails(&mut g, 116.0);
    g.text(96.0, 120.0, "Busbar system 3 x Cu 30 x 10 mm, 60 mm; PEN split into N and PE", 2.5);
    feeder(&mut g, 120.0, "=MD+MD-Q1", "Automation\nNH000 3 x 63 A", "=MD+MD-P1", "W11", 1);
    feeder(&mut g, 220.0, "=MD+MD-Q2", "Battery storage\nNH00 3 x 80 A", "=MD+MD-P2", "W12", 2);
    for (y, name) in RAILS {
        g.link(380.0, y, &format!("MD-{name}"));
    }
}

/// Rails of the busbar (L1 136 … PE 168), from the feed-in terminals at y `from`.
const RAILS: [(f32, &str); 5] = [(136.0, "L1"), (144.0, "L2"), (152.0, "L3"), (160.0, "N"), (168.0, "PE")];

fn rails(g: &mut Pg, from: f32) {
    for (x, (y, _)) in [60.0, 68.0, 76.0].into_iter().zip(RAILS) {
        g.path(&[(x, from), (x, y), (380.0, y)]);
        busbar(g);
    }
    g.path(&[(84.0, from), (84.0, 160.0), (380.0, 160.0)]).path(&[(84.0, 160.0), (84.0, 168.0), (380.0, 168.0)]);
}

/// The wire drawn last is a copper bar of the busbar system.
fn busbar(g: &mut Pg) {
    if let Some(w) = g.prj.pages[g.page].wires.last_mut() {
        w.cross_section = BUSBAR.into();
    }
}

/// Cross-section of the bars of the busbar system (Cu 30 x 10 mm).
const BUSBAR: &str = "300 mm²";

/// A feeder of the busbar: NH fuse switch disconnector, direct energy meter, the cable
/// to the cabinet it feeds (cores L1 L2 L3 N here, PE from terminal -X2:`pe`).
fn feeder(g: &mut Pg, x: f32, q: &str, text: &str, meter: &str, cable: &str, pe: u32) {
    for (i, (y, _)) in RAILS.iter().take(3).enumerate() {
        g.w(x + 8.0 * i as f32, *y, x + 8.0 * i as f32, 184.0);
    }
    g.sym("fuse_swdisc3", x, 188.0, 0, q, "", text);
    g.sym("kwh3", x, 200.0, 0, meter, "", "");
    for i in 0..3 {
        let xi = x + 8.0 * i as f32;
        g.w(xi, 192.0, xi, 196.0);
    }
    g.w(x + 24.0, 160.0, x + 24.0, 196.0);
    let cable = format!("=MD-{cable}");
    for (i, core) in ["BN", "BK", "GY", "BU"].into_iter().enumerate() {
        let xi = x + 8.0 * i as f32;
        let name = format!("{}-{}", &cable[4..], ["L1", "L2", "L3", "N"][i]);
        g.core(xi, 216.0, xi, 236.0, &cable, core).link(xi, 236.0, &name);
    }
    g.sym("term_pe", x + 40.0, 220.0, 0, "=MD+MD-X2", &format!("PE{pe}"), "").w(x + 40.0, 168.0, x + 40.0, 216.0);
    g.core(x + 40.0, 224.0, x + 40.0, 236.0, &cable, "GNYE").link(x + 40.0, 236.0, &format!("{}-PE", &cable[4..]));
}

/// Feeder Q3 with transformer-operated metering, the surge arrester of the main
/// distribution.
pub(super) fn feeder_q3(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "1", "Feeder pumps, metering and surge protection", PageKind::Schematic, "MD", "MD");
    let mut g = Pg { prj, page };
    let rails = [(24.0, "L1"), (32.0, "L2"), (40.0, "L3"), (48.0, "N"), (56.0, "PE")];
    for (i, (y, name)) in rails.into_iter().enumerate() {
        g.link(20.0, y, &format!("MD-{name}")).w(20.0, y, 400.0, y);
        if i < 3 {
            busbar(&mut g);
        }
    }
    // Surge arrester type 1 + 2 between the conductors and PE, with remote signalling.
    for (i, (x, l)) in [(40.0, "L1"), (48.0, "L2"), (56.0, "L3"), (64.0, "N")].into_iter().enumerate() {
        let text = if i == 0 { "Surge protection" } else { "" };
        g.sym("spd", x, 80.0, 0, "-F10", "", text).pins(&[l, "PE"]).w(x, rails[i].0, x, 76.0).w(x, 84.0, x, 92.0);
    }
    g.w(40.0, 92.0, 72.0, 92.0).w(72.0, 56.0, 72.0, 92.0);
    // Q3: the outputs spread to the current transformers; the voltage taps before them.
    g.sym("fuse_swdisc3", 120.0, 72.0, 0, "-Q3", "", "Pumps\nNH2 3 x 80 A");
    g.w(120.0, 24.0, 120.0, 68.0).w(128.0, 32.0, 128.0, 68.0).w(136.0, 40.0, 136.0, 68.0);
    g.w(120.0, 76.0, 120.0, 112.0);
    g.path(&[(128.0, 76.0), (128.0, 80.0), (152.0, 80.0), (152.0, 136.0)]);
    g.path(&[(136.0, 76.0), (136.0, 84.0), (184.0, 84.0), (184.0, 160.0)]);
    // A set of three current transformers.
    for (i, (x, y)) in [(120.0, 120.0), (152.0, 144.0), (184.0, 168.0)].into_iter().enumerate() {
        let n = i + 1;
        let pins = [format!("{n}P1"), format!("{n}P2"), format!("{n}S1"), format!("{n}S2")];
        g.sym("ct", x, y, 0, "-T11", "", if i == 0 { "150/5 A" } else { "" }).pins(&pins.each_ref().map(String::as_str));
    }
    // Voltage taps through the fuses -F11 to the meter.
    g.path(&[(120.0, 96.0), (300.0, 96.0), (300.0, 180.0)]);
    g.path(&[(152.0, 100.0), (308.0, 100.0), (308.0, 180.0)]);
    g.path(&[(184.0, 104.0), (316.0, 104.0), (316.0, 180.0)]);
    g.sym("fuse3", 300.0, 184.0, 0, "-F11", "", "Voltage taps");
    g.sym("kwh3_ct", 252.0, 200.0, 0, "-P3", "", "");
    for x in [300.0, 308.0, 316.0] {
        g.w(x, 188.0, x, 196.0);
    }
    g.w(324.0, 48.0, 324.0, 196.0);
    // Secondaries of the current transformers.
    for (i, (sx, sy)) in [(128.0, 120.0), (160.0, 144.0), (192.0, 168.0)].into_iter().enumerate() {
        let x1 = 252.0 + 16.0 * i as f32;
        g.path(&[(sx, sy - 4.0), (x1, sy - 4.0), (x1, 196.0)]);
        g.path(&[(sx, sy + 4.0), (x1 + 8.0, sy + 4.0), (x1 + 8.0, 196.0)]);
    }
    // To the pump cabinet +CC2.
    for (x, y, core, name) in [(120.0, 128.0, "BN", "L1"), (152.0, 152.0, "BK", "L2"), (184.0, 176.0, "GY", "L3")] {
        g.core(x, y, x, 236.0, "=MD-W13", core).link(x, 236.0, &format!("W13-{name}"));
    }
    g.sym("term_n", 216.0, 220.0, 0, "=MD+MD-X2", "N3", "").w(216.0, 48.0, 216.0, 216.0);
    g.core(216.0, 224.0, 216.0, 236.0, "=MD-W13", "BU").link(216.0, 236.0, "W13-N");
    g.sym("term_pe", 232.0, 220.0, 0, "=MD+MD-X2", "PE3", "").w(232.0, 56.0, 232.0, 216.0);
    g.core(232.0, 224.0, 232.0, 236.0, "=MD-W13", "GNYE").link(232.0, 236.0, "W13-PE");
    g.text(200.0, 64.0, "-W13 NYY-J 5 x 35 to +CC2", 2.5);
}

/// Incoming terminals of a cabinet at y 36 (`l`: 3 phases or 1), from the interruption
/// points of `cable`; feeds the rails L1 80, L2 88, L3 96, N 104, PE 112 through the
/// switch `q` (key `switch`) or directly.
pub(super) fn incoming(g: &mut Pg, strip: &str, cable: &str, switch: &str, q: &str, text: &str) {
    for (i, (x, n)) in [(60.0, "L1"), (68.0, "L2"), (76.0, "L3")].into_iter().enumerate() {
        g.link(x, 20.0, &format!("{cable}-{n}")).w(x, 20.0, x, 32.0);
        g.sym("term", x, 36.0, 0, strip, &(i + 1).to_string(), "").w(x, 40.0, x, 56.0);
    }
    g.link(84.0, 20.0, &format!("{cable}-N")).w(84.0, 20.0, 84.0, 32.0);
    g.sym("term_n", 84.0, 36.0, 0, strip, "N", "").path(&[(84.0, 40.0), (84.0, 104.0), (400.0, 104.0)]);
    g.link(92.0, 20.0, &format!("{cable}-PE")).w(92.0, 20.0, 92.0, 32.0);
    g.sym("term_pe", 92.0, 36.0, 0, strip, "PE", "").path(&[(92.0, 40.0), (92.0, 112.0), (400.0, 112.0)]);
    g.sym(switch, 60.0, 60.0, 0, q, "", text);
    for (x, y) in [(60.0, 80.0), (68.0, 88.0), (76.0, 96.0)] {
        g.path(&[(x, 64.0), (x, y), (400.0, y)]);
    }
}

/// Rails L1 80, L2 88, L3 96, N 104, PE 112 continued from the interruption points
/// `<prefix>-L1` … at the left edge.
pub(super) fn rails_from(g: &mut Pg, prefix: &str) {
    for (y, name) in [(80.0, "L1"), (88.0, "L2"), (96.0, "L3"), (104.0, "N"), (112.0, "PE")] {
        g.link(20.0, y, &format!("{prefix}-{name}")).w(20.0, y, 400.0, y);
    }
}

/// Supply of the automation cabinet: main switch, socket, heater with thermostat, the
/// 24 V power supply fed from the backup output of the battery storage.
pub(super) fn control_supply(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "1", "Supply automation cabinet", PageKind::Schematic, "CTL", "CC1");
    let mut g = Pg { prj, page };
    incoming(&mut g, "-X0", "W11", "disc3", "-Q1", "Main switch");
    g.sym("cb1", 140.0, 128.0, 0, "-F1", "", "Service socket").w(140.0, 80.0, 140.0, 124.0).w(140.0, 132.0, 140.0, 176.0);
    g.sym("socket", 140.0, 180.0, 0, "-X2", "", "Service socket");
    g.w(148.0, 104.0, 148.0, 176.0).w(156.0, 112.0, 156.0, 176.0);
    g.sym("cb1", 200.0, 128.0, 0, "-F2", "", "Heater").w(200.0, 88.0, 200.0, 124.0).w(200.0, 132.0, 200.0, 148.0);
    g.sym("temp_nc", 200.0, 152.0, 0, "-B2", "", "Thermostat 5 °C").w(200.0, 156.0, 200.0, 172.0);
    g.sym("heater", 200.0, 176.0, 0, "-E2", "", "Anti-condensation heater");
    g.path(&[(200.0, 180.0), (200.0, 196.0), (212.0, 196.0), (212.0, 104.0)]);
    // 24 V power supply on the backup output (=ESS+ACD-F13, 5.2.3).
    for (i, (x, name, key)) in [(280.0, "W23-L", "term"), (288.0, "W23-N", "term_n"), (296.0, "W23-PE", "term_pe")].into_iter().enumerate() {
        g.link(x, 128.0, name).w(x, 128.0, x, 132.0);
        g.sym(key, x, 136.0, 0, "-X1", ["1", "N", "PE"][i], "").w(x, 140.0, x, 156.0);
    }
    g.sym("psu", 280.0, 160.0, 0, "-T1", "", "24 V DC 10 A\nbacked up").pins(&["L", "N", "PE", "+", "-"]);
    g.path(&[(280.0, 176.0), (280.0, 192.0)]).link(280.0, 192.0, "+24V");
    g.path(&[(288.0, 176.0), (288.0, 184.0), (304.0, 184.0), (304.0, 192.0)]).link(304.0, 192.0, "0V");
    g.text(240.0, 214.0, "Control voltage from the backup output of the battery storage", 2.5);
}

/// PLC CPU and the EtherCAT coupler of the pump cabinet; inputs, outputs, analog values.
pub(super) fn plc(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "2", "PLC CPU and EtherCAT coupler", PageKind::Schematic, "CTL", "CC1");
    let mut g = Pg { prj, page };
    g.dc_rails();
    g.sym("box1", 60.0, 120.0, 0, "-A1", "", "PLC CPU").pins(&["L+", "M"]).w(60.0, 24.0, 60.0, 112.0).w(60.0, 128.0, 60.0, 236.0);
    g.sym("box2", 160.0, 120.0, 0, "+CC2-A10", "", "EtherCAT coupler").pins(&["Us", "Up", "0Vs", "0Vp"]);
    g.w(160.0, 24.0, 160.0, 112.0).w(168.0, 24.0, 168.0, 112.0).w(160.0, 128.0, 160.0, 236.0).w(168.0, 128.0, 168.0, 236.0);
    g.text(200.0, 100.0, "-A1: Modbus TCP to the Cerbo GX and the network switch (-X1),", 2.5);
    g.text(200.0, 107.0, "EtherCAT to the coupler +CC2-A10 (-X2) with the terminals -A11 / -A12.", 2.5);
    g.text(200.0, 114.0, "Cards -A2 (DI), -A3 (DO), -A4 (AI) in the CPU rack: pages 3.3 to 3.5.", 2.5);

    // Digital inputs.
    let page = sec_page(g.prj, section, "3", "PLC digital inputs", PageKind::Schematic, "CTL", "CC1");
    let mut g = Pg { prj: g.prj, page };
    g.dc_rails();
    for (x, tag, pins, addr, text) in [
        (40.0, "=WAT+HR-MA1", ["S1", "S2"], "I0.0", "Valve open"),
        (80.0, "=WAT+HR-MA1", ["S3", "S4"], "I0.1", "Valve closed"),
    ] {
        g.sym("no", x, 60.0, 0, tag, "", "").pins(&pins).w(x, 24.0, x, 56.0).w(x, 64.0, x, 68.0);
        g.sym("term", x, 72.0, 0, "-X3", if addr == "I0.0" { "3" } else { "4" }, "").w(x, 76.0, x, 80.0);
        g.sym("plc_in", x, 84.0, 0, "-A2", addr, text);
    }
    g.sym("level_no", 120.0, 60.0, 0, "=WAT+TR-BL2", "", "Dry-run protection").w(120.0, 24.0, 120.0, 56.0).w(120.0, 64.0, 120.0, 80.0);
    g.sym("plc_in", 120.0, 84.0, 0, "-A2", "I0.2", "Tank empty");
    // Changeover contacts: 14 closes on a fault, 12 opens.
    for (x, tag, pins, a, b, what) in [
        (160.0, "=MD+MD-F10", ["11", "12", "14"], "I0.3", "I0.4", "SPD main distribution"),
        (240.0, "=PV+GAK-F3", ["11", "12", "14"], "I0.5", "I0.6", "SPD PV string 1"),
        (320.0, "=ESS+TR-K1", ["R1-C", "R1-NC", "R1-NO"], "I0.7", "I1.0", "Cerbo GX alarm"),
    ] {
        g.sym("co", x, 60.0, 2, tag, "", "").pins(&pins).mirror().w(x, 24.0, x, 56.0).w(x, 64.0, x, 80.0);
        g.sym("plc_in", x, 84.0, 0, "-A2", a, &format!("{what}: fault"));
        g.path(&[(x + 4.0, 64.0), (x + 4.0, 70.0), (x + 40.0, 70.0), (x + 40.0, 80.0)]);
        g.sym("plc_in", x + 40.0, 84.0, 0, "-A2", b, &format!("{what}: OK"));
    }
    g.sym("co", 160.0, 160.0, 2, "=PV+GAK-F4", "", "").pins(&["11", "12", "14"]).mirror();
    g.path(&[(150.0, 24.0), (150.0, 156.0), (160.0, 156.0)]);
    g.w(160.0, 164.0, 160.0, 180.0).sym("plc_in", 160.0, 184.0, 0, "-A2", "I1.1", "SPD PV string 2: fault");
    g.path(&[(164.0, 164.0), (164.0, 170.0), (200.0, 170.0), (200.0, 180.0)]).sym("plc_in", 200.0, 184.0, 0, "-A2", "I1.2", "SPD PV string 2: OK");

    // Digital outputs: the valve actuator, fault lamp and horn on the door.
    let page = sec_page(g.prj, section, "4", "PLC digital outputs", PageKind::Schematic, "CTL", "CC1");
    let mut g = Pg { prj: g.prj, page };
    g.dc_rails();
    // The valve actuator through the field terminals -X3.
    g.sym("plc_out", 48.0, 120.0, 0, "-A3", "Q0.0", "Open valve").w(48.0, 124.0, 48.0, 156.0);
    g.sym("term", 48.0, 160.0, 0, "-X3", "1", "").w(48.0, 164.0, 48.0, 196.0);
    g.sym("plc_out", 88.0, 120.0, 0, "-A3", "Q0.1", "Close valve").w(88.0, 124.0, 88.0, 156.0);
    g.sym("term", 88.0, 160.0, 0, "-X3", "2", "").path(&[(88.0, 164.0), (88.0, 180.0), (56.0, 180.0), (56.0, 196.0)]);
    g.sym("actuator", 40.0, 200.0, 0, "=WAT+HR-MA1", "", "Remote controlled valve =WAT+HR-QM2");
    g.path(&[(40.0, 196.0), (40.0, 188.0), (32.0, 188.0), (32.0, 236.0)]);
    for (x, key, tag, addr, text) in [(160.0, "lamp", "-P1", "Q0.2", "Fault"), (200.0, "horn", "-P2", "Q0.3", "Alarm")] {
        g.sym("plc_out", x, 120.0, 0, "-A3", addr, text).w(x, 124.0, x, 196.0);
        g.sym(key, x, 200.0, 0, tag, "", text).w(x, 204.0, x, 236.0);
    }

    // Analog inputs: level of the storage tank, pressure behind the pumps.
    let page = sec_page(g.prj, section, "5", "PLC analog inputs", PageKind::Schematic, "CTL", "CC1");
    let mut g = Pg { prj: g.prj, page };
    g.dc_rails();
    for (x, tag, addr, text) in [(60.0, "=WAT+TR-BL1", "IW64", "Level storage tank\n0 - 2 m"), (160.0, "=WAT+TR-BP1", "IW66", "Supply pressure\n0 - 10 bar")] {
        g.sym("transmitter", x, 80.0, 0, tag, "", text).w(x, 24.0, x, 76.0);
        g.path(&[(x + 8.0, 76.0), (x + 8.0, 68.0), (x + 40.0, 68.0), (x + 40.0, 120.0)]);
        g.sym("plc_ai", x + 40.0, 124.0, 0, "-A4", addr, text);
    }
}

/// Power circuits of the pump cabinet: incoming fuse switch, two pumps direct on line;
/// their control through the EtherCAT terminals.
pub(super) fn pump_power(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "1", "Power pumps", PageKind::Schematic, "WAT", "CC2");
    let mut g = Pg { prj, page };
    incoming(&mut g, "-X0", "W13", "fuse_swdisc3", "-Q1", "Incoming\nNH00 3 x 50 A");
    // The rails go on to the compressor and the heat pump (5.1, 6.1).
    for (y, name) in [(80.0, "L1"), (88.0, "L2"), (96.0, "L3"), (104.0, "N"), (112.0, "PE")] {
        g.link(400.0, y, &format!("CC2-{name}"));
    }
    for (k, x) in [140.0, 240.0].into_iter().enumerate() {
        let n = k + 1;
        g.sym("mps3", x, 140.0, 0, &format!("-Q1{n}"), "", &format!("Pump {n}"));
        g.w(x, 80.0, x, 136.0).w(x + 8.0, 88.0, x + 8.0, 136.0).w(x + 16.0, 96.0, x + 16.0, 136.0);
        for i in 0..3 {
            let xi = x + 8.0 * i as f32;
            g.w(xi, 144.0, xi, 164.0);
        }
        g.sym("main3", x, 168.0, 0, &format!("-K1{n}"), "", &format!("Pump {n}"));
        let strip = format!("-X1{n}");
        for i in 0..3 {
            let xi = x + 8.0 * i as f32;
            g.w(xi, 172.0, xi, 188.0).sym("term", xi, 192.0, 0, &strip, &(i + 1).to_string(), "");
        }
        g.sym("term_pe", x + 24.0, 192.0, 0, &strip, "PE", "").w(x + 24.0, 112.0, x + 24.0, 188.0);
        let cable = format!("=WAT-W4{n}");
        for (i, core) in ["BN", "BK", "GY", "GNYE"].into_iter().enumerate() {
            let xi = x + 8.0 * i as f32;
            g.core(xi, 196.0, xi, 216.0, &cable, core);
        }
        g.sym("motor3", x, 220.0, 0, &format!("+TR-M{n}"), "", &format!("Pump motor {n}\n2.2 kW"));
    }

    let page = sec_page(g.prj, section, "2", "Control pumps", PageKind::Schematic, "WAT", "CC2");
    let mut g = Pg { prj: g.prj, page };
    g.dc_rails();
    for (k, x) in [40.0, 120.0].into_iter().enumerate() {
        let n = k + 1;
        g.sym("no", x, 60.0, 0, &format!("-Q1{n}"), "", "").pins(&["13", "14"]).w(x, 24.0, x, 56.0).w(x, 64.0, x, 80.0);
        g.sym("plc_in", x, 84.0, 0, "=CTL-A11", &format!("DI{}", 2 * k + 1), &format!("Pump {n} ready"));
        g.sym("no", x + 40.0, 60.0, 0, &format!("-K1{n}"), "", "").pins(&["13", "14"]).w(x + 40.0, 24.0, x + 40.0, 56.0).w(x + 40.0, 64.0, x + 40.0, 80.0);
        g.sym("plc_in", x + 40.0, 84.0, 0, "=CTL-A11", &format!("DI{}", 2 * k + 2), &format!("Pump {n} running"));
    }
    for (k, x) in [240.0, 300.0].into_iter().enumerate() {
        let n = k + 1;
        g.sym("plc_out", x, 120.0, 0, "=CTL-A12", &format!("DO{n}"), &format!("Pump {n} on")).w(x, 124.0, x, 196.0);
        g.sym("coil", x, 200.0, 0, &format!("-K1{n}"), "", &format!("Pump {n}")).w(x, 204.0, x, 236.0);
    }
}
