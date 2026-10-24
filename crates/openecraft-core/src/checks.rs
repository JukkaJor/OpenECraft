//! The built-in project checks ("Prüfläufe"): finds typical engineering mistakes.
//! Every check is a registry entry ([`Check`]) with a snake_case name and a title, so
//! the front ends list and run them like the checks of plugins; [`builtin`] gives the
//! list, [`run_builtin`] the issues of all of them.

use openecraft_i18n::{tr, tr_const, tr_fmt};
use crate::ext::Check;
use crate::index::Index;
use crate::model::*;
use crate::netlist::{pin_designation, Netlist, PinRef};
use crate::panel::{self, RoutedWire};
use crate::reports::{fill_spot, loc_str, Analysis, Issue, Severity};
use crate::symbols::{self, Func};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

fn entry(name: &'static str, title: &'static str, run: impl Fn(&Project, &Analysis) -> Vec<Issue> + Send + Sync + 'static) -> Check {
    Check { name: name.into(), title: title.into(), run: Arc::new(move |prj, an| Ok(run(prj, an))) }
}

/// The built-in checks in the order their issues are listed.
pub fn builtin() -> Vec<Check> {
    vec![
        entry("devices", tr_const!("Devices and connection points"), |prj, an| check(prj, &an.index, &an.netlist)),
        entry("mounting_pages", tr_const!("Mounting layout pages"), |prj, _| check_layout(prj)),
        entry("wires", tr_const!("Cables and wire numbers"), |prj, an| check_wires(prj, &an.netlist, &an.open)),
        entry("potentials", tr_const!("Potential definitions"), |prj, an| check_potentials(prj, &an.netlist, &an.open)),
        entry("plugs", tr_const!("Plugs"), |prj, _| crate::plugs::problems(prj).into_iter().map(|(message, target)| Issue { severity: Severity::Warning, message, target }).collect()),
        entry("terminal_strips_and_plc", tr_const!("Terminal strips and PLC addresses"), |prj, an| check_strips_and_plc(prj, &an.netlist)),
        entry("plc_hardware", tr_const!("PLC hardware"), |prj, _| crate::plc_hw::problems(prj)),
        entry("duct_fill", tr_const!("Duct fill levels"), |prj, an| check_duct_fill(prj, &an.duct_fill)),
        entry("climate", tr_const!("Enclosure climate"), |prj, _| check_climate(prj)),
        entry("connection_definitions", tr_const!("Connection definitions"), |prj, an| crate::conndef::problems(prj, &an.open)),
        entry("cable_definitions", tr_const!("Cable definitions"), |prj, _| crate::cabledef::problems(prj)),
        entry("cable_sizing", tr_const!("Cable sizing"), |prj, an| crate::sizing::problems(prj, &an.netlist, &an.open, &an.connections)),
        entry("short_circuit", tr_const!("Short circuit and disconnection"), |prj, an| crate::shortcircuit::problems(prj, &an.netlist, &an.connections)),
        entry("loads", tr_const!("Loads and power supplies"), |prj, an| crate::loads::problems(prj, &an.netlist)),
        entry("emc", tr_const!("EMC"), |prj, an| crate::emc::problems(prj, &an.netlist, &an.connections)),
        entry("safety", tr_const!("Safety functions"), |prj, _| crate::safety::problems(prj)),
        entry("rule_sets", tr_const!("Rule set verification"), |prj, an| crate::rules::problems(prj, &an.netlist, &an.connections)),
        entry("fluid_and_process", tr_const!("Fluid power and process"), |prj, an| check_fluid_and_process(prj, &an.index, &an.open)),
        entry("single_line", tr_const!("Single-line / multi-line"), |prj, an| {
            crate::singleline::problems(prj, &an.index, &an.netlist).into_iter().map(|(severity, message, target)| Issue { severity, message, target }).collect()
        }),
        entry("wiring_marks", tr_const!("Wiring marks"), |prj, an| check_wiring_marks(prj, &an.wiring)),
        entry("integration", tr_const!("Enterprise integration"), |prj, _| crate::integration::problems(prj)),
        entry("field_wiring", tr_const!("Field wiring"), |_, an| an.field.issues.clone()),
        entry("routing_3d", tr_const!("3D routing"), |_, an| check_routing(&an.routing)),
        entry("structure", tr_const!("Structure identifiers"), |prj, _| {
            crate::structure::problems(prj).into_iter().map(|message| Issue { severity: Severity::Warning, message, target: None }).collect()
        }),
        entry("translations", tr_const!("Project languages"), |prj, _| check_translations(prj)),
        entry("harnesses", tr_const!("Harnesses"), |prj, an| check_harnesses(prj, an)),
    ]
}

