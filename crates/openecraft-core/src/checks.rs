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
use crate::reports::{fill_spot, loc_str};
use crate::analysis::Analysis;
use crate::issue::{Issue, Severity};
use crate::symbols::Func;
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
        entry("plugs", tr_const!("Plugs"), |prj, _| crate::plugs::problems(prj).into_iter().map(|(message, target)| Issue::plain(Severity::Warning, message, target)).collect()),
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
            crate::singleline::problems(prj, &an.index, &an.netlist).into_iter().map(|(severity, message, target)| Issue::plain(severity, message, target)).collect()
        }),
        entry("wiring_marks", tr_const!("Wiring marks"), |prj, an| check_wiring_marks(prj, &an.wiring)),
        entry("integration", tr_const!("Enterprise integration"), |prj, _| crate::integration::problems(prj)),
        entry("field_wiring", tr_const!("Field wiring"), |_, an| an.field.issues.clone()),
        entry("routing_3d", tr_const!("3D routing"), |_, an| check_routing(&an.routing)),
        entry("structure", tr_const!("Structure identifiers"), |prj, _| {
            crate::structure::problems(prj).into_iter().map(|message| Issue::plain(Severity::Warning, message, None)).collect()
        }),
        entry("translations", tr_const!("Project languages"), |prj, _| check_translations(prj)),
        entry("harnesses", tr_const!("Harnesses"), check_harnesses),
    ]
}

/// Applies the project's check settings to the issues the checks found: changed
/// levels (`Off` removes the message) and accepted messages. The host skips disabled
/// checks before running them ([`is_enabled`]).
pub fn apply_settings(prj: &Project, issues: Vec<Issue>) -> Vec<Issue> {
    use crate::model::checks::Level;
    let s = &prj.check_settings;
    issues
        .into_iter()
        .filter_map(|mut i| {
            match s.levels.get(&i.code()) {
                Some(Level::Off) => return None,
                Some(Level::Error) => i.severity = Severity::Error,
                Some(Level::Warning) => i.severity = Severity::Warning,
                Some(Level::Info) => i.severity = Severity::Info,
                None => {}
            }
            if !s.accepted.is_empty() {
                let id = i.fingerprint();
                i.accepted = s.accepted.iter().any(|a| a.id == id);
            }
            Some(i)
        })
        .collect()
}

/// The check runs (not switched off in the project's check settings).
pub fn is_enabled(prj: &Project, check: &str) -> bool {
    !prj.check_settings.disabled.iter().any(|d| d == check)
}

/// Accepts a message as checked (or takes the acceptance back with `accept` false).
pub fn accept(prj: &mut Project, issue: &Issue, accept: bool, note: &str, user: &str, date: &str) {
    let id = issue.fingerprint();
    prj.check_settings.accepted.retain(|a| a.id != id);
    if accept {
        prj.check_settings.accepted.push(crate::model::checks::Accepted { id, message: issue.message(), note: note.into(), user: user.into(), date: date.into() });
    }
}

/// Accepted messages that no check reports any more (the object changed or the
/// problem was solved).
pub fn stale_accepted<'a>(prj: &'a Project, issues: &[Issue]) -> Vec<&'a crate::model::checks::Accepted> {
    let live: std::collections::HashSet<String> = issues.iter().map(Issue::fingerprint).collect();
    prj.check_settings.accepted.iter().filter(|a| !live.contains(&a.id)).collect()
}

/// The messages as CSV (`Severity;Code;Message;Page;Item;Accepted`).
pub fn issues_csv(prj: &Project, issues: &[Issue]) -> String {
    let cell = |s: &str| if s.contains([';', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
    let mut out = String::from("Severity;Code;Message;Page;Item;Accepted\n");
    for i in issues {
        let page = i.target.and_then(|t| prj.pages.get(t.0)).map(|p| p.name.clone()).unwrap_or_default();
        let item = i.target.map(|t| t.1).filter(|id| *id != 0).map(|id| id.to_string()).unwrap_or_default();
        out.push_str(&format!("{};{};{};{};{};{}\n", i.severity.key(), cell(&i.code()), cell(&i.message()), cell(&page), item, if i.accepted { "yes" } else { "" }));
    }
    out
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
            Issue::plain(severity, p.message.clone(), None)
        })
        .collect();
    if !routing.changes.is_empty() {
        v.push(crate::issue!(Severity::Info, None, "{} frozen routes changed their length", routing.changes.len()));
    }
    v
}

