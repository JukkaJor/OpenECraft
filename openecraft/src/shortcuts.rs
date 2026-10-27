//! Keyboard shortcuts the user can change (Settings › Keyboard shortcuts): every command
//! of `crate::commands` and the few keys only the page editor reads (actions), their
//! default keys, the user's own keys (stored by the app), conflicts and the text shown
//! in the ribbon, the menus and the help. Bindings are named by the command id, so the
//! keys stored by earlier versions (`open`, `rotate`, `wire_tool`, …) keep working.

use crate::commands::{self, Cmd};
use egui::{InputState, Key, Modifiers};
use openecraft_i18n::tr_const;
use std::collections::BTreeMap;

/// Storage key of the user's shortcuts.
pub const STORAGE_KEY: &str = "shortcuts";

/// Keys that only the page editor reads while a tool is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    WireCorner,
    Autoroute,
    FinishPolyline,
}

impl Action {
    pub const ALL: [Action; 3] = [Action::WireCorner, Action::Autoroute, Action::FinishPolyline];

    /// Id in the stored settings.
    pub fn id(self) -> &'static str {
        match self {
            Action::WireCorner => "wire_corner",
            Action::Autoroute => "autoroute",
            Action::FinishPolyline => "finish_polyline",
        }
    }

    /// English title, translated where shown.
    pub fn title(self) -> &'static str {
        match self {
            Action::WireCorner => tr_const!("Toggle corner direction while drawing wires"),
            Action::Autoroute => tr_const!("Toggle routing around symbols (wire tool)"),
            Action::FinishPolyline => tr_const!("Finish polyline or polygon"),
        }
    }

    pub fn defaults(self) -> Vec<Shortcut> {
        let plain = |key| Shortcut { command: false, shift: false, alt: false, key };
        match self {
            Action::WireCorner => vec![plain(Key::Space)],
            Action::Autoroute => vec![plain(Key::A)],
            Action::FinishPolyline => vec![plain(Key::Enter)],
        }
    }
}

/// Every binding: the commands, then the actions of the page editor.
pub fn binding_ids() -> Vec<&'static str> {
    commands::ALL.iter().map(|d| d.id).chain(Action::ALL.iter().map(|a| a.id())).collect()
}

/// English title of a binding.
pub fn title(id: &str) -> &'static str {
    commands::by_id(id).map(|d| d.title).or_else(|| Action::ALL.into_iter().find(|a| a.id() == id).map(Action::title)).unwrap_or("")
}

/// The default keys of a binding.
pub fn defaults(id: &str) -> Vec<Shortcut> {
    match commands::by_id(id) {
        Some(d) => d.default_keys(),
        None => Action::ALL.into_iter().find(|a| a.id() == id).map(Action::defaults).unwrap_or_default(),
    }
}

/// A key with modifiers; `command` is Ctrl (Cmd on macOS).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub command: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: Key,
}

impl Shortcut {
    /// `Ctrl+Shift+Z`, `F`, `PageDown`.
    pub fn parse(s: &str) -> Option<Shortcut> {
        let mut sc = Shortcut { command: false, shift: false, alt: false, key: Key::A };
        let parts: Vec<&str> = s.split('+').map(str::trim).collect();
        let (key, mods) = parts.split_last()?;
        for m in mods {
            match m.to_ascii_lowercase().as_str() {
                "ctrl" | "cmd" | "command" => sc.command = true,
                "shift" => sc.shift = true,
                "alt" | "option" => sc.alt = true,
                _ => return None,
            }
        }
        sc.key = Key::from_name(key)?;
        Some(sc)
    }

    pub fn text(&self) -> String {
        let mut s = String::new();
        for (on, name) in [(self.command, "Ctrl+"), (self.shift, "Shift+"), (self.alt, "Alt+")] {
            if on {
                s.push_str(name);
            }
        }
        s.push_str(self.key.name());
        s
    }

