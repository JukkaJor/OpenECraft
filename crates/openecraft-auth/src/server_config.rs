//! Configuration file of a server (`--config`, TOML): the settings of its command line
//! options, keyed by the option names (`admin_user` for `--admin-user`). Options on the
//! command line or in the environment win over the file; relative paths in the file are
//! relative to the file. Unknown keys are refused, so that a misspelt setting does not go
//! unnoticed.

use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

/// Reads the configuration file of a server.
pub fn load<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The settings of a configuration file's text.
pub fn parse<T: DeserializeOwned>(text: &str) -> Result<T, String> {
    toml::from_str(text).map_err(|e| e.to_string())
}

/// A path of the configuration file `config`, relative to the folder of the file.
pub fn relative_to(config: &Path, path: PathBuf) -> PathBuf {
    match config.parent() {
        Some(dir) if path.is_relative() => dir.join(path),
        _ => path,
    }
}

#[cfg(test)]
mod tests;
