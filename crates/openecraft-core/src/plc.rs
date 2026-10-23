//! PLC overview: inputs / outputs per card, addressing from the card's start byte and
//! symbol table exchange with PLC programming tools (CSV: name, data type, address,
//! comment).

use crate::model::*;
use crate::netlist::{pin_designation, Netlist};
use crate::symbols::{self, Func};
use std::collections::{BTreeMap, HashSet};

/// A PLC input or output placed in the schematic.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub page: usize,
    pub sym: usize,
    pub id: u64,
    pub card: String,
    pub address: String,
    pub input: bool,
    pub function_text: String,
    /// Devices connected to the point.
    pub connected: String,
    pub location: String,
}

fn is_plc(s: &SymbolInst) -> bool {
    symbols::get(&s.sym).map(|d| d.func) == Some(Func::Plc)
}

/// Address sort key: prefix, byte, bit (`I1.7` → ('I', 1, 7)).
pub fn address_key(addr: &str) -> (String, u32, u32) {
    let a = addr.trim().trim_start_matches('%').to_uppercase();
    let prefix: String = a.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let rest = &a[prefix.len()..];
    let (byte, bit) = rest.split_once('.').unwrap_or((rest, "0"));
    (prefix, byte.parse().unwrap_or(u32::MAX), bit.parse().unwrap_or(0))
}

/// Tags of all PLC cards.
pub fn cards(prj: &Project) -> Vec<String> {
    let mut v: Vec<String> =
        prj.pages.iter().flat_map(|p| p.symbols.iter()).filter(|s| is_plc(s) && !s.tag.is_empty()).map(|s| s.tag.clone()).collect();
    v.extend(prj.plc_cards.keys().cloned());
    v.sort_by(|a, b| natural_cmp(a, b));
    v.dedup();
    v
}

/// All PLC points, sorted by card and address.
pub fn points(prj: &Project, nl: &Netlist) -> Vec<Point> {
    let mut v = Vec::new();
    for (pi, pg) in prj.pages.iter().enumerate() {
        if pg.kind != PageKind::Schematic {
            continue;
        }
        for (si, s) in pg.symbols.iter().enumerate().filter(|(_, s)| is_plc(s)) {
            let connected = nl
                .pin_net
                .get(&(s.id, 0))
                .map(|&n| {
                    let mut t: Vec<String> =
                        nl.nets[n].pins.iter().filter(|p| (p.page, p.sym) != (pi, si)).map(|p| pin_designation(prj, *p)).collect();
                    t.sort_by(|a, b| natural_cmp(a, b));
                    t.dedup();
                    t.join(", ")
                })
                .unwrap_or_default();
            v.push(Point {
                page: pi,
                sym: si,
                id: s.id,
                card: s.tag.clone(),
                address: s.label.clone(),
                input: s.sym == "plc_in",
                function_text: s.function_text.replace('\n', " "),
                connected,
                location: crate::frame::reference(prj, pi, s.pos),
            });
        }
    }
    v.sort_by(|a, b| natural_cmp(&a.card, &b.card).then_with(|| address_key(&a.address).cmp(&address_key(&b.address))));
    v
}

/// Gives the points of a card consecutive addresses from its start byte (inputs and
/// outputs separately, in schematic order). Returns the number of changed addresses.
pub fn readdress(prj: &mut Project, card: &str) -> usize {
    let start = prj.plc_cards.get(card).map(|c| c.start_byte).unwrap_or(0);
    let mut changed = 0;
    for input in [true, false] {
        let mut list: Vec<(usize, usize)> = Vec::new();
        for (pi, pg) in prj.pages.iter().enumerate() {
            for (si, s) in pg.symbols.iter().enumerate() {
                if is_plc(s) && s.tag == card && (s.sym == "plc_in") == input {
                    list.push((pi, si));
                }
            }
        }
        list.sort_by(|a, b| {
            let (sa, sb) = (&prj.pages[a.0].symbols[a.1], &prj.pages[b.0].symbols[b.1]);
            a.0.cmp(&b.0).then(sa.pos.x.total_cmp(&sb.pos.x)).then(sa.pos.y.total_cmp(&sb.pos.y))
        });
        let prefix = if input { 'I' } else { 'Q' };
        for (n, (pi, si)) in list.into_iter().enumerate() {
            let addr = format!("{prefix}{}.{}", start + n as u32 / 8, n % 8);
            let s = &mut prj.pages[pi].symbols[si];
            if s.label != addr {
                s.label = addr;
                changed += 1;
            }
        }
    }
    changed
}

