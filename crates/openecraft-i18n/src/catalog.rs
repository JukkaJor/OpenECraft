//! The translation catalog: tab separated `English<TAB>Deutsch<TAB>Français<TAB>Español`
//! lines in `catalog/*.tsv`. The English text is the lookup key and must be identical to
//! the literal passed to `tr` / `tr_fmt!` / `tr_const!` (`\n` and `\"` are written as in
//! Rust source). Lines starting with `#` are comments.

pub type Entry = (&'static str, &'static str, &'static str, &'static str);

const FILES: [&str; 2] = [include_str!("catalog/translations.tsv"), include_str!("catalog/extra.tsv")];

/// All catalog entries; a malformed line yields `None` (reported by the tests).
pub fn parse() -> impl Iterator<Item = Option<Entry>> {
    FILES
        .iter()
        .flat_map(|f| f.lines())
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|line| {
            let mut cols = line.split('\t');
            let entry = (unescape(cols.next()?), unescape(cols.next()?), unescape(cols.next()?), unescape(cols.next()?));
            cols.next().is_none().then_some(entry)
        })
}

pub fn all() -> impl Iterator<Item = Entry> {
    parse().flatten()
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
