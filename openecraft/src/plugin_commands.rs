//! The commands of the plugins in the editor: the *Plugin commands* pane lists them by
//! plugin with a search; the Plugins tab of the ribbon shows only the favorites, the
//! recently used ones and a menu per external plugin. No UI code:
//! `app/plugins_ui.rs` and `app/ribbon_ui.rs` draw them.

use serde::{Deserialize, Serialize};

/// Storage key of the favorites and recently used commands.
pub const STORAGE_KEY: &str = "plugin_commands";

/// At most this many recently used commands are kept.
pub const RECENT: usize = 6;

/// Favorites and recently used commands by name (command names are unique).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginCommands {
    pub favorites: Vec<String>,
    /// Newest first.
    pub recent: Vec<String>,
    #[serde(skip)]
    pub query: String,
}

impl PluginCommands {
    /// A command was run: it moves to the front of the recent list.
    pub fn used(&mut self, name: &str) {
        self.recent.retain(|r| r != name);
        self.recent.insert(0, name.to_string());
        self.recent.truncate(RECENT);
    }

    pub fn is_favorite(&self, name: &str) -> bool {
        self.favorites.iter().any(|f| f == name)
    }

    pub fn toggle_favorite(&mut self, name: &str) {
        if self.is_favorite(name) {
            self.favorites.retain(|f| f != name);
        } else {
            self.favorites.push(name.to_string());
        }
    }

    /// Whether the texts of a command match the search (all words, `*` / `?` placeholders).
    pub fn matches(&self, texts: &[&str]) -> bool {
        let all = texts.join(" ");
        self.query.split_whitespace().all(|w| crate::commands::wildcard_match(w, &all))
    }

    /// The recently used commands that are no favorites.
    pub fn recent_only(&self) -> Vec<String> {
        self.recent.iter().filter(|r| !self.is_favorite(r)).cloned().collect()
    }
}

/// The items of `all` named in `names`, in the order of `names` (unknown names skipped,
/// for example of a plugin switched off).
pub fn pick<'a, T>(names: &[String], all: &'a [T], name: impl Fn(&T) -> &str) -> Vec<&'a T> {
    names.iter().filter_map(|n| all.iter().find(|c| name(c) == n)).collect()
}

#[cfg(test)]
mod tests;
