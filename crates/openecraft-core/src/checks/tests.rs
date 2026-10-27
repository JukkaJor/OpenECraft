use super::*;
use crate::sample::demo_project;

#[test]
fn built_in_checks_have_unique_snake_case_names() {
    let checks = builtin();
    let mut names = std::collections::HashSet::new();
    for c in &checks {
        assert!(c.name.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'), "{}", c.name);
        assert!(names.insert(c.name.clone()), "duplicate check {}", c.name);
        assert!(!c.title.is_empty());
    }
    assert!(checks.len() > 20);
}

#[test]
fn build_leaves_the_issues_to_the_checks() {
    let prj = demo_project();
    assert!(Analysis::build(&prj).issues.is_empty());
    let an = Analysis::checked(&prj);
    assert_eq!(an.issues.len(), run_builtin(&prj, &an).len());
    assert!(an.issues.windows(2).all(|w| w[0].severity <= w[1].severity), "sorted by severity");
}

#[test]
fn check_schemes_are_exchanged_without_the_accepted_messages() {
    let mut a = crate::sample::demo_project();
    a.check_settings.disabled.push("cables".into());
    a.check_settings.levels.insert("devices.abc123".into(), crate::model::checks::Level::Info);
    a.check_settings.accepted.push(crate::model::checks::Accepted { id: "x".into(), message: "m".into(), ..Default::default() });
    let json = scheme_json(&a);
    assert!(!json.contains("accepted"));
    let mut b = crate::sample::demo_project();
    b.check_settings.accepted.push(crate::model::checks::Accepted { id: "y".into(), ..Default::default() });
    apply_scheme(&mut b, &json).unwrap();
    assert_eq!(b.check_settings.disabled, vec!["cables".to_string()]);
    assert_eq!(b.check_settings.levels.len(), 1);
    assert_eq!(b.check_settings.accepted.len(), 1, "own acceptances stay");
    assert!(apply_scheme(&mut b, "[1, 2]").is_err());
}

/// Regression: a project file or any other JSON object was taken as an empty scheme
/// and switched every check on again.
#[test]
fn other_json_objects_are_no_check_scheme() {
    let mut prj = crate::sample::demo_project();
    prj.check_settings.disabled.push("cables".into());
    let project_file = serde_json::to_string(&crate::sample::demo_project()).unwrap();
    for json in [project_file.as_str(), r#"{"name": "x"}"#, "5"] {
        let err = apply_scheme(&mut prj, json).unwrap_err();
        assert!(err.contains("Not a check scheme"), "{err}");
    }
    assert_eq!(prj.check_settings.disabled, vec!["cables".to_string()], "unchanged");
    apply_scheme(&mut prj, "{}").unwrap();
    assert!(prj.check_settings.disabled.is_empty(), "an empty scheme switches every check on");
}
