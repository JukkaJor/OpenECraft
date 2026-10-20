use super::*;
use egui::epaint::text::{FontDefinitions, Fonts, TextOptions};
use egui::{FontFamily, FontId};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// All `.rs` files below `dir`, except test files (`tests.rs`), whose strings are
/// test data and never shown in the UI.
fn source_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(source_files(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
            && path.file_name().and_then(|n| n.to_str()) != Some("tests.rs")
        {
            out.push(path);
        }
    }
    out
}

#[test]
fn all_ui_characters_exist_in_bundled_fonts() {
    let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::default());
    let font = FontId::new(14.0, FontFamily::Proportional);
    // The application and the parts library (catalog texts are shown in the UI).
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut missing: BTreeMap<char, Vec<String>> = BTreeMap::new();
    let mut checked = 0;
    let files = [root.join("src"), root.join("crates/openecraft-parts/src")].iter().flat_map(|d| source_files(d)).collect::<Vec<_>>();
    for path in files {
        let src = std::fs::read_to_string(&path).unwrap();
        let name = path.strip_prefix(root).unwrap_or(&path).display().to_string();
        for (line, c) in literal_chars(&src) {
            if c.is_ascii() || c.is_whitespace() {
                continue;
            }
            checked += 1;
            if !fonts.has_glyph(&font, c) {
                missing.entry(c).or_default().push(format!("{name}:{line}"));
            }
        }
    }
    assert!(checked > 0, "no non-ASCII characters found – scanner broken?");
    let report: Vec<String> = missing
        .iter()
        .map(|(c, places)| format!("'{c}' (U+{:04X}) used at {}", *c as u32, places.join(", ")))
        .collect();
    assert!(
        missing.is_empty(),
        "characters without glyph in the bundled fonts (rendered as empty boxes on desktop):\n{}",
        report.join("\n")
    );
}

#[test]
fn scanner_finds_literals_and_skips_comments() {
    let src = "// ⬉ comment\nlet a = \"x→y\"; /* ✖ */ let b = '✔'; fn f<'a>(s: &'a str) {} let r = r#\"ϑ\"#;";
    let found: String = literal_chars(src).into_iter().map(|(_, c)| c).collect();
    assert_eq!(found, "x→y✔ϑ");
}
