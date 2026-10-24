//! Everything derived from the project that the front ends, reports, checks and
//! exports share: device index, netlists, connections, panel wiring, 3D routing and
//! field plan, the report tables of the registered reports (generated on demand) and
//! the issues the host's checks found.

use crate::ext::Report;
use crate::index::Index;
pub use crate::issue::{Issue, Severity};
use crate::model::{Project, ReportId};
use crate::netlist::{connections, Connection, Netlist};
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

impl Analysis {
    pub fn build(prj: &Project) -> Analysis {
        let idx = Index::build(prj);
        let netlist = Netlist::build(prj, true);
        let open = Netlist::build(prj, false);
        let (mut wiring, open_wires) = panel::virtual_wiring(prj, &netlist, &open);
        let conns = connections(prj, &netlist, &open);
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
