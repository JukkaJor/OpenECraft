use super::*;

#[test]
fn lang_codes_round_trip() {
    for l in Lang::ALL {
        assert_eq!(Lang::from_code(l.code()), Some(l));
    }
    assert_eq!(Lang::from_code("de_DE.UTF-8"), Some(Lang::De));
    assert_eq!(Lang::from_code("fr-CA"), Some(Lang::Fr));
    assert_eq!(Lang::from_code("ja"), None);
}

#[test]
fn translates_and_falls_back_to_english() {
    assert_eq!(translate("Cancel", Lang::En), "Cancel");
    assert_eq!(translate("Cancel", Lang::De), "Abbrechen");
    assert_eq!(translate("Cancel", Lang::Fr), "Annuler");
    assert_eq!(translate("Cancel", Lang::Es), "Cancelar");
    assert_eq!(translate("no such text", Lang::De), "no such text");
}

#[test]
fn placeholders_are_filled_in_order_or_by_index() {
    assert_eq!(fill("Saved {} as {}", &[&"a", &3]), "Saved a as 3");
    assert_eq!(fill("{1} von {0}", &[&"x", &"y"]), "y von x");
    assert_eq!(fill("keeps {unknown} and {}", &[&1]), "keeps {unknown} and 1");
}

#[test]
fn catalog_is_complete_and_consistent() {
    let mut seen = std::collections::HashSet::new();
    for (en, de, fr, es) in catalog::all() {
        assert!(seen.insert(en), "duplicate catalog entry: {en}");
        let count = |s: &str| s.matches("{}").count() + s.matches("{0}").count() + s.matches("{1}").count() + s.matches("{2}").count();
        for (name, t) in [("German", de), ("French", fr), ("Spanish", es)] {
            assert!(!t.trim().is_empty(), "{name} translation missing: {en}");
            assert_eq!(count(en), count(t), "{name} placeholders differ: {en}");
            assert_eq!(en.starts_with(' '), t.starts_with(' '), "{name} leading space differs: {en}");
            assert_eq!(en.ends_with(' '), t.ends_with(' '), "{name} trailing space differs: {en}");
        }
    }
}

#[test]
fn catalog_lines_are_well_formed() {
    let bad = catalog::parse().filter(|e| e.is_none()).count();
    assert_eq!(bad, 0, "{bad} malformed catalog lines (need exactly 4 tab separated columns)");
}

/// Resolves the escapes of a Rust string literal body.
fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// The string literal that starts right after `marker` in `src` (raw body, no unescaping).
fn literal_after(src: &str, from: usize) -> Option<(&str, usize)> {
    let rest = &src[from..];
    let start = rest.find('"')?;
    let mut end = start + 1;
    let bytes = rest.as_bytes();
    while end < bytes.len() && bytes[end] != b'"' {
        end += if bytes[end] == b'\\' { 2 } else { 1 };
    }
    Some((&rest[start + 1..end], from + end + 1))
}

/// All texts passed to `tr("…")`, `tr_fmt!("…"`, `tr_const!("…")` and the names of the
/// built-in symbols in `symbols.rs`.
fn texts_in_source(src: &str, is_symbols: bool) -> Vec<String> {
    let mut out = Vec::new();
    for marker in ["tr(", "tr_fmt!(", "tr_const!("] {
        let mut at = 0;
        while let Some(i) = src[at..].find(marker) {
            let pos = at + i;
            at = pos + marker.len();
            let prev = src[..pos].chars().last();
            if prev.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '!') {
                continue;
            }
            // only a literal directly after the parenthesis (tr(some_variable) is skipped)
            if !src[at..].trim_start().starts_with('"') {
                continue;
            }
            if let Some((text, _)) = literal_after(src, at) {
                out.push(unescape(text));
            }
        }
    }
    if is_symbols {
        // def("key", "Name", …) and the potential table ("pot_x", "Name", "label")
        for marker in ["def(\"", "(\"pot"] {
            let mut at = 0;
            while let Some(i) = src[at..].find(marker) {
                let pos = at + i + marker.find('"').unwrap();
                at = pos + 1;
                if let Some((_, after_key)) = literal_after(src, pos) {
                    if let Some((name, _)) = literal_after(src, after_key) {
                        out.push(unescape(name));
                    }
                }
            }
        }
    }
    out
}

fn source_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            source_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") && path.file_name().is_some_and(|n| n != "tests.rs") {
            out.push(path);
        }
    }
}

/// Every text the applications translate must be in the catalog in all languages.
#[test]
fn all_texts_used_in_the_sources_are_in_the_catalog() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let known: std::collections::HashSet<&str> = catalog::all().map(|e| e.0).collect();
    // All workspace members (`[workspace] members` of the root manifest).
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let members = manifest.split("members = [").nth(1).and_then(|r| r.split(']').next()).expect("workspace members");
    let mut files = Vec::new();
    for member in members.split(',').map(|m| m.trim().trim_matches('"')).filter(|m| !m.is_empty()) {
        source_files(&root.join(member).join("src"), &mut files);
    }
    let mut missing = Vec::new();
    let mut checked = 0;
    for file in files {
        let src = std::fs::read_to_string(&file).unwrap();
        let is_symbols = file.ends_with("symbols.rs");
        for text in texts_in_source(&src, is_symbols) {
            checked += 1;
            if !known.contains(text.as_str()) {
                missing.push(format!("{} -> {text:?}", file.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    assert!(checked > 500, "source scan found only {checked} texts – scanner broken?");
    assert!(missing.is_empty(), "texts without catalog entry (add them to catalog/extra.tsv):\n{}", missing.join("\n"));
}
