//! The PV system and the battery storage: PV generators, generator junction box,
//! Inverter RS Smart Solar, Pylontech batteries on Lynx distributors, MultiPlus-II
//! inverter / chargers, the backup distribution and the communication of the Cerbo GX.

use super::*;

/// Two PV strings through the generator junction box to their Inverter RS.
pub(super) fn pv(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "1", "PV generators, generator junction box, Inverter RS", PageKind::Schematic, "PV", "GAK");
    let mut g = Pg { prj, page };
    for (k, x0) in [40.0, 220.0].into_iter().enumerate() {
        let n = k + 1;
        let side = if n == 1 { "East" } else { "West" };
        g.sym("pv", x0, 40.0, 0, &format!("+ROOF-G{n}"), "", &format!("String {n} ({side})\n9 x 430 Wp"));
        // String cable to the fuses of the generator junction box.
        let cable = format!("=PV-W5{n}");
        g.w(x0, 36.0, x0, 28.0).core(x0, 28.0, x0 + 60.0, 28.0, &cable, "RD").w(x0 + 60.0, 28.0, x0 + 60.0, 56.0);
        g.path(&[(x0, 56.0), (x0, 64.0), (x0 + 16.0, 64.0), (x0 + 16.0, 40.0)]).core(x0 + 16.0, 40.0, x0 + 76.0, 40.0, &cable, "BK").w(x0 + 76.0, 40.0, x0 + 76.0, 56.0);
        let fuse = format!("-F{n}");
        g.sym("fuse", x0 + 60.0, 60.0, 0, &fuse, "", "gPV 15 A").pins(&["1", "2"]);
        g.sym("fuse", x0 + 76.0, 60.0, 0, &fuse, "", "").pins(&["3", "4"]);
        g.w(x0 + 60.0, 64.0, x0 + 60.0, 116.0).w(x0 + 76.0, 64.0, x0 + 76.0, 116.0);
        // Surge arrester behind the fuses, its earth to the equipotential bonding.
        let spd = format!("-F{}", n + 2);
        g.path(&[(x0 + 60.0, 80.0), (x0 + 100.0, 80.0), (x0 + 100.0, 92.0)]);
        g.path(&[(x0 + 76.0, 88.0), (x0 + 116.0, 88.0), (x0 + 116.0, 92.0)]);
        g.sym("spd", x0 + 100.0, 96.0, 0, &spd, "", "Surge protection type 2").pins(&["L+", "PE"]);
        g.sym("spd", x0 + 116.0, 96.0, 0, &spd, "", "").pins(&["L-", "PE"]);
        g.path(&[(x0 + 100.0, 100.0), (x0 + 100.0, 108.0), (x0 + 132.0, 108.0)]).w(x0 + 116.0, 100.0, x0 + 116.0, 108.0);
        g.sym("pot_pe", x0 + 132.0, 104.0, 0, "", "", "");
        let base = 2 * k;
        g.sym("term", x0 + 60.0, 120.0, 0, "-X1", &(base + 1).to_string(), "");
        g.sym("term", x0 + 76.0, 120.0, 0, "-X1", &(base + 2).to_string(), "");
        // PV input of the Inverter RS.
        let cable = format!("=PV-W5{}", n + 2);
        g.core(x0 + 60.0, 124.0, x0 + 60.0, 156.0, &cable, "RD");
        g.core(x0 + 76.0, 124.0, x0 + 76.0, 140.0, &cable, "BK").path(&[(x0 + 76.0, 140.0), (x0 + 68.0, 140.0), (x0 + 68.0, 156.0)]);
        g.sym("hybrid_inverter", x0 + 60.0, 160.0, 0, &format!("+TR-T{n}"), "", &format!("Inverter RS {n}\n48/6000"));
        g.w(x0 + 84.0, 156.0, x0 + 84.0, 136.0).link(x0 + 84.0, 136.0, &format!("RS{n}+"));
        g.w(x0 + 92.0, 156.0, x0 + 92.0, 148.0).link(x0 + 92.0, 148.0, &format!("RS{n}-"));
        // AC output to the generator distribution.
        let cable = format!("=PV-W5{}", n + 4);
        g.core(x0 + 60.0, 176.0, x0 + 60.0, 200.0, &cable, "BN").link(x0 + 60.0, 200.0, &format!("RS{n}-L"));
        g.path(&[(x0 + 68.0, 176.0), (x0 + 68.0, 184.0), (x0 + 84.0, 184.0)]).core(x0 + 84.0, 184.0, x0 + 84.0, 200.0, &cable, "BU").link(x0 + 84.0, 200.0, &format!("RS{n}-N"));
        g.path(&[(x0 + 76.0, 176.0), (x0 + 76.0, 180.0), (x0 + 108.0, 180.0)]).core(x0 + 108.0, 180.0, x0 + 108.0, 200.0, &cable, "GNYE").link(x0 + 108.0, 200.0, &format!("RS{n}-PE"));
    }
    g.text(40.0, 224.0, "PV strings on the roof +ROOF, generator junction box +GAK and Inverter RS on the wall of +TR", 2.5);
}

