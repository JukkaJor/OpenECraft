use super::*;

#[test]
fn undo_redo_roundtrip() {
    let mut prj = Project::default();
    let mut h = History::new(&prj);
    prj.info.name = "A".into();
    h.track(&prj, false);
    assert!(!h.can_undo());
    h.track(&prj, true);
    prj.info.name = "B".into();
    h.track(&prj, true);
    h.undo(&mut prj);
    assert_eq!(prj.info.name, "A");
    h.redo(&mut prj);
    assert_eq!(prj.info.name, "B");
}
