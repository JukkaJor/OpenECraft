use super::*;
use crate::parts::{FnKind, Part};
use crate::reports::Analysis;
use crate::testkit::Kit;

/// L1 → -F1 → cable -W1 (one core) → heater -E1.
fn feeder(protection: (FnKind, &str, &str), cross_section: &str, length: f32) -> Kit {
    let mut k = Kit::new();
    let l1 = k.sym("pot_l1", 100.0, 20.0, "", "L1");
    let f1 = k.sym("cb1", 100.0, 40.0, "-F1", "");
    let e1 = k.sym("heater", 100.0, 80.0, "-E1", "");
    k.wire((l1, ""), (f1, "1"));
    k.core((f1, "2"), (e1, "1"), "-W1", "1");
    k.cable("-W1", "1 2 GNYE", cross_section, length);
    let (kind, current, tech) = protection;
    k.part("-F1", kind, "1 2", |p| {
        p.current = current.into();
        p.tech_data = tech.into();
    });
    k
}

fn faults_of(k: &Kit) -> Vec<Fault> {
    let an = Analysis::checked(&k.prj);
    faults(&k.prj, &an.netlist, &an.connections)
}

fn messages(k: &Kit) -> Vec<String> {
    Analysis::checked(&k.prj).issues.into_iter().map(|i| i.message).collect()
}

#[test]
fn short_circuit_currents_at_the_device_and_the_end_of_the_cable() {
    let k = feeder((FnKind::Breaker1, "16 A", "Characteristic: C\nBreaking capacity: 10 kA"), "2.5 mm²", 0.0);
    let f = &faults_of(&k)[0];
    assert_eq!(f.ik_max, 10000.0);
    // Without cable: the loop impedance of the supply (2 × Zsupply).
    let z = C_MAX * 400.0 / (3f32.sqrt() * 10000.0);
    assert!((f.ik_min_end - C_MIN * 400.0 / 3f32.sqrt() / (2.0 * z)).abs() < 1.0, "{}", f.ik_min_end);
    assert_eq!(f.breaks(), Some(true));
    assert!(f.disconnects() && f.withstands());

    let k = feeder((FnKind::Breaker1, "16 A", "Characteristic: C\nBreaking capacity: 10 kA"), "2.5 mm²", 50.0);
    let f = &faults_of(&k)[0];
    assert_eq!((f.cable.as_str(), f.cross_section), ("-W1", 2.5));
    // 2 × 50 m of 2.5 mm² at 0.0225 Ω·mm²/m: 0.9 Ω.
    assert!(f.ik_min_end < 260.0 && f.ik_min_end > 200.0, "{}", f.ik_min_end);
    assert!(f.ik_max_end > f.ik_min_end && f.ik_max_end < 10000.0);
    assert_eq!(f.limit, Some(0.4));
    assert!((f.trip_current - 160.0).abs() < 1.0);
    assert!(f.disconnects());
}

#[test]
fn breaking_capacity_below_the_short_circuit_current() {
    let mut k = feeder((FnKind::Breaker1, "16 A", "Breaking capacity: 6 kA"), "2.5 mm²", 10.0);
    assert!(messages(&k).iter().any(|m| m == "-F1: breaking capacity 6 kA below the short-circuit current 10.00 kA"));
    k.prj.electrical.supply.ik_max = 6.0;
    assert!(!messages(&k).iter().any(|m| m.contains("breaking capacity")));
    k.prj.parts[0].tech_data.clear();
    assert!(messages(&k).iter().any(|m| m == "1 protective devices of the mains have no breaking capacity data"));
}

#[test]
fn long_cables_do_not_disconnect_in_time() {
    let mut k = feeder((FnKind::Breaker1, "16 A", "Characteristic: C\nBreaking capacity: 10 kA"), "1.5 mm²", 300.0);
    let f = &faults_of(&k)[0];
    assert!(!f.disconnects() && f.ik_min_end < 160.0);
    assert!(messages(&k).iter().any(|m| m.starts_with("-F1: fault current") && m.contains("at -W1 does not disconnect within 0.4 s (needs 160 A)")));
    // A residual current device disconnects.
    let l1 = k.prj.pages[0].symbols[0].id;
    let q1 = k.sym("rcd4", 60.0, 40.0, "-Q1", "");
    k.wire((l1, ""), (q1, "1"));
    let f1 = k.prj.pages[0].symbols[1].id;
    k.prj.pages[0].wires.remove(0);
    k.wire((q1, "2"), (f1, "1"));
    k.part("-Q1", FnKind::ResidualCurrent, "1 3 5 N 2 4 6 N'", |p| p.current = "40 A".into());
    let f = &faults_of(&k)[0];
    assert_eq!(f.residual, Some(30.0));
    assert!(f.disconnects());
    // IT system: no disconnection at the first fault.
    k.prj.electrical.supply.earthing = Earthing::It;
    assert_eq!(faults_of(&k)[0].limit, None);
}

