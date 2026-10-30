use super::*;
use openecraft_core::model::Page;
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
    // Undo back to the saved state leaves no changes, redo brings them back.
    s.undo();
    assert!(!s.dirty() && s.history.can_redo());
    s.redo();
    assert!(s.dirty());
    assert_eq!(s.prj.info.name, "Changed");
    s.mark_saved();
    assert!(!s.dirty());
    s.undo();
    assert!(s.dirty(), "the saved file holds the change undone");
    s.redo();
    assert!(!s.dirty());
}

#[test]
fn a_commit_is_reported_so_the_shell_repaints() {
    let mut s = Session::new(demo_project(), None);
    assert!(!s.commit(true), "nothing changed");
    s.prj.info.name = "Changed".into();
    assert!(!s.commit(false));
    assert!(s.commit(true));
    assert!(!s.commit(true));
}

#[test]
fn a_project_restored_with_unsaved_changes_stays_dirty_until_saved() {
    let mut s = Session::new(demo_project(), Some("plant.oec".into()));
    assert!(s.untouched());
    s.mark_unsaved();
    assert!(s.dirty() && s.untouched());
    s.prj.info.name = "Changed".into();
    s.commit(true);
    s.undo();
    assert!(s.dirty(), "no state of the history is the file's");
    assert!(!s.untouched());
    s.mark_saved();
    assert!(!s.dirty());
}

#[test]
fn pages_with_contents_ask_and_released_or_locked_ones_cannot_be_deleted() {
    let mut s = Session::new(demo_project(), None);
    let full = s.prj.pages.iter().position(|p| !p.symbols.is_empty()).unwrap();
    assert!(s.page_items(full) > 0);
    assert!(s.may_delete_page(full, false));
    assert!(!s.may_delete_page(full, true), "locked by another user");
    s.prj.pages[full].locked = true;
    assert!(!s.may_delete_page(full, false), "released");
    let empty = Project { pages: vec![Page::new(1, "1", "", PageKind::Schematic), Page::new(2, "2", "", PageKind::Schematic)], ..Default::default() };
    let s = Session::new(empty, None);
    assert_eq!(s.page_items(1), 0, "an empty page is deleted without asking");
    assert!(s.may_delete_page(1, false));
    let mut one = s.prj.clone();
    one.pages.truncate(1);
    assert!(!Session::new(one, None).may_delete_page(0, false), "the last page stays");
}

#[test]
fn the_guard_asks_only_with_unsaved_changes() {
    let mut g = Guard::default();
    assert_eq!(g.request(false, "open"), Some("open"));
    assert_eq!(g.pending(), None);
    assert_eq!(g.request(true, "open"), None);
    assert_eq!(g.pending(), Some(&"open"));
    g.cancel();
    assert_eq!(g.pending(), None);
    g.request(true, "close");
    assert_eq!(g.discard(), Some("close"));
    assert_eq!(g.pending(), None);
}

#[test]
fn save_in_the_guard_runs_the_action_after_a_successful_save() {
    let mut g = Guard::default();
    g.request(true, "new");
    g.save();
    assert!(g.saving());
    // A save on a server: nothing happens while it runs.
    assert_eq!(g.after_save(true, true), None);
    assert!(g.saving() && g.pending().is_some());
    assert_eq!(g.after_save(false, false), Some("new"));
    assert!(!g.saving() && g.pending().is_none());
    assert_eq!(g.after_save(false, false), None, "runs once");
}

#[test]
fn a_cancelled_or_failed_save_keeps_the_question() {
    let mut g = Guard::default();
    g.request(true, "open");
    g.save();
    assert_eq!(g.after_save(true, false), None);
    assert!(g.not_saved && !g.saving());
    assert_eq!(g.pending(), Some(&"open"), "the dialog stays open");
    // Without Save chosen a clean project runs nothing on its own.
    assert_eq!(g.after_save(false, false), None);
    assert_eq!(g.discard(), Some("open"));
}

#[test]
fn replacing_the_project_starts_a_new_history() {
    let mut s = Session::new(demo_project(), Some("a.oec".into()));
    s.page = 3;
    s.prj.info.name = "Changed, then discarded".into();
    s.commit(true);
    let rev = s.history.rev;
    s.replace(Project::default(), None);
    assert_eq!((s.page, s.file_name.as_deref(), s.dirty()), (0, None, false));
    // Undo would put the discarded project under the new name (and saving would
    // overwrite the new file with it).
    assert!(!s.history.can_undo());
    s.undo();
    assert_eq!(s.prj.info.name, Project::default().info.name);
    assert!(s.history.rev > rev, "the analysis is built again");
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

#[test]
fn released_generated_and_locked_pages_cannot_be_edited() {
    let mut s = Session::new(demo_project(), None);
    let page = s.prj.pages.iter().position(|p| p.kind == PageKind::Schematic).unwrap();
    assert!(s.may_edit_page(page, false));
    assert!(!s.may_edit_page(page, true), "locked by another user or a viewer");
    s.prj.pages[page].locked = true;
    assert!(!s.may_edit_page(page, false), "released");
    s.prj.pages.push(Page::new(999, "99", "", PageKind::Mounting(1)));
    assert!(!s.may_edit_page(s.prj.pages.len() - 1, false), "generated");
    assert!(!s.may_edit_page(s.prj.pages.len(), false), "no such page");
}

/// Refreshes until the analysis built in the background is taken.
fn opened(s: &mut Session, plugins: &Plugins) {
    let start = std::time::Instant::now();
    while s.opening().is_some() {
        assert!(start.elapsed().as_secs() < 120, "the analysis is not ready");
        std::thread::sleep(std::time::Duration::from_millis(5));
        s.refresh(plugins, false);
    }
}

#[test]
fn an_opened_project_is_analysed_in_the_background() {
    let plugins = Plugins::new(Vec::<String>::new());
    let mut s = Session::new(Project::new_default(), None);
    s.refresh(&plugins, false);
    assert!(s.opening().is_none(), "the project of a new session is analysed at once");
    s.replace(demo_project(), Some("demo.oec".into()));
    s.refresh(&plugins, false);
    if openecraft_core::analysis::threads() > 1 {
        assert!(s.opening().is_some());
    }
    opened(&mut s, &plugins);
    let devices = s.analysis.index.devices.len();
    assert!(devices > 0 && devices == Index::build(&s.prj).devices.len());
    assert!(!s.dirty());
    // Edits are analysed at once again.
    s.prj.info.name = "Changed".into();
    s.commit(true);
    s.refresh(&plugins, false);
    assert!(s.opening().is_none());
}

#[test]
fn a_project_replaced_while_opening_gets_its_own_analysis() {
    let plugins = Plugins::new(Vec::<String>::new());
    let mut s = Session::new(Project::new_default(), None);
    s.replace(demo_project(), None);
    s.refresh(&plugins, false);
    s.replace(Project::new_default(), None);
    s.refresh(&plugins, false);
    opened(&mut s, &plugins);
    assert_eq!(s.analysis.index.devices.len(), Index::build(&s.prj).devices.len());
}
