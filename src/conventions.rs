//! Checks for the project conventions described in `AGENTS.md` (test layout), and the
//! list of source directories the source scans (conventions, glyphs) work on.

use std::path::PathBuf;

/// Workspace root (the application crate is the root package).
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `src` directories of all workspace members (from `[workspace] members` in Cargo.toml).
pub fn source_dirs() -> Vec<PathBuf> {
    let manifest = std::fs::read_to_string(root().join("Cargo.toml")).unwrap();
    let members = manifest.split("members = [").nth(1).and_then(|r| r.split(']').next()).expect("workspace members");
    let dirs: Vec<PathBuf> = members
        .split(',')
        .map(|m| m.trim().trim_matches('"'))
        .filter(|m| !m.is_empty())
        .map(|m| root().join(m).join("src"))
        .collect();
    assert!(dirs.len() >= 2, "workspace members not found");
    dirs
}

#[cfg(test)]
mod tests;