#[test]
fn cables_must_withstand_the_energy_let_through() {
    let k = feeder((FnKind::Fuse, "32 A", "Utilisation category: gG\nBreaking capacity: 100 kA"), "0.75 mm²", 18.0);
    let f = &faults_of(&k)[0];
    let (e, allowed) = f.energy.unwrap();
    assert_eq!(allowed, (115.0f32 * 0.75).powi(2));
    assert!(e > allowed && !f.withstands());
    assert!(messages(&k).iter().any(|m| m.starts_with("-F1: lets through") && m.ends_with(&format!("cable -W1 withstands {allowed:.0} A²s"))));
    let k = feeder((FnKind::Fuse, "32 A", "Utilisation category: gG\nBreaking capacity: 100 kA"), "6 mm²", 18.0);
    assert!(faults_of(&k)[0].withstands());
}

fn protection(current: &str, tech: &str, kind: FnKind) -> Protection {
    Protection::of_part(&Part { current: current.into(), tech_data: tech.into(), ..Default::default() }, kind).unwrap()
}

#[test]
fn selectivity_of_breakers_and_fuses() {
    let c32 = protection("32 A", "Characteristic: C", FnKind::Breaker1);
    let c16 = protection("16 A", "Characteristic: C", FnKind::Breaker1);
    // At most up to the lowest instantaneous trip of the upstream breaker (5 × 32 A).
    let limit = selectivity_limit(&c32, &c16, 10000.0);
    assert!(limit > 2.0 * 16.0 && limit <= 160.0, "{limit}");
    // Equal breakers are not selective.
    assert!(selectivity_limit(&c16, &c16, 10000.0) < 2.0 * 16.0);
    let (f50, f25, f32) = (protection("50 A", "", FnKind::Fuse), protection("25 A", "", FnKind::Fuse), protection("32 A", "", FnKind::Fuse));
    assert_eq!(selectivity_limit(&f50, &f25, 10000.0), 10000.0);
    assert!(selectivity_limit(&f32, &f25, 10000.0) < 10000.0);
    // Below the rated current of the downstream device there is nothing to select.
    assert_eq!(selectivity_limit(&c32, &c16, 10.0), 10.0);
}

#[test]
fn selectivity_with_the_device_upstream() {
    let mut k = Kit::new();
    let l1 = k.sym("pot_l1", 100.0, 20.0, "", "L1");
    let f0 = k.sym("cb1", 100.0, 40.0, "-F0", "");
    let f1 = k.sym("cb1", 100.0, 60.0, "-F1", "");
    k.wire((l1, ""), (f0, "1"));
    k.wire((f0, "2"), (f1, "1"));
    k.part("-F0", FnKind::Breaker1, "1 2", |p| {
        p.current = "32 A".into();
        p.tech_data = "Characteristic: C\nBreaking capacity: 10 kA".into();
    });
    k.part("-F1", FnKind::Breaker1, "1 2", |p| {
        p.current = "16 A".into();
        p.tech_data = "Characteristic: B\nBreaking capacity: 10 kA".into();
    });
    let an = Analysis::checked(&k.prj);
    let s = selectivity(&k.prj, &an.netlist, &an.connections);
    assert_eq!(s.len(), 1);
    assert_eq!((s[0].upstream.as_str(), s[0].downstream.as_str()), ("-F0", "-F1"));
    assert!(!s[0].is_total());
    let partial = |an: &Analysis| an.issues.iter().find(|i| i.message.starts_with("-F1 is selective to -F0 only up to")).map(|i| i.severity);
    assert_eq!(partial(&an), Some(Severity::Info));
    assert_eq!(an.report(&k.prj, ReportKind::Selectivity).rows.len(), 1);
    // A customer rule set asks for total selectivity.
    k.prj.electrical.rule_sets.push(crate::rules::RuleSet {
        name: "Customer".into(),
        active: true,
        limits: crate::rules::LimitOverrides { total_selectivity: Some(true), ..Default::default() },
        ..Default::default()
    });
    assert_eq!(partial(&Analysis::checked(&k.prj)), Some(Severity::Warning));
}

#[test]
fn tt_systems_need_a_residual_current_device() {
    let mut k = feeder((FnKind::Breaker1, "16 A", "Characteristic: C\nBreaking capacity: 10 kA"), "2.5 mm²", 10.0);
    k.prj.electrical.supply.earthing = Earthing::Tt;
    let f = &faults_of(&k)[0];
    assert!(f.needs_rcd && !f.disconnects());
    assert!((f.limit.unwrap() - 0.2).abs() < 1e-6);
    assert!(messages(&k).contains(&"-F1: a TT system needs a residual current device".to_string()));
}
