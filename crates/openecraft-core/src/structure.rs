//! Structure identifiers (IEC 81346): a device tag such as `=A1+CAB1-K1` names the
//! function (`=A1`), the location (`+CAB1`) and the device itself (`-K1`).
//!
//! Tags are stored complete. Pages carry a function and a location; on a page a
//! tag is shown without the aspects that equal the page's, and aspects missing in
//! an entered tag are taken from the page.

use crate::model::Page;

/// A device tag split into its aspects (stored without the prefix signs, except
/// the device name which keeps its `-`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dt {
    pub function: String,
    pub location: String,
    pub name: String,
}

impl Dt {
    /// Parses `=A1+CAB1-K1`, `+CAB1-K1`, `-K1` …; the device name starts at the
    /// first `-`.
    pub fn parse(tag: &str) -> Dt {
        let tag = tag.trim();
        let (structure, name) = match tag.find('-') {
            Some(i) => (&tag[..i], &tag[i..]),
            None if tag.starts_with(['=', '+']) => (tag, ""),
            None => ("", tag),
        };
        let mut dt = Dt { name: name.to_string(), ..Default::default() };
        let mut sign = None;
        let mut buf = String::new();
        for c in structure.chars().chain(std::iter::once('\0')) {
            if matches!(c, '=' | '+' | '\0') {
                match sign {
                    Some('=') => dt.function = std::mem::take(&mut buf),
                    Some('+') => dt.location = std::mem::take(&mut buf),
                    _ => buf.clear(),
                }
                sign = Some(c);
            } else {
                buf.push(c);
            }
        }
        dt
    }

    /// `=A1+CAB1` (empty without structure).
    pub fn structure(&self) -> String {
        structure(&self.function, &self.location)
    }

    pub fn tag(&self) -> String {
        format!("{}{}", self.structure(), self.name)
    }
}

/// `=function+location` with empty aspects left out.
pub fn structure(function: &str, location: &str) -> String {
    let mut s = String::new();
    if !function.is_empty() {
        s.push('=');
        s.push_str(function);
    }
    if !location.is_empty() {
        s.push('+');
        s.push_str(location);
    }
    s
}

/// Removes prefix signs and blanks from an entered aspect (`"=A1 "` → `"A1"`).
pub fn clean_aspect(s: &str) -> String {
    s.trim().trim_start_matches(['=', '+']).replace(' ', "")
}

/// Structure of a page, e.g. `=A1+CAB1`.
pub fn page_structure(page: &Page) -> String {
    structure(&page.function, &page.location)
}

/// Page name with structure, e.g. `=A1+CAB1/3` (just `3` without structure).
pub fn page_full_name(page: &Page) -> String {
    let s = page_structure(page);
    if s.is_empty() {
        page.name.clone()
    } else {
        format!("{s}/{}", page.name)
    }
}

/// Complete tag of a tag entered on a page: missing aspects come from the page.
pub fn complete(page: &Page, entered: &str) -> String {
    let mut dt = Dt::parse(entered);
    if dt.function.is_empty() {
        dt.function = page.function.clone();
    }
    if dt.location.is_empty() {
        dt.location = page.location.clone();
    }
    dt.tag()
}

/// Shortest form of a complete tag on a page: aspects equal to the page's are left out.
pub fn relative(page: &Page, tag: &str) -> String {
    let mut dt = Dt::parse(tag);
    if dt.function == page.function {
        dt.function.clear();
    }
    if dt.location == page.location {
        dt.location.clear();
    }
    dt.tag()
}

#[cfg(test)]
mod tests;
