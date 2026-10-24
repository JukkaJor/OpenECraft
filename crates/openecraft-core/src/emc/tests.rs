use super::*;
use crate::reports::Analysis;
use crate::testkit::Kit;

fn messages(k: &Kit) -> Vec<String> {
    Analysis::build(&k.prj).issues.into_iter().map(|i| i.message).collect()
}

/// L1 L2 L3 → frequency converter -T1 → cable -W1 → motor -M1.
fn drive() -> Kit {
    let mut k = Kit::new();
    let t1 = k.sym("vfd", 100.0, 40.0, "-T1", "");
    let m1 = k.sym("motor3", 100.0, 100.0, "-M1", "");
    for (i, l) in ["L1", "L2", "L3"].iter().enumerate() {
        let p = k.sym(&format!("pot_{}", l.to_lowercase()), 100.0 + 8.0 * i as f32, 20.0, "", l);
        k.wire((p, ""), (t1, l));
        k.core((t1, ["U", "V", "W"][i]), (m1, ["U1", "V1", "W1"][i]), "-W1", ["BN", "BK", "GY"][i]);
    }
    k.cable("-W1", "BN BK GY GNYE", "1.5 mm²", 10.0);
    k.part("-T1", FnKind::FrequencyConverter, "L1 L2 L3 PE U V W", |p| p.current = "2.2 A".into());
    k
}

#[test]
fn motor_cables_of_frequency_converters_are_shielded_at_both_ends() {
    let mut k = drive();
    let c = &classes(&k.prj, &Analysis::build(&k.prj).netlist, &Analysis::build(&k.prj).connections)[0];
    assert_eq!((c.drive.as_deref(), c.power, c.signal, c.shielded), (Some("-T1"), true, false, false));
    assert!(messages(&k).contains(&"Motor cable -W1 of frequency converter -T1 must be shielded".to_string()));
    k.prj.cables[0].shielded = Some(true);
    k.prj.cables[0].shield_ends = ShieldEnds::First;
    assert!(messages(&k).contains(&"Shield of motor cable -W1 must be connected at both ends".to_string()));
    k.prj.cables[0].shield_ends = ShieldEnds::Both;
    assert!(!messages(&k).iter().any(|m| m.contains("motor cable") || m.contains("Motor cable")));
}

#[test]
fn analogue_signals_and_mixed_cables() {
    let mut k = Kit::new();
    let b1 = k.sym("transmitter", 100.0, 40.0, "-B1", "");
    let a1 = k.sym("plc_ai", 100.0, 80.0, "-A1", "IW64");
    k.core((b1, "+"), (a1, ""), "-W2", "1");
    k.cable("-W2", "1 2", "0.5 mm²", 10.0);
    // Power and a 24 V signal in one cable.
    let l1 = k.sym("pot_l1", 200.0, 20.0, "", "L1");
    let p24 = k.sym("pot_24", 300.0, 20.0, "", "+24V");
    let e1 = k.sym("heater", 200.0, 40.0, "-E1", "");
    let e2 = k.sym("heater", 240.0, 80.0, "-E2", "");
    let p1 = k.sym("lamp", 300.0, 40.0, "-P1", "");
    let p2 = k.sym("lamp", 340.0, 80.0, "-P2", "");
    k.wire((l1, ""), (e1, "1"));
    k.wire((p24, ""), (p1, "X1"));
    k.core((e1, "1"), (e2, "1"), "-W3", "1");
    k.core((p1, "X1"), (p2, "X1"), "-W3", "2");
    k.cable("-W3", "1 2 GNYE", "1.5 mm²", 5.0);
    let m = messages(&k);
    assert!(!m.iter().any(|m| m.contains("analogue")));
    assert!(m.contains(&"Cable -W3 carries power and signal conductors together".to_string()), "{m:?}");
    assert!(!m.iter().any(|m| m.contains("surge")));
    // The EMC library asks for shielded analogue cables and surge protection.
    k.prj.electrical.standards.push("emc".into());
    let m = messages(&k);
    assert!(m.contains(&"Cable -W2 carries analogue signals of -A1, -B1 and must be shielded".to_string()), "{m:?}");
    assert!(m.iter().any(|m| m.starts_with("No surge protective device")));
    k.prj.cables[0].cable_type = "LiYCY 2x0.5".into();
    k.sym("spd", 400.0, 40.0, "-F9", "");
    let m = messages(&k);
    assert!(!m.iter().any(|m| m.contains("analogue") || m.starts_with("No surge")), "{m:?}");
}