/// Batteries on Lynx distributor -F1, the DC bus with SmartShunt -P1, the inverters
/// on Lynx distributors -F2 / -F3, the supply of the Cerbo GX.
pub(super) fn dc(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "1", "Battery storage DC", PageKind::Schematic, "ESS", "TR");
    let mut g = Pg { prj, page };
    g.sym("pot", 24.0, 36.0, 0, "", "BAT+", "").w(24.0, 40.0, 400.0, 40.0);
    for (i, x) in [40.0, 72.0, 104.0, 136.0].into_iter().enumerate() {
        let n = i + 1;
        g.w(x, 40.0, x, 76.0);
        g.sym("fuse", x, 80.0, 0, "-F1", "", if i == 0 { "Lynx batteries" } else { "" }).pins(&["BUS+", &format!("F{n}")]);
        g.core(x, 84.0, x, 116.0, &format!("=ESS-W7{n}"), "RD");
        g.sym("battery", x, 120.0, 0, &format!("+RACK-G{n}"), "", &format!("US5000 {n}"));
        // The minus in the same cable to a stud of the negative busbar of -F1.
        g.core(x, 124.0, x, 136.0, &format!("=ESS-W7{n}"), "BK");
        g.sym("term", x, 140.0, 0, "-F1", &format!("N{n}"), "");
        g.w(x, 144.0, x, 160.0);
    }
    g.w(40.0, 160.0, 200.0, 160.0).w(200.0, 160.0, 200.0, 172.0);
    g.sym("ammeter", 200.0, 176.0, 0, "-P1", "", "SmartShunt").pins(&["BAT-", "LOAD-"]);
    g.w(200.0, 180.0, 200.0, 200.0).w(200.0, 200.0, 380.0, 200.0);
    g.sym("pot", 380.0, 196.0, 0, "", "BAT-", "");
    // Cerbo GX on the battery bus.
    g.w(176.0, 40.0, 176.0, 76.0).sym("fuse", 176.0, 80.0, 0, "-F4", "", "2 A").w(176.0, 84.0, 176.0, 92.0);
    g.sym("box1", 176.0, 100.0, 0, "-K1", "", "Cerbo GX").pins(&["V+", "V-"]);
    g.path(&[(176.0, 108.0), (212.0, 108.0), (212.0, 200.0)]);
    for (k, (x, lynx, fuse, name, cable)) in [
        (232.0, "-F2", "F1", "MP1", "=ESS-W81"),
        (264.0, "-F2", "F2", "MP2", "=ESS-W82"),
        (296.0, "-F2", "F3", "MP3", "=ESS-W83"),
        (328.0, "-F2", "F4", "RS1", "=ESS-W84"),
        (360.0, "-F3", "F1", "RS2", "=ESS-W85"),
    ]
    .into_iter()
    .enumerate()
    {
        let text = match k {
            0 => "Lynx inverters",
            4 => "Lynx Inverter RS 2",
            _ => "",
        };
        g.w(x, 40.0, x, 76.0).sym("fuse", x, 80.0, 0, lynx, "", text).pins(&["BUS+", fuse]);
        g.core(x, 84.0, x, 112.0, cable, "RD").link(x, 112.0, &format!("{name}+"));
        // The minus in the same cable from the stud of the negative busbar under the fuse.
        g.w(x + 12.0, 200.0, x + 12.0, 196.0).sym("term", x + 12.0, 192.0, 0, lynx, &fuse.replace('F', "N"), "");
        g.core(x + 12.0, 188.0, x + 12.0, 176.0, cable, "BK").link(x + 12.0, 176.0, &format!("{name}-"));
    }
    g.text(40.0, 224.0, "Battery bank 4 x 4.8 kWh on Lynx distributor -F1 (minus on its negative busbar N1 - N4); SmartShunt -P1 in the negative of the batteries", 2.5);
}