/// Issues of all built-in checks, errors first.
pub fn run_builtin(prj: &Project, an: &Analysis) -> Vec<Issue> {
    let mut issues: Vec<Issue> = builtin().iter().flat_map(|c| (c.run)(prj, an).unwrap_or_default()).collect();
    issues.sort_by_key(|i| i.severity);
    issues
}

/// Problems of the 3D routing and frozen routes whose length changed.
fn check_routing(routing: &crate::routing3d::Routing) -> Vec<Issue> {
    let mut v: Vec<Issue> = routing
        .problems
        .iter()
        .map(|p| {
            let severity = match p.level {
                crate::routing3d::Level::Error => Severity::Error,
                crate::routing3d::Level::Warning => Severity::Warning,
                crate::routing3d::Level::Info => Severity::Info,
            };
            Issue { severity, message: p.message.clone(), target: None }
        })
        .collect();
    if !routing.changes.is_empty() {
        v.push(Issue { severity: Severity::Info, message: tr_fmt!("{} frozen routes changed their length", routing.changes.len()), target: None });
    }
    v
}

/// Project texts without a translation into a target language.
fn check_translations(prj: &Project) -> Vec<Issue> {
    let mut v = Vec::new();
    for lang in &prj.languages.targets {
        let n = crate::translation::missing(prj, lang).len();
        if n > 0 {
            v.push(Issue { severity: Severity::Info, message: tr_fmt!("{} texts are not translated into {}", n, crate::translation::language_name(lang)), target: None });
        }
    }
    v
}

