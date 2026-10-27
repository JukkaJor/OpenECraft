//! Settings of the project check: switch checks off, change the level of a kind of
//! message, accept single messages as checked. The command line's `check` shows the
//! codes and fingerprints these commands take, `list_checks` the checks.

use crate::params::{optional, param, Args, Kind};
use crate::{Info, Plugin, Registry};
use super::{add_commands, info, Spec};
use openecraft_core::model::checks::{Accepted, Level};
use openecraft_core::model::Project;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub struct CheckSettings;

impl Plugin for CheckSettings {
    fn info(&self) -> Info {
        info("check_settings", tr_const!("Check settings"), tr_const!("Switch checks off, change message levels, accept messages as checked"))
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        add_commands(reg, COMMANDS)
    }
}

pub static COMMANDS: &[Spec] = &[
    Spec {
        name: "check_settings",
        help: tr_const!("Shows the settings of the project check: checks switched off, changed message levels and accepted messages"),
        params: &[],
        run: show,
    },
    Spec {
        name: "set_check",
        help: tr_const!("Switches a check on or off (names as shown by list_checks)"),
        params: &[param("name", Kind::Str, tr_const!("Name of the check")), param("enabled", Kind::Bool, tr_const!("Run the check"))],
        run: set_check,
    },
    Spec {
        name: "set_message_level",
        help: tr_const!("Changes the level of a kind of message (code as shown by check)"),
        params: &[
            param("code", Kind::Str, tr_const!("Message code, e.g. wires.3f9a1c")),
            param("level", Kind::Str, tr_const!("error, warning, info, off, or default to take the change back")),
        ],
        run: set_level,
    },
    Spec {
        name: "accept_message",
        help: tr_const!("Accepts a message as checked, so that it is no longer counted until it changes"),
        params: &[
            param("fingerprint", Kind::Str, tr_const!("Fingerprint of the message as shown by check")),
            optional("accept", Kind::Bool, tr_const!("false takes the acceptance back (default true)")),
            optional("message", Kind::Str, tr_const!("The message text, kept for the list of accepted messages")),
            optional("note", Kind::Str, tr_const!("Why the message is accepted")),
            optional("user", Kind::Str, tr_const!("Who accepted it")),
            optional("date", Kind::Str, tr_const!("Date of the acceptance")),
        ],
        run: accept,
    },
];

fn level_code(l: Level) -> &'static str {
    match l {
        Level::Error => "error",
        Level::Warning => "warning",
        Level::Info => "info",
        Level::Off => "off",
    }
}

fn show(prj: &mut Project, _: &Args) -> Result<Value, String> {
    let s = &prj.check_settings;
    Ok(json!({
        "disabled": s.disabled,
        "levels": s.levels.iter().map(|(code, l)| (code.clone(), json!(level_code(*l)))).collect::<serde_json::Map<_, _>>(),
        "accepted": s.accepted.iter().map(|a| json!({ "fingerprint": a.id, "message": a.message, "note": a.note, "user": a.user, "date": a.date })).collect::<Vec<_>>(),
    }))
}

fn set_check(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let name = a.req("name").trim().to_string();
    if !crate::is_snake_case(&name) {
        return Err(tr_fmt!("Unknown check: {}", name));
    }
    let builtin = super::check_names().contains(&name);
    let d = &mut prj.check_settings.disabled;
    d.retain(|n| *n != name);
    if !a.flag("enabled") {
        d.push(name.clone());
        d.sort();
    }
    let mut out = show(prj, a)?;
    // Checks of external plugins are not known here: a name that is no built-in
    // check is kept for them, but said.
    if !builtin {
        out["note"] = json!(tr_fmt!("No built-in check is named {}: it applies to a check of a plugin of that name (list_checks)", name));
    }
    Ok(out)
}

/// `devices.3f9a1c` or `routing_3d`: the code of a message as `check` shows it.
fn is_code(code: &str) -> bool {
    let (name, hash) = code.split_once('.').unwrap_or((code, "0"));
    crate::is_snake_case(name) && !hash.is_empty() && hash.chars().all(|c| c.is_ascii_hexdigit())
}

fn set_level(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let code = a.req("code").trim().to_string();
    if !is_code(&code) {
        return Err(tr_fmt!("{} is not a message code as shown by check (e.g. wires.3f9a1c)", code));
    }
    let level = match a.req("level").trim().to_ascii_lowercase().as_str() {
        "error" => Some(Level::Error),
        "warning" => Some(Level::Warning),
        "info" => Some(Level::Info),
        "off" => Some(Level::Off),
        "default" | "" => None,
        other => return Err(tr_fmt!("Unknown level: {}", other)),
    };
    match level {
        Some(l) => prj.check_settings.levels.insert(code, l),
        None => prj.check_settings.levels.remove(&code),
    };
    show(prj, a)
}

fn accept(prj: &mut Project, a: &Args) -> Result<Value, String> {
    let id = a.req("fingerprint").trim().to_string();
    // code:hash:item, as `check` shows it
    let parts: Vec<&str> = id.split(':').collect();
    let valid = matches!(parts.as_slice(), [code, hash, item] if is_code(code) && hash.len() == 8 && hash.chars().all(|c| c.is_ascii_hexdigit()) && item.parse::<u64>().is_ok());
    if !valid {
        return Err(tr_fmt!("{} is not a message fingerprint as shown by check (code:hash:item)", id));
    }
    let accepted = &mut prj.check_settings.accepted;
    accepted.retain(|x| x.id != id);
    if a.value("accept").and_then(Value::as_bool).unwrap_or(true) {
        let s = |n: &str| a.str(n).unwrap_or_default().to_string();
        accepted.push(Accepted { id, message: s("message"), note: s("note"), user: s("user"), date: s("date") });
    }
    show(prj, a)
}

#[cfg(test)]
mod tests;