    pub fn modifiers(&self) -> Modifiers {
        let mut m = Modifiers::NONE;
        if self.command {
            m |= Modifiers::COMMAND;
        }
        if self.shift {
            m |= Modifiers::SHIFT;
        }
        if self.alt {
            m |= Modifiers::ALT;
        }
        m
    }

    /// The modifiers held are exactly the ones of the shortcut.
    pub fn held(&self, m: Modifiers) -> bool {
        m.command == self.command && m.shift == self.shift && m.alt == self.alt
    }

    pub fn pressed(&self, i: &InputState) -> bool {
        i.key_pressed(self.key) && self.held(i.modifiers)
    }
}

/// The user's shortcuts: changed bindings with their keys (none = no shortcut), the
/// others keep their defaults.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shortcuts {
    custom: BTreeMap<String, Vec<Shortcut>>,
}

impl Shortcuts {
    pub fn keys(&self, id: &str) -> Vec<Shortcut> {
        self.custom.get(id).cloned().unwrap_or_else(|| defaults(id))
    }

    pub fn set(&mut self, id: &str, keys: Vec<Shortcut>) {
        if keys == defaults(id) {
            self.custom.remove(id);
        } else {
            self.custom.insert(id.to_string(), keys);
        }
    }

    pub fn reset(&mut self, id: &str) {
        self.custom.remove(id);
    }

    pub fn reset_all(&mut self) {
        self.custom.clear();
    }

    pub fn is_default(&self, id: &str) -> bool {
        !self.custom.contains_key(id)
    }

    /// The keys of a binding for menus and the help (`Ctrl+Y / Ctrl+Shift+Z`).
    pub fn text(&self, id: &str) -> String {
        self.keys(id).iter().map(Shortcut::text).collect::<Vec<_>>().join(" / ")
    }

    /// The keys of a command.
    pub fn cmd_text(&self, cmd: Cmd) -> String {
        self.text(commands::def(cmd).id)
    }

    pub fn pressed(&self, i: &InputState, id: &str) -> bool {
        self.keys(id).iter().any(|s| s.pressed(i))
    }

    pub fn cmd_pressed(&self, i: &InputState, cmd: Cmd) -> bool {
        self.pressed(i, commands::def(cmd).id)
    }

    pub fn action_pressed(&self, i: &InputState, a: Action) -> bool {
        self.pressed(i, a.id())
    }

    /// Like [`Shortcuts::pressed`], and takes the key press so that widgets do not see it.
    pub fn consume(&self, i: &mut InputState, id: &str) -> bool {
        match self.keys(id).into_iter().find(|s| s.pressed(i)) {
            Some(s) => {
                i.consume_key(s.modifiers(), s.key);
                true
            }
            None => false,
        }
    }

    /// Shortcuts used by more than one binding.
    pub fn conflicts(&self) -> Vec<(Shortcut, Vec<&'static str>)> {
        let mut by_key: Vec<(Shortcut, Vec<&'static str>)> = Vec::new();
        for id in binding_ids() {
            for s in self.keys(id) {
                match by_key.iter_mut().find(|(k, _)| *k == s) {
                    Some((_, v)) => v.push(id),
                    None => by_key.push((s, vec![id])),
                }
            }
        }
        by_key.retain(|(_, v)| v.len() > 1);
        by_key
    }

    /// For the app's storage: binding id → key texts.
    pub fn to_storage(&self) -> BTreeMap<String, Vec<String>> {
        self.custom.iter().map(|(id, v)| (id.clone(), v.iter().map(Shortcut::text).collect())).collect()
    }

    /// Unknown bindings and keys are left out.
    pub fn from_storage(stored: &BTreeMap<String, Vec<String>>) -> Shortcuts {
        let mut s = Shortcuts::default();
        let ids = binding_ids();
        for (id, keys) in stored {
            if ids.contains(&id.as_str()) {
                s.custom.insert(id.clone(), keys.iter().filter_map(|k| Shortcut::parse(k)).collect());
            }
        }
        s
    }
}

#[cfg(test)]
mod tests;
