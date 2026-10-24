//! Part lifecycle: project parts that are phased out or discontinued (data portal),
//! as check, report and a command that changes devices over to the successors.

use crate::params::{optional, Args, Kind};
use crate::{Registry, Table};
use crate::{Info, Plugin, Text};
use openecraft_core::model::Project;
use openecraft_core::reports::{Issue, Severity};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use openecraft_core::parts::portal::Lifecycle;
use serde_json::{json, Value};

pub struct PartLifecycle;

impl Plugin for PartLifecycle {
    fn info(&self) -> Info {
        Info {
            id: "part_lifecycle".into(),
            name: Text::Borrowed(tr_const!("Part lifecycle")),
            version: env!("CARGO_PKG_VERSION").into(),
            description: Text::Borrowed(tr_const!("Devices with parts that are phased out or discontinued, and their change to the successors")),
        }
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        reg.check("part_lifecycle", tr_const!("Part lifecycle"), |prj, _| Ok(issues(prj)))
            .report("part_lifecycle", tr_const!("Parts phased out or discontinued"), |prj, _| {
                let mut table = Table::new([tr_const!("Part number"), tr_const!("Lifecycle"), tr_const!("Successor"), tr_const!("Devices")]);
                for (number, lifecycle, successor, devices) in affected(prj) {
                    table.push(vec![number, tr(lifecycle.title()).to_string(), successor, devices.join(", ")], None);
                }
                Ok(table)
            })
            .command(
                "use_successor_parts",
                tr_const!("Changes devices with phased out or discontinued parts over to the successor parts (the successors must be project parts)"),
                vec![optional("number", Kind::Str, tr_const!("Only this part number"))],
                use_successors,
            );
        Ok(())
    }
}

/// (part number, lifecycle, successor, device tags) of the used parts that are not active.
fn affected(prj: &Project) -> Vec<(String, Lifecycle, String, Vec<String>)> {
    let mut out = Vec::new();
    for p in prj.parts.iter().filter(|p| !p.lifecycle.is_active()) {
        let devices: Vec<String> = prj.devices.iter().filter(|(_, d)| d.parts.iter().any(|r| r.number == p.number)).map(|(tag, _)| tag.clone()).collect();
        if !devices.is_empty() {
            out.push((p.number.clone(), p.lifecycle, p.successor.clone(), devices));
        }
    }
    out
}

fn issues(prj: &Project) -> Vec<Issue> {
    let mut out = Vec::new();
    for (number, lifecycle, successor, devices) in affected(prj) {
        let severity = if lifecycle == Lifecycle::Discontinued { Severity::Warning } else { Severity::Info };
        for tag in devices {
            let message = match (lifecycle, successor.is_empty()) {
                (Lifecycle::Discontinued, true) => tr_fmt!("{}: part {} is discontinued", tag, number),
                (Lifecycle::Discontinued, false) => tr_fmt!("{}: part {} is discontinued, successor {}", tag, number, successor),
                (_, true) => tr_fmt!("{}: part {} is being phased out", tag, number),
                (_, false) => tr_fmt!("{}: part {} is being phased out, successor {}", tag, number, successor),
            };
            out.push(Issue { severity, message, target: None });
        }
    }
    out
}

fn use_successors(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let only = a.str("number");
    let mut changed = Vec::new();
    let mut missing = Vec::new();
    for (number, _, successor, devices) in affected(prj) {
        if only.is_some_and(|n| n != number) || successor.is_empty() {
            continue;
        }
        if !prj.parts.iter().any(|p| p.number == successor) {
            missing.push(successor);
            continue;
        }
        for tag in devices {
            if let Some(d) = prj.devices.get_mut(&tag) {
                for r in d.parts.iter_mut().filter(|r| r.number == number) {
                    r.number = successor.clone();
                }
                changed.push(json!({ "device": tag, "old": number, "new": successor }));
            }
        }
    }
    missing.sort();
    missing.dedup();
    Ok(json!({ "changed": changed, "missing_successors": missing }))
}

#[cfg(test)]
mod tests;