/// The three MultiPlus-II on the AC input of the battery storage, one per phase.
pub(super) fn ac_input(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "2", "Inverter / chargers AC input", PageKind::Schematic, "ESS", "ACD");
    let mut g = Pg { prj, page };
    super::power::incoming(&mut g, "-X0", "W12", "disc3", "-Q1", "Main switch");
    g.link(400.0, 112.0, "ACD-PE");
    for (k, (x, rail)) in [(140.0, 80.0), (220.0, 88.0), (300.0, 96.0)].into_iter().enumerate() {
        let n = k + 1;
        g.sym("cb1", x, 128.0, 0, &format!("-F{n}"), "", "C50").w(x, rail, x, 124.0).w(x, 132.0, x, 156.0);
        g.w(x + 8.0, 104.0, x + 8.0, 156.0).w(x + 16.0, 112.0, x + 16.0, 156.0);
        g.sym("inverter_charger", x, 160.0, 0, &format!("+TR-T{n}"), "", &format!("MultiPlus-II {n}\nL{n}"));
        g.w(x, 176.0, x, 200.0).link(x, 200.0, &format!("MP{n}-L"));
        g.path(&[(x + 8.0, 176.0), (x + 8.0, 188.0), (x + 16.0, 188.0), (x + 16.0, 200.0)]).link(x + 16.0, 200.0, &format!("MP{n}-N"));
        g.w(x + 24.0, 176.0, x + 24.0, 212.0).link(x + 24.0, 212.0, &format!("MP{n}+"));
        g.w(x + 32.0, 176.0, x + 32.0, 220.0).link(x + 32.0, 220.0, &format!("MP{n}-"));
    }
}

/// Backup outputs of the MultiPlus-II through the residual current device to the
/// sockets, the rack and the control voltage of the automation cabinet; the AC
/// outputs of the Inverter RS on sockets of their own and the trip lamp of the
/// residual current device.
pub(super) fn backup(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "3", "Backup distribution", PageKind::Schematic, "ESS", "ACD");
    let mut g = Pg { prj, page };
    g.link(60.0, 24.0, "MP1-L").w(60.0, 24.0, 60.0, 56.0);
    g.link(100.0, 24.0, "MP2-L").path(&[(100.0, 24.0), (100.0, 40.0), (68.0, 40.0), (68.0, 56.0)]);
    g.link(140.0, 24.0, "MP3-L").path(&[(140.0, 24.0), (140.0, 48.0), (76.0, 48.0), (76.0, 56.0)]);
    g.w(84.0, 32.0, 220.0, 32.0).w(84.0, 32.0, 84.0, 56.0);
    for (k, x) in [180.0, 200.0, 220.0].into_iter().enumerate() {
        g.link(x, 24.0, &format!("MP{}-N", k + 1)).w(x, 24.0, x, 32.0);
    }
    g.sym("rcd4", 60.0, 60.0, 0, "-F10", "", "Backup outputs");
    for (x, y) in [(60.0, 80.0), (68.0, 88.0), (76.0, 96.0), (84.0, 104.0)] {
        g.path(&[(x, 64.0), (x, y), (400.0, y)]);
    }
    g.link(20.0, 112.0, "ACD-PE").w(20.0, 112.0, 400.0, 112.0);
    g.sym("cb1", 120.0, 128.0, 0, "-F11", "", "Backup socket").w(120.0, 80.0, 120.0, 124.0).w(120.0, 132.0, 120.0, 176.0);
    g.sym("socket", 120.0, 180.0, 0, "-X11", "", "Backup socket");
    g.w(128.0, 104.0, 128.0, 176.0).w(136.0, 112.0, 136.0, 176.0);
    // Rack PDU and the control voltage of the automation cabinet through terminals -X1.
    for (x, rail, f, k, text) in [(170.0, 88.0, "-F12", 1, "19-inch rack"), (230.0, 96.0, "-F13", 2, "24 V supply +CC1")] {
        g.sym("cb1", x, 128.0, 0, f, "", text).w(x, rail, x, 124.0).w(x, 132.0, x, 152.0);
        g.sym("term", x, 156.0, 0, "-X1", &k.to_string(), "");
        g.sym("term_n", x + 8.0, 156.0, 0, "-X1", &format!("N{k}"), "").w(x + 8.0, 104.0, x + 8.0, 152.0);
        g.sym("term_pe", x + 16.0, 156.0, 0, "-X1", &format!("PE{k}"), "").w(x + 16.0, 112.0, x + 16.0, 152.0);
    }
    for (i, core) in ["BN", "BU", "GNYE"].into_iter().enumerate() {
        let x = 170.0 + 8.0 * i as f32;
        g.core(x, 160.0, x, 196.0, "=ESS-W22", core);
        let x = 230.0 + 8.0 * i as f32;
        g.core(x, 160.0, x, 184.0, "=ESS-W23", core).link(x, 184.0, ["W23-L", "W23-N", "W23-PE"][i]);
    }
    g.sym("socket", 170.0, 200.0, 0, "+RACK-X3", "", "PDU 19-inch rack");
    // Outputs of the Inverter RS.
    for (k, x) in [290.0, 340.0].into_iter().enumerate() {
        let n = k + 1;
        for (i, (key, label, name)) in [("term", "1", "L"), ("term_n", "N1", "N"), ("term_pe", "PE1", "PE")].into_iter().enumerate() {
            let xi = x + 8.0 * i as f32;
            let label = label.replace('1', &n.to_string());
            g.link(xi, 136.0, &format!("RS{n}-{name}")).w(xi, 136.0, xi, 148.0);
            g.sym(key, xi, 152.0, 0, "-X2", &label, "");
        }
        g.w(x, 156.0, x, 164.0).sym("cb1", x, 168.0, 0, &format!("-F2{n}"), "", &format!("Inverter RS {n}")).w(x, 172.0, x, 196.0);
        g.w(x + 8.0, 156.0, x + 8.0, 196.0).w(x + 16.0, 156.0, x + 16.0, 196.0);
        g.sym("socket", x, 200.0, 0, &format!("-X2{n}"), "", &format!("Socket Inverter RS {n}"));
    }
    // The signal contact CZ001 beside the residual current device -F10 (its NC closes
    // when -F10 trips) lights a lamp, fed from Inverter RS 2, which stays on.
    g.path(&[(340.0, 180.0), (384.0, 180.0), (384.0, 184.0)]);
    g.sym("nc", 384.0, 188.0, 0, "-F10", "", "").pins(&["21", "22"]).w(384.0, 192.0, 384.0, 200.0);
    g.sym("lamp", 384.0, 204.0, 0, "-P10", "", "RCD tripped");
    g.path(&[(384.0, 208.0), (384.0, 216.0), (364.0, 216.0), (364.0, 190.0), (348.0, 190.0)]);
}

