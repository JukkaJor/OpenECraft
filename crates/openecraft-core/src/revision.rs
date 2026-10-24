//! Revision management: a revision stores a snapshot of all pages; changes made
//! since the last released revision are listed and marked on the pages. Released
//! pages are locked against changes.
//!
//! Release workflow: with approvers defined ([`Workflow`]) a new revision starts as
//! a draft, is submitted for review, approved or rejected by the approvers
//! (`review::Approval`, recorded with user and date) and released once approved.
//! Releasing stores a baseline of the whole project.

use openecraft_i18n::{tr, tr_fmt};
use crate::geom::{p, P};
use crate::model::{is_zero, natural_cmp, Page, Project};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Pages of a revision. Shared, so that undo snapshots of the project stay cheap.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Snapshot(pub Arc<Vec<Page>>);

impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0 == other.0
    }
}

/// Stored state of a revision; approved / rejected follow from the approvals
/// ([`status`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Draft,
    InReview,
    #[default]
    Released,
}

fn is_released(s: &State) -> bool {
    *s == State::Released
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Revision {
    /// Revision index, e.g. "A", "B" or "01".
    pub name: String,
    pub date: String,
    pub author: String,
    pub description: String,
    pub pages: Snapshot,
    #[serde(skip_serializing_if = "is_released")]
    pub state: State,
    /// Review round: counts the submissions; approvals of earlier rounds are void.
    #[serde(skip_serializing_if = "is_zero")]
    pub round: u32,
}

/// Who approves revisions before they are released.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Workflow {
    /// User names of the approvers; empty = revisions are released directly.
    pub approvers: Vec<String>,
    /// Approvals needed; 0 = all approvers.
    pub required: usize,
}

impl Workflow {
    pub fn is_default(&self) -> bool {
        *self == Workflow::default()
    }

    pub fn active(&self) -> bool {
        self.approvers.iter().any(|a| !a.trim().is_empty())
    }

    pub fn needed(&self) -> usize {
        let n = self.approvers.iter().filter(|a| !a.trim().is_empty()).count();
        if self.required == 0 {
            n
        } else {
            self.required.min(n)
        }
    }

    pub fn is_approver(&self, user: &str) -> bool {
        let user = user.trim();
        !user.is_empty() && self.approvers.iter().any(|a| a.trim().eq_ignore_ascii_case(user))
    }
}

/// Status of a revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Draft,
    InReview,
    Approved,
    Rejected,
    Released,
}

impl Status {
    pub fn title(self) -> &'static str {
        match self {
            Status::Draft => tr("draft"),
            Status::InReview => tr("in review"),
            Status::Approved => tr("approved"),
            Status::Rejected => tr("rejected"),
            Status::Released => tr("released"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
}

impl ChangeKind {
    pub fn title(self) -> &'static str {
        match self {
            ChangeKind::Added => tr("added"),
            ChangeKind::Removed => tr("removed"),
            ChangeKind::Modified => tr("changed"),
        }
    }
}

/// One changed item of a page.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub kind: ChangeKind,
    /// "Symbol", "Wire", "Text", "Graphic" or "Page".
    pub what: &'static str,
    pub id: u64,
    pub label: String,
    /// Where to put the revision mark.
    pub pos: P,
}

/// Changes of one page (by page id; `page` is the current index if it still exists).
#[derive(Clone, Debug, PartialEq)]
pub struct PageChanges {
    pub page_id: u64,
    pub page: Option<usize>,
    pub name: String,
    pub changes: Vec<Change>,
}

fn diff<T: PartialEq>(
    old: &[T],
    new: &[T],
    id: impl Fn(&T) -> u64,
    what: &'static str,
    describe: impl Fn(&T) -> (String, P),
    out: &mut Vec<Change>,
) {
    for n in new {
        match old.iter().find(|o| id(o) == id(n)) {
            None => {
                let (label, pos) = describe(n);
                out.push(Change { kind: ChangeKind::Added, what, id: id(n), label, pos });
            }
            Some(o) if o != n => {
                let (label, pos) = describe(n);
                out.push(Change { kind: ChangeKind::Modified, what, id: id(n), label, pos });
            }
            Some(_) => {}
        }
    }
    for o in old.iter().filter(|o| !new.iter().any(|n| id(n) == id(o))) {
        let (label, pos) = describe(o);
        out.push(Change { kind: ChangeKind::Removed, what, id: id(o), label, pos });
    }
}

