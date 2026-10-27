//! Level 7: the conveyor (level 4) through its life cycle: revision A released and
//! locked, a change request from the customer with review comments and redlining,
//! the change made on an unlocked page (marked since revision A) and revision B in
//! the approval workflow.

use super::*;
use crate::model::review::{Markup, MarkupKind, Pen, Priority, RequestStatus, Status};

pub(super) fn project() -> Project {
    let mut prj = demo_project();
    prj.info.name = "Conveyor, change B".into();
    prj.info.date = "2026-10-27".into();
    let at = prj.pages.len();
    let id = prj.new_id();
    prj.pages.push(Page::new(id, &(at + 1).to_string(), "Revision overview", PageKind::Report(ReportKind::RevisionOverview.into())));

    // Revision A: released without approval, all pages locked, baseline stored.
    crate::revision::create(&mut prj, "A", "2026-10-01", "J. Miller", "First release for manufacturing", true);

    // The customer asks for a second start button; the review records it.
    let control = prj.pages.iter().position(|p| p.name == "3").expect("control page");
    let control_id = prj.pages[control].id;
    let cr = crate::review::add_request(&mut prj, "Second start button at the operator station", "Customer", "2026-10-12");
    if let Some(r) = prj.review.requests.iter_mut().find(|r| r.id == cr) {
        r.description = "The conveyor must also be started from the operator station next to the outfeed.".into();
        r.origin = "Customer review".into();
        r.priority = Priority::High;
        r.status = RequestStatus::InWork;
        r.pages = vec![control_id];
        r.devices = vec!["-S5".into()];
        r.revision = "B".into();
    }
    let c = crate::review::add_comment(&mut prj, control_id, p(100.0, 60.0), "Customer", "2026-10-12", "Add a start button at the operator station, parallel to -S3.");
    if let Some(c) = prj.review.comment_mut(c) {
        c.status = Status::Resolved;
        c.request = cr;
        c.replies.push(crate::model::review::Reply { author: "J. Miller".into(), date: "2026-10-20".into(), text: "Added as -S5, see revision B.".into() });
    }
    let c = crate::review::add_comment(&mut prj, control_id, p(130.0, 128.0), "M. Weber", "2026-10-21", "Should the lamp -P1 flash while the conveyor starts?");
    if let Some(c) = prj.review.comment_mut(c) {
        c.replies.push(crate::model::review::Reply { author: "J. Miller".into(), date: "2026-10-22".into(), text: "Open: to be decided with the customer.".into() });
    }
    let author = "Customer".to_string();
    crate::review::add_markup(
        &mut prj,
        Markup { page: control_id, kind: MarkupKind::Cloud, pen: Pen::Red, a: p(30.0, 58.0), b: p(100.0, 96.0), author: author.clone(), date: "2026-10-12".into(), ..Default::default() },
    );
    crate::review::add_markup(
        &mut prj,
        Markup { page: control_id, kind: MarkupKind::Text, pen: Pen::Green, a: p(96.0, 100.0), b: p(96.0, 100.0), text: "-S5 added".into(), author: "J. Miller".into(), date: "2026-10-20".into(), done: true, ..Default::default() },
    );

    // The change: page 3 unlocked, -S5 in parallel to the start button -S3.
    prj.pages[control].locked = false;
    let mut g = Pg { prj: &mut prj, page: control };
    g.sym("pb_no", 88.0, 76.0, 0, "-S5", "", "Start 2");
    g.path(&[(64.0, 64.0), (88.0, 64.0), (88.0, 72.0)]).path(&[(88.0, 80.0), (88.0, 88.0), (64.0, 88.0)]);
    let db = PartsDb::default();
    assign_parts(&mut prj, &db, &[("-S5", &["GEN.PB-GN-1NO"][..])]);
    number_wires(&mut prj);

    // Revision B goes through the approval workflow: two approvers, one has approved.
    prj.workflow.approvers = vec!["M. Weber".into(), "S. Fischer".into()];
    crate::revision::create(&mut prj, "B", "2026-10-22", "J. Miller", "Second start button (CR-001)", false);
    let b = prj.revisions.len() - 1;
    let _ = crate::revision::submit(&mut prj, b);
    let _ = crate::revision::decide(&mut prj, b, "M. Weber", "2026-10-23", true, "Checked against CR-001");
    prj
}
