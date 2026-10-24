use super::*;
use crate::parts::FnKind;
use crate::analysis::Analysis;
use crate::testkit::Kit;

#[test]
fn rule_sets_tighten_the_limits() {
    let mut prj = Project::default();
    assert_eq!(limits(&prj), Limits::default());
    prj.electrical.standards = vec!["iec60204-1".into(), "ul508a".into()];
    let l = limits(&prj);
    assert_eq!((l.voltage_drop, l.min_cross_section_power, l.min_cross_section_control), (5.0, 2.08, 0.82));
    assert!(!l.shield_analog_cables);
    prj.electrical.rule_sets.push(RuleSet {
        name: "Customer".into(),
        active: true,
        limits: LimitOverrides { voltage_drop: Some(3.0), load_reserve: Some(20.0), shield_analog_cables: Some(true), ..Default::default() },
        ..Default::default()
    });
    let l = limits(&prj);
    assert_eq!((l.voltage_drop, l.load_reserve, l.usable(), l.shield_analog_cables), (3.0, 20.0, 0.8, true));
    // Inactive rule sets do not count.
    prj.electrical.rule_sets[0].active = false;
    assert_eq!(limits(&prj).voltage_drop, 5.0);
    assert!(library().iter().all(|r| !r.id.is_empty() && !r.name.is_empty()));
}

#[test]
fn conditions() {
    let mut it = Item::default();
    it.set("color", "gnye");
    it.set("cross_section", "1.5 mm²");
    let holds = |field: &str, op: Op, value: &str| Condition::new(field, op, value).holds(&it);
    assert!(holds("color", Op::OneOf, "BK, GNYE"));
    assert!(!holds("color", Op::NoneOf, "BK, GNYE"));
    assert!(holds("color", Op::Equals, "GNYE") && holds("color", Op::StartsWith, "GN") && holds("color", Op::Contains, "ny"));
    assert!(holds("cross_section", Op::AtLeast, "1.5") && !holds("cross_section", Op::AtLeast, "2.5 mm²"));
    assert!(holds("cross_section", Op::AtMost, "2.5"));
    assert!(holds("missing", Op::Empty, "") && !holds("missing", Op::NotEmpty, ""));
    assert!(!holds("missing", Op::AtLeast, "1"));
}

/// Two motors whose PE terminals are connected by a wire of the given colour.
fn pe_wire(color: &str) -> Kit {
    let mut k = Kit::new();
    let pe = k.sym("pot_pe", 100.0, 20.0, "", "PE");
    let m1 = k.sym("motor3", 76.0, 60.0, "-M1", "");
    let m2 = k.sym("motor3", 136.0, 90.0, "-M2", "");
    k.wire((pe, ""), (m1, "PE"));
    let w = k.wire((m1, "PE"), (m2, "PE"));
    k.prj.pages[0].wires.iter_mut().filter(|x| x.id == w || x.id == w - 1).for_each(|x| x.color = color.into());
    k.part("-M1", FnKind::Motor3, "U1 V1 W1 PE", |p| p.certificates = "CE, UL".into());
    k
}

fn verify_kit(k: &Kit) -> Vec<CheckResult> {
    let an = Analysis::checked(&k.prj);
    verify(&k.prj, &an.connections)
}

#[test]
fn verification_against_a_standard() {
    let mut k = pe_wire("GN");
    assert!(verify_kit(&k).is_empty());
    k.prj.electrical.standards.push("iec60204-1".into());
    let r = verify_kit(&k);
    let pe = r.iter().find(|r| r.check.message.contains("green-yellow")).unwrap();
    assert_eq!((pe.set.as_str(), pe.checked, pe.failed.len()), ("IEC 60204-1", 1, 1));
    assert_eq!(pe.failed[0].label, "-M1:PE – -M2:PE");
    // -M2 has no part.
    let parts = r.iter().find(|r| r.check.scope == Scope::Device).unwrap();
    assert_eq!(parts.failed.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(), ["-M2"]);
    let an = Analysis::checked(&k.prj);
    assert!(an.issues.iter().any(|i| i.severity == Severity::Warning && i.message == "-M1:PE – -M2:PE: Protective conductors must be green-yellow (IEC 60204-1, 13.2.2) (IEC 60204-1)"));
    assert!(!an.report(&k.prj, ReportKind::Verification).unwrap().rows.is_empty());
    let k = {
        let mut k = pe_wire("GNYE");
        k.prj.electrical.standards.push("iec60204-1".into());
        k
    };
    assert!(verify_kit(&k).iter().find(|r| r.check.message.contains("green-yellow")).unwrap().failed.is_empty());
}

#[test]
fn customer_rule_sets() {
    let mut k = pe_wire("GNYE");
    k.prj.electrical.rule_sets.push(RuleSet {
        name: "ACME plant".into(),
        active: true,
        checks: vec![
            Check::new(Scope::Part, &[], ("manufacturer", Op::OneOf, "Siemens, Phoenix"), Level::Error, "Only approved manufacturers"),
            Check::new(Scope::Part, &[("certificates", Op::NotEmpty, "")], ("certificates", Op::Contains, "UL"), Level::Warning, "UL"),
            Check::new(Scope::Project, &[], ("prop:Plant", Op::NotEmpty, ""), Level::Info, "Plant code"),
        ],
        ..Default::default()
    });
    let r = verify_kit(&k);
    assert_eq!(r.iter().map(|r| (r.checked, r.failed.len())).collect::<Vec<_>>(), [(1, 1), (1, 0), (1, 1)]);
    k.prj.info.properties.insert("Plant".into(), "P7".into());
    assert!(verify_kit(&k)[2].failed.is_empty());
    let an = Analysis::checked(&k.prj);
    assert!(an.issues.iter().any(|i| i.severity == Severity::Error && i.message.ends_with(": Only approved manufacturers (ACME plant)")));

    // Exchange as JSON: a list or a single rule set.
    let json = export(&k.prj.electrical.rule_sets);
    assert_eq!(import(&json).unwrap(), k.prj.electrical.rule_sets);
    let single = serde_json::to_string(&k.prj.electrical.rule_sets[0]).unwrap();
    assert_eq!(import(&single).unwrap().len(), 1);
    assert!(import("{ nonsense").unwrap_err().starts_with("Not a valid rule set file"));
}

#[test]
fn minimum_cross_sections() {
    let mut k = Kit::new();
    let l1 = k.sym("pot_l1", 100.0, 20.0, "", "L1");
    let e1 = k.sym("heater", 100.0, 40.0, "-E1", "");
    let e2 = k.sym("heater", 140.0, 60.0, "-E2", "");
    k.wire((l1, ""), (e1, "1"));
    let w = k.wire((e1, "1"), (e2, "1"));
    k.prj.pages[0].wires.iter_mut().filter(|x| x.id == w).for_each(|x| x.cross_section = "1.5 mm²".into());
    let issues = |k: &Kit| Analysis::checked(&k.prj).issues.into_iter().map(|i| i.message).filter(|m| m.contains("power circuits")).collect::<Vec<_>>();
    assert!(issues(&k).is_empty());
    k.prj.electrical.standards.push("ul508a".into());
    assert_eq!(issues(&k), ["-E1:1 – -E2:1: cross-section 1.5 mm² below the 2.08 mm² of power circuits"]);
}
