//! Project-wide find and replace (in the spirit of EPLAN's Find): page names and
//! descriptions, device tags, function texts, free texts, wire numbers, terminals,
//! cables and part numbers. Replacing changes texts only (free texts, function texts,
//! page, device and cable descriptions) and leaves released pages alone; tags are
//! renamed with `ops::rename_device`.

use crate::model::Project;
use crate::symbols::Func;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Kind {
    /// Page name or description.
    Page,
    Device,
    FunctionText,
    /// Free text on a page.
    Text,
    WireNumber,
    Terminal,
    Cable,
    Part,
    /// Device or cable description.
    Description,
}

impl Kind {
    pub const ALL: [Kind; 9] = [Kind::Page, Kind::Device, Kind::FunctionText, Kind::Text, Kind::WireNumber, Kind::Terminal, Kind::Cable, Kind::Part, Kind::Description];

    pub fn code(self) -> &'static str {
        match self {
            Kind::Page => "page",
            Kind::Device => "device",
            Kind::FunctionText => "function_text",
            Kind::Text => "text",
            Kind::WireNumber => "wire_number",
            Kind::Terminal => "terminal",
            Kind::Cable => "cable",
            Kind::Part => "part",
            Kind::Description => "description",
        }
    }

    pub fn from_code(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.code() == s.trim().to_ascii_lowercase())
    }

    /// Kinds whose texts can be replaced.
    pub fn replaceable(self) -> bool {
        matches!(self, Kind::Page | Kind::FunctionText | Kind::Text | Kind::Description)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Query {
    pub text: String,
    pub case_sensitive: bool,
    /// Only whole words (bounded by anything but letters, digits and `_`).
    pub whole_word: bool,
    /// Kinds searched; empty = all.
    pub kinds: Vec<Kind>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub kind: Kind,
    /// Page index, if the hit is on a page.
    pub page: Option<usize>,
    /// Id of the item (symbol, wire, text, cable); 0 for pages and devices without one.
    pub item: u64,
    /// The text that matched.
    pub text: String,
}

impl Query {
    pub fn new(text: &str) -> Query {
        Query { text: text.into(), ..Default::default() }
    }

    fn wants(&self, k: Kind) -> bool {
        self.kinds.is_empty() || self.kinds.contains(&k)
    }

    /// Byte ranges of the matches in `hay`.
    fn ranges(&self, hay: &str) -> Vec<(usize, usize)> {
        if self.text.is_empty() {
            return Vec::new();
        }
        let (h, n) = if self.case_sensitive { (hay.to_string(), self.text.clone()) } else { (hay.to_lowercase(), self.text.to_lowercase()) };
        // Lower-casing can change byte lengths; fall back to no match in that case.
        if h.len() != hay.len() || n.len() != self.text.len() {
            return Vec::new();
        }
        let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        let mut out = Vec::new();
        let mut from = 0;
        while let Some(at) = h[from..].find(&n) {
            let (a, b) = (from + at, from + at + n.len());
            if !self.whole_word || (!word(h[..a].chars().next_back()) && !word(h[b..].chars().next())) {
                out.push((a, b));
            }
            from = b.max(a + 1);
            while from < h.len() && !h.is_char_boundary(from) {
                from += 1;
            }
        }
        out
    }

    pub fn matches(&self, hay: &str) -> bool {
        !self.ranges(hay).is_empty()
    }

    /// `hay` with every match replaced by `with`; None without a match.
    pub fn replace(&self, hay: &str, with: &str) -> Option<String> {
        let ranges = self.ranges(hay);
        if ranges.is_empty() {
            return None;
        }
        let mut out = String::new();
        let mut last = 0;
        for (a, b) in ranges {
            out.push_str(&hay[last..a]);
            out.push_str(with);
            last = b;
        }
        out.push_str(&hay[last..]);
        Some(out)
    }
}

/// Everything in the project that matches, in page order (project-wide hits last).
pub fn find(prj: &Project, q: &Query) -> Vec<Hit> {
    let mut out = Vec::new();
    let mut devices: BTreeSet<String> = BTreeSet::new();
    let mut numbers: BTreeSet<String> = BTreeSet::new();
    for (pi, pg) in prj.pages.iter().enumerate() {
        if q.wants(Kind::Page) && (q.matches(&pg.name) || q.matches(&pg.description)) {
            out.push(Hit { kind: Kind::Page, page: Some(pi), item: 0, text: format!("{} {}", pg.name, pg.description).trim().to_string() });
        }
        for s in &pg.symbols {
            let func = prj.symbol(&s.sym).map(|d| d.func);
            if func == Some(Func::Terminal) || matches!(func, Some(Func::Plug(_))) {
                let t = format!("{}:{}", s.tag, s.label);
                if q.wants(Kind::Terminal) && q.matches(&t) {
                    out.push(Hit { kind: Kind::Terminal, page: Some(pi), item: s.id, text: t });
                }
            } else if q.wants(Kind::Device) && !s.tag.is_empty() && q.matches(&s.tag) && devices.insert(s.tag.clone()) {
                out.push(Hit { kind: Kind::Device, page: Some(pi), item: s.id, text: s.tag.clone() });
            }
            if q.wants(Kind::FunctionText) && q.matches(&s.function_text) {
                out.push(Hit { kind: Kind::FunctionText, page: Some(pi), item: s.id, text: s.function_text.replace('\n', " ") });
            }
        }
        for t in &pg.texts {
            if q.wants(Kind::Text) && q.matches(&t.text) {
                out.push(Hit { kind: Kind::Text, page: Some(pi), item: t.id, text: t.text.replace('\n', " ") });
            }
        }
        for w in &pg.wires {
            if q.wants(Kind::WireNumber) && !w.number.is_empty() && q.matches(&w.number) && numbers.insert(w.number.clone()) {
                out.push(Hit { kind: Kind::WireNumber, page: Some(pi), item: w.id, text: w.number.clone() });
            }
        }
    }
    for c in &prj.cables {
        if q.wants(Kind::Cable) && (q.matches(&c.tag) || q.matches(&c.cable_type)) {
            out.push(Hit { kind: Kind::Cable, page: None, item: c.id, text: format!("{} {}", c.tag, c.cable_type).trim().to_string() });
        }
        if q.wants(Kind::Description) && q.matches(&c.description) {
            out.push(Hit { kind: Kind::Description, page: None, item: c.id, text: format!("{}: {}", c.tag, c.description) });
        }
    }
    for (tag, d) in &prj.devices {
        let at = first_location(prj, tag);
        if q.wants(Kind::Part) {
            for r in d.parts.iter().filter(|r| q.matches(&r.number)) {
                out.push(Hit { kind: Kind::Part, page: at.map(|a| a.0), item: at.map_or(0, |a| a.1), text: format!("{tag}: {}", r.number) });
            }
        }
        if q.wants(Kind::Description) && q.matches(&d.description) {
            out.push(Hit { kind: Kind::Description, page: at.map(|a| a.0), item: at.map_or(0, |a| a.1), text: format!("{tag}: {}", d.description) });
        }
    }
    out
}

/// First place of a device (page, symbol id).
fn first_location(prj: &Project, tag: &str) -> Option<(usize, u64)> {
    prj.pages.iter().enumerate().find_map(|(pi, pg)| pg.symbols.iter().find(|s| s.tag == tag).map(|s| (pi, s.id)))
}

/// Replaces the matches in the replaceable texts the query covers (released pages
/// stay as they are). Returns the number of changed texts.
pub fn replace(prj: &mut Project, q: &Query, with: &str) -> usize {
    let mut n = 0;
    let mut swap = |s: &mut String| {
        if let Some(new) = q.replace(s, with) {
            *s = new;
            n += 1;
        }
    };
    for pg in prj.pages.iter_mut().filter(|pg| !pg.locked) {
        if q.wants(Kind::Page) {
            swap(&mut pg.description);
        }
        if q.wants(Kind::FunctionText) {
            pg.symbols.iter_mut().for_each(|s| swap(&mut s.function_text));
        }
        if q.wants(Kind::Text) {
            pg.texts.iter_mut().for_each(|t| swap(&mut t.text));
        }
    }
    if q.wants(Kind::Description) {
        prj.devices.values_mut().for_each(|d| swap(&mut d.description));
        prj.cables.iter_mut().for_each(|c| swap(&mut c.description));
    }
    n
}

#[cfg(test)]
mod tests;
