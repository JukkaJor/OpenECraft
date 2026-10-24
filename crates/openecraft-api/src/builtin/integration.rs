//! Integration commands: exchange with ERP, PDM, PLM, ECAD, MCAD, procurement and wiring
//! machines through the connectors of the integration layer, with profiles (column
//! mapping), a preview of imported changes and conflict detection.

use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::integration::{self, Profile};
use openecraft_core::model::Project;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub struct Integration;

impl Plugin for Integration {
    fn info(&self) -> Info {
        info("integration", tr_const!("Integration"), tr_const!("Integration profiles and status"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "set_integration_profile",
        help: tr_const!("Adds, replaces or removes a profile: a connector with the column names the other system uses"),
        params: &[
            param("name", Kind::Str, tr_const!("Profile name, e.g. SAP")),
            optional("connector", Kind::Str, tr_const!("Connector id (required unless removing)")),
            optional("mapping", Kind::Object, tr_const!("Column mapping {field code: column name}")),
            optional("remove", Kind::Bool, tr_const!("Remove the profile")),
        ],
        run: set_profile,
    },
    Spec {
        name: "integration_status",
        help: tr_const!("Shows the data of the other systems stored in the project (part links, documents) and the exchange log"),
        params: &[],
        run: status,
    },
];

pub fn connector(a: &Args) -> Result<&'static integration::Connector, String> {
    let id = a.req("connector");
    integration::connector(id).ok_or_else(|| tr_fmt!("Unknown connector: {}", id))
}

pub fn mapping(a: &Args) -> BTreeMap<String, String> {
    a.value("mapping").and_then(Value::as_object).map(|m| m.iter().filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string()))).collect()).unwrap_or_default()
}

fn set_profile(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let name = a.req("name").trim().to_string();
    let profiles = &mut prj.integration.profiles;
    if a.flag("remove") {
        let before = profiles.len();
        profiles.retain(|p| p.name != name);
        return Ok(json!({ "removed": before - profiles.len() }));
    }
    let id = a.str("connector").ok_or_else(|| tr_fmt!("Missing parameter: {}", "connector"))?;
    integration::connector(id).ok_or_else(|| tr_fmt!("Unknown connector: {}", id))?;
    let map = mapping(a);
    if let Some(bad) = map.keys().find(|k| integration::Field::from_code(k).is_none()) {
        return Err(tr_fmt!("Unknown field: {}", bad));
    }
    let p = Profile { name: name.clone(), connector: id.to_string(), mapping: map };
    match profiles.iter_mut().find(|p| p.name == name) {
        Some(old) => *old = p,
        None => profiles.push(p),
    }
    Ok(json!({ "profiles": profiles.len() }))
}

fn status(prj: &mut Project, _: &Args) -> Result<Value, String> {
    let i = &prj.integration;
    Ok(json!({
        "items": i.items,
        "documents": i.documents,
        "project": i.project,
        "profiles": i.profiles,
        "log": i.log,
    }))
}
