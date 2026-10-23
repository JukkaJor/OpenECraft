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

#[test]
fn rebase_changes_all_steps() {
    let mut prj = Project::default();
    let mut h = History::new(&prj);
    prj.info.name = "A".into();
    h.track(&prj, true);
    prj.info.name = "B".into();
    h.track(&prj, true);
    let rev = h.rev;
    h.rebase(&mut prj, |p| {
        let mut p = p.clone();
        p.info.company = "remote".into();
        p
    });
    assert!(h.rev > rev);
    assert_eq!(prj.info.company, "remote");
    h.undo(&mut prj);
    assert_eq!((prj.info.name.as_str(), prj.info.company.as_str()), ("A", "remote"));
    h.undo(&mut prj);
    assert_eq!(prj.info.company, "remote");
    h.redo(&mut prj);
    h.redo(&mut prj);
    assert_eq!((prj.info.name.as_str(), prj.info.company.as_str()), ("B", "remote"));
}
