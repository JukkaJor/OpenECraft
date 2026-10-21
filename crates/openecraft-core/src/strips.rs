//! Terminal strips: terminal order, renumbering, jumpers, spare terminals, the part
//! of every terminal and the layout of the strip on the rail.

use crate::model::*;
use crate::netlist::{pin_designation, Netlist};
use crate::parts::FnKind;
use crate::symbols::{self, Func};

/// A terminal of a strip as placed in the schematic.
#[derive(Clone, Debug, PartialEq)]
pub struct Terminal {
    pub page: usize,
    pub sym: usize,
    pub id: u64,
    pub label: String,
}

fn is_terminal(s: &SymbolInst) -> bool {
    symbols::get(&s.sym).map(|d| d.func) == Some(Func::Terminal)
}

/// Tags of all terminal strips.
pub fn strip_tags(prj: &Project) -> Vec<String> {
    let mut v: Vec<String> =
        prj.pages.iter().flat_map(|p| p.symbols.iter()).filter(|s| is_terminal(s) && !s.tag.is_empty()).map(|s| s.tag.clone()).collect();
    v.extend(prj.strips.keys().cloned());
    v.sort_by(|a, b| natural_cmp(a, b));
    v.dedup();
    v
}

/// Terminals of a strip in strip order: the stored order first, the rest sorted by
/// designation (then schematic order).
pub fn terminals(prj: &Project, strip: &str) -> Vec<Terminal> {
    let mut v = Vec::new();
    for (pi, pg) in prj.pages.iter().enumerate() {
        if pg.kind != PageKind::Schematic {
            continue;
        }
        for (si, s) in pg.symbols.iter().enumerate() {
            if s.tag == strip && is_terminal(s) {
                v.push(Terminal { page: pi, sym: si, id: s.id, label: s.label.clone() });
            }
        }
    }
    let order = prj.strips.get(strip).map(|d| d.order.as_slice()).unwrap_or(&[]);
    let rank = |t: &Terminal| order.iter().position(|o| *o == t.label).unwrap_or(usize::MAX);
    v.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| natural_cmp(&a.label, &b.label)).then((a.page, a.sym).cmp(&(b.page, b.sym))));
    v
}

/// Moves a terminal one place up or down and stores the resulting order.
pub fn move_terminal(prj: &mut Project, strip: &str, index: usize, up: bool) {
    let mut labels: Vec<String> = terminals(prj, strip).into_iter().map(|t| t.label).collect();
    let other = if up { index.checked_sub(1) } else { Some(index + 1).filter(|i| *i < labels.len()) };
    if let (Some(o), true) = (other, index < labels.len()) {
        labels.swap(index, o);
        prj.strips.entry(strip.to_string()).or_default().order = labels;
    }
}

/// Numbers the terminals 1, 2, 3 … in strip order; terminals with a non-numeric
/// designation (e.g. PE) keep it. Jumpers and order follow. Returns the number of
/// changed designations.
pub fn renumber(prj: &mut Project, strip: &str) -> usize {
    let list = terminals(prj, strip);
    let mut n = 0;
    let mut changed = 0;
    let mut renamed = Vec::new();
    for t in &list {
        if !t.label.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        n += 1;
        let new = n.to_string();
        if t.label != new {
            renamed.push((t.label.clone(), new.clone()));
            prj.pages[t.page].symbols[t.sym].label = new;
            changed += 1;
        }
    }
    if let Some(data) = prj.strips.get_mut(strip) {
        let map = |l: &String| renamed.iter().find(|(o, _)| o == l).map(|(_, n)| n.clone()).unwrap_or_else(|| l.clone());
        data.order = list.iter().map(|t| map(&t.label)).collect();
        data.jumpers = data.jumpers.iter().map(|(l, j)| (map(l), *j)).collect();
    }
    changed
}

/// One row of the terminal diagram / strip editor.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub terminal: Option<Terminal>,
    pub label: String,
    pub jumper: u32,
    pub external: String,
    pub internal: String,
    pub potential: String,
    pub location: String,
    pub function_text: String,
}

fn targets(prj: &Project, nl: &Netlist, net: Option<usize>, exclude: (usize, usize)) -> String {
    let Some(ni) = net else { return String::new() };
    let mut v: Vec<String> = nl.nets[ni].pins.iter().filter(|p| (p.page, p.sym) != exclude).map(|p| pin_designation(prj, *p)).collect();
    v.sort_by(|a, b| natural_cmp(a, b));
    v.dedup();
    v.join(", ")
}

