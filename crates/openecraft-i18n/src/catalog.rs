//! The translation catalog: tab separated `English<TAB>Deutsch<TAB>Français<TAB>Español`
//! lines in `catalog/*.tsv`. The English text is the lookup key and must be identical to
//! the literal passed to `tr` / `tr_fmt!` / `tr_const!` (`\n` and `\"` are written as in
//! Rust source). Lines starting with `#` are comments.
//!
//! The further languages have one file each, `catalog/lang/<code>.tsv`, with
//! `English<TAB>translation` lines; a text without a line there is shown in English.

pub type Entry = (&'static str, &'static str, &'static str, &'static str);

const FILES: [&str; 2] = [include_str!("catalog/translations.tsv"), include_str!("catalog/extra.tsv")];

/// The catalogs of the further languages by ISO code.
const FURTHER: [(&str, &str); 16] = [
    ("pt", include_str!("catalog/lang/pt.tsv")),
    ("it", include_str!("catalog/lang/it.tsv")),
    ("nl", include_str!("catalog/lang/nl.tsv")),
    ("pl", include_str!("catalog/lang/pl.tsv")),
    ("cs", include_str!("catalog/lang/cs.tsv")),
    ("ru", include_str!("catalog/lang/ru.tsv")),
    ("uk", include_str!("catalog/lang/uk.tsv")),
    ("tr", include_str!("catalog/lang/tr.tsv")),
    ("sv", include_str!("catalog/lang/sv.tsv")),
    ("da", include_str!("catalog/lang/da.tsv")),
    ("fi", include_str!("catalog/lang/fi.tsv")),
    ("hu", include_str!("catalog/lang/hu.tsv")),
    ("ro", include_str!("catalog/lang/ro.tsv")),
    ("zh", include_str!("catalog/lang/zh.tsv")),
    ("ja", include_str!("catalog/lang/ja.tsv")),
    ("ko", include_str!("catalog/lang/ko.tsv")),
];

fn lines(file: &'static str) -> impl Iterator<Item = &'static str> {
    file.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
}

/// All catalog entries; a malformed line yields `None` (reported by the tests).
pub fn parse() -> impl Iterator<Item = Option<Entry>> {
    FILES.iter().flat_map(|f| lines(f)).map(|line| {
        let mut cols = line.split('\t');
        let entry = (unescape(cols.next()?), unescape(cols.next()?), unescape(cols.next()?), unescape(cols.next()?));
        cols.next().is_none().then_some(entry)
    })
}

pub fn all() -> impl Iterator<Item = Entry> {
    parse().flatten()
}

/// `(English, translation)` of a further language (none for an unknown code); a
/// malformed line yields `None`.
pub fn further(code: &'static str) -> impl Iterator<Item = Option<(&'static str, &'static str)>> {
    FURTHER.iter().filter(move |(c, _)| *c == code).flat_map(|(_, f)| lines(f)).map(|line| {
        let mut cols = line.split('\t');
        let entry = (unescape(cols.next()?), unescape(cols.next()?));
        cols.next().is_none().then_some(entry)
    })
}

/// Resolves the Rust escapes used in the catalog (`\n`, `\t`, `\"`, `\\`).
fn unescape(s: &'static str) -> &'static str {
    if !s.contains('\\') {
        return s;
    }
    let mut out = String::with_capacity(s.len());
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
            None => out.push('\\'),
        }
    }
    Box::leak(out.into_boxed_str())
}
