use super::*;
use openecraft_core::sample::demo_project;

#[test]
fn a_new_session_starts_clean_on_the_first_schematic_page() {
    let s = Session::new(demo_project(), Some("demo.oec".into()));
    assert!(!s.dirty());
    assert_eq!(s.prj.pages[s.page].kind, PageKind::Schematic);
    assert_eq!(s.file_name.as_deref(), Some("demo.oec"));
}

#[test]
fn edits_make_the_session_dirty_until_saved_or_undone() {
    let mut s = Session::new(demo_project(), None);
    s.prj.info.name = "Changed".into();
    s.commit(false);
    assert!(!s.dirty(), "no commit while an edit is in progress");
    s.commit(true);
    assert!(s.dirty());
    // Undo is a change of its own (the revision counts, not the content).
    s.undo();
    assert!(s.dirty() && s.history.can_redo());
    s.redo();
    assert_eq!(s.prj.info.name, "Changed");
    s.mark_saved();
    assert!(!s.dirty());
}

#[test]
fn replacing_the_project_is_one_saved_step() {
    let mut s = Session::new(demo_project(), Some("a.oec".into()));
    s.page = 3;
    s.replace(Project::default(), None);
    assert_eq!((s.page, s.file_name.as_deref(), s.dirty()), (0, None, false));
    assert!(s.history.can_undo());
}

#[test]
fn refresh_follows_the_history_and_the_plugins() {
    let plugins = Plugins::new(Vec::<String>::new());
    let mut s = Session::new(demo_project(), None);
    s.refresh(&plugins, false);
    let devices = s.index.devices.len();
    assert!(devices > 0);
    assert!(s.analysis.reports().len() > 40, "the analysis knows the registered reports");
    let issues = s.analysis.issues.len();
    let page = s.page;
    s.prj.pages[page].symbols.clear();
    s.refresh(&plugins, false);
    assert_eq!((s.analysis.issues.len(), s.index.devices.len()), (issues, devices), "nothing is rebuilt before the change is committed");
    s.refresh(&plugins, true);
    assert!(s.index.devices.len() < devices, "the index follows every frame while editing");
    assert_eq!(s.analysis.issues.len(), issues);
    s.commit(true);
    s.refresh(&plugins, false);
    assert!(s.analysis.index.devices.len() < devices, "the analysis follows the committed change");
    let rev = s.history.rev;
    s.invalidate();
    s.refresh(&plugins, false);
    assert_eq!(s.history.rev, rev, "a rebuild is no change");
}