/// Terminal diagram rows of a strip including spare terminals. `nl`: potentials,
/// `open`: terminal sides apart.
pub fn rows(prj: &Project, strip: &str, nl: &Netlist, open: &Netlist) -> Vec<Row> {
    let data = prj.strips.get(strip).cloned().unwrap_or_default();
    let mut out = Vec::new();
    let mut max = 0u32;
    for t in terminals(prj, strip) {
        let s = &prj.pages[t.page].symbols[t.sym];
        max = max.max(t.label.parse().unwrap_or(0));
        out.push(Row {
            label: t.label.clone(),
            jumper: data.jumpers.get(&t.label).copied().unwrap_or(0),
            external: targets(prj, open, open.pin_net.get(&(s.id, 0)).copied(), (t.page, t.sym)),
            internal: targets(prj, open, open.pin_net.get(&(s.id, 1)).copied(), (t.page, t.sym)),
            potential: nl.pin_net.get(&(s.id, 0)).map(|&n| nl.nets[n].name.clone()).unwrap_or_default(),
            location: format!("/{}.{}", prj.pages[t.page].name, column_of(s.pos)),
            function_text: s.function_text.replace('\n', " "),
            terminal: Some(t),
        });
    }
    for i in 1..=data.spares {
        out.push(Row {
            terminal: None,
            label: (max + i).to_string(),
            jumper: 0,
            external: String::new(),
            internal: String::new(),
            potential: String::new(),
            location: String::new(),
            function_text: "spare".into(),
        });
    }
    out
}

/// Width of a terminal whose part has no mounting data (mm).
pub const TERMINAL_WIDTH: f32 = 5.2;

/// Symbol of protective earth terminals.
const PE_SYMBOL: &str = "term_pe";

/// A protective earth terminal part (tech data `Terminal type: PE`).
pub fn is_pe_part(part: &Part) -> bool {
    part.tech_value("Terminal type").is_some_and(|t| t.eq_ignore_ascii_case("PE"))
}

/// A part whose functions are all terminals.
pub fn is_terminal_part(part: &Part) -> bool {
    !part.functions.is_empty() && part.functions.iter().all(|f| f.kind == FnKind::Terminal)
}

/// Terminal parts of a strip in the order of its parts list (project parts first,
/// then the built-in catalog).
fn terminal_parts<'a>(prj: &'a Project, strip: &str) -> Vec<&'a Part> {
    let Some(data) = prj.devices.get(strip) else { return Vec::new() };
    data.parts
        .iter()
        .filter_map(|r| prj.parts.iter().chain(crate::parts::catalog()).find(|p| p.number == r.number))
        .filter(|p| is_terminal_part(p))
        .collect()
}

/// The part of a terminal: PE terminals take the strip's PE terminal part, all others
/// its first other terminal part; either falls back to the first terminal part.
pub fn terminal_part<'a>(prj: &'a Project, strip: &str, pe: bool) -> Option<&'a Part> {
    let parts = terminal_parts(prj, strip);
    parts.iter().find(|p| is_pe_part(p) == pe).or(parts.first()).copied()
}

/// What sits on the rail at one place of a strip.
#[derive(Clone, Debug, PartialEq)]
pub enum ElementKind {
    /// A terminal of the schematic (designation).
    Terminal(String),
    /// A spare terminal (designation).
    Spare(String),
}

/// One element of a terminal strip on the rail.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub kind: ElementKind,
    /// Part number; empty if the strip has no terminal part.
    pub part: String,
    /// Width along the rail (mm).
    pub width: f32,
}

/// The strip on the rail in order: its terminals, then the spare terminals, each
/// with its part and width. Reports, the bill of materials and the panel layout all
/// work from this.
pub fn strip_layout(prj: &Project, strip: &str) -> Vec<Element> {
    let element = |kind, pe| {
        let part = terminal_part(prj, strip, pe);
        let width = part.and_then(|p| p.size()).map(|s| s[0]).unwrap_or(TERMINAL_WIDTH);
        Element { kind, part: part.map(|p| p.number.clone()).unwrap_or_default(), width }
    };
    let mut out = Vec::new();
    let mut max = 0u32;
    for t in terminals(prj, strip) {
        let pe = prj.pages[t.page].symbols[t.sym].sym == PE_SYMBOL;
        max = max.max(t.label.parse().unwrap_or(0));
        out.push(element(ElementKind::Terminal(t.label), pe));
    }
    let spares = prj.strips.get(strip).map(|d| d.spares).unwrap_or(0);
    for i in 1..=spares {
        out.push(element(ElementKind::Spare((max + i).to_string()), false));
    }
    out
}

/// Jumper groups of all strips: (strip, jumper number) -> terminal designations.
pub fn jumper_groups(prj: &Project) -> Vec<(String, Vec<String>)> {
    let mut v = Vec::new();
    for (strip, data) in &prj.strips {
        let mut numbers: Vec<u32> = data.jumpers.values().copied().filter(|j| *j > 0).collect();
        numbers.sort();
        numbers.dedup();
        for j in numbers {
            let labels: Vec<String> = data.jumpers.iter().filter(|(_, x)| **x == j).map(|(l, _)| l.clone()).collect();
            if labels.len() > 1 {
                v.push((strip.clone(), labels));
            }
        }
    }
    v
}

#[cfg(test)]
mod tests;
