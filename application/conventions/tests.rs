use super::*;
use std::path::Path;

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out
}

fn all_rust_files() -> Vec<PathBuf> {
    source_dirs().iter().flat_map(|d| rust_files(d)).collect()
}

fn display(path: &Path) -> String {
    path.strip_prefix(root()).unwrap_or(path).display().to_string().replace('\\', "/")
}

fn is_test_file(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()) == Some("tests.rs")
}

/// Unit tests live in `<crate>/src/<module>/tests.rs`, never inline in the module file.
#[test]
fn unit_tests_are_in_separate_files() {
    let mut violations = Vec::new();
    for path in all_rust_files().into_iter().filter(|p| !is_test_file(p)) {
        let text = std::fs::read_to_string(&path).unwrap();
        let name = display(&path);
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("//") {
                continue;
            }
            if t.starts_with("#[test]") || t.starts_with("mod tests {") || t.starts_with("mod tests{") {
                violations.push(format!("{name}:{}: inline test code – move it to {}", i + 1, tests_path(&name)));
            }
            if t == "mod tests;" && (i == 0 || lines[i - 1].trim() != "#[cfg(test)]") {
                violations.push(format!("{name}:{}: `mod tests;` must be preceded by `#[cfg(test)]`", i + 1));
            }
        }
    }
    assert!(violations.is_empty(), "test layout violations (see AGENTS.md):\n{}", violations.join("\n"));
}

/// Every `tests.rs` belongs to a module file next to its directory (`foo.rs` + `foo/tests.rs`).
#[test]
fn test_files_belong_to_a_module() {
    let mut orphans = Vec::new();
    for path in all_rust_files().into_iter().filter(|p| is_test_file(p)) {
        let dir = path.parent().unwrap();
        let module_file = dir.with_extension("rs");
        let declared = std::fs::read_to_string(&module_file)
            .map(|t| t.lines().any(|l| l.trim() == "mod tests;"))
            .unwrap_or(false);
        if !declared {
            orphans.push(format!(
                "{} is not declared via `#[cfg(test)] mod tests;` in {}",
                display(&path),
                display(&module_file)
            ));
        }
    }
    assert!(orphans.is_empty(), "{}", orphans.join("\n"));
}

fn tests_path(module_file: &str) -> String {
    format!("{}/tests.rs", module_file.trim_end_matches(".rs"))
}

#[test]
fn all_workspace_crates_are_checked() {
    let dirs = source_dirs();
    for member in ["application", "crates/openecraft-parts/src", "crates/openecraft-ui/src", "parts-server/src", "parts-manager/src"] {
        assert!(dirs.iter().any(|d| d.ends_with(member)), "{member} missing in {dirs:?}");
    }
}

/// UI crates (the others have no user-visible texts of their own).
fn ui_source_files() -> Vec<PathBuf> {
    ["application", "crates/openecraft-ui/src", "parts-manager/src"]
        .iter()
        .flat_map(|d| rust_files(&root().join(d)))
        .filter(|p| !is_test_file(p))
        .collect()
}

/// Texts shown to the user go through `tr` / `tr_fmt!` (see AGENTS.md, languages): a
/// widget call with a plain string literal is an untranslated text.
#[test]
fn ui_texts_are_translated() {
    let calls = [
        ".button(\"", ".small_button(\"", ".label(\"", ".heading(\"", ".weak(\"", ".strong(\"", ".link(\"", ".menu_button(\"",
        ".collapsing(\"", ".on_hover_text(\"", ".hint_text(\"", ".selectable_label(", ".selected_text(\"", ".hyperlink_to(\"",
        "Button::new(\"", "RichText::new(\"", "Window::new(\"",
    ];
    let mut violations = Vec::new();
    for path in ui_source_files() {
        let text = std::fs::read_to_string(&path).unwrap();
        for (i, line) in text.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("//") {
                continue;
            }
            for call in calls {
                let mut rest = t;
                while let Some(at) = rest.find(call) {
                    let after = &rest[at + call.len()..];
                    rest = after;
                    if !call.ends_with('"') {
                        continue;
                    }
                    let literal: String = after.chars().take_while(|c| *c != '"').collect();
                    let letters = literal.chars().filter(|c| c.is_alphabetic()).count();
                    if letters >= 3 {
                        violations.push(format!("{}:{}: {t}", display(&path), i + 1));
                    }
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "texts shown in the UI must be translated: wrap them in `tr(\"…\")` / `tr_fmt!(…)` and add them to crates/openecraft-i18n/src/catalog/extra.tsv (English, German, French, Spanish):\n{}",
        violations.join("\n")
    );
}

/// Dependency rules inside `openecraft-core` (see `docs/architecture.md`): a module may
/// refer only to the modules listed for it. Modules that are not listed are not
/// checked yet; the table grows as the crate is layered.
const CORE_RULES: &[(&str, &[&str])] = &[
    ("geom", &[]),
    ("xml", &[]),
    ("symbols", &["geom", "symlib"]),
    ("symlib", &["geom", "model", "symbols"]),
    ("history", &["model"]),
    ("translation", &["model"]),
    ("options", &["model"]),
    ("frame", &["geom", "model"]),
    ("parts", &["model", "symbols"]),
    ("routing", &["geom", "model", "symbols"]),
    ("devdesc", &["parts", "xml"]),
    ("testkit", &["geom", "model", "parts", "symbols"]),
];

#[test]
fn core_modules_respect_the_layer_rules() {
    let deps = core_module_dependencies();
    let mut violations = Vec::new();
    for (module, allowed) in CORE_RULES {
        let Some(found) = deps.get(*module) else {
            violations.push(format!("{module}: no such module in openecraft-core (update CORE_RULES)"));
            continue;
        };
        for dep in found {
            if !allowed.contains(&dep.as_str()) {
                violations.push(format!("{module} must not refer to crate::{dep}"));
            }
        }
    }
    assert!(violations.is_empty(), "layer rule violations in openecraft-core (see docs/architecture.md):
{}", violations.join("
"));
}

#[test]
fn crate_references_are_parsed_from_paths_and_lists() {
    let refs = crate_references("use crate::geom::P;
use crate::{model, netlist::Netlist, self};
let x = crate::panel::snap(1.0);");
    let v: Vec<&str> = refs.iter().map(String::as_str).collect();
    assert_eq!(v, ["geom", "model", "netlist", "panel"]);
}
