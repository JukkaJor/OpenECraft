//! Reports (device list, BOM, terminal diagram, ...) and the project check.

use crate::index::Index;
use crate::model::*;
use crate::netlist::{pin_designation, Netlist, PinRef};
use crate::symbols::{self, Func};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Clone, Debug, Default)]
pub struct Table {
    pub headers: Vec<&'static str>,
    /// Relative column widths.
    pub widths: Vec<f32>,
    pub rows: Vec<Vec<String>>,
    /// Optional jump target (page index, item id) per row.
    pub targets: Vec<Option<(usize, u64)>>,
}

impl Table {
    fn new(cols: &[(&'static str, f32)]) -> Table {
        Table {
            headers: cols.iter().map(|c| c.0).collect(),
            widths: cols.iter().map(|c| c.1).collect(),
            ..Default::default()
        }
    }

    fn push(&mut self, row: Vec<String>, target: Option<(usize, u64)>) {
        self.rows.push(row);
        self.targets.push(target);
    }

    pub fn to_csv(&self) -> String {
        let esc = |s: &str| {
            if s.contains([';', '"', '\n']) {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        };
        let mut out = self.headers.iter().map(|h| esc(h)).collect::<Vec<_>>().join(";");
        out.push('\n');
        for r in &self.rows {
            out.push_str(&r.iter().map(|c| esc(c)).collect::<Vec<_>>().join(";"));
            out.push('\n');
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
    pub target: Option<(usize, u64)>,
}

/// Everything derived from the project that is expensive enough to cache.
#[derive(Default)]
pub struct Analysis {
    pub netlist: Netlist,
    pub reports: HashMap<ReportKind, Table>,
    pub issues: Vec<Issue>,
}

impl Analysis {
    pub fn build(prj: &Project) -> Analysis {
        let idx = Index::build(prj);
        let netlist = Netlist::build(prj, true);
        let open = Netlist::build(prj, false);
        let mut reports = HashMap::new();
        for k in ReportKind::ALL {
            let t = match k {
                ReportKind::TableOfContents => toc(prj),
                ReportKind::DeviceList => device_list(prj, &idx),
                ReportKind::BillOfMaterials => bom(prj, &idx),
                ReportKind::TerminalDiagram => terminal_diagram(prj, &netlist, &open),
                ReportKind::ConnectionList => connection_list(prj, &netlist),
                ReportKind::PlcAssignment => plc_list(prj, &netlist),
                ReportKind::PotentialList => potential_list(prj, &netlist),
            };
            reports.insert(k, t);
        }
        let issues = check(prj, &idx, &netlist);
        Analysis { netlist, reports, issues }
    }
}

fn loc_str(prj: &Project, page: usize, pos: crate::geom::P) -> String {
    format!("/{}.{}", prj.pages[page].name, column_of(pos))
}

fn toc(prj: &Project) -> Table {
    let mut t = Table::new(&[("Page", 1.0), ("Description", 4.0), ("Type", 2.0), ("Devices", 1.0)]);
    for (i, pg) in prj.pages.iter().enumerate() {
        t.push(
            vec![pg.name.clone(), pg.description.clone(), pg.kind.label(), pg.symbols.len().to_string()],
            Some((i, 0)),
        );
    }
    t
}

fn device_list(prj: &Project, idx: &Index) -> Table {
    let mut t = Table::new(&[
        ("Device tag", 1.2),
        ("Main function", 2.0),
        ("Function text", 2.0),
        ("Location", 1.0),
        ("Aux. functions", 1.6),
        ("Part number", 1.6),
        ("Description", 3.0),
    ]);
    for tag in idx.sorted_tags() {
        let dev = &idx.devices[tag];
        let first = dev.mains.first().or(dev.others.first()).or(dev.auxs.first()).copied();
        let Some(loc) = first else { continue };
        let inst = prj.pages[loc.page].symbols.iter().find(|s| s.id == loc.sym_id);
        let (name, ftext) = inst
            .map(|s| (symbols::get(&s.sym).map(|d| d.name).unwrap_or("?").to_string(), s.function_text.clone()))
            .unwrap_or_default();
        let name = if dev.mains.is_empty() && !dev.others.is_empty() {
            format!("{name} ×{}", dev.others.len())
        } else if dev.mains.is_empty() {
            "(missing main function)".into()
        } else {
            name
        };
        let data = prj.devices.get(tag).cloned().unwrap_or_default();
        let desc = if data.description.is_empty() {
            data.parts
                .first()
                .and_then(|r| prj.parts.iter().find(|p| p.number == r.number))
                .map(|p| p.description.clone())
                .unwrap_or_default()
        } else {
            data.description.clone()
        };
        let part_list = data
            .parts
            .iter()
            .map(|r| if r.count > 1 { format!("{}× {}", r.count, r.number) } else { r.number.clone() })
            .collect::<Vec<_>>()
            .join(", ");
        t.push(
            vec![
                tag.clone(),
                name,
                ftext.replace('\n', " "),
                loc_str(prj, loc.page, loc.pos),
                dev.auxs.len().to_string(),
                part_list,
                desc,
            ],
            Some((loc.page, loc.sym_id)),
        );
    }
    t
}

fn bom(prj: &Project, idx: &Index) -> Table {
    let mut t = Table::new(&[
        ("Pos", 0.5),
        ("Qty", 0.6),
        ("Unit", 0.5),
        ("Part number", 1.8),
        ("Type number", 1.4),
        ("Manufacturer", 1.0),
        ("Description", 3.2),
        ("Devices", 2.0),
        ("Unit price", 0.9),
        ("Total", 0.9),
    ]);
    let mut groups: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    for tag in idx.sorted_tags() {
        let Some(data) = prj.devices.get(tag) else { continue };
        if data.parts.is_empty() {
            continue;
        }
        let dev = &idx.devices[tag];
        // Terminal strips are ordered per terminal, everything else per device.
        let terminals = dev
            .others
            .iter()
            .filter(|l| {
                prj.pages[l.page]
                    .symbols
                    .iter()
                    .find(|s| s.id == l.sym_id)
                    .and_then(|s| symbols::get(&s.sym))
                    .map(|d| d.func == Func::Terminal)
                    .unwrap_or(false)
            })
            .count();
        let per_device = if dev.mains.is_empty() && terminals > 0 { terminals } else { 1 };
        for r in &data.parts {
            let g = groups.entry(r.number.clone()).or_default();
            g.0 += per_device * r.count.max(1) as usize;
            if !g.1.contains(tag) {
                g.1.push(tag.clone());
            }
        }
    }
    let mut sum = 0.0;
    for (i, (part, (qty, tags))) in groups.iter().enumerate() {
        let pd = prj.parts.iter().find(|p| &p.number == part).cloned().unwrap_or_else(|| Part {
            description: "(part not in project – synchronize parts)".into(),
            ..Default::default()
        });
        let total = pd.total_price(*qty as f32);
        sum += total;
        let unit_price = if pd.price_unit > 1 { format!("{:.2}/{}", pd.price, pd.price_unit) } else { format!("{:.2}", pd.price) };
        t.push(
            vec![
                (i + 1).to_string(),
                qty.to_string(),
                pd.quantity_unit.clone(),
                part.clone(),
                pd.type_number,
                pd.manufacturer,
                pd.description,
                tags.join(", "),
                format!("{unit_price} {}", pd.currency),
                format!("{total:.2}"),
            ],
            None,
        );
    }
    if !groups.is_empty() {
        let mut last = vec![String::new(); 10];
        last[6] = "Sum".into();
        last[9] = format!("{sum:.2}");
        t.push(last, None);
    }
    t
}

fn targets(prj: &Project, nl: &Netlist, net: Option<usize>, exclude: (usize, usize)) -> String {
    let Some(ni) = net else { return String::new() };
    let mut v: Vec<String> = nl.nets[ni]
        .pins
        .iter()
        .filter(|p| (p.page, p.sym) != exclude)
        .map(|p| pin_designation(prj, *p))
        .collect();
    v.sort_by(|a, b| natural_cmp(a, b));
    v.dedup();
    v.join(", ")
}

/// (sort key 1, sort key 2, remaining cells, jump target)
type SortRow = (String, String, Vec<String>, (usize, u64));

fn terminal_diagram(prj: &Project, nl: &Netlist, open: &Netlist) -> Table {
    let mut t = Table::new(&[
        ("Strip", 0.8),
        ("Terminal", 0.8),
        ("External (top)", 3.0),
        ("Internal (bottom)", 3.0),
        ("Potential", 1.0),
        ("Location", 0.8),
        ("Function text", 1.6),
    ]);
    let mut rows: Vec<SortRow> = Vec::new();
    for (pi, pg) in prj.pages.iter().enumerate() {
        for (si, s) in pg.symbols.iter().enumerate() {
            let Some(def) = symbols::get(&s.sym) else { continue };
            if def.func != Func::Terminal {
                continue;
            }
            let top = targets(prj, open, open.pin_net.get(&(s.id, 0)).copied(), (pi, si));
            let bot = targets(prj, open, open.pin_net.get(&(s.id, 1)).copied(), (pi, si));
            let pot = nl.pin_net.get(&(s.id, 0)).map(|&n| nl.nets[n].name.clone()).unwrap_or_default();
            rows.push((
                s.tag.clone(),
                s.label.clone(),
                vec![top, bot, pot, loc_str(prj, pi, s.pos), s.function_text.replace('\n', " ")],
                (pi, s.id),
            ));
        }
    }
    rows.sort_by(|a, b| natural_cmp(&a.0, &b.0).then_with(|| natural_cmp(&a.1, &b.1)));
    for (strip, num, rest, target) in rows {
        let mut r = vec![strip, num];
        r.extend(rest);
        t.push(r, Some(target));
    }
    t
}

fn connection_list(prj: &Project, nl: &Netlist) -> Table {
    let mut t = Table::new(&[
        ("Potential", 1.0),
        ("From", 1.6),
        ("To", 1.6),
        ("Page", 0.8),
        ("Colour", 0.8),
        ("Cross-section", 1.0),
    ]);
    for net in &nl.nets {
        // Device connection points only (potentials / connectors are not physical ends).
        let mut pins: Vec<&PinRef> = net
            .pins
            .iter()
            .filter(|p| {
                let s = &prj.pages[p.page].symbols[p.sym];
                symbols::get(&s.sym).map(|d| d.func.has_tag()).unwrap_or(false)
            })
            .collect();
        if pins.len() < 2 {
            continue;
        }
        pins.sort_by(|a, b| {
            let (sa, sb) = (&prj.pages[a.page].symbols[a.sym], &prj.pages[b.page].symbols[b.sym]);
            a.page.cmp(&b.page).then(sa.pos.x.total_cmp(&sb.pos.x)).then(sa.pos.y.total_cmp(&sb.pos.y))
        });
        let (color, cs) = net
            .wires
            .iter()
            .find_map(|(pi, wid)| {
                prj.pages[*pi].wires.iter().find(|w| w.id == *wid && (!w.color.is_empty() || !w.cross_section.is_empty()))
            })
            .map(|w| (w.color.clone(), w.cross_section.clone()))
            .unwrap_or_default();
        for pair in pins.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let s = &prj.pages[a.page].symbols[a.sym];
            t.push(
                vec![
                    net.name.clone(),
                    pin_designation(prj, *a),
                    pin_designation(prj, *b),
                    loc_str(prj, a.page, s.pos),
                    color.clone(),
                    cs.clone(),
                ],
                Some((a.page, s.id)),
            );
        }
    }
    t
}

fn plc_list(prj: &Project, nl: &Netlist) -> Table {
    let mut t = Table::new(&[
        ("PLC card", 1.0),
        ("Address", 1.0),
        ("Type", 0.8),
        ("Function text", 2.5),
        ("Connected to", 3.0),
        ("Location", 0.8),
    ]);
    let mut rows = Vec::new();
    for (pi, pg) in prj.pages.iter().enumerate() {
        for (si, s) in pg.symbols.iter().enumerate() {
            if symbols::get(&s.sym).map(|d| d.func) != Some(Func::Plc) {
                continue;
            }
            let kind = if s.sym == "plc_in" { "DI" } else { "DO" };
            let to = targets(prj, nl, nl.pin_net.get(&(s.id, 0)).copied(), (pi, si));
            rows.push((
                s.tag.clone(),
                s.label.clone(),
                vec![kind.to_string(), s.function_text.replace('\n', " "), to, loc_str(prj, pi, s.pos)],
                (pi, s.id),
            ));
        }
    }
    rows.sort_by(|a, b| natural_cmp(&a.0, &b.0).then_with(|| natural_cmp(&a.1, &b.1)));
    for (card, addr, rest, target) in rows {
        let mut r = vec![card, addr];
        r.extend(rest);
        t.push(r, Some(target));
    }
    t
}

fn potential_list(prj: &Project, nl: &Netlist) -> Table {
    let mut t = Table::new(&[("Potential", 1.0), ("Connection points", 4.0), ("Pages", 1.5)]);
    let mut nets: Vec<_> = nl.nets.iter().filter(|n| n.named).collect();
    nets.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    for net in nets {
        let mut pins: Vec<String> = net
            .pins
            .iter()
            .filter(|p| {
                let s = &prj.pages[p.page].symbols[p.sym];
                symbols::get(&s.sym).map(|d| d.func.has_tag()).unwrap_or(false)
            })
            .map(|p| pin_designation(prj, *p))
            .collect();
        pins.sort_by(|a, b| natural_cmp(a, b));
        pins.dedup();
        let pages: BTreeSet<usize> = net.pins.iter().map(|p| p.page).chain(net.wires.iter().map(|w| w.0)).collect();
        let pages: Vec<String> = pages.iter().map(|&p| prj.pages[p].name.clone()).collect();
        t.push(vec![net.name.clone(), pins.join(", "), pages.join(", ")], None);
    }
    t
}

/// Project check ("Prüfläufe"): finds typical engineering mistakes.
pub fn check(prj: &Project, idx: &Index, nl: &Netlist) -> Vec<Issue> {
    let mut v = Vec::new();
    let mut issue = |severity, message: String, target| v.push(Issue { severity, message, target });
    for tag in idx.sorted_tags() {
        let dev = &idx.devices[tag];
        let t = |l: &crate::index::Loc| Some((l.page, l.sym_id));
        if dev.mains.is_empty() && !dev.auxs.is_empty() {
            issue(Severity::Error, format!("{tag}: auxiliary function(s) without main function"), t(&dev.auxs[0]));
        }
        if dev.mains.len() > 1 {
            issue(Severity::Error, format!("{tag}: main function placed {} times", dev.mains.len()), t(&dev.mains[1]));
        }
        if !dev.others.is_empty() {
            // Duplicate terminal numbers / PLC addresses
            let mut seen: HashMap<String, usize> = HashMap::new();
            for l in &dev.others {
                if let Some(s) = prj.pages[l.page].symbols.iter().find(|s| s.id == l.sym_id) {
                    let c = seen.entry(s.label.clone()).or_default();
                    *c += 1;
                    if *c == 2 {
                        issue(Severity::Warning, format!("{tag}: designation '{}' used more than once", s.label), t(l));
                    }
                }
            }
        }
        let data = prj.devices.get(tag);
        for r in data.map(|d| d.parts.as_slice()).unwrap_or(&[]) {
            if !prj.parts.iter().any(|p| p.number == r.number) {
                let l = dev.mains.first().or(dev.others.first()).or(dev.auxs.first());
                issue(
                    Severity::Error,
                    format!("{tag}: part {} is not in the project parts (Parts management › synchronize)", r.number),
                    l.and_then(t),
                );
            }
        }
        // Function templates: more functions placed than the parts provide?
        let usage = crate::parts::usage(prj, None, tag);
        let has_templates = usage.values().any(|(_, avail)| *avail > 0);
        if has_templates {
            for (kind, (placed, avail)) in &usage {
                if *kind != crate::parts::FnKind::Terminal && placed > avail {
                    let l = dev.mains.first().or(dev.auxs.first()).or(dev.others.first());
                    issue(
                        Severity::Warning,
                        format!("{tag}: {placed}× {} placed, assigned parts provide {avail}", kind.title()),
                        l.and_then(t),
                    );
                }
            }
        }
        let has_part = data.map(|d| !d.parts.is_empty()).unwrap_or(false);
        if !has_part && (!dev.mains.is_empty() || !dev.others.is_empty()) {
            let l = dev.mains.first().or(dev.others.first()).unwrap();
            issue(Severity::Info, format!("{tag}: no part assigned"), t(l));
        }
    }
    for (name, locs) in &idx.interruptions {
        if locs.len() == 1 {
            issue(
                Severity::Warning,
                format!("Interruption point '{name}' has no counterpart"),
                Some((locs[0].page, locs[0].sym_id)),
            );
        }
    }
    for (pi, pg) in prj.pages.iter().enumerate() {
        for s in &pg.symbols {
            let Some(def) = symbols::get(&s.sym) else {
                issue(Severity::Error, format!("Unknown symbol '{}'", s.sym), Some((pi, s.id)));
                continue;
            };
            if def.func.has_tag() && s.tag.trim().is_empty() {
                issue(Severity::Error, format!("{} without device tag on page {}", def.name, pg.name), Some((pi, s.id)));
            }
        }
    }
    let mut unconnected: Vec<_> = nl.unconnected.clone();
    unconnected.sort();
    for (pi, sid, pin) in unconnected {
        if let Some((si, s)) = prj.pages[pi].symbols.iter().enumerate().find(|(_, s)| s.id == sid) {
            let d = pin_designation(prj, PinRef { page: pi, sym: si, pin });
            issue(
                Severity::Warning,
                format!("Open connection point {d} ({})", loc_str(prj, pi, s.pos)),
                Some((pi, sid)),
            );
        }
    }
    for net in &nl.nets {
        if net.name.contains(" / ") {
            let target = net.pins.first().map(|p| (p.page, prj.pages[p.page].symbols[p.sym].id));
            issue(Severity::Error, format!("Short circuit between potentials {}", net.name), target);
        }
    }
    v.sort_by_key(|i| i.severity);
    v
}
