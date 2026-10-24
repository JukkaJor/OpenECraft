use super::*;
use crate::reports::Analysis;
use crate::testkit::Kit;

#[test]
fn powers_and_diversity() {
    assert_eq!(parse_power("0.75 kW"), Some(750.0));
    assert_eq!(parse_power("60 W"), Some(60.0));
    assert_eq!(parse_power("100 VA"), None);
    assert_eq!(parse_apparent("1.6 kVA"), Some(1600.0));
    assert_eq!(output_voltage("230 V AC / 24 V DC"), Some(24.0));
    assert_eq!([1, 2, 4, 6, 12].map(rated_diversity), [1.0, 0.9, 0.8, 0.7, 0.6]);
}

/// L1 L2 L3 → -Q1 → motor -M1 (1.9 A); L1 → -F1 → heater -E1 (1 kW) → N.
fn machine(heater: &str) -> Kit {
    let mut k = Kit::new();
    let pots: Vec<u64> = ["L1", "L2", "L3"].iter().enumerate().map(|(i, l)| k.sym(&format!("pot_{}", l.to_lowercase()), 100.0 + 8.0 * i as f32, 20.0, "", l)).collect();
    let q1 = k.sym("mps3", 100.0, 40.0, "-Q1", "");
    let m1 = k.sym("motor3", 100.0, 70.0, "-M1", "");
    for (i, (a, b)) in [("1", "2"), ("3", "4"), ("5", "6")].iter().enumerate() {
        k.wire((pots[i], ""), (q1, a));
        k.wire((q1, b), (m1, ["U1", "V1", "W1"][i]));
    }
    k.part("-Q1", FnKind::MotorProtection, "1 2 3 4 5 6", |p| p.current = "2.5 - 4 A".into());
    k.part("-M1", FnKind::Motor3, "U1 V1 W1 PE", |p| {
        p.current = "1.9 A".into();
        p.power = "0.75 kW".into();
    });
    let f1 = k.sym("cb1", 200.0, 40.0, "-F1", "");
    let e1 = k.sym("heater", 200.0, 70.0, "-E1", "");
    let n = k.sym("pot_n", 200.0, 100.0, "", "N");
    k.wire((pots[0], ""), (f1, "1"));
    k.wire((f1, "2"), (e1, "1"));
    k.wire((e1, "2"), (n, ""));
    k.part("-F1", FnKind::Breaker1, "1 2", |p| p.current = "16 A".into());
    k.part("-E1", FnKind::Heater, "1 2", |p| p.power = heater.into());
    k
}

#[test]
fn loads_of_the_mains_and_their_feeders() {
    let k = machine("1 kW");
    let c = calculate(&k.prj, &Netlist::build(&k.prj, true));
    assert_eq!(c.loads.len(), 2);
    let m = &c.loads[0];
    assert_eq!((m.device.as_str(), m.feeder.as_str(), m.three_phase), ("-M1", "-Q1", true));
    assert!((m.current - 1.9).abs() < 1e-4);
    assert!((m.apparent - 3f32.sqrt() * 0.4 * 1.9).abs() < 1e-4);
    let e = &c.loads[1];
    let i = 1000.0 / (400.0 / 3f32.sqrt());
    assert_eq!((e.device.as_str(), e.feeder.as_str(), e.three_phase), ("-E1", "-F1", false));
    assert!((e.current - i).abs() < 1e-3 && (e.power - 1.0).abs() < 1e-6);
    assert_eq!(c.feeders.iter().map(|f| f.device.as_str()).collect::<Vec<_>>(), ["-Q1", "-F1"]);
    assert!((c.phases["L1"] - (1.9 + i)).abs() < 1e-3 && (c.phases["L2"] - 1.9).abs() < 1e-4);
    assert!(c.unbalance > 50.0);
    assert_eq!(c.diversity, 0.9);
    assert!((c.demand - 0.9 * c.installed.1).abs() < 1e-5);
}

