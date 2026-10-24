use super::*;
use crate::sample::demo_project;

#[test]
fn demo_project_passes() {
    let v = verify(&demo_project());
    assert!(v.passed(), "{:?}", v.findings);
    assert_eq!(v.count(Severity::Warning), 0, "{:?}", v.findings);
    assert_eq!(v.repairable(), 0);
}

#[test]
fn broken_data_is_found_and_repaired() {
    let mut prj = demo_project();
    let dup = prj.pages[1].symbols[0].id;
    prj.pages[2].texts.push(TextItem { id: dup, text: "dup".into(), ..Default::default() });
    prj.devices.insert("-Q99".into(), DeviceData { description: "gone".into(), ..Default::default() });
    prj.strips.insert("-X99".into(), Default::default());
    crate::options::define(&mut prj, "Opt", "", "").unwrap();
    let wire = prj.pages[1].wires[0].id;
    crate::options::assign(&mut prj, "Opt", &[wire]).unwrap();
    prj.pages[1].wires.retain(|w| w.id != wire);
    let name = prj.pages[1].name.clone();
    prj.pages[2].name = name;

    let v = verify(&prj);
    assert!(!v.passed());
    let has = |s: &str| v.findings.iter().any(|f| f.message.contains(s));
    assert!(has("more than once") && has("-Q99") && has("-X99") && has("Opt"));
    assert_eq!(v.repairable(), 4);

    let done = repair(&mut prj);
    assert_eq!(done.len(), 3, "{done:?}");
    let after = verify(&prj);
    assert_eq!(after.repairable(), 0, "{:?}", after.findings);
    assert!(!prj.devices.contains_key("-Q99"));
    // The page name is left to the user.
    assert_eq!(after.count(Severity::Warning), 1);
}

#[test]
fn missing_parts_and_unknown_symbols_are_errors() {
    let mut prj = demo_project();
    prj.parts.clear();
    prj.pages[1].symbols[0].sym = "no_such_symbol".into();
    let v = verify(&prj);
    assert!(v.findings.iter().any(|f| f.severity == Severity::Error && f.message.contains("no_such_symbol")));
    assert!(v.findings.iter().any(|f| f.severity == Severity::Error && f.message.contains("not in the project parts")));
    assert!(repair(&mut prj).is_empty());
}
