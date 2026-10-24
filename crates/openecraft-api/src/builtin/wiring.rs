//! Wiring assistant commands: the routed wires of the panel layout as a checklist with
//! the wiring state, marking wires as wired or as a problem.

use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::analysis::Analysis;
use openecraft_core::model::Project;
use openecraft_core::wiring::{self, Mark, Order, Status};
use openecraft_core::wirenum;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct Wiring;

impl Plugin for Wiring {
    fn info(&self) -> Info {
        info("wiring", tr_const!("Wiring"), tr_const!("Wire numbering and wiring marks"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "renumber_wires",
        help: tr_const!("Numbers the wires by the numbering scheme of the project"),
        params: &[optional("overwrite_fixed", Kind::Bool, tr_const!("Also renumber wires with a fixed number"))],
        run: renumber_wires,
    },
    Spec {
        name: "wiring_list",
        help: tr_const!("Lists the routed wires of the panel layout as a wiring checklist with end treatment, state and progress"),
        params: &[
            optional("enclosure", Kind::Str, tr_const!("Enclosure name (all if missing)")),
            optional("order", Kind::Str, tr_const!("device (default), position, cross_section or number")),
            optional("status", Kind::Str, tr_const!("Only wires with this state: open, wired, changed or problem")),
        ],
        run: wiring_list,
    },
    Spec {
        name: "mark_wire",
        help: tr_const!("Marks a routed wire as wired or as a problem, or as open again; the wire is given by its two ends"),
        params: &[
            param("from", Kind::Str, tr_const!("One end, e.g. -K1:A1")),
            param("to", Kind::Str, tr_const!("Other end, e.g. -X1:3")),
            param("state", Kind::Str, tr_const!("wired, problem or open")),
            optional("note", Kind::Str, tr_const!("Note, e.g. what the problem is")),
        ],
        run: mark_wire,
    },
    Spec {
        name: "remove_old_wiring_marks",
        help: tr_const!("Removes the wiring marks of wires that no longer exist"),
        params: &[],
        run: remove_old_marks,
    },
];

fn renumber_wires(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let an = Analysis::build(prj);
    let n = wirenum::renumber(prj, &an.open, &an.netlist, a.flag("overwrite_fixed"));
    Ok(json!({ "renumbered": n }))
}

fn status_code(s: Status) -> &'static str {
    match s {
        Status::Open => "open",
        Status::Done => "wired",
        Status::Changed => "changed",
        Status::Problem => "problem",
    }
}

fn wiring_list(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let an = Analysis::build(prj);
    let enclosure = match a.str("enclosure") {
        Some(name) => Some(prj.layout.enclosures.iter().position(|e| e.name == name).ok_or_else(|| tr_fmt!("No enclosure named {}", name))?),
        None => None,
    };
    let order = match a.str("order") {
        Some(o) => Order::from_code(o).ok_or_else(|| tr_fmt!("Unknown order: {}", o))?,
        None => Order::Device,
    };
    let filter = a.str("status").map(|s| s.trim().to_ascii_lowercase());
    let steps: Vec<Value> = wiring::steps(prj, &an.wiring, enclosure, order)
        .into_iter()
        .filter(|st| filter.as_deref().is_none_or(|f| status_code(st.status) == f))
        .map(|st| {
            let w = &an.wiring[st.wire];
            json!({
                "enclosure": w.enclosure_names(&prj.layout),
                "from": w.from,
                "to": w.to,
                "number": w.number,
                "potential": w.net,
                "color": w.color,
                "cross_section": w.cross_section,
                "length_mm": w.length.round(),
                "strip_mm": st.ends.map(|e| e.strip),
                "ferrule": st.ends.and_then(|e| e.ferrule).map(|f| f.part),
                "status": status_code(st.status),
                "note": st.note,
            })
        })
        .collect();
    let p = wiring::progress(prj, &an.wiring, enclosure);
    Ok(json!({
        "wires": steps,
        "progress": {
            "total": p.total,
            "wired": p.done - p.changed,
            "changed": p.changed,
            "problems": p.problems,
            "percent": (p.fraction() * 1000.0).round() / 10.0,
            "length_wired_m": (p.length_done / 10.0).round() / 100.0,
            "length_total_m": (p.length_total / 10.0).round() / 100.0,
        },
        "old_marks": wiring::stale(prj, &an.wiring).len(),
    }))
}

fn mark_wire(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let mark = match a.req("state").trim().to_ascii_lowercase().as_str() {
        "wired" | "done" => Some(Mark::Done),
        "problem" => Some(Mark::Problem),
        "open" => None,
        x => return Err(tr_fmt!("Unknown state: {}", x)),
    };
    let an = Analysis::build(prj);
    let (from, to) = (a.req("from").trim(), a.req("to").trim());
    let w = an
        .wiring
        .iter()
        .find(|w| (w.from == from && w.to == to) || (w.from == to && w.to == from))
        .ok_or_else(|| tr_fmt!("No routed wire {} – {}", from, to))?;
    wiring::set_mark(prj, w, mark, a.str("note").unwrap_or_default());
    let p = wiring::progress(prj, &an.wiring, None);
    Ok(json!({ "from": w.from, "to": w.to, "status": status_code(wiring::status(prj, w)), "wired": p.done - p.changed, "total": p.total }))
}

fn remove_old_marks(prj: &mut Project, _: &Args) -> Result<Value, String> {
    let an = Analysis::build(prj);
    let removed = wiring::remove_stale(prj, &an.wiring);
    Ok(json!({ "removed": removed }))
}

#[cfg(test)]
mod tests;