/// Project texts without a translation into a target language.
fn check_translations(prj: &Project) -> Vec<Issue> {
    let mut v = Vec::new();
    for lang in &prj.languages.targets {
        let n = crate::translation::missing(prj, lang).len();
        if n > 0 {
            v.push(crate::issue!(Severity::Info, None, "{} texts are not translated into {}", n, crate::translation::language_name(lang)));
        }
    }
    v
}

/// Project check ("Prüfläufe"): finds typical engineering mistakes.
pub fn check(prj: &Project, idx: &Index, nl: &Netlist) -> Vec<Issue> {
    let mut v = Vec::new();
    for tag in idx.sorted_tags() {
        let dev = &idx.devices[tag];
        let t = |l: &crate::index::Loc| Some((l.page, l.sym_id));
        if dev.mains.is_empty() && !dev.auxs.is_empty() {
            v.push(crate::issue!(Severity::Error, t(&dev.auxs[0]), "{}: auxiliary function(s) without main function", tag));
        }
        if dev.mains.len() > 1 {
            v.push(crate::issue!(Severity::Error, t(&dev.mains[1]), "{}: main function placed {} times", tag, dev.mains.len()));
        }
        if !dev.others.is_empty() {
            // Duplicate terminal numbers / PLC addresses (plug pins: see `plugs::problems`)
            let mut seen: HashMap<String, usize> = HashMap::new();
            for l in &dev.others {
                let s = prj.pages[l.page].symbols.iter().find(|s| s.id == l.sym_id);
                if let Some(s) = s.filter(|s| !matches!(prj.symbol(&s.sym).map(|d| d.func), Some(Func::Plug(_)))) {
                    let c = seen.entry(s.label.clone()).or_default();
                    *c += 1;
                    if *c == 2 {
                        v.push(crate::issue!(Severity::Warning, t(l), "{}: designation '{}' used more than once", tag, s.label));
                    }
                }
            }
        }
        let data = prj.devices.get(tag);
        for r in data.map(|d| d.parts.as_slice()).unwrap_or(&[]) {
            if !prj.parts.iter().any(|p| p.number == r.number) {
                let l = dev.mains.first().or(dev.others.first()).or(dev.auxs.first());
                v.push(crate::issue!(Severity::Error, l.and_then(t), "{}: part {} is not in the project parts (Parts management › synchronize)", tag, r.number));
            }
        }
        // Function templates: more functions placed than the parts provide?
        let usage = crate::parts::usage(prj, None, tag);
        let has_templates = usage.values().any(|(_, avail)| *avail > 0);
        if has_templates {
            for (kind, (placed, avail)) in &usage {
                if !matches!(kind, crate::parts::FnKind::Terminal | crate::parts::FnKind::PlugPin) && placed > avail {
                    let l = dev.mains.first().or(dev.auxs.first()).or(dev.others.first());
                    v.push(crate::issue!(Severity::Warning, l.and_then(t), "{}: {}× {} placed, assigned parts provide {}", tag, placed, tr(kind.title()), avail));
                }
            }
        }
        let has_part = data.map(|d| !d.parts.is_empty()).unwrap_or(false);
        if !has_part && (!dev.mains.is_empty() || !dev.others.is_empty()) {
            let l = dev.mains.first().or(dev.others.first()).unwrap();
            v.push(crate::issue!(Severity::Info, t(l), "{}: no part assigned", tag));
        }
    }
    for (name, locs) in &idx.interruptions {
        if locs.len() == 1 {
            v.push(crate::issue!(Severity::Warning, Some((locs[0].page, locs[0].sym_id)), "Interruption point '{}' has no counterpart", name));
        }
    }
    for (pi, pg) in prj.pages.iter().enumerate() {
        for s in &pg.symbols {
            let Some(def) = prj.symbol(&s.sym) else {
                v.push(crate::issue!(Severity::Error, Some((pi, s.id)), "Unknown symbol '{}'", s.sym));
                continue;
            };
            if def.func.has_tag() && s.tag.trim().is_empty() {
                v.push(crate::issue!(Severity::Error, Some((pi, s.id)), "{} without device tag on page {}", def.name, pg.name));
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
            v.push(crate::issue!(Severity::Warning, Some((pi, sid)), "Open connection point {} ({})", d, loc_str(prj, pi, s.pos)));
        }
    }
    for net in &nl.nets {
        if net.name.contains(" / ") {
            let target = net.pins.first().map(|p| (p.page, prj.pages[p.page].symbols[p.sym].id));
            v.push(crate::issue!(Severity::Error, target, "Short circuit between potentials {}", net.name));
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
                v.push(crate::issue!(Severity::Warning, Some((pi, 0)), "Page {}: its enclosure no longer exists", pg.name));
            }
        }
    }
    if prj.layout.enclosures.is_empty() {
        return v;
    }
    for d in panel::devices_to_place(prj) {
        let target = prj.pages.iter().enumerate().find_map(|(pi, pg)| pg.symbols.iter().find(|s| s.tag == d.tag).map(|s| (pi, s.id)));
        v.push(crate::issue!(Severity::Info, target, "{}: not placed in the panel layout", d.tag));
    }
    v
}

