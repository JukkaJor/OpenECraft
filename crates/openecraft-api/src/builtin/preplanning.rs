//! Pre-planning ([`openecraft_core::preplanning`]): segments with planned devices,
//! segment templates, the comparison with the schematic as check and report, and the
//! exchange as CSV / Excel. Segments are named by their path (`Plant/Area 1`).

use crate::builtin::common::{num_in, NO_MAX};
use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry, Table, Text};
use openecraft_core::ext::Export;
use openecraft_core::index::Index;
use openecraft_core::model::Project;
use openecraft_core::preplanning::{self as pp, PlannedDevice};
use openecraft_i18n::{tr, tr_const, tr_fmt};
use serde_json::{json, Value};

const SEGMENT: &str = tr_const!("Segment path, e.g. Plant/Area 1");

pub struct Preplanning;

impl Plugin for Preplanning {
    fn info(&self) -> Info {
        Info {
            id: "preplanning".into(),
            name: Text::Borrowed(tr_const!("Pre-planning")),
            version: env!("CARGO_PKG_VERSION").into(),
            description: Text::Borrowed(tr_const!("Plant segments with planned devices before the schematic: templates, power budget, state against the schematic, exchange as CSV / Excel")),
        }
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        reg.report("preplanning", tr_const!("Pre-planning"), |prj, an| Ok(report(prj, &an.index)))
            .exporter_with("preplanning_csv", tr_const!("Pre-planning (CSV)"), "csv", false, vec![], |prj, _, _| Ok(Export::Text(pp::csv(prj))))
            .exporter_with("preplanning_xlsx", tr_const!("Pre-planning (Excel)"), "xlsx", true, vec![], |prj, _, _| {
                Ok(Export::Binary(openecraft_core::xlsx::write(&[("Preplanning".into(), pp::sheet(prj))])))
            })
            .command("preplanning", tr_const!("Lists the segments in tree order with structure, planned power and the planned devices with their state in the schematic"), vec![], |prj, _| Ok(tree_json(prj)))
            .command(
                "add_segment",
                tr_const!("Adds a segment below another one (or at the top)"),
                vec![param("name", Kind::Str, tr_const!("Name of the segment")), optional("parent", Kind::Str, SEGMENT)],
                add_segment,
            )
            .command(
                "set_segment",
                tr_const!("Changes name, description, function, location or parent of a segment"),
                vec![
                    param("segment", Kind::Str, SEGMENT),
                    optional("name", Kind::Str, tr_const!("Name of the segment")),
                    optional("description", Kind::Str, tr_const!("Description")),
                    optional("function", Kind::Str, tr_const!("Function (= without the sign; empty: the parent's)")),
                    optional("location", Kind::Str, tr_const!("Location (+ without the sign; empty: the parent's)")),
                    optional("parent", Kind::Str, tr_const!("New parent segment (empty: top level)")),
                ],
                set_segment,
            )
            .command("remove_segment", tr_const!("Removes a segment with everything below it"), vec![param("segment", Kind::Str, SEGMENT)], |prj, a| {
                let id = find(prj, a.req("segment"))?;
                Ok(json!({ "removed": pp::remove_segment(prj, id) }))
            })
            .command(
                "plan_device",
                tr_const!("Adds a planned device to a segment or changes it"),
                vec![
                    param("segment", Kind::Str, SEGMENT),
                    param("tag", Kind::Str, tr_const!("Device tag within the segment, e.g. -M1")),
                    optional("kind", Kind::Str, tr_const!("What it is: motor, valve, sensor, …")),
                    optional("description", Kind::Str, tr_const!("Description")),
                    optional("power", Kind::Num, tr_const!("Planned power in kW")),
                    optional("voltage", Kind::Str, tr_const!("Voltage")),
                    optional("part", Kind::Str, tr_const!("Planned part number")),
                    optional("macro", Kind::Str, tr_const!("Page macro that generates its schematic")),
                ],
                plan_device,
            )
            .command(
                "remove_planned_device",
                tr_const!("Removes a planned device from a segment"),
                vec![param("segment", Kind::Str, SEGMENT), param("tag", Kind::Str, tr_const!("Device tag within the segment, e.g. -M1"))],
                |prj, a| {
                    let id = find(prj, a.req("segment"))?;
                    let s = pp::segment_mut(prj, id).ok_or_else(|| tr("Unknown segment").to_string())?;
                    let before = s.devices.len();
                    s.devices.retain(|d| d.tag != a.req("tag"));
                    if s.devices.len() == before {
                        return Err(tr_fmt!("No planned device {}", a.req("tag")));
                    }
                    Ok(json!({ "devices": s.devices.len() }))
                },
            )
            .command(
                "save_segment_template",
                tr_const!("Saves the planned devices of a segment as a template"),
                vec![param("segment", Kind::Str, SEGMENT), param("name", Kind::Str, tr_const!("Name of the template"))],
                |prj, a| {
                    let id = find(prj, a.req("segment"))?;
                    pp::save_template(prj, id, a.req("name"))?;
                    Ok(json!({ "templates": prj.preplanning.templates.iter().map(|t| t.name.clone()).collect::<Vec<_>>() }))
                },
            )
            .command(
                "apply_segment_template",
                tr_const!("Adds the devices of a template to a segment"),
                vec![param("segment", Kind::Str, SEGMENT), param("template", Kind::Str, tr_const!("Name of the template"))],
                |prj, a| {
                    let id = find(prj, a.req("segment"))?;
                    Ok(json!({ "added": pp::apply_template(prj, id, a.req("template"))? }))
                },
            );
        Ok(())
    }
}

