//! Plug commands: plugs with their pins, both sides of each pin and unused pins, the
//! pin-out on the mating face and flat cables on IDC connectors.

use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::model::Project;
use openecraft_core::netlist::Netlist;
use openecraft_core::pinout::{self, Side};
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
    Spec {
        name: "get_pinout",
        help: tr_const!("Shows the mating face of a plug: pin layout of its part, the position of each pin on the face (mm, seen on the pin side) with potential, wire colour and cable core of both sides"),
        params: &[param("plug", Kind::Str, tr_const!("Plug tag, e.g. -X10"))],
        run: get_pinout,
    },
    Spec {
        name: "connect_flat_cable",
        help: tr_const!("Makes the wires at the pins of a plug cores of a flat (ribbon) cable: pin n takes core n, as on IDC connectors"),
        params: &[
            param("cable", Kind::Str, tr_const!("Cable tag, e.g. -W20")),
            param("plug", Kind::Str, tr_const!("Plug tag, e.g. -X10")),
            optional("side", Kind::Str, tr_const!("Side of the plug the cable leaves: socket or pin (default: socket)")),
        ],
        run: connect_flat_cable,
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
    require_plug(prj, plug)?;
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

fn require_plug(prj: &Project, plug: &str) -> Result<(), String> {
    if plugs::plug_tags(prj).iter().any(|t| t == plug) { Ok(()) } else { Err(tr_fmt!("No plug {}", plug)) }
}

fn get_pinout(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let plug = a.req("plug");
    require_plug(prj, plug)?;
    let (nl, open) = (Netlist::build(prj, true), Netlist::build(prj, false));
    let po = pinout::pinout(prj, plug, &nl, &open);
    let end = |e: &pinout::End| json!({ "targets": e.targets, "color": e.color, "core": e.core });
    let pins: Vec<Value> = po
        .pins
        .iter()
        .map(|p| {
            let at = p.slot.and_then(|s| po.face.slots.get(s)).map(|[x, y]| json!([(x * 100.0).round() / 100.0, (y * 100.0).round() / 100.0]));
            json!({ "pin": p.label, "spare": !p.used(), "at": at, "potential": p.potential, "function_text": p.function_text, "pin_side": end(&p.pin), "socket_side": end(&p.socket) })
        })
        .collect();
    let parts: Vec<Value> = po.parts.iter().map(|(n, d, g)| json!({ "part": n, "description": d, "gender": g })).collect();
    Ok(json!({ "plug": plug, "parts": parts, "layout": po.layout.as_ref().map(|l| l.text()), "coding": po.coding, "pins": pins }))
}

fn connect_flat_cable(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let plug = a.req("plug");
    require_plug(prj, plug)?;
    let side = match a.str("side").unwrap_or("socket") {
        "socket" => Side::Socket,
        "pin" => Side::Pin,
        other => return Err(tr_fmt!("Unknown side {} (socket or pin)", other)),
    };
    let wires = pinout::connect_flat_cable(prj, a.req("cable"), plug, side)?;
    Ok(json!({ "cable": a.req("cable"), "plug": plug, "wires": wires }))
}

#[cfg(test)]
mod tests;