/// Cable assignments, wire numbers and device tags.
fn check_wires(prj: &Project, nl: &Netlist, open: &Netlist) -> Vec<Issue> {
    let mut v = Vec::new();
    let mut tags: HashMap<&str, usize> = HashMap::new();
    for c in &prj.cables {
        *tags.entry(c.tag.as_str()).or_default() += 1;
        if c.tag.trim().is_empty() {
            v.push(crate::issue!(Severity::Error, None, "Cable without tag"));
        }
    }
    for (tag, n) in tags.iter().filter(|(t, n)| **n > 1 && !t.is_empty()) {
        v.push(crate::issue!(Severity::Error, None, "Cable {} defined {} times", tag, n));
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
                None => v.push(crate::issue!(Severity::Error, at, "Wire assigned to unknown cable {}", w.cable)),
                Some(_) if w.core.is_empty() => v.push(crate::issue!(Severity::Warning, at, "{}: wire without core designation", w.cable)),
                Some(c) if !c.core_list().contains(&w.core) => {
                    v.push(crate::issue!(Severity::Warning, at, "{}: core {} is not a core of the cable ({})", w.cable, w.core, c.cores))
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
            v.push(crate::issue!(Severity::Error, at, "{}: core {} used for {} connections", cable, core, nets.len()));
        }
    }
    // Wire numbers: one per connection, not shared by different potentials.
    let mut by_number: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    for net in &open.nets {
        let numbers = crate::wirenum::numbers_of(prj, net);
        let at = net.wires.first().copied();
        if numbers.len() > 1 {
            v.push(crate::issue!(Severity::Warning, at, "Different wire numbers in one connection: {}", numbers.join(", ")));
        }
        let potential = net.pins.first().and_then(|p| nl.pin_net.get(&(prj.pages[p.page].symbols[p.sym].id, p.pin)).copied());
        if let (Some(n), Some(pot)) = (numbers.first(), potential) {
            by_number.entry(n.clone()).or_default().insert(pot);
        }
    }
    for (number, pots) in by_number.iter().filter(|(_, p)| p.len() > 1) {
        v.push(crate::issue!(Severity::Warning, None, "Wire number {} is used for {} different potentials", number, pots.len()));
    }
    // Device tags need a device name after the structure.
    for (pi, pg) in prj.pages.iter().enumerate() {
        for s in pg.symbols.iter().filter(|s| !s.tag.is_empty()) {
            if crate::structure::Dt::parse(&s.tag).name.is_empty() {
                v.push(crate::issue!(Severity::Error, Some((pi, s.id)), "{}: device tag without device name", s.tag));
            }
        }
    }
    v
}

