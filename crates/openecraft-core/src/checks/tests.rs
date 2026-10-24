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
