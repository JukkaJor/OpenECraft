//! Checks for the project conventions described in `AGENTS.md` (test layout), and the
//! list of source directories the source scans (conventions, glyphs) work on.

use std::path::PathBuf;

/// Workspace root (the application crate is the root package).
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Source directories of all workspace members (from `[workspace] members` in Cargo.toml).
pub fn source_dirs() -> Vec<PathBuf> {
    let manifest = std::fs::read_to_string(root().join("Cargo.toml")).unwrap();
    let members = manifest.split("members = [").nth(1).and_then(|r| r.split(']').next()).expect("workspace members");
    let dirs: Vec<PathBuf> = members
        .split(',')
        .map(|m| m.trim().trim_matches('"'))
        .filter(|m| !m.is_empty())
        .map(|m| if m == "." { root().join("application") } else { root().join(m).join("src") })
        .collect();
    assert!(dirs.len() >= 2, "workspace members not found");
    dirs
}

use std::collections::{BTreeMap, BTreeSet};

/// Source directory of `openecraft-core`.
pub fn core_src() -> PathBuf {
    root().join("crates/openecraft-core/src")
}

/// Rust sources of a module of `openecraft-core`: its file and the files of its
/// directory (without the test files).
fn module_sources(src: &std::path::Path, module: &str) -> Vec<PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") && path.file_name().is_some_and(|n| n != "tests.rs") {
                out.push(path);
            }
        }
    }
    let mut out = vec![src.join(format!("{module}.rs"))];
    let dir = src.join(module);
    if dir.is_dir() {
        walk(&dir, &mut out);
    }
    out
}

/// First path segments after `crate::` in a source text: `openecraft_core::geom::P` → `geom`,
/// `use crate::{model, netlist::Netlist}` → `model`, `netlist`.
pub fn crate_references(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    // Comments (also doc links) are not dependencies.
    let code: String = text.lines().filter(|l| !l.trim_start().starts_with("//")).map(|l| format!("{l}\n")).collect();
    let mut rest = code.as_str();
    while let Some(i) = rest.find("crate::") {
        rest = &rest[i + "crate::".len()..];
        if let Some(list) = rest.strip_prefix('{') {
            let end = list.find('}').unwrap_or(list.len());
            for item in list[..end].split(',') {
                let first = item.trim().split("::").next().unwrap_or("").trim();
                if !first.is_empty() && first != "self" && first != "*" {
                    out.insert(first.to_string());
                }
            }
        } else {
            let name: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                out.insert(name);
            }
        }
    }
    out
}

/// The modules of `openecraft-core` (declared in its `lib.rs`) with the modules each
/// of them refers to through `crate::…` paths.
pub fn core_module_dependencies() -> BTreeMap<String, BTreeSet<String>> {
    let src = core_src();
    let lib = std::fs::read_to_string(src.join("lib.rs")).unwrap();
    let modules: Vec<String> = lib
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub mod ").or_else(|| l.trim().strip_prefix("mod ")))
        .map(|m| m.trim_end_matches(';').to_string())
        .collect();
    assert!(modules.len() > 20, "core modules not found in lib.rs");
    let mut out = BTreeMap::new();
    for module in &modules {
        let mut refs = BTreeSet::new();
        for file in module_sources(&src, module) {
            let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            refs.extend(crate_references(&text).into_iter().filter(|r| modules.contains(r) && r != module));
        }
        out.insert(module.clone(), refs);
    }
    out
}

#[cfg(test)]
mod tests;
