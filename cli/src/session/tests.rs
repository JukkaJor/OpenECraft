use super::*;
use crate::testing::{session, temp_dir};

#[test]
fn tools_need_an_open_project() {
    let mut s = session();
    assert!(s.project().is_err());
    assert!(s.save(None).is_err());
    assert!(s.undo().is_err());
    assert!(!s.unsaved());
}

#[test]
fn new_project_is_unsaved_until_written() {
    let dir = temp_dir("session-save");
    let mut s = session();
    s.replace(Project::default(), None);
    assert!(s.unsaved());
    assert!(s.save(None).is_err(), "no file yet");
    let file = dir.join("a.oec");
    s.save(Some(file.to_str().unwrap())).unwrap();
    assert!(!s.unsaved());
    assert_eq!(s.path.as_deref(), Some(file.as_path()));

    s.project_mut().unwrap().info.name = "Changed".into();
    assert!(s.unsaved());
    s.save(None).unwrap();

    let mut other = session();
    other.open(file.to_str().unwrap()).unwrap();
    assert_eq!(other.project().unwrap().info.name, "Changed");
    assert!(!other.unsaved());
}

#[test]
fn open_reports_unreadable_and_invalid_files() {
    let dir = temp_dir("session-open");
    let mut s = session();
    assert!(s.open(dir.join("missing.oec").to_str().unwrap()).unwrap_err().starts_with("Cannot read"));
    let bad = dir.join("bad.oec");
    std::fs::write(&bad, "not json").unwrap();
    assert!(s.open(bad.to_str().unwrap()).unwrap_err().starts_with("Not a valid project file"));
}

#[test]
fn every_commit_is_one_undo_step() {
    let mut s = session();
    s.replace(Project::default(), None);
    assert!(!s.can_undo());
    s.project_mut().unwrap().info.name = "A".into();
    s.commit();
    s.project_mut().unwrap().info.name = "B".into();
    s.commit();
    assert!(s.undo().unwrap());
    assert_eq!(s.project().unwrap().info.name, "A");
    assert!(s.undo().unwrap());
    assert!(!s.undo().unwrap(), "nothing left to undo");
    assert!(s.redo().unwrap());
    assert_eq!(s.project().unwrap().info.name, "A");
}