/// Tag name for a PLC tool: letters, digits and underscores.
fn tag_name(p: &Point) -> String {
    let base = if p.function_text.trim().is_empty() { format!("{}_{}", p.card, p.address) } else { p.function_text.clone() };
    let mut s: String = base.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    while s.contains("__") {
        s = s.replace("__", "_");
    }
    let s = s.trim_matches('_').to_string();
    if s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{s}")
    } else {
        s
    }
}

/// Symbol table for PLC programming tools: `Name;Data Type;Logical Address;Comment`.
pub fn symbol_table(prj: &Project, nl: &Netlist) -> String {
    let mut out = String::from("Name;Data Type;Logical Address;Comment\n");
    let mut used = HashSet::new();
    for p in points(prj, nl) {
        let mut name = tag_name(&p);
        let mut i = 2;
        while !used.insert(name.clone()) {
            name = format!("{}_{i}", tag_name(&p));
            i += 1;
        }
        let comment = format!("{} {}", p.card, p.function_text).trim().replace(';', ",");
        out.push_str(&format!("{name};Bool;%{};{comment}\n", p.address));
    }
    out
}

/// Takes the comments (or names) of a PLC symbol table as function texts of the
/// points with the same address. Columns are found by their header (`Address` /
/// `Logical Address`, `Comment`, `Name`); `;`, `,` or tab separated. Returns the
/// number of updated points.
pub fn import_symbol_table(prj: &mut Project, text: &str) -> Result<usize, String> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().ok_or("empty file")?;
    let sep = [';', '\t', ','].into_iter().max_by_key(|c| header.matches(*c).count()).unwrap_or(';');
    let cols: Vec<String> = header.split(sep).map(|c| c.trim().trim_matches('"').to_lowercase()).collect();
    let find = |names: &[&str]| cols.iter().position(|c| names.contains(&c.as_str()));
    let addr_col = find(&["logical address", "address", "adresse"]).ok_or("no address column")?;
    let comment_col = find(&["comment", "kommentar"]);
    let name_col = find(&["name", "symbol"]);
    let mut texts: BTreeMap<(String, u32, u32), String> = BTreeMap::new();
    for line in lines {
        let cells: Vec<&str> = line.split(sep).map(|c| c.trim().trim_matches('"')).collect();
        let Some(addr) = cells.get(addr_col).filter(|a| !a.is_empty()) else { continue };
        let comment = comment_col.and_then(|i| cells.get(i)).filter(|c| !c.is_empty());
        let text = comment.or(name_col.and_then(|i| cells.get(i))).map(|t| t.to_string()).unwrap_or_default();
        texts.insert(address_key(addr), text);
    }
    let mut n = 0;
    for pg in &mut prj.pages {
        for s in pg.symbols.iter_mut().filter(|s| is_plc(s)) {
            if let Some(t) = texts.get(&address_key(&s.label)) {
                if !t.is_empty() && s.function_text != *t {
                    s.function_text = t.clone();
                    n += 1;
                }
            }
        }
    }
    Ok(n)
}

/// Addresses used by more than one point (over all cards).
pub fn duplicate_addresses(prj: &Project, nl: &Netlist) -> Vec<(String, Vec<Point>)> {
    let mut by: BTreeMap<(String, u32, u32), Vec<Point>> = BTreeMap::new();
    for p in points(prj, nl) {
        by.entry(address_key(&p.address)).or_default().push(p);
    }
    by.into_values().filter(|v| v.len() > 1).map(|v| (v[0].address.clone(), v)).collect()
}

#[cfg(test)]
mod tests;