/// Project check ("Prüfläufe"): finds typical engineering mistakes.
pub fn check(prj: &Project, idx: &Index, nl: &Netlist) -> Vec<Issue> {
    let mut v = Vec::new();
    let mut issue = |severity, message: String, target| v.push(Issue { severity, message, target });
    for tag in idx.sorted_tags() {
        let dev = &idx.devices[tag];
        let t = |l: &crate::index::Loc| Some((l.page, l.sym_id));
        if dev.mains.is_empty() && !dev.auxs.is_empty() {
            issue(Severity::Error, tr_fmt!("{}: auxiliary function(s) without main function", tag), t(&dev.auxs[0]));
        }
        if dev.mains.len() > 1 {
            issue(Severity::Error, tr_fmt!("{}: main function placed {} times", tag, dev.mains.len()), t(&dev.mains[1]));
        }
        if !dev.others.is_empty() {
            // Duplicate terminal numbers / PLC addresses (plug pins: see `plugs::problems`)
            let mut seen: HashMap<String, usize> = HashMap::new();
            for l in &dev.others {
                let s = prj.pages[l.page].symbols.iter().find(|s| s.id == l.sym_id);
                if let Some(s) = s.filter(|s| !matches!(symbols::get(&s.sym).map(|d| d.func), Some(Func::Plug(_)))) {
                    let c = seen.entry(s.label.clone()).or_default();
                    *c += 1;
                    if *c == 2 {
                        issue(Severity::Warning, tr_fmt!("{}: designation '{}' used more than once", tag, s.label), t(l));
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
                    tr_fmt!("{}: part {} is not in the project parts (Parts management › synchronize)", tag, r.number),
                    l.and_then(t),
                );
            }
        }
        // Function templates: more functions placed than the parts provide?
        let usage = crate::parts::usage(prj, None, tag);
        let has_templates = usage.values().any(|(_, avail)| *avail > 0);
        if has_templates {
            for (kind, (placed, avail)) in &usage {
                if !matches!(kind, crate::parts::FnKind::Terminal | crate::parts::FnKind::PlugPin) && placed > avail {
                    let l = dev.mains.first().or(dev.auxs.first()).or(dev.others.first());
                    issue(
                        Severity::Warning,
                        tr_fmt!("{}: {}× {} placed, assigned parts provide {}", tag, placed, tr(kind.title()), avail),
                        l.and_then(t),
                    );
                }
            }
        }
        let has_part = data.map(|d| !d.parts.is_empty()).unwrap_or(false);
        if !has_part && (!dev.mains.is_empty() || !dev.others.is_empty()) {
            let l = dev.mains.first().or(dev.others.first()).unwrap();
            issue(Severity::Info, tr_fmt!("{}: no part assigned", tag), t(l));
        }
    }
    for (name, locs) in &idx.interruptions {
        if locs.len() == 1 {
            issue(
                Severity::Warning,
                tr_fmt!("Interruption point '{}' has no counterpart", name),
                Some((locs[0].page, locs[0].sym_id)),
            );
        }
    }
    for (pi, pg) in prj.pages.iter().enumerate() {
        for s in &pg.symbols {
            let Some(def) = symbols::get(&s.sym) else {
                issue(Severity::Error, tr_fmt!("Unknown symbol '{}'", s.sym), Some((pi, s.id)));
                continue;
            };
            if def.func.has_tag() && s.tag.trim().is_empty() {
                issue(Severity::Error, tr_fmt!("{} without device tag on page {}", def.name, pg.name), Some((pi, s.id)));
            }
        }
    }
    let mut unconnected: Vec<_> = nl.unconnected.clone();
    unconnected.sort();
    for (pi, sid, pin) in unconnected {
        if let Some((si, s)) = prj.pages[pi].symbols.iter().enumerate().find(|(_, s)| s.id == sid) {
            if crate::fluid::vents(prj, PinRef { page: pi, sym: si, pin }) {
                continue; // reported by the fluid check
            }
            let d = pin_designation(prj, PinRef { page: pi, sym: si, pin });
            issue(
                Severity::Warning,
                tr_fmt!("Open connection point {} ({})", d, loc_str(prj, pi, s.pos)),
                Some((pi, sid)),
            );
        }
    }
    for net in &nl.nets {
        if net.name.contains(" / ") {
            let target = net.pins.first().map(|p| (p.page, prj.pages[p.page].symbols[p.sym].id));
            issue(Severity::Error, tr_fmt!("Short circuit between potentials {}", net.name), target);
        }
    }
    v.sort_by_key(|i| i.severity);
    v
}

/// Panel layout checks: enclosures of mounting pages and unplaced devices; the
/// checks of the layout itself are the built-in plugin `panel_checks` (openecraft-api).
fn check_layout(prj: &Project) -> Vec<Issue> {
    let mut v = Vec::new();
    for (pi, pg) in prj.pages.iter().enumerate() {
        if let PageKind::Mounting(id) = pg.kind {
            if !prj.layout.enclosures.iter().any(|e| e.id == id) {
                v.push(Issue { severity: Severity::Warning, message: tr_fmt!("Page {}: its enclosure no longer exists", pg.name), target: Some((pi, 0)) });
            }
        }
    }
    if prj.layout.enclosures.is_empty() {
        return v;
    }
    for d in panel::devices_to_place(prj) {
        let target = prj.pages.iter().enumerate().find_map(|(pi, pg)| pg.symbols.iter().find(|s| s.tag == d.tag).map(|s| (pi, s.id)));
        v.push(Issue { severity: Severity::Info, message: tr_fmt!("{}: not placed in the panel layout", d.tag), target });
    }
    v
}

/// Cable assignments, wire numbers and device tags.
fn check_wires(prj: &Project, nl: &Netlist, open: &Netlist) -> Vec<Issue> {
    let mut v = Vec::new();
    let mut issue = |severity, message: String, target| v.push(Issue { severity, message, target });
    let mut tags: HashMap<&str, usize> = HashMap::new();
    for c in &prj.cables {
        *tags.entry(c.tag.as_str()).or_default() += 1;
        if c.tag.trim().is_empty() {
            issue(Severity::Error, tr("Cable without tag").into(), None);
        }
    }
    for (tag, n) in tags.iter().filter(|(t, n)| **n > 1 && !t.is_empty()) {
        issue(Severity::Error, tr_fmt!("Cable {} defined {} times", tag, n), None);
    }
    // Cable cores: known cable, listed core, one connection per core.
    let mut core_use: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (ni, net) in open.nets.iter().enumerate() {
        for (pi, id) in &net.wires {
            let Some(w) = prj.pages[*pi].wires.iter().find(|w| w.id == *id) else { continue };
            if w.cable.is_empty() {
                continue;
            }
            let at = Some((*pi, w.id));
            match prj.cable(&w.cable) {
                None => issue(Severity::Error, tr_fmt!("Wire assigned to unknown cable {}", w.cable), at),
                Some(_) if w.core.is_empty() => issue(Severity::Warning, tr_fmt!("{}: wire without core designation", w.cable), at),
                Some(c) if !c.core_list().contains(&w.core) => {
                    issue(Severity::Warning, tr_fmt!("{}: core {} is not a core of the cable ({})", w.cable, w.core, c.cores), at)
                }
                Some(_) => {}
            }
            let users = core_use.entry((w.cable.clone(), w.core.clone())).or_default();
            if !users.contains(&ni) {
                users.push(ni);
            }
        }
    }
    for ((cable, core), nets) in &core_use {
        if nets.len() > 1 && !core.is_empty() {
            let at = open.nets[nets[1]].wires.first().copied();
            issue(Severity::Error, tr_fmt!("{}: core {} used for {} connections", cable, core, nets.len()), at);
        }
    }
    // Wire numbers: one per connection, not shared by different potentials.
    let mut by_number: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    for net in &open.nets {
        let numbers = crate::wirenum::numbers_of(prj, net);
        let at = net.wires.first().copied();
        if numbers.len() > 1 {
            issue(Severity::Warning, tr_fmt!("Different wire numbers in one connection: {}", numbers.join(", ")), at);
        }
        let potential = net.pins.first().and_then(|p| nl.pin_net.get(&(prj.pages[p.page].symbols[p.sym].id, p.pin)).copied());
        if let (Some(n), Some(pot)) = (numbers.first(), potential) {
            by_number.entry(n.clone()).or_default().insert(pot);
        }
    }
    for (number, pots) in by_number.iter().filter(|(_, p)| p.len() > 1) {
        issue(Severity::Warning, tr_fmt!("Wire number {} is used for {} different potentials", number, pots.len()), None);
    }
    // Device tags need a device name after the structure.
    for (pi, pg) in prj.pages.iter().enumerate() {
        for s in pg.symbols.iter().filter(|s| !s.tag.is_empty()) {
            if crate::structure::Dt::parse(&s.tag).name.is_empty() {
                issue(Severity::Error, tr_fmt!("{}: device tag without device name", s.tag), Some((pi, s.id)));
            }
        }
    }
    v
}

/// Potential definitions and the colours / cross-sections of the connections.
fn check_potentials(prj: &Project, nl: &Netlist, open: &Netlist) -> Vec<Issue> {
    use crate::potentials::{is_protective, normalize_color, props_of, Source};
    let mut v = Vec::new();
    let mut issue = |severity, message: String, target| v.push(Issue { severity, message, target });
    let mut names: HashMap<&str, usize> = HashMap::new();
    for d in &prj.potentials {
        *names.entry(d.name.as_str()).or_default() += 1;
    }
    for (name, n) in names.iter().filter(|(_, n)| **n > 1) {
        issue(Severity::Error, tr_fmt!("Potential {} defined {} times", name, n), None);
    }
    if !prj.potentials.is_empty() {
        let mut undefined: BTreeMap<String, (usize, u64)> = BTreeMap::new();
        for (pi, pg) in prj.pages.iter().enumerate().filter(|(_, p)| p.kind == PageKind::Schematic) {
            for s in &pg.symbols {
                let pot = symbols::get(&s.sym).map(|d| d.func == Func::Potential).unwrap_or(false);
                if pot && !s.label.is_empty() && prj.potential(&s.label).is_none() {
                    undefined.entry(s.label.clone()).or_insert((pi, s.id));
                }
            }
        }
        for (name, at) in undefined {
            issue(Severity::Info, tr_fmt!("Potential {} has no definition (Project › Potentials)", name), Some(at));
        }
    }
    for net in open.nets.iter().filter(|n| !n.fluid) {
        let Some(&(pi, wid)) = net.wires.first() else { continue };
        let at = Some((pi, wid));
        let p = props_of(prj, nl, net);
        if p.potential.contains(" / ") || crate::wirenum::wires_of(prj, net).iter().any(|w| !w.cable.is_empty()) {
            continue;
        }
        let color = normalize_color(&p.color);
        if is_protective(prj, &p.potential) {
            if !color.is_empty() && color != "GNYE" {
                issue(Severity::Error, tr_fmt!("Protective conductor {} with wire colour {} (must be GNYE)", p.potential, p.color), at);
            }
        } else if color == "GNYE" {
            let name = if p.potential.is_empty() { net.name.clone() } else { p.potential.clone() };
            issue(Severity::Error, tr_fmt!("Wire colour GNYE is reserved for protective conductors ({})", name), at);
        }
        let Some(def) = prj.potential(&p.potential) else { continue };
        if p.color_source == Source::Manual && !def.color.is_empty() && color != normalize_color(&def.color) {
            issue(Severity::Warning, tr_fmt!("{}: wire colour {} differs from the potential definition ({})", def.name, p.color, def.color), at);
        }
        let mm2 = crate::production::cross_section_mm2;
        if p.cross_section_source == Source::Manual && !def.cross_section.is_empty() && mm2(&p.cross_section) != mm2(&def.cross_section) {
            issue(
                Severity::Warning,
                tr_fmt!("{}: cross-section {} differs from the potential definition ({})", def.name, p.cross_section, def.cross_section),
                at,
            );
        }
    }
    v
}

/// Jumpers on missing terminals, PLC addresses used on several cards.
fn check_strips_and_plc(prj: &Project, nl: &Netlist) -> Vec<Issue> {
    let mut v = Vec::new();
    for (strip, data) in &prj.strips {
        let labels: Vec<String> = crate::strips::terminals(prj, strip).into_iter().map(|t| t.label).collect();
        for (label, j) in data.jumpers.iter().filter(|(_, j)| **j > 0) {
            if !labels.contains(label) {
                v.push(Issue { severity: Severity::Warning, message: tr_fmt!("{}: jumper {} on terminal {}, which is not placed", strip, j, label), target: None });
            }
        }
        for j in crate::strips::strip_jumpers(prj, strip) {
            let Some(message) = j.problem_text(strip) else { continue };
            let severity = if j.problem == Some(crate::strips::JumperProblem::NoPart) { Severity::Info } else { Severity::Warning };
            v.push(Issue { severity, message, target: None });
        }
    }
    for (addr, points) in crate::plc::duplicate_addresses(prj, nl) {
        let mut cards: Vec<&str> = points.iter().map(|p| p.card.as_str()).collect();
        cards.dedup();
        if cards.len() > 1 {
            v.push(Issue {
                severity: Severity::Error,
                message: tr_fmt!("PLC address {} used on several cards: {}", addr, cards.join(", ")),
                target: Some((points[1].page, points[1].id)),
            });
        }
    }
    v
}

/// Ducts above the fill limit (warning) or close to it (above 80 % of it, hint).
fn check_duct_fill(prj: &Project, fill: &[panel::DuctFill]) -> Vec<Issue> {
    let limit = prj.layout.duct_fill_limit;
    let mut v = Vec::new();
    for f in fill {
        let severity = if f.percent > limit {
            Severity::Warning
        } else if f.percent > 0.8 * limit {
            Severity::Info
        } else {
            continue;
        };
        let at = fill_spot(prj, f);
        let name = &prj.layout.enclosures[f.enclosure].name;
        let message = tr_fmt!(
            "{}: cable duct at {} mm filled {} % (limit {} %)",
            name,
            format!("{:.0} / {:.0}", at.x, at.y),
            format!("{:.0}", f.percent),
            format!("{limit:.0}")
        );
        v.push(Issue { severity, message, target: None });
    }
    v
}

/// Enclosures that get too warm without cooling.
fn check_climate(prj: &Project) -> Vec<Issue> {
    use crate::climate::Cooling;
    let mut v = Vec::new();
    for e in &prj.layout.enclosures {
        let r = crate::climate::calculate(prj, e);
        if r.cooling != Cooling::None {
            v.push(Issue {
                severity: Severity::Warning,
                message: tr_fmt!(
                    "{}: {} W power loss heat the enclosure to {} °C (highest {} °C): {} needed",
                    e.name,
                    format!("{:.0}", r.loss),
                    format!("{:.0}", r.inside),
                    format!("{:.0}", e.climate.inside_max),
                    r.cooling.text()
                ),
                target: None,
            });
        }
    }
    v
}

/// Hoses, fluid device groups, electro-fluid links, P&ID instruments and PCT loops of
/// the pre-planning.
fn check_fluid_and_process(prj: &Project, idx: &Index, open: &Netlist) -> Vec<Issue> {
    let fluid = crate::fluid::problems(prj, open).into_iter().chain(crate::electrofluid::problems(prj, idx)).map(|(severity, message, target)| Issue { severity, message, target });
    let process = crate::process::problems(prj, idx).into_iter().map(|(message, warning, target)| Issue { severity: if warning { Severity::Warning } else { Severity::Info }, message, target });
    fluid.chain(process).collect()
}

/// Wires changed after they were wired, and problems reported from the workshop.
fn check_wiring_marks(prj: &Project, wiring: &[RoutedWire]) -> Vec<Issue> {
    let mut v = Vec::new();
    for w in wiring {
        match crate::wiring::status(prj, w) {
            crate::wiring::Status::Changed => v.push(Issue { severity: Severity::Warning, message: tr_fmt!("Wire {} – {} changed after it was wired", w.from, w.to), target: None }),
            crate::wiring::Status::Problem => {
                let note = crate::wiring::mark_of(prj, w).map(|m| m.note.clone()).unwrap_or_default();
                v.push(Issue { severity: Severity::Warning, message: tr_fmt!("Wiring problem {} – {}: {}", w.from, w.to, note), target: None });
            }
            _ => {}
        }
    }
    let stale = crate::wiring::stale(prj, wiring).len();
    if stale > 0 {
        v.push(Issue { severity: Severity::Info, message: tr_fmt!("{} wiring marks belong to wires that no longer exist", stale), target: None });
    }
    v
}

/// Assigned connections that no longer exist, harnesses without wires.
fn check_harnesses(prj: &Project, an: &Analysis) -> Vec<Issue> {
    let mut v = Vec::new();
    for (tag, [a, b]) in crate::harness::stale(prj, an) {
        v.push(Issue { severity: Severity::Warning, message: tr_fmt!("Harness {}: connection {} – {} does not exist in the schematic", tag, a, b), target: None });
    }
    for h in &prj.harnesses {
        if !an.connections.iter().any(|c| c.harness == h.tag) {
            v.push(Issue { severity: Severity::Info, message: tr_fmt!("Harness {} has no wires", h.tag), target: None });
        }
    }
    v
}

#[cfg(test)]
mod tests;
