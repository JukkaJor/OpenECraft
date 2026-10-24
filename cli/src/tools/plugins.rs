//! Plugins: the loaded plugins with what they add (their commands are tools of their own).

use super::{optional, Args, Kind, Tool};
use crate::session::Session;
use openecraft_api::Origin;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[Tool {
    name: "plugins",
    help: tr_const!("Loaded plugins (built-in and from the plugin folders of --plugins / OPENECRAFT_PLUGINS) with their commands, checks, reports and exports; enables or disables one for this session"),
    params: &[
        optional("enable", Kind::Str, tr_const!("Id of the plugin to enable")),
        optional("disable", Kind::Str, tr_const!("Id of the plugin to disable")),
    ],
    needs_project: false,
    run: plugins,
}];

fn plugins(s: &mut Session, a: &Args) -> Result<Value, String> {
    for (name, on) in [("enable", true), ("disable", false)] {
        if let Some(id) = a.str(name) {
            if !s.plugins.list().iter().any(|l| l.info.id == id) {
                return Err(tr_fmt!("Unknown plugin: {}", id));
            }
            s.plugins.set_enabled(id, on);
        }
    }
    let names = |v: Vec<&str>| json!(v);
    let list: Vec<Value> = s
        .plugins
        .list()
        .iter()
        .map(|l| {
            let r = &l.registry;
            json!({
                "id": l.info.id,
                "name": l.info.name,
                "version": l.info.version,
                "description": l.info.description,
                "folder": match &l.origin { Origin::BuiltIn => None, Origin::External(dir) => Some(dir.display().to_string()) },
                "enabled": l.enabled,
                "commands": names(r.commands.iter().map(|x| x.name.as_str()).collect()),
                "checks": names(r.checks.iter().map(|x| x.name.as_str()).collect()),
                "reports": names(r.reports.iter().map(|x| x.name.as_str()).collect()),
                "exports": names(r.exporters.iter().map(|x| x.name.as_str()).collect()),
            })
        })
        .collect();
    Ok(json!({ "plugins": list, "errors": s.plugins.errors }))
}

#[cfg(test)]
mod tests;
