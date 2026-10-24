use super::*;
use crate::parts::FnKind;
use crate::testkit::Kit;

#[test]
fn levels_and_their_combination() {
    assert_eq!(Pl::parse("PL e"), Some(Pl::E));
    assert_eq!(Pl::parse("d"), Some(Pl::D));
    assert_eq!(Pl::parse("x"), None);
    assert_eq!(Pl::of_pfh(2.5e-9), Some(Pl::E));
    assert_eq!(Pl::of_pfh(5e-7), Some(Pl::D));
    assert_eq!(Pl::of_pfh(2e-6), Some(Pl::C));
    assert_eq!(Pl::of_pfh(1e-3), None);
    assert_eq!([Pl::B, Pl::C, Pl::D, Pl::E].map(Pl::sil), [Some(1), Some(1), Some(2), Some(3)]);
    // Table 11: more than N subsystems of the lowest PL lower it by one.
    assert_eq!(combine(&[Pl::E, Pl::E, Pl::E]), Some(Pl::E));
    assert_eq!(combine(&[Pl::D, Pl::D, Pl::D, Pl::D]), Some(Pl::C));
    assert_eq!(combine(&[Pl::C, Pl::C, Pl::C]), Some(Pl::B));
    assert_eq!(combine(&[Pl::E, Pl::D]), Some(Pl::D));
    assert_eq!(combine(&[]), None);
    assert_eq!(parse_number("1 300 000"), Some(1.3e6));
    assert_eq!(parse_number("2.5E-9 1/h"), Some(2.5e-9));
    assert_eq!(parse_number("150 a"), Some(150.0));
}

#[test]
fn simplified_procedure() {
    assert_eq!(figure5(Category::B, 0, 2), Some(Pl::B));
    assert_eq!(figure5(Category::One, 0, 3), Some(Pl::C));
    assert_eq!(figure5(Category::Two, 1, 1), Some(Pl::A));
    assert_eq!(figure5(Category::Three, 2, 3), Some(Pl::E));
    assert_eq!(figure5(Category::Three, 1, 3), Some(Pl::D));
    assert_eq!(figure5(Category::Four, 3, 3), Some(Pl::E));
    assert_eq!(figure5(Category::Four, 3, 2), None);
    assert_eq!(band(95.0, [60.0, 90.0, 99.0]), 2);
    assert_eq!(band(2.0, [3.0, 10.0, 30.0]), 0);
}

/// Emergency stop -S1 → safety relay -A1 → contactors -K1, -K2.
fn machine() -> Kit {
    let mut k = Kit::new();
    k.sym("estop", 100.0, 40.0, "-S1", "");
    k.sym("coil", 140.0, 40.0, "-A1", "");
    k.sym("coil", 180.0, 40.0, "-K1", "");
    k.sym("coil", 220.0, 40.0, "-K2", "");
    k.part("-S1", FnKind::PushbuttonNc, "11 12", |p| p.tech_data = "B10d: 100000".into());
    k.part("-A1", FnKind::Coil, "A1 A2", |p| p.tech_data = "PL: e\nPFHd: 2.5E-9".into());
    for t in ["-K1", "-K2"] {
        k.part(t, FnKind::Coil, "A1 A2", |p| p.tech_data = "B10d: 1300000".into());
    }
    let sub = |name: &str, devices: &[&str], operations: f32| Subsystem {
        name: name.into(),
        devices: devices.iter().map(|d| d.to_string()).collect(),
        category: Category::Three,
        dc: 90.0,
        operations,
        mttfd: 0.0,
        ccf: true,
    };
    k.prj.electrical.safety.push(SafetyFunction {
        name: "SF1 Emergency stop".into(),
        description: String::new(),
        required: Pl::D,
        subsystems: vec![sub("Input", &["-S1"], 1000.0), sub("Logic", &["-A1"], 0.0), sub("Output", &["-K1", "-K2"], 50000.0)],
    });
    k
}

#[test]
fn performance_level_of_a_safety_function() {
    let mut k = machine();
    let e = evaluate(&k.prj, &k.prj.electrical.safety[0]);
    // Input: B10d 100000 at 1000 operations per year → MTTFd 1000 a, capped at 100.
    assert_eq!(e.subsystems[0].mttfd, Some(100.0));
    assert_eq!(e.subsystems[0].pl, Some(Pl::E));
    assert_eq!((e.subsystems[1].mttfd, e.subsystems[1].pl), (None, Some(Pl::E)));
    // Output: 2 × 1300000 / (0.1 × 50000) → 130 a.
    assert_eq!(e.subsystems[2].mttfd, Some(100.0));
    assert_eq!(e.achieved, Some(Pl::E));
    assert!(e.is_met() && problems(&k.prj).is_empty());

    // Many operations: the output channel drops to a medium MTTFd.
    k.prj.electrical.safety[0].subsystems[2].operations = 1_000_000.0;
    let e = evaluate(&k.prj, &k.prj.electrical.safety[0]);
    assert!((e.subsystems[2].mttfd.unwrap() - 6.5).abs() < 0.01);
    assert_eq!(e.subsystems[2].pl, Some(Pl::C));
    assert_eq!(e.achieved, Some(Pl::C));
    let p = problems(&k.prj);
    assert_eq!(p.len(), 1);
    assert_eq!((p[0].severity, p[0].message().as_str()), (Severity::Error, "Safety function SF1 Emergency stop reaches PL c, PL d is required"));
}

#[test]
fn missing_data_and_category_requirements() {
    let mut k = machine();
    let f = &mut k.prj.electrical.safety[0];
    f.subsystems[0].operations = 0.0;
    f.subsystems[2].ccf = false;
    f.subsystems[2].devices.push("-K9".into());
    let e = evaluate(&k.prj, &k.prj.electrical.safety[0]);
    assert_eq!(e.achieved, None);
    assert_eq!(e.subsystems[0].problems, ["Input: operations per year needed for the B10d of -S1"]);
    assert_eq!(e.subsystems[2].problems, ["Device -K9 does not exist"]);
    k.prj.electrical.safety[0].subsystems[2].devices.pop();
    let e = evaluate(&k.prj, &k.prj.electrical.safety[0]);
    assert_eq!(e.subsystems[2].problems, ["Output: category 3 needs measures against common cause failure"]);
    let messages: Vec<String> = problems(&k.prj).into_iter().map(|i| i.message()).collect();
    assert!(messages.contains(&"Safety function SF1 Emergency stop: Output: category 3 needs measures against common cause failure".to_string()));
    assert_eq!(pl_text(Some(Pl::D)), "d (SIL 2)");
    assert_eq!(pl_text(None), "not reached");
}
