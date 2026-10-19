//! Guards against characters that the bundled egui fonts cannot render.
//!
//! In the browser egui falls back to the browser's font rasterizer, but the native
//! builds only have the bundled fonts: a missing glyph shows up as an empty box
//! ("tofu"). This test scans every string and char literal in the sources and checks
//! that the default fonts contain all of its characters.

use egui::epaint::text::{FontDefinitions, Fonts, TextOptions};
use egui::{FontFamily, FontId};
use std::collections::BTreeMap;

/// Collects the characters of all string / char literals of a Rust source file
/// (comments are skipped), together with the line they appear on.
fn literal_chars(src: &str) -> Vec<(usize, char)> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut line = 1;
    let mut i = 0;
    let bump_line = |c: char, line: &mut usize| {
        if c == '\n' {
            *line += 1;
        }
    };
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    bump_line(chars[i], &mut line);
                    i += 1;
                }
            }
            continue;
        }
        // Raw strings: r"…", r#"…"#, br"…"
        if c == 'r' && (next == Some('"') || next == Some('#')) && (i == 0 || !chars[i - 1].is_alphanumeric() || chars[i - 1] == 'b') {
            let mut j = i + 1;
            let mut hashes = 0;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                j += 1;
                let closing: String = std::iter::once('"').chain(std::iter::repeat_n('#', hashes)).collect();
                let closing: Vec<char> = closing.chars().collect();
                while j < chars.len() && chars[j..].iter().take(closing.len()).ne(closing.iter()) {
                    out.push((line, chars[j]));
                    bump_line(chars[j], &mut line);
                    j += 1;
                }
                i = j + closing.len();
                continue;
            }
        }
        if c == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                out.push((line, chars[i]));
                bump_line(chars[i], &mut line);
                i += 1;
            }
            i += 1;
            continue;
        }
        if c == '\'' {
            // Char literal ('x', '\n', '\u{…}') or lifetime ('a).
            if next == Some('\\') {
                while i + 1 < chars.len() && chars[i + 1] != '\'' {
                    i += 1;
                }
                i += 2;
                continue;
            }
            if chars.get(i + 2) == Some(&'\'') {
                out.push((line, chars[i + 1]));
                i += 3;
                continue;
            }
        }
        bump_line(c, &mut line);
        i += 1;
    }
    out
}

#[test]
fn all_ui_characters_exist_in_bundled_fonts() {
    let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::default());
    let font = FontId::new(14.0, FontFamily::Proportional);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut missing: BTreeMap<char, Vec<String>> = BTreeMap::new();
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "glyphs.rs" {
            continue; // contains test data
        }
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