/// Potential definitions and the colours / cross-sections of the connections.
fn check_potentials(prj: &Project, nl: &Netlist, open: &Netlist) -> Vec<Issue> {
    use crate::potentials::{is_protective, normalize_color, props_of, Source};
    let mut v = Vec::new();
    let mut names: HashMap<&str, usize> = HashMap::new();
    for d in &prj.potentials {
        *names.entry(d.name.as_str()).or_default() += 1;
    }
    for (name, n) in names.iter().filter(|(_, n)| **n > 1) {
        v.push(crate::issue!(Severity::Error, None, "Potential {} defined {} times", name, n));
    }
    if !prj.potentials.is_empty() {
        let mut undefined: BTreeMap<String, (usize, u64)> = BTreeMap::new();
        for (pi, pg) in prj.pages.iter().enumerate().filter(|(_, p)| p.kind == PageKind::Schematic) {
            for s in &pg.symbols {
                let pot = prj.symbol(&s.sym).map(|d| d.func == Func::Potential).unwrap_or(false);
                if pot && !s.label.is_empty() && prj.potential(&s.label).is_none() {
                    undefined.entry(s.label.clone()).or_insert((pi, s.id));
                }
            }
        }
        for (name, at) in undefined {
            v.push(crate::issue!(Severity::Info, Some(at), "Potential {} has no definition (Project › Potentials)", name));
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
                v.push(crate::issue!(Severity::Error, at, "Protective conductor {} with wire colour {} (must be GNYE)", p.potential, p.color));
            }
        } else if color == "GNYE" {
            let name = if p.potential.is_empty() { net.name.clone() } else { p.potential.clone() };
            v.push(crate::issue!(Severity::Error, at, "Wire colour GNYE is reserved for protective conductors ({})", name));
        }
        let Some(def) = prj.potential(&p.potential) else { continue };
        if p.color_source == Source::Manual && !def.color.is_empty() && color != normalize_color(&def.color) {
            v.push(crate::issue!(Severity::Warning, at, "{}: wire colour {} differs from the potential definition ({})", def.name, p.color, def.color));
        }
        let mm2 = crate::production::cross_section_mm2;
        if p.cross_section_source == Source::Manual && !def.cross_section.is_empty() && mm2(&p.cross_section) != mm2(&def.cross_section) {
            v.push(crate::issue!(Severity::Warning, at, "{}: cross-section {} differs from the potential definition ({})", def.name, p.cross_section, def.cross_section));
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
                v.push(crate::issue!(Severity::Warning, None, "{}: jumper {} on terminal {}, which is not placed", strip, j, label));
            }
        }
        for j in crate::strips::strip_jumpers(prj, strip) {
            let Some(message) = j.problem_text(strip) else { continue };
            let severity = if j.problem == Some(crate::strips::JumperProblem::NoPart) { Severity::Info } else { Severity::Warning };
            v.push(Issue::plain(severity, message, None));
        }
    }
    for (addr, points) in crate::plc::duplicate_addresses(prj, nl) {
        let mut cards: Vec<&str> = points.iter().map(|p| p.card.as_str()).collect();
        cards.dedup();
        if cards.len() > 1 {
            v.push(crate::issue!(Severity::Error, Some((points[1].page, points[1].id)), "PLC address {} used on several cards: {}", addr, cards.join(", ")));
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
        v.push(Issue::plain(severity, message, None));
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
            v.push(crate::issue!(Severity::Warning, None, "{}: {} W power loss heat the enclosure to {} °C (highest {} °C): {} needed", e.name, format!("{:.0}", r.loss), format!("{:.0}", r.inside), format!("{:.0}", e.climate.inside_max), r.cooling.text()));
        }
    }
    v
}

/// Hoses, fluid device groups, electro-fluid links, P&ID instruments and PCT loops of
/// the pre-planning.
fn check_fluid_and_process(prj: &Project, idx: &Index, open: &Netlist) -> Vec<Issue> {
    let fluid = crate::fluid::problems(prj, open).into_iter().chain(crate::electrofluid::problems(prj, idx)).map(|(severity, message, target)| Issue::plain(severity, message, target));
    let process = crate::process::problems(prj, idx).into_iter().map(|(message, warning, target)| Issue::plain(if warning { Severity::Warning } else { Severity::Info }, message, target));
    fluid.chain(process).collect()
}

/// Wires changed after they were wired, and problems reported from the workshop.
fn check_wiring_marks(prj: &Project, wiring: &[RoutedWire]) -> Vec<Issue> {
    let mut v = Vec::new();
    for w in wiring {
        match crate::wiring::status(prj, w) {
            crate::wiring::Status::Changed => v.push(crate::issue!(Severity::Warning, None, "Wire {} – {} changed after it was wired", w.from, w.to)),
            crate::wiring::Status::Problem => {
                let note = crate::wiring::mark_of(prj, w).map(|m| m.note.clone()).unwrap_or_default();
                v.push(crate::issue!(Severity::Warning, None, "Wiring problem {} – {}: {}", w.from, w.to, note));
            }
            _ => {}
        }
    }
    let stale = crate::wiring::stale(prj, wiring).len();
    if stale > 0 {
        v.push(crate::issue!(Severity::Info, None, "{} wiring marks belong to wires that no longer exist", stale));
    }
    v
}

/// Assigned connections that no longer exist, harnesses without wires.
fn check_harnesses(prj: &Project, an: &Analysis) -> Vec<Issue> {
    let mut v = Vec::new();
    for (tag, [a, b]) in crate::harness::stale(prj, an) {
        v.push(crate::issue!(Severity::Warning, None, "Harness {}: connection {} – {} does not exist in the schematic", tag, a, b));
    }
    for h in &prj.harnesses {
        if !an.connections.iter().any(|c| c.harness == h.tag) {
            v.push(crate::issue!(Severity::Info, None, "Harness {} has no wires", h.tag));
        }
    }
    v
}

#[cfg(test)]
mod tests;
