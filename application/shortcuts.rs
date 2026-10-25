//! Keyboard shortcuts the user can change (View › Keyboard shortcuts…): the actions,
//! their default keys, the user's own keys (stored by the app), conflicts and the text
//! shown in menus and the help.

use egui::{InputState, Key, Modifiers};
use openecraft_i18n::tr;
use std::collections::BTreeMap;

/// Storage key of the user's shortcuts.
pub const STORAGE_KEY: &str = "shortcuts";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    Open,
    Save,
    Undo,
    Redo,
    Find,
    SelectAll,
    PreviousPage,
    NextPage,
    FitPage,
    WireTool,
    TextTool,
    Rotate,
    Mirror,
    WireCorner,
    Autoroute,
    FinishPolyline,
}

impl Action {
    pub const ALL: [Action; 16] = [
        Action::Open,
        Action::Save,
        Action::Undo,
        Action::Redo,
        Action::Find,
        Action::SelectAll,
        Action::PreviousPage,
        Action::NextPage,
        Action::FitPage,
        Action::WireTool,
        Action::TextTool,
        Action::Rotate,
        Action::Mirror,
        Action::WireCorner,
        Action::Autoroute,
        Action::FinishPolyline,
    ];

    /// Id in the stored settings.
    pub fn id(self) -> &'static str {
        match self {
            Action::Open => "open",
            Action::Save => "save",
            Action::Undo => "undo",
            Action::Redo => "redo",
            Action::Find => "find",
            Action::SelectAll => "select_all",
            Action::PreviousPage => "previous_page",
            Action::NextPage => "next_page",
            Action::FitPage => "fit_page",
            Action::WireTool => "wire_tool",
            Action::TextTool => "text_tool",
            Action::Rotate => "rotate",
            Action::Mirror => "mirror",
            Action::WireCorner => "wire_corner",
            Action::Autoroute => "autoroute",
            Action::FinishPolyline => "finish_polyline",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Action::Open => tr("Open project"),
            Action::Save => tr("Save project"),
            Action::Undo => tr("Undo"),
            Action::Redo => tr("Redo"),
            Action::Find => tr("Find and replace"),
            Action::SelectAll => tr("Select all"),
            Action::PreviousPage => tr("Previous page"),
            Action::NextPage => tr("Next page"),
            Action::FitPage => tr("Fit page"),
            Action::WireTool => tr("Wire tool"),
            Action::TextTool => tr("Text tool"),
            Action::Rotate => tr("Rotate (selection or symbol being placed)"),
            Action::Mirror => tr("Mirror (selection or symbol being placed)"),
            Action::WireCorner => tr("Toggle corner direction while drawing wires"),
            Action::Autoroute => tr("Toggle routing around symbols (wire tool)"),
            Action::FinishPolyline => tr("Finish polyline or polygon"),
        }
    }

    pub fn defaults(self) -> Vec<Shortcut> {
        let cmd = |key| Shortcut { command: true, shift: false, alt: false, key };
        let plain = |key| Shortcut { command: false, shift: false, alt: false, key };
        match self {
            Action::Open => vec![cmd(Key::O)],
            Action::Save => vec![cmd(Key::S)],
            Action::Undo => vec![cmd(Key::Z)],
            Action::Redo => vec![cmd(Key::Y), Shortcut { shift: true, ..cmd(Key::Z) }],
            Action::Find => vec![cmd(Key::F)],
            Action::SelectAll => vec![cmd(Key::A)],
            Action::PreviousPage => vec![plain(Key::PageUp)],
            Action::NextPage => vec![plain(Key::PageDown)],
            Action::FitPage => vec![plain(Key::F), plain(Key::Home)],
            Action::WireTool => vec![plain(Key::W)],
            Action::TextTool => vec![plain(Key::T)],
            Action::Rotate => vec![plain(Key::R)],
            Action::Mirror => vec![plain(Key::M)],
            Action::WireCorner => vec![plain(Key::Space)],
            Action::Autoroute => vec![plain(Key::A)],
            Action::FinishPolyline => vec![plain(Key::Enter)],
        }
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

/// The user's shortcuts: changed actions with their keys (none = no shortcut), the
/// others keep their defaults.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shortcuts {
    custom: BTreeMap<Action, Vec<Shortcut>>,
}

impl Shortcuts {
    pub fn keys(&self, a: Action) -> Vec<Shortcut> {
        self.custom.get(&a).cloned().unwrap_or_else(|| a.defaults())
    }

    pub fn set(&mut self, a: Action, keys: Vec<Shortcut>) {
        if keys == a.defaults() {
            self.custom.remove(&a);
        } else {
            self.custom.insert(a, keys);
        }
    }

    pub fn reset(&mut self, a: Action) {
        self.custom.remove(&a);
    }

    pub fn reset_all(&mut self) {
        self.custom.clear();
    }

    pub fn is_default(&self, a: Action) -> bool {
        !self.custom.contains_key(&a)
    }

    /// The keys of an action for menus and the help (`Ctrl+Y / Ctrl+Shift+Z`).
    pub fn text(&self, a: Action) -> String {
        self.keys(a).iter().map(Shortcut::text).collect::<Vec<_>>().join(" / ")
    }

    pub fn pressed(&self, i: &InputState, a: Action) -> bool {
        self.keys(a).iter().any(|s| s.pressed(i))
    }

    /// Like [`Shortcuts::pressed`], and takes the key press so that widgets do not see it.
    pub fn consume(&self, i: &mut InputState, a: Action) -> bool {
        match self.keys(a).into_iter().find(|s| s.pressed(i)) {
            Some(s) => {
                i.consume_key(s.modifiers(), s.key);
                true
            }
            None => false,
        }
    }

    /// Shortcuts used by more than one action.
    pub fn conflicts(&self) -> Vec<(Shortcut, Vec<Action>)> {
        let mut by_key: Vec<(Shortcut, Vec<Action>)> = Vec::new();
        for a in Action::ALL {
            for s in self.keys(a) {
                match by_key.iter_mut().find(|(k, _)| *k == s) {
                    Some((_, v)) => v.push(a),
                    None => by_key.push((s, vec![a])),
                }
            }
        }
        by_key.retain(|(_, v)| v.len() > 1);
        by_key
    }

    /// For the app's storage: action id → key texts.
    pub fn to_storage(&self) -> BTreeMap<String, Vec<String>> {
        self.custom.iter().map(|(a, v)| (a.id().to_string(), v.iter().map(Shortcut::text).collect())).collect()
    }

    /// Unknown actions and keys are left out.
    pub fn from_storage(stored: &BTreeMap<String, Vec<String>>) -> Shortcuts {
        let mut s = Shortcuts::default();
        for (id, keys) in stored {
            if let Some(a) = Action::ALL.into_iter().find(|a| a.id() == id) {
                s.custom.insert(a, keys.iter().filter_map(|k| Shortcut::parse(k)).collect());
            }
        }
        s
    }
}

#[cfg(test)]
mod tests;
