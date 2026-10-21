//! Revision management: a revision stores a snapshot of all pages; changes made
//! since the last revision are listed and marked on the pages. Released pages are
//! locked against changes.

use crate::geom::{p, P};
use crate::model::{natural_cmp, Page, Project};
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Revision {
    /// Revision index, e.g. "A", "B" or "01".
    pub name: String,
    pub date: String,
    pub author: String,
    pub description: String,
    pub pages: Snapshot,
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
            ChangeKind::Added => "added",
            ChangeKind::Removed => "removed",
            ChangeKind::Modified => "changed",
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
                if (&o.name, &o.description, &o.function, &o.location, o.kind) != (&n.name, &n.description, &n.function, &n.location, n.kind) {
                    changes.push(Change { kind: ChangeKind::Modified, what: "Page", id: n.id, label: "page properties".into(), pos: p(0.0, 0.0) });
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

/// Changes since the last revision (empty without revisions).
pub fn pending(prj: &Project) -> Vec<PageChanges> {
    match prj.revisions.last() {
        Some(r) => compare(&r.pages.0, &prj.pages),
        None => Vec::new(),
    }
}

/// Suggested name of the next revision: A → B, Z → AA, 01 → 02.
pub fn next_name(prj: &Project) -> String {
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

/// Creates a revision from the current pages; the project revision becomes its name.
/// With `release` all pages are locked.
pub fn create(prj: &mut Project, name: &str, date: &str, author: &str, description: &str, release: bool) {
    if release {
        for pg in &mut prj.pages {
            pg.locked = true;
        }
    }
    prj.info.revision = name.to_string();
    let pages = Snapshot(Arc::new(prj.pages.clone()));
    prj.revisions.push(Revision { name: name.into(), date: date.into(), author: author.into(), description: description.into(), pages });
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
            rows.push((pc.name.clone(), c.kind.title().to_string(), c.what.to_string(), c.label.clone(), target));
        }
    }
    rows
}

#[cfg(test)]
mod tests;