#[test]
fn overloaded_feeders_supply_and_unbalance() {
    let mut k = machine("5 kW");
    let messages = |k: &Kit| Analysis::checked(&k.prj).issues.into_iter().map(|i| i.message).collect::<Vec<_>>();
    assert!(messages(&k).iter().any(|m| m == "-F1: load 21.7 A above the usable 16.0 A"));
    k.prj.parts.iter_mut().find(|p| p.power == "5 kW").unwrap().power = "1 kW".into();
    assert!(!messages(&k).iter().any(|m| m.contains("above the usable")));
    k.prj.electrical.supply.rated_current = 2.0;
    assert!(messages(&k).iter().any(|m| m.starts_with("Demand ") && m.ends_with("above the usable 2.0 A of the supply")));
    // A load reserve of 80 % leaves 3.2 A of -F1 for the heater.
    k.prj.electrical.supply.rated_current = 0.0;
    k.prj.electrical.rule_sets.push(crate::rules::RuleSet {
        active: true,
        limits: crate::rules::LimitOverrides { load_reserve: Some(80.0), max_unbalance: Some(20.0), ..Default::default() },
        ..Default::default()
    });
    let m = messages(&k);
    assert!(m.iter().any(|m| m == "-F1: load 4.3 A above the usable 3.2 A"), "{m:?}");
    assert!(m.iter().any(|m| m.starts_with("Phase unbalance") && m.ends_with("above 20 %")));
}

/// +24V of -T1 → -S1 (NO) → coil -K1 → 0V; lamp -P1 on +24V.
#[test]
fn load_budget_of_a_power_supply() {
    let mut k = Kit::new();
    let l1 = k.sym("pot_l1", 100.0, 20.0, "", "L1");
    let t1 = k.sym("psu", 100.0, 40.0, "-T1", "");
    k.wire((l1, ""), (t1, "L"));
    let s1 = k.sym("no", 100.0, 80.0, "-S1", "");
    let k1 = k.sym("coil", 100.0, 100.0, "-K1", "");
    let p1 = k.sym("lamp", 140.0, 80.0, "-P1", "");
    k.wire((t1, "L+"), (s1, "13"));
    k.wire((s1, "14"), (k1, "A1"));
    k.wire((t1, "L+"), (p1, "X1"));
    k.wire((k1, "A2"), (t1, "M"));
    k.wire((p1, "X2"), (t1, "M"));
    k.part("-T1", FnKind::PowerSupply, "L N PE L+ M", |p| {
        p.voltage = "230 V AC / 24 V DC".into();
        p.current = "5 A".into();
    });
    k.part("-K1", FnKind::Coil, "A1 A2", |p| p.tech_data = "Coil power: 6 W".into());
    k.part("-P1", FnKind::Signal, "X1 X2", |_| {});
    let nl = Netlist::build(&k.prj, true);
    let c = calculate(&k.prj, &nl);
    let b = &c.budgets[0];
    assert_eq!((b.device.as_str(), b.voltage, b.rated), ("-T1", 24.0, 5.0));
    assert_eq!(b.loads, [("-K1".to_string(), 0.25)]);
    assert_eq!(b.without_data, ["-P1"]);
    // The power supply is a load of the mains: 120 W / 0.9.
    assert!((c.loads[0].power - 0.12 / 0.9).abs() < 1e-4);
    k.prj.parts.iter_mut().find(|p| p.number.contains("P1")).unwrap().tech_data = "Current consumption: 20 mA".into();
    k.prj.parts.iter_mut().find(|p| p.number.contains("T1")).unwrap().current = "0.1 A".into();
    let an = Analysis::checked(&k.prj);
    assert!(an.issues.iter().any(|i| i.message == "-T1: load 0.27 A above the usable 0.10 A of the power supply"));
    assert_eq!(an.report(&k.prj, ReportKind::PowerSupplyBudget).unwrap().rows.len(), 3);
}
