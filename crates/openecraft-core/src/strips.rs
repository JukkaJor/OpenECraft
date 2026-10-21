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
/// Widths of end brackets and end plates whose parts are unknown (mm).
pub const END_BRACKET_WIDTH: f32 = 9.0;
pub const END_PLATE_WIDTH: f32 = 1.5;

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

fn part_by_number<'a>(prj: &'a Project, number: &str) -> Option<&'a Part> {
    prj.parts.iter().chain(crate::parts::catalog()).find(|p| p.number == number)
}

/// What sits on the rail at one place of a strip.
#[derive(Clone, Debug, PartialEq)]
pub enum ElementKind {
    /// A terminal of the schematic (designation).
    Terminal(String),
    /// A spare terminal (designation).
    Spare(String),
    /// Holds the strip on the rail, at both ends.
    EndBracket,
    /// Covers the open side of the last terminal of a kind.
    EndPlate,
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

/// Accessory element named by the tech data `key` of a terminal part (`End bracket`,
/// `End plate`).
fn accessory(prj: &Project, terminal_part: &str, key: &str, kind: ElementKind, default_width: f32) -> Option<Element> {
    let number = part_by_number(prj, terminal_part)?.tech_value(key)?.trim().to_string();
    let width = part_by_number(prj, &number).and_then(|p| p.size()).map(|s| s[0]).unwrap_or(default_width);
    Some(Element { kind, part: number, width })
}

/// The strip on the rail in order, each element with its part and width: an end
/// bracket, the terminals, the spare terminals, an end bracket. An end plate follows
/// the last terminal of each part (where the next terminal has another part, and at
/// the end). The accessory parts come from the tech data of the terminal parts
/// (`End bracket: …`, `End plate: …`), unless the strip has `manual_accessories`.
/// Reports, the bill of materials and the panel layout all work from this.
pub fn strip_layout(prj: &Project, strip: &str) -> Vec<Element> {
    let terminals = terminal_elements(prj, strip);
    if terminals.is_empty() || prj.strips.get(strip).is_some_and(|d| d.manual_accessories) {
        return terminals;
    }
    let bracket = |part: &str| accessory(prj, part, "End bracket", ElementKind::EndBracket, END_BRACKET_WIDTH);
    let mut out: Vec<Element> = bracket(&terminals[0].part).into_iter().collect();
    for (i, e) in terminals.iter().enumerate() {
        out.push(e.clone());
        if terminals.get(i + 1).is_none_or(|next| next.part != e.part) {
            out.extend(accessory(prj, &e.part, "End plate", ElementKind::EndPlate, END_PLATE_WIDTH));
        }
    }
    out.extend(bracket(&terminals[terminals.len() - 1].part));
    out
}

/// Terminals and spare terminals of a strip with their parts.
fn terminal_elements(prj: &Project, strip: &str) -> Vec<Element> {
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

/// Why a jumper cannot be fitted as planned.
#[derive(Clone, Debug, PartialEq)]
pub enum JumperProblem {
    /// The terminal part names no jumper parts (tech data `Jumper: …`).
    NoPart,
    /// It needs more poles than the longest jumper part has.
    TooShort { needed: u32, longest: u32 },
    /// An end plate or end bracket lies between its terminals.
    Blocked,
    /// It connects terminals of different parts.
    MixedParts,
    /// It includes a PE terminal.
    Pe,
}

/// A plug-in jumper of a strip: from the first to the last terminal with the same
/// jumper number.
#[derive(Clone, Debug, PartialEq)]
pub struct Jumper {
    pub number: u32,
    /// Jumper part; empty without a suitable one.
    pub part: String,
    /// Poles of the part (cut to the length of `span` if longer).
    pub poles: u32,
    /// Terminals the jumper spans, in strip order.
    pub span: Vec<String>,
    /// Terminals in the span that are not jumpered: their teeth are broken off.
    pub broken: Vec<String>,
    pub problem: Option<JumperProblem>,
}

/// Poles of a jumper part (tech data `Poles: 3`).
fn poles(prj: &Project, number: &str) -> u32 {
    part_by_number(prj, number).and_then(|p| p.tech_value("Poles")).and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}

/// The jumpers of a strip with their parts: the shortest jumper part of the terminal
/// part (tech data `Jumper: …`) that spans the group.
pub fn strip_jumpers(prj: &Project, strip: &str) -> Vec<Jumper> {
    let Some(data) = prj.strips.get(strip) else { return Vec::new() };
    let layout = strip_layout(prj, strip);
    let label = |e: &Element| match &e.kind {
        ElementKind::Terminal(l) | ElementKind::Spare(l) => Some(l.clone()),
        _ => None,
    };
    let mut numbers: Vec<u32> = data.jumpers.values().copied().filter(|j| *j > 0).collect();
    numbers.sort_unstable();
    numbers.dedup();
    let mut out = Vec::new();
    for number in numbers {
        let group: Vec<&String> = data.jumpers.iter().filter(|(_, j)| **j == number).map(|(l, _)| l).collect();
        let at: Vec<usize> = layout.iter().enumerate().filter(|(_, e)| label(e).is_some_and(|l| group.contains(&&l))).map(|(i, _)| i).collect();
        let (Some(&lo), Some(&hi)) = (at.first(), at.last()) else { continue };
        if at.len() < 2 {
            continue;
        }
        let elements = &layout[lo..=hi];
        let span: Vec<String> = elements.iter().filter_map(label).collect();
        let broken: Vec<String> = span.iter().filter(|l| !group.contains(l)).cloned().collect();
        let first = &layout[lo].part;
        let candidates: Vec<(String, u32)> = part_by_number(prj, first)
            .and_then(|p| p.tech_value("Jumper"))
            .map(|list| list.split(',').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).map(|n| (n.clone(), poles(prj, &n))).collect())
            .unwrap_or_default();
        let needed = span.len() as u32;
        let fitting = candidates.iter().filter(|c| c.1 >= needed).min_by_key(|c| c.1);
        let (part, poles) = fitting.cloned().unwrap_or_default();
        let problem = if elements.iter().any(|e| matches!(e.kind, ElementKind::EndPlate | ElementKind::EndBracket)) {
            Some(JumperProblem::Blocked)
        } else if elements.iter().any(|e| e.part != *first) {
            Some(JumperProblem::MixedParts)
        } else if part_by_number(prj, first).is_some_and(is_pe_part) {
            Some(JumperProblem::Pe)
        } else if candidates.is_empty() {
            Some(JumperProblem::NoPart)
        } else if fitting.is_none() {
            Some(JumperProblem::TooShort { needed, longest: candidates.iter().map(|c| c.1).max().unwrap_or(0) })
        } else {
            None
        };
        out.push(Jumper { number, part, poles, span, broken, problem });
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
