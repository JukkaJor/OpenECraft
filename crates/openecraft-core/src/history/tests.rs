use super::*;

#[test]
fn steps_are_counted_for_the_undo_list() {
    let mut prj = Project::default();
    let mut h = History::new(&prj);
    assert_eq!((h.undo_steps(), h.redo_steps()), (0, 0));
    for name in ["A", "B", "C"] {
        prj.info.name = name.into();
        h.track(&prj, true);
    }
    assert_eq!(h.undo_steps(), 3);
    h.undo(&mut prj);
    h.undo(&mut prj);
    assert_eq!((h.undo_steps(), h.redo_steps()), (1, 2));
    assert_eq!(prj.info.name, "A");
}

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

#[test]
fn undo_and_redo_bring_back_the_id_of_a_state() {
    let mut prj = Project::default();
    let mut h = History::new(&prj);
    let start = h.state();
    prj.info.name = "A".into();
    assert!(h.track(&prj, true));
    assert!(!h.track(&prj, true), "nothing new to commit");
    let a = h.state();
    assert_ne!(a, start);
    h.undo(&mut prj);
    assert_eq!(h.state(), start);
    h.redo(&mut prj);
    assert_eq!(h.state(), a);
    // A new change after undo is a new state, not the one undone.
    h.undo(&mut prj);
    prj.info.name = "B".into();
    h.track(&prj, true);
    assert!(h.state() != a && h.state() != start);
    h.rebase(&mut prj, |p| p.clone());
    let rebased = h.state();
    h.undo(&mut prj);
    assert_ne!(h.state(), start, "a rebased state is not the state before");
    h.redo(&mut prj);
    assert_eq!(h.state(), rebased);
}