/// The segment of a path.
pub fn find(prj: &Project, path: &str) -> Result<u64, String> {
    let want = path.trim().trim_matches(pp::PATH_SEP);
    prj.preplanning.segments.iter().map(|s| s.id).find(|id| pp::path(prj, *id) == want).ok_or_else(|| tr_fmt!("No segment {}", path))
}

fn report(prj: &Project, idx: &Index) -> Table {
    let mut t = Table::new([tr_const!("Segment"), tr_const!("Device tag"), tr_const!("Kind"), tr_const!("Description"), tr_const!("Power (kW)"), tr_const!("Voltage"), tr_const!("Part"), tr_const!("State")]);
    for r in pp::rows(prj, idx) {
        let power = if r.device.power > 0.0 { r.device.power.to_string() } else { String::new() };
        t.push(vec![r.path, r.tag, r.device.kind, r.device.description, power, r.device.voltage, r.device.part, r.state.title().to_string()], None);
    }
    t
}

fn tree_json(prj: &Project) -> Value {
    let idx = Index::build(prj);
    let segments: Vec<Value> = pp::tree(prj)
        .into_iter()
        .map(|(depth, s)| {
            let (function, location) = pp::structure(prj, s.id);
            let devices: Vec<Value> = s
                .devices
                .iter()
                .map(|d| {
                    let tag = pp::full_tag(prj, s.id, d);
                    let state = match pp::state(prj, &idx, &tag, d) {
                        pp::State::Planned => "planned",
                        pp::State::Engineered => "engineered",
                        pp::State::PartDiffers => "part_differs",
                    };
                    json!({ "tag": tag, "kind": d.kind, "description": d.description, "power": d.power, "voltage": d.voltage, "part": d.part, "macro": d.macro_name, "state": state })
                })
                .collect();
            json!({ "path": pp::path(prj, s.id), "depth": depth, "description": s.description, "function": function, "location": location, "power": pp::power(prj, s.id), "devices": devices })
        })
        .collect();
    let templates: Vec<String> = prj.preplanning.templates.iter().map(|t| t.name.clone()).collect();
    json!({ "segments": segments, "templates": templates })
}

fn add_segment(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let parent = match a.str("parent").filter(|p| !p.trim().is_empty()) {
        Some(p) => find(prj, p)?,
        None => 0,
    };
    let name = a.req("name").trim();
    if name.is_empty() || name.contains(pp::PATH_SEP) {
        return Err(tr_fmt!("Invalid segment name: {}", name));
    }
    if prj.preplanning.segments.iter().any(|s| s.parent == parent && s.name == name) {
        return Err(tr_fmt!("Segment {} already exists", name));
    }
    let id = pp::add_segment(prj, parent, name);
    Ok(json!({ "path": pp::path(prj, id) }))
}

fn set_segment(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let id = find(prj, a.req("segment"))?;
    if let Some(parent) = a.str("parent") {
        let parent = if parent.trim().is_empty() { 0 } else { find(prj, parent)? };
        if !pp::move_segment(prj, id, parent) {
            return Err(tr("A segment cannot be moved below itself").into());
        }
    }
    let s = pp::segment_mut(prj, id).ok_or_else(|| tr("Unknown segment").to_string())?;
    if let Some(n) = a.str("name") {
        if n.trim().is_empty() || n.contains(pp::PATH_SEP) {
            return Err(tr_fmt!("Invalid segment name: {}", n));
        }
        s.name = n.trim().to_string();
    }
    for (key, field) in [("description", &mut s.description), ("function", &mut s.function), ("location", &mut s.location)] {
        if let Some(v) = a.str(key) {
            *field = v.trim().trim_start_matches(['=', '+']).to_string();
        }
    }
    Ok(json!({ "path": pp::path(prj, id) }))
}

fn plan_device(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let id = find(prj, a.req("segment"))?;
    let tag = a.req("tag").trim().to_string();
    let s = pp::segment_mut(prj, id).ok_or_else(|| tr("Unknown segment").to_string())?;
    let at = match s.devices.iter().position(|d| d.tag == tag) {
        Some(i) => i,
        None => {
            s.devices.push(PlannedDevice { tag: tag.clone(), ..Default::default() });
            s.devices.len() - 1
        }
    };
    let d = &mut s.devices[at];
    for (key, field) in [("kind", &mut d.kind), ("description", &mut d.description), ("voltage", &mut d.voltage), ("part", &mut d.part), ("macro", &mut d.macro_name)] {
        if let Some(v) = a.str(key) {
            *field = v.trim().to_string();
        }
    }
    if let Some(p) = num_in(a, "power", 0.0, NO_MAX)? {
        d.power = p;
    }
    let d = d.clone();
    Ok(json!({ "tag": pp::full_tag(prj, id, &d), "segment_power": pp::power(prj, id) }))
}

#[cfg(test)]
mod tests;
