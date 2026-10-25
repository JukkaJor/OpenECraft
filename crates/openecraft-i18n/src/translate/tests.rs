use super::*;

#[test]
fn lang_codes_round_trip() {
    for l in Lang::ALL {
        assert_eq!(Lang::from_code(l.code()), Some(l));
    }
    assert_eq!(Lang::from_code("de_DE.UTF-8"), Some(Lang::De));
    assert_eq!(Lang::from_code("fr-CA"), Some(Lang::Fr));
    assert_eq!(Lang::from_code("pt-BR"), Some(Lang::Pt));
    assert_eq!(Lang::from_code("zh_CN.UTF-8"), Some(Lang::Zh));
    assert_eq!(Lang::from_code("tlh"), None);
}

#[test]
fn every_further_language_has_a_catalog() {
    for l in Lang::ALL.into_iter().filter(|l| !l.is_required()) {
        assert!(!catalog_translations(l).is_empty(), "no translations for {}", l.code());
    }
    assert!(catalog::further("de").next().is_none(), "the required languages are in the main catalog");
}

#[test]
fn further_languages_translate_and_fall_back_to_english() {
    assert_eq!(translate("no such text", Lang::Ja), "no such text");
    for l in Lang::ALL.into_iter().filter(|l| !l.is_required()) {
        let (en, t) = catalog_translations(l)[0];
        assert_eq!(translate(en, l), t);
    }
}

/// `{}` / `{0}` … placeholders and named template fields (`{page}`) of a text.
fn placeholders(s: &str) -> (usize, Vec<&str>) {
    let mut positional = 0;
    let mut named = Vec::new();
    let mut rest = s;
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}') else { break };
        let key = &rest[start + 1..start + end];
        if key.is_empty() || key.chars().all(|c| c.is_ascii_digit()) {
            positional += 1;
        } else if !key.contains(char::is_whitespace) {
            named.push(key);
        }
        rest = &rest[start + end + 1..];
    }
    named.sort_unstable();
    (positional, named)
}

#[test]
fn further_catalogs_are_consistent() {
    let known: std::collections::HashSet<&str> = catalog::all().map(|e| e.0).collect();
    for l in Lang::ALL.into_iter().filter(|l| !l.is_required()) {
        let code = l.code();
        let bad = catalog::further(code).filter(|e| e.is_none()).count();
        assert_eq!(bad, 0, "{bad} malformed lines in catalog/lang/{code}.tsv (need English<TAB>translation)");
        let mut seen = std::collections::HashSet::new();
        for (en, t) in catalog_translations(l) {
            assert!(seen.insert(en), "{code}: duplicate entry: {en}");
            assert!(known.contains(en), "{code}: not in the main catalog: {en}");
            assert!(!t.trim().is_empty(), "{code}: empty translation: {en}");
            assert_eq!(placeholders(en), placeholders(t), "{code}: placeholders differ: {en}");
            assert_eq!(en.starts_with(' '), t.starts_with(' '), "{code}: leading space differs: {en}");
            assert_eq!(en.ends_with(' '), t.ends_with(' '), "{code}: trailing space differs: {en}");
        }
    }
}

#[test]
fn placeholders_are_counted_and_named() {
    assert_eq!(placeholders("{} of {1} on {page}"), (2, vec!["page"]));
    assert_eq!(placeholders("{name, description}"), (0, vec![]));
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
fn output_language_is_set_for_this_thread_only() {
    let ui = lang();
    let inner = with_lang(Lang::De, || {
        let other = std::thread::spawn(lang).join().unwrap();
        (tr("Open"), with_lang(Lang::Fr, lang), lang(), other)
    });
    assert_eq!(inner.0, translate("Open", Lang::De));
    assert_eq!((inner.1, inner.2), (Lang::Fr, Lang::De));
    assert_eq!(inner.3, ui);
    assert_eq!(lang(), ui);
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

/// All texts passed to `tr("…")`, `tr_fmt!("…"`, `tr_const!("…")`, the templates of
/// `issue!(severity, target, "…")` and the names of the built-in symbols in `symbols.rs`.
fn texts_in_source(src: &str, is_symbols: bool) -> Vec<String> {
    let mut out = Vec::new();
    // the template of an issue follows the severity and the target
    let mut at = 0;
    while let Some(i) = src[at..].find("issue!(") {
        at += i + "issue!(".len();
        if let Some((text, _)) = literal_after(src, at) {
            out.push(unescape(text));
        }
    }
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
        let dir = if member == "." { root.join("application") } else { root.join(member).join("src") };
        source_files(&dir, &mut files);
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
