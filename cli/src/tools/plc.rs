//! PLC hardware tools: stations with racks and modules, bus systems and their port
//! connections, device descriptions, automatic configuration and AutomationML import.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::aml;
use openecraft_core::devdesc;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};
use openecraft_api::builtin::plc::{description_json, hardware_json};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "import_device_descriptions",
        help: tr_const!("Imports device descriptions from a GSDML (PROFINET) or ESI (EtherCAT) file into the project"),
        params: &[param("path", Kind::Str, tr_const!("GSDML or ESI file"))],
        needs_project: true,
        run: import_device_descriptions,
    },
    Tool {
        name: "import_aml",
        help: tr_const!("Imports a PLC hardware configuration from AutomationML (e.g. exported from TIA Portal): stations, modules, addresses, subnets, port connections and tag comments"),
        params: &[optional("path", Kind::Str, tr_const!("AutomationML file")), optional("text", Kind::Str, tr_const!("AutomationML content instead of a file"))],
        needs_project: true,
        run: import_aml,
    },
];

fn import_device_descriptions(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("path");
    let text = std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, openecraft_i18n::io_error(&e)))?;
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let list = devdesc::import(&text, file)?;
    let out: Vec<Value> = list.iter().map(description_json).collect();
    let added = s.project_mut()?.plc.add_descriptions(list);
    Ok(json!({ "added": added, "descriptions": out }))
}

fn import_aml(s: &mut Session, a: &Args) -> Result<Value, String> {
    let text = match (a.str("text"), a.str("path")) {
        (Some(t), _) => t.to_string(),
        (None, Some(path)) => std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, openecraft_i18n::io_error(&e)))?,
        (None, None) => return Err(tr_fmt!("Missing parameter: {}", "path")),
    };
    let prj = s.project_mut()?;
    let r = aml::import(prj, &text)?;
    let mut v = hardware_json(prj);
    v["imported"] = json!({ "stations": r.stations, "modules": r.modules, "bus_systems": r.networks, "connections": r.connections, "addresses_changed": r.addresses, "texts_changed": r.texts });
    Ok(v)
}

#[cfg(test)]
mod tests;
