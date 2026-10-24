//! Plug commands: plugs with their pins, both sides of each pin and unused pins.

use crate::params::{param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::model::Project;
use openecraft_core::netlist::Netlist;
use openecraft_core::plugs;
use openecraft_core::symbols::PinKind;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct Plugs;

impl Plugin for Plugs {
    fn info(&self) -> Info {
        info("plugs", tr_const!("Plugs"), tr_const!("Plugs and their pins"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "list_plugs",
        help: tr_const!("Lists the plugs with the number of pins used, the pin count of their parts and the parts"),
        params: &[],
        run: list_plugs,
    },
    Spec {
        name: "get_plug",
        help: tr_const!("Shows the pins of a plug in plug order: what is connected on the socket side and on the pin side, potential, unused pins"),
        params: &[param("plug", Kind::Str, tr_const!("Plug tag, e.g. -X10"))],
        run: get_plug,
    },
];

fn list_plugs(prj: &mut Project, _: &Args) -> Result<Value, String> {
    Ok(plugs::plug_tags(prj)
        .into_iter()
        .map(|tag| {
            let parts: Vec<String> = prj.devices.get(&tag).map(|d| d.parts.iter().map(|r| r.number.clone()).collect()).unwrap_or_default();
            json!({
                "plug": tag,
                "pins_used": plugs::designations(prj, &tag).len(),
                "pin_count": plugs::capacity(prj, &tag),
                "parts": parts,
            })
        })
        .collect())
}

fn get_plug(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let plug = a.req("plug");
    if !plugs::plug_tags(prj).iter().any(|t| t == plug) {
        return Err(tr_fmt!("No plug {}", plug));
    }
    let (nl, open) = (Netlist::build(prj, true), Netlist::build(prj, false));
    let kind = |k: PinKind| match k {
        PinKind::Pair => "pair",
        PinKind::Male => "male",
        PinKind::Female => "female",
    };
    let rows: Vec<Value> = plugs::rows(prj, plug, &nl, &open)
        .into_iter()
        .map(|r| {
            json!({
                "pin": r.label,
                "spare": r.pins.is_empty(),
                "symbols": r.pins.iter().map(|p| json!({ "id": p.id, "page": prj.pages[p.page].name, "kind": kind(p.kind) })).collect::<Vec<_>>(),
                "socket_side": r.socket,
                "pin_side": r.pin,
                "potential": r.potential,
                "function_text": r.function_text,
            })
        })
        .collect();
    Ok(json!({ "plug": plug, "pin_count": plugs::capacity(prj, plug), "pins": rows }))
}

#[cfg(test)]
mod tests;
