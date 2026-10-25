//! Times the analysis and the checks of a large project made of copies of the demo
//! schematic pages: `cargo run --release -p openecraft-api --example scale -- 1000`.

use openecraft_api::Plugins;
use openecraft_core::analysis::{Analysis, Cache};
use openecraft_core::model::{PageKind, Project};
use std::time::Instant;

fn main() {
    let pages: usize = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(1000);
    let prj = large_project(pages);
    let wires: usize = prj.pages.iter().map(|p| p.wires.len()).sum();
    let symbols: usize = prj.pages.iter().map(|p| p.symbols.len()).sum();
    println!("{} pages, {symbols} symbols, {wires} wires", prj.pages.len());
    let plugins = Plugins::new(Vec::<String>::new());

    let mut cache = Cache::default();
    let t = Instant::now();
    let an = Analysis::build_with(&prj, &mut cache);
    println!("analysis: {:?}", t.elapsed());
    profile(&prj);
    let t = Instant::now();
    let issues = plugins.check(&prj, &an);
    println!("checks (parallel): {:?} ({} issues)", t.elapsed(), issues.len());
    let t = Instant::now();
    for c in plugins.checks() {
        let t = Instant::now();
        let _ = (c.run)(&prj, &an);
        let d = t.elapsed();
        if d.as_millis() >= 5 {
            println!("  {}: {d:?}", c.name);
        }
    }
    println!("checks (one after the other): {:?}", t.elapsed());

    // An edit on one page: only that page is scanned again.
    let mut changed = prj.clone();
    let page = changed.pages.iter().position(|p| p.kind == PageKind::Schematic && !p.wires.is_empty()).unwrap_or(0);
    changed.pages[page].wires.pop();
    let t = Instant::now();
    let an = Analysis::build_with(&changed, &mut cache);
    println!("analysis after a change of one page: {:?} ({} page scans)", t.elapsed(), cache.scanned());
    let t = Instant::now();
    plugins.check(&changed, &an);
    println!("checks after a change of one page: {:?}", t.elapsed());
    let mut scans = openecraft_core::netlist::Scans::default();
    openecraft_core::netlist::Netlist::build_with(&prj, true, &mut scans);
    let t = Instant::now();
    openecraft_core::netlist::Netlist::build_with(&changed, true, &mut scans);
    println!("  netlist after a change of one page: {:?}", t.elapsed());
}

fn profile(prj: &Project) {
    use openecraft_core::{netlist, panel};
    let step = |name: &str, t: Instant| println!("  {name}: {:?}", t.elapsed());
    let t = Instant::now();
    let _ = openecraft_core::index::Index::build(prj);
    step("index", t);
    let t = Instant::now();
    let nl = netlist::Netlist::build(prj, true);
    step("netlist", t);
    let t = Instant::now();
    let open = netlist::Netlist::build(prj, false);
    step("open netlist", t);
    let t = Instant::now();
    let conns = netlist::connections(prj, &nl, &open);
    step("connections", t);
    let t = Instant::now();
    let (mut wiring, _) = panel::virtual_wiring(prj, &conns);
    step("panel wiring", t);
    let t = Instant::now();
    let _ = openecraft_core::routing3d::analyse(prj, &mut wiring, &conns, &open);
    step("routing", t);
    let t = Instant::now();
    let _ = openecraft_core::field::plan(prj, &conns);
    step("field", t);
    let t = Instant::now();
    let _ = openecraft_core::revision::pending(prj);
    step("revision", t);
}

/// The demo project with its schematic pages copied until it has `pages` pages;
/// every copy gets new ids and device tags of its own (not placed in the panel).
fn large_project(pages: usize) -> Project {
    let mut prj = openecraft_core::sample::demo_project();
    let templates: Vec<_> = prj.pages.iter().filter(|p| p.kind == PageKind::Schematic).cloned().collect();
    let mut n = 0;
    while prj.pages.len() < pages {
        n += 1;
        for t in &templates {
            let mut p = t.clone();
            p.id = prj.new_id();
            p.name = format!("{}.{n}", t.name);
            p.function = format!("C{n}");
            for s in &mut p.symbols {
                s.id = prj.new_id();
                if s.tag.starts_with('-') {
                    s.tag = format!("=C{n}{}", s.tag);
                }
            }
            for w in &mut p.wires {
                w.id = prj.new_id();
            }
            prj.pages.push(p);
        }
    }
    prj.normalize();
    prj
}
