//! Fluid and process tools: hoses / pipes between fluid ports, their defaults, and
//! the PCT loops of the pre-planning with their engineering status.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::fluid::{self, Medium};
use openecraft_core::index::Index;
use openecraft_core::netlist::Netlist;
use openecraft_core::process::{self, PctLoop, Status};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "list_hoses",
        help: tr_const!("Lists the hoses and pipes (connections between fluid ports) with medium, nominal size, colour and length"),
        params: &[],
        needs_project: true,
        run: list_hoses,
    },
    Tool {
        name: "set_fluid_defaults",
        help: tr_const!("Sets the defaults of hoses that do not set their own values, and the hose number prefix"),
        params: &[
            optional("medium", Kind::Str, tr_const!("compressed_air, vacuum, hydraulic_oil, water, coolant, lubricant, gas, steam or product")),
            optional("size", Kind::Str, tr_const!("Nominal size, e.g. 8 mm or DN25")),
            optional("color", Kind::Str, tr_const!("Hose colour")),
            optional("prefix", Kind::Str, tr_const!("Hose number prefix, e.g. H")),
        ],
        needs_project: true,
        run: set_fluid_defaults,
    },
    Tool {
        name: "list_pct_loops",
        help: tr_const!("Lists the PCT loops of the pre-planning and of the P&ID instruments with their devices and engineering status"),
        params: &[],
        needs_project: true,
        run: list_pct_loops,
    },
    Tool {
        name: "set_pct_loop",
        help: tr_const!("Creates or changes a PCT loop of the pre-planning"),
        params: &[
            param("name", Kind::Str, tr_const!("Loop number, e.g. 101")),
            optional("rename", Kind::Str, tr_const!("New name")),
            optional("description", Kind::Str, tr_const!("Description")),
            optional("range", Kind::Str, tr_const!("Measuring range or set point")),
            optional("devices", Kind::Str, tr_const!("Further device tags of the loop, separated by blanks")),
        ],
        needs_project: true,
        run: set_pct_loop,
    },
    Tool {
        name: "delete_pct_loop",
        help: tr_const!("Deletes a PCT loop of the pre-planning"),
        params: &[param("name", Kind::Str, tr_const!("Loop number"))],
        needs_project: true,
        run: delete_pct_loop,
    },
];

fn status_code(s: Status) -> &'static str {
    match s {
        Status::Planned => "planned",
        Status::InProgress => "in_engineering",
        Status::Engineered => "engineered",
    }
}

fn list_hoses(s: &mut Session, _: &Args) -> Result<Value, String> {
    let prj = s.project()?;
    let open = Netlist::build(prj, false);
    let hoses: Vec<Value> = fluid::hoses(prj, &open)
        .into_iter()
        .map(|h| json!({ "number": h.number, "net": h.net, "from": h.from, "to": h.to, "medium": h.medium.code(), "size": h.size, "color": h.color, "length_m": h.length }))
        .collect();
    let f = &prj.fluid;
    Ok(json!({ "hoses": hoses, "defaults": { "medium": f.medium.code(), "size": f.size, "color": f.color, "prefix": f.prefix } }))
}

fn set_fluid_defaults(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    if let Some(m) = a.str("medium") {
        prj.fluid.medium = Medium::from_code(m).ok_or_else(|| tr_fmt!("Unknown medium: {}", m))?;
    }
    for (name, field) in [("size", &mut prj.fluid.size), ("color", &mut prj.fluid.color), ("prefix", &mut prj.fluid.prefix)] {
        if let Some(v) = a.str(name) {
            *field = v.trim().into();
        }
    }
    list_hoses(s, a)
}

fn loops_json(s: &Session) -> Result<Value, String> {
    let prj = s.project()?;
    let idx = Index::build(prj);
    let loops: Vec<Value> = process::loops(prj, &idx)
        .into_iter()
        .map(|l| {
            let variable = process::variable(l.functions.first().map(String::as_str).unwrap_or(""));
            json!({
                "name": l.name,
                "description": l.description,
                "range": l.range,
                "defined": l.defined,
                "functions": l.functions,
                "measured_variable": variable,
                "devices": l.devices,
                "missing_devices": l.missing,
                "status": status_code(l.status),
            })
        })
        .collect();
    Ok(Value::Array(loops))
}

fn list_pct_loops(s: &mut Session, _: &Args) -> Result<Value, String> {
    loops_json(s)
}

fn set_pct_loop(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let name = a.req("name").trim().to_string();
    if name.is_empty() {
        return Err(tr_fmt!("Missing parameter: {}", "name"));
    }
    if let Some(new) = a.str("rename").map(str::trim).filter(|n| *n != name) {
        if new.is_empty() || prj.pct_loops.iter().any(|l| l.name == new) {
            return Err(tr_fmt!("PCT loop {} already exists", new));
        }
    }
    let i = match prj.pct_loops.iter().position(|l| l.name == name) {
        Some(i) => i,
        None => {
            prj.pct_loops.push(PctLoop { name: name.clone(), ..Default::default() });
            prj.pct_loops.len() - 1
        }
    };
    let l = &mut prj.pct_loops[i];
    for (field, value) in [("description", &mut l.description), ("range", &mut l.range)] {
        if let Some(v) = a.str(field) {
            *value = v.trim().into();
        }
    }
    if let Some(d) = a.str("devices") {
        l.devices = d.split_whitespace().map(String::from).collect();
    }
    if let Some(new) = a.str("rename") {
        l.name = new.trim().into();
    }
    loops_json(s)
}

fn delete_pct_loop(s: &mut Session, a: &Args) -> Result<Value, String> {
    let prj = s.project_mut()?;
    let name = a.req("name");
    let i = prj.pct_loops.iter().position(|l| l.name == name).ok_or_else(|| tr_fmt!("No PCT loop {}", name))?;
    prj.pct_loops.remove(i);
    loops_json(s)
}

#[cfg(test)]
mod tests;
