//! Everything derived from the project that the front ends, reports, checks and
//! exports share: device index, netlists, connections, panel wiring, 3D routing and
//! field plan, the report tables of the registered reports (generated on demand) and
//! the issues the host's checks found.

use crate::ext::Report;
use crate::index::Index;
pub use crate::issue::{Issue, Severity};
use crate::model::{Project, ReportId};
use crate::netlist::{connections, Connection, Netlist, Scans};
use crate::panel::{self, OpenWire, RoutedWire};
use crate::table::Table;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Everything derived from the project that is expensive enough to cache. The report
/// tables are generated when they are first asked for ([`Analysis::report`]).
#[derive(Default)]
pub struct Analysis {
    /// Device / cross-reference index.
    pub index: Index,
    pub netlist: Netlist,
    /// Netlist with the terminal sides apart: one net per physical connection
    /// (wire numbers, cables).
    pub open: Netlist,
    pub issues: Vec<Issue>,
    /// Virtual wiring of the panel layout.
    pub wiring: Vec<RoutedWire>,
    /// Connections the panel layout could not route.
    pub open_wires: Vec<OpenWire>,
    /// Changes since the last revision (revision marks).
    pub changes: Vec<crate::revision::PageChanges>,
    /// Point-to-point connections.
    pub connections: Vec<Connection>,
    /// Fill levels of the cable ducts the wiring runs through.
    pub duct_fill: Vec<panel::DuctFill>,
    /// 3D routing of wires, cables and tubes.
    pub routing: crate::routing3d::Routing,
    /// Cables planned through the route network of the plant.
    pub field: crate::field::Plan,
    /// Reports the tables come from: the built-in ones until the host sets the
    /// reports of its registry ([`Analysis::set_reports`]).
    reports: Vec<Report>,
    /// Generated report tables.
    tables: Mutex<HashMap<ReportId, Arc<Table>>>,
}

/// What an analysis keeps for the next one of the same project: the scans of the pages
/// that did not change are reused (see [`Analysis::build_with`]).
#[derive(Clone, Default)]
pub struct Cache {
    netlist: Scans,
    open: Scans,
}

impl Cache {
    /// Pages scanned again by the last build (both netlists).
    pub fn scanned(&self) -> usize {
        self.netlist.scanned + self.open.scanned
    }
}

impl Analysis {
    pub fn build(prj: &Project) -> Analysis {
        Analysis::build_with(prj, &mut Cache::default())
    }

    /// Like [`Analysis::build`], taking from `cache` what did not change since the last
    /// build with it. On machines with several cores the two netlists are built at
    /// the same time.
    pub fn build_with(prj: &Project, cache: &mut Cache) -> Analysis {
        let Cache { netlist: bridged, open: apart } = cache;
        let (netlist, open) = join(|| Netlist::build_with(prj, true, bridged), || Netlist::build_with(prj, false, apart));
        let idx = Index::build(prj);
        let conns = connections(prj, &netlist, &open);
        let (mut wiring, open_wires) = panel::virtual_wiring(prj, &conns);
        let routing = crate::routing3d::analyse(prj, &mut wiring, &conns, &open);
        let fill = panel::duct_fill(prj, &wiring);
        let field = crate::field::plan(prj, &conns);
        let changes = crate::revision::pending(prj);
        Analysis {
            index: idx,
            netlist,
            open,
            issues: Vec::new(),
            wiring,
            open_wires,
            changes,
            connections: conns,
            duct_fill: fill,
            routing,
            field,
            reports: Vec::new(),
            tables: Mutex::default(),
        }
    }


    /// The reports the tables come from.
    pub fn reports(&self) -> &[Report] {
        &self.reports
    }

    /// Takes the reports of the host's registry (built-in and of plugins).
    pub fn set_reports(&mut self, reports: impl IntoIterator<Item = Report>) {
        self.reports = reports.into_iter().collect();
        self.clear_reports();
    }

    /// The table of a report: generated on the first call (from the project the
    /// analysis was built from) and kept. `None` for a report that is not registered.
    pub fn report(&self, prj: &Project, id: impl Into<ReportId>) -> Option<Arc<Table>> {
        let id = id.into();
        if let Some(t) = self.tables.lock().ok().and_then(|m| m.get(&id).cloned()) {
            return Some(t);
        }
        let r = self.reports.iter().find(|r| r.name == id.as_str())?;
        let table = Arc::new((r.run)(prj, self).unwrap_or_default());
        if let Ok(mut m) = self.tables.lock() {
            m.insert(id, table.clone());
        }
        Some(table)
    }

    /// Forgets the generated tables (e.g. after a change of the language).
    pub fn clear_reports(&self) {
        if let Ok(mut m) = self.tables.lock() {
            m.clear();
        }
    }

}

/// Number of threads work may be spread over: 1 where the platform has none (browser).
pub fn threads() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get())
}

/// Runs `a` and `b`, on two threads where the platform has them. The language of
/// generated texts carries over to the other thread.
pub fn join<A: Send, B: Send>(a: impl FnOnce() -> A + Send, b: impl FnOnce() -> B + Send) -> (A, B) {
    if threads() < 2 {
        return (a(), b());
    }
    let lang = openecraft_i18n::lang();
    std::thread::scope(|s| {
        let a = s.spawn(move || openecraft_i18n::with_lang(lang, a));
        let b = b();
        (a.join().unwrap_or_else(|e| std::panic::resume_unwind(e)), b)
    })
}

/// `f` of every item, in the order of `items`, spread over the available threads
/// (e.g. the checks of a project, which do not depend on each other).
pub fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let n = threads().min(items.len());
    if n < 2 {
        return items.iter().map(f).collect();
    }
    let lang = openecraft_i18n::lang();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut done: Vec<(usize, R)> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..n)
            .map(|_| {
                s.spawn(|| {
                    openecraft_i18n::with_lang(lang, || {
                        let mut out = Vec::new();
                        loop {
                            let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let Some(item) = items.get(i) else { break };
                            out.push((i, f(item)));
                        }
                        out
                    })
                })
            })
            .collect();
        workers.into_iter().flat_map(|w| w.join().unwrap_or_else(|e| std::panic::resume_unwind(e))).collect()
    });
    done.sort_by_key(|(i, _)| *i);
    done.into_iter().map(|(_, r)| r).collect()
}

#[cfg(test)]
mod tests;
