use super::*;

#[test]
fn filters_split_the_checklist() {
    assert!(Filter::Todo.shows(Status::Open) && Filter::Todo.shows(Status::Changed) && Filter::Todo.shows(Status::Problem));
    assert!(!Filter::Todo.shows(Status::Done));
    assert!(Filter::Mounted.shows(Status::Done) && Filter::Mounted.shows(Status::Changed));
    assert!(!Filter::Mounted.shows(Status::Open) && !Filter::Mounted.shows(Status::Problem));
    assert!([Status::Open, Status::Done, Status::Changed, Status::Problem].into_iter().all(|s| Filter::All.shows(s)));
}

#[test]
fn every_status_has_its_own_icon() {
    let icons: std::collections::BTreeSet<&str> = [Status::Open, Status::Done, Status::Changed, Status::Problem].into_iter().map(|s| icon(s).0).collect();
    assert_eq!(icons.len(), 4);
}