/// VE.Bus, VE.Can, the CAN bus of the batteries and Ethernet: sockets of the devices,
/// joined by patch cables.
pub(super) fn communication(prj: &mut Project, section: u64) {
    let page = sec_page(prj, section, "4", "Communication", PageKind::Schematic, "ESS", "TR");
    let mut g = Pg { prj, page };
    const LINKS: [(&str, &str, &str, &str, &str); 12] = [
        ("VE.Bus", "-K1", "VE.Bus", "-T1", "VE.Bus1"),
        ("VE.Bus", "-T1", "VE.Bus2", "-T2", "VE.Bus1"),
        ("VE.Bus", "-T2", "VE.Bus2", "-T3", "VE.Bus1"),
        ("VE.Can", "-K1", "VE.Can", "=PV-T1", "VE.Can1"),
        ("VE.Can", "=PV-T1", "VE.Can2", "=PV-T2", "VE.Can1"),
        ("BMS-Can", "-K1", "BMS-Can", "+RACK-G1", "CAN"),
        ("Battery link", "+RACK-G1", "Link1", "+RACK-G2", "Link0"),
        ("Battery link", "+RACK-G2", "Link1", "+RACK-G3", "Link0"),
        ("Battery link", "+RACK-G3", "Link1", "+RACK-G4", "Link0"),
        ("Ethernet", "-K1", "LAN", "+RACK-K2", "P1"),
        ("Modbus TCP", "=CTL+CC1-A1", "X1", "+RACK-K2", "P2"),
        ("EtherCAT", "=CTL+CC1-A1", "X2", "=CTL+CC2-A10", "X1"),
    ];
    for (i, (what, a, pa, b, pb)) in LINKS.into_iter().enumerate() {
        let (x, y) = (40.0 + 72.0 * (i % 5) as f32, 64.0 + 64.0 * (i / 5) as f32);
        g.text(x, y - 22.0, what, 2.5);
        g.socket(x, y, a, pa).socket(x + 32.0, y, b, pb);
        g.w(x, y - 4.0, x, y - 12.0).core(x, y - 12.0, x + 32.0, y - 12.0, &format!("=ESS-W{}", 91 + i), "RJ45").w(x + 32.0, y - 12.0, x + 32.0, y - 4.0);
    }
    g.text(40.0, 224.0, "Cerbo GX -K1: VE.Bus to the MultiPlus-II, VE.Can to the Inverter RS, BMS-Can to the batteries, LAN to the network switch", 2.5);
}