fn mid(a: P, b: P) -> P {
    p((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
}

/// Item level differences between two page lists (matched by page and item ids).
pub fn compare(old: &[Page], new: &[Page]) -> Vec<PageChanges> {
    let mut out = Vec::new();
    for (i, n) in new.iter().enumerate() {
        let mut changes = Vec::new();
        match old.iter().find(|o| o.id == n.id) {
            None => changes.push(Change { kind: ChangeKind::Added, what: "Page", id: n.id, label: n.description.clone(), pos: p(0.0, 0.0) }),
            Some(o) => {
                if (&o.name, &o.description, crate::structure::page_structure(o), o.kind) != (&n.name, &n.description, crate::structure::page_structure(n), n.kind) {
                    changes.push(Change { kind: ChangeKind::Modified, what: "Page", id: n.id, label: tr("page properties").into(), pos: p(0.0, 0.0) });
                }
                diff(&o.symbols, &n.symbols, |s| s.id, "Symbol", |s| (format!("{} {}", s.tag, s.label).trim().to_string(), s.pos), &mut changes);
                diff(&o.wires, &n.wires, |w| w.id, "Wire", |w| (w.number.clone(), mid(w.a, w.b)), &mut changes);
                diff(&o.texts, &n.texts, |t| t.id, "Text", |t| (t.text.lines().next().unwrap_or("").to_string(), t.pos), &mut changes);
                diff(&o.shapes, &n.shapes, |s| s.id, "Graphic", |s| (String::new(), mid(s.a, s.b)), &mut changes);
            }
        }
        if !changes.is_empty() {
            out.push(PageChanges { page_id: n.id, page: Some(i), name: n.name.clone(), changes });
        }
    }
    for o in old.iter().filter(|o| !new.iter().any(|n| n.id == o.id)) {
        let removed = Change { kind: ChangeKind::Removed, what: "Page", id: o.id, label: o.description.clone(), pos: p(0.0, 0.0) };
        out.push(PageChanges { page_id: o.id, page: None, name: o.name.clone(), changes: vec![removed] });
    }
    out
}

/// Changes since the last released revision (empty without one).
pub fn pending(prj: &Project) -> Vec<PageChanges> {
    match last_released(prj) {
        Some(r) => compare(&r.pages.0, &prj.pages),
        None => Vec::new(),
    }
}

pub fn last_released(prj: &Project) -> Option<&Revision> {
    prj.revisions.iter().rev().find(|r| r.state == State::Released)
}

/// The revision being prepared (draft or in review), if any.
pub fn open_revision(prj: &Project) -> Option<usize> {
    prj.revisions.iter().rposition(|r| r.state != State::Released)
}

/// Name of the revision being prepared, or the suggested name of the next one:
/// A → B, Z → AA, 01 → 02.
pub fn next_name(prj: &Project) -> String {
    if let Some(i) = open_revision(prj) {
        return prj.revisions[i].name.clone();
    }
    let Some(last) = prj.revisions.last().map(|r| r.name.trim().to_string()) else {
        return "A".into();
    };
    if let Ok(n) = last.parse::<u32>() {
        return format!("{:0width$}", n + 1, width = last.len());
    }
    let mut chars: Vec<char> = last.chars().collect();
    match chars.last_mut() {
        Some(c) if c.is_ascii_uppercase() && *c != 'Z' => *c = (*c as u8 + 1) as char,
        Some(c) if *c == 'Z' => chars.push('A'),
        _ => chars.push('A'),
    }
    chars.into_iter().collect()
}

/// Creates a revision from the current pages. Without approval workflow it is
/// released at once (the project revision becomes its name, a baseline is stored;
/// with `lock` all pages are locked), otherwise it starts as a draft.
pub fn create(prj: &mut Project, name: &str, date: &str, author: &str, description: &str, lock: bool) {
    let pages = Snapshot(Arc::new(prj.pages.clone()));
    let state = if prj.workflow.active() { State::Draft } else { State::Released };
    prj.revisions.push(Revision { name: name.into(), date: date.into(), author: author.into(), description: description.into(), pages, state, round: 0 });
    if state == State::Released {
        finish_release(prj, prj.revisions.len() - 1, lock);
    }
}

fn finish_release(prj: &mut Project, i: usize, lock: bool) {
    if lock {
        for pg in &mut prj.pages {
            pg.locked = true;
        }
    }
    let r = &mut prj.revisions[i];
    r.state = State::Released;
    r.pages = Snapshot(Arc::new(prj.pages.clone()));
    let (name, date, author, description) = (r.name.clone(), r.date.clone(), r.author.clone(), r.description.clone());
    prj.info.revision = name.clone();
    crate::compare::add_baseline(prj, &tr_fmt!("Revision {}", name), &date, &author, &description);
}

/// Decisions of the current review round of revision `i`, the latest per user.
pub fn decisions(prj: &Project, i: usize) -> Vec<&crate::review::Approval> {
    let Some(r) = prj.revisions.get(i) else { return Vec::new() };
    let mut out: Vec<&crate::review::Approval> = Vec::new();
    for a in prj.review.approvals.iter().filter(|a| !a.deleted && a.revision == r.name && a.round == r.round) {
        match out.iter_mut().find(|o| o.user.eq_ignore_ascii_case(&a.user)) {
            Some(o) => *o = a,
            None => out.push(a),
        }
    }
    out
}

pub fn status(prj: &Project, i: usize) -> Status {
    let Some(r) = prj.revisions.get(i) else { return Status::Draft };
    match r.state {
        State::Released => Status::Released,
        State::Draft => Status::Draft,
        State::InReview => {
            let d = decisions(prj, i);
            if d.iter().any(|a| !a.approved) {
                Status::Rejected
            } else if d.iter().filter(|a| a.approved && prj.workflow.is_approver(&a.user)).count() >= prj.workflow.needed() {
                Status::Approved
            } else {
                Status::InReview
            }
        }
    }
}

/// Submits a draft (or a revision in review again, e.g. after a rejection) for
/// review with the current pages; earlier approvals become void.
pub fn submit(prj: &mut Project, i: usize) -> Result<(), String> {
    match prj.revisions.get(i).map(|r| r.state) {
        Some(State::Draft | State::InReview) => {}
        _ => return Err(tr("Only revisions that are not released can be submitted").into()),
    }
    let pages = Snapshot(Arc::new(prj.pages.clone()));
    let r = &mut prj.revisions[i];
    r.pages = pages;
    r.round += 1;
    r.state = State::InReview;
    Ok(())
}

/// Takes a revision in review back to draft.
pub fn withdraw(prj: &mut Project, i: usize) {
    if let Some(r) = prj.revisions.get_mut(i).filter(|r| r.state == State::InReview) {
        r.state = State::Draft;
    }
}

/// Removes a revision that is not released.
pub fn discard(prj: &mut Project, i: usize) {
    if prj.revisions.get(i).is_some_and(|r| r.state != State::Released) {
        prj.revisions.remove(i);
    }
}

/// Records the decision of `user` on revision `i` in review.
pub fn decide(prj: &mut Project, i: usize, user: &str, date: &str, approved: bool, note: &str) -> Result<(), String> {
    use crate::review::Record;
    let Some(r) = prj.revisions.get(i) else { return Err(tr("Unknown revision").into()) };
    if r.state != State::InReview {
        return Err(tr("The revision is not in review").into());
    }
    if !prj.workflow.is_approver(user) {
        return Err(tr_fmt!("{} is not an approver of this project", user));
    }
    let (name, round) = (r.name.clone(), r.round);
    let existing = prj.review.approvals.iter_mut().find(|a| !a.deleted && a.revision == name && a.round == round && a.user.eq_ignore_ascii_case(user));
    match existing {
        Some(a) => {
            a.approved = approved;
            a.date = date.into();
            a.note = note.into();
            a.bump();
        }
        None => {
            let id = prj.new_id();
            prj.review.approvals.push(crate::review::Approval {
                id,
                rev: 1,
                deleted: false,
                revision: name,
                round,
                user: user.trim().into(),
                date: date.into(),
                approved,
                note: note.into(),
            });
        }
    }
    Ok(())
}

/// Releases an approved revision; the pages must still be the approved ones.
pub fn release(prj: &mut Project, i: usize, lock: bool) -> Result<(), String> {
    match status(prj, i) {
        Status::Approved => {}
        Status::Draft if !prj.workflow.active() => {}
        Status::Released => return Err(tr("The revision is already released").into()),
        _ => return Err(tr("The revision is not approved").into()),
    }
    if prj.revisions[i].state == State::InReview && !compare(&prj.revisions[i].pages.0, &prj.pages).is_empty() {
        return Err(tr("The pages were changed after the submission; submit the revision again").into());
    }
    finish_release(prj, i, lock);
    Ok(())
}

/// Report row: page, kind, item, description and jump target.
pub type OverviewRow = (String, String, String, String, Option<(usize, u64)>);

/// Report rows of the changes since the last revision.
pub fn overview(prj: &Project) -> Vec<OverviewRow> {
    let mut rows = Vec::new();
    let mut pages = pending(prj);
    pages.sort_by(|a, b| a.page.unwrap_or(usize::MAX).cmp(&b.page.unwrap_or(usize::MAX)).then_with(|| natural_cmp(&a.name, &b.name)));
    for pc in pages {
        for c in &pc.changes {
            let target = pc.page.map(|pi| (pi, if c.kind == ChangeKind::Removed || c.what == "Page" { 0 } else { c.id }));
            rows.push((pc.name.clone(), c.kind.title().to_string(), tr(c.what).to_string(), c.label.clone(), target));
        }
    }
    rows
}

#[cfg(test)]
mod tests;
