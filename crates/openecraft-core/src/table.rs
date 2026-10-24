//! Result table of reports: headers, relative column widths, rows and an optional
//! jump target per row. Shared by the built-in reports and the reports of plugins.
//!
//! Headers are [`Text`]s: borrowed ones are English catalog keys and are translated
//! where the table is shown; owned ones (reports of external plugins) are shown as
//! they are. Forms refer to columns by header; a stored name matches the key or any of
//! its translations, so forms made in one language work in the others.

use crate::params::{text, Text};
use openecraft_i18n::{translate, Lang};
use std::borrow::Cow;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub headers: Vec<Text>,
    /// Relative column widths (one per header).
    pub widths: Vec<f32>,
    pub rows: Vec<Vec<String>>,
    /// Optional jump target (page index, item id) per row.
    pub targets: Vec<Option<(usize, u64)>>,
}

impl Table {
    /// A table with equally wide columns.
    pub fn new(headers: impl IntoIterator<Item = impl Into<Text>>) -> Table {
        let headers: Vec<Text> = headers.into_iter().map(Into::into).collect();
        let widths = vec![1.0; headers.len()];
        Table { headers, widths, ..Default::default() }
    }

    /// A table with English header keys and relative widths.
    pub fn with_widths(cols: &[(&'static str, f32)]) -> Table {
        Table { headers: cols.iter().map(|c| Text::Borrowed(c.0)).collect(), widths: cols.iter().map(|c| c.1).collect(), ..Default::default() }
    }

    pub fn push(&mut self, row: Vec<String>, target: Option<(usize, u64)>) {
        self.rows.push(row);
        self.targets.push(target);
    }

    /// The headers in the current language.
    pub fn header_texts(&self) -> Vec<&str> {
        self.headers.iter().map(text).collect()
    }

    /// Header `i` in the current language.
    pub fn header_text(&self, i: usize) -> &str {
        self.headers.get(i).map(text).unwrap_or("")
    }

    /// The headers as stored in forms (English keys, or the texts of plugin reports).
    pub fn header_keys(&self) -> Vec<&str> {
        self.headers.iter().map(|h| h.as_ref()).collect()
    }

    /// Index of the column a form names (key or any translation).
    pub fn column(&self, name: &str) -> Option<usize> {
        self.headers.iter().position(|h| header_matches(h, name))
    }

    /// CSV like the built-in reports (`;` separated, headers translated).
    pub fn to_csv(&self) -> String {
        let esc = |s: &str| if s.contains([';', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
        let line = |cells: Vec<&str>| cells.into_iter().map(esc).collect::<Vec<_>>().join(";") + "\n";
        let mut out = line(self.header_texts());
        for r in &self.rows {
            out.push_str(&line(r.iter().map(String::as_str).collect()));
        }
        out
    }
}

/// True if a name stored in a form means this header: the key itself or one of its
/// translations (forms written before the keys were stable hold translated names).
pub fn header_matches(header: &Text, name: &str) -> bool {
    match header {
        Cow::Borrowed(key) => *key == name || Lang::ALL.iter().any(|l| translate(key, *l) == name),
        Cow::Owned(s) => s == name,
    }
}

#[cfg(test)]
mod tests;
