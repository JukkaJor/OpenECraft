use super::*;
use openecraft_core::collab::{Opened, Session};
use openecraft_core::model::{Page, PageKind};

fn opened() -> Remote {
    let mut r = Remote::default();
    let opened = Opened { session: Session { user: "anna".into(), session: "s1".into() }, first_id: 100, snapshot: Snapshot::default() };
    r.shared = Some(Shared::open("plant", opened).0);
    r
}

#[test]
fn answers_of_old_sessions_are_dropped() {
    let mut r = opened();
    let ctx = egui::Context::default();
    r.pending = 3;
    r.inbox.lock().unwrap().extend([
        Reply::Status("old".into(), Ok(Status::default())),
        Reply::Status("s1".into(), Ok(Status::default())),
        Reply::List(Ok(vec![ProjectSummary { name: "p".into(), ..Default::default() }])),
    ]);
    let replies = r.poll(&ctx);
    assert_eq!(replies.len(), 2);
    assert!(matches!(replies[0], Reply::Status(ref s, _) if s == "s1"));
    assert_eq!(r.projects.len(), 1);
}

#[test]
fn errors_are_shown() {
    let mut r = opened();
    let ctx = egui::Context::default();
    r.pending = 1;
    r.inbox.lock().unwrap().push(Reply::Locked("s1".into(), vec![Target::Project], Err("project data is locked by ben".into())));
    r.poll(&ctx);
    assert!(r.status.contains("locked by ben"), "{}", r.status);
}

#[test]
fn lock_requests_are_remembered() {
    let mut r = opened();
    r.url = "http://127.0.0.1:9".into();
    r.lock(vec![Target::Page(4)]);
    assert!(r.shared.as_ref().unwrap().requested.contains(&Target::Page(4)));
    assert!(r.busy());
}

#[test]
fn targets_are_described() {
    let mut prj = Project::default();
    prj.pages.push(Page::new(42, "7", "", PageKind::Schematic));
    assert!(Remote::describe(&prj, Target::Page(42)).contains('7'));
    assert!(Remote::describe(&prj, Target::Page(5)).contains('5'));
}
