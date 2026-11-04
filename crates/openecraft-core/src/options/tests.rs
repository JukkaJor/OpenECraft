use super::*;
use crate::config::ConfigOption;
use crate::index::Index;
use crate::sample::demo_project;

fn with_pages(n: usize) -> Project {
    let mut prj = Project::new_default();
    for i in 1..n {
        let id = prj.new_id();
        prj.pages.push(Page::new(id, &(i + 1).to_string(), "Schematic", PageKind::Schematic));
    }
    prj
}

#[test]
fn inactive_content_is_taken_out_and_put_back() {
    let mut prj = demo_project();
    let before = prj.clone();
    let (pi, tag) = prj.pages.iter().enumerate().find_map(|(i, p)| p.symbols.iter().find(|s| !s.tag.is_empty()).map(|s| (i, s.tag.clone()))).unwrap();
    let ids: Vec<u64> = prj.pages[pi].symbols.iter().filter(|s| s.tag == tag).map(|s| s.id).collect();
    let wire = prj.pages[pi].wires[0].id;
    define(&mut prj, "Heating", "Optional heater", "").unwrap();
    assert_eq!(assign(&mut prj, "Heating", &[ids.clone(), vec![wire]].concat()).unwrap(), ids.len() + 1);
    assert_eq!(prj.model_options.option_of(wire).unwrap().name, "Heating");

    assert_eq!(set_active(&mut prj, "Heating", false).unwrap(), ids.len() + 1);
    assert!(!prj.pages[pi].symbols.iter().any(|s| ids.contains(&s.id)));
    assert!(!prj.pages[pi].wires.iter().any(|w| w.id == wire));
    let count = |prj: &Project| Index::build(prj).devices.get(&tag).map_or(0, |d| d.mains.len() + d.auxs.len() + d.others.len());
    assert_eq!(count(&prj) + ids.len(), count(&before));
    assert_eq!(set_active(&mut prj, "Heating", false).unwrap(), 0);

    set_active(&mut prj, "Heating", true).unwrap();
    assert_eq!(prj.pages, before.pages);
    remove(&mut prj, "Heating").unwrap();
    assert!(prj.model_options.is_empty());
}

#[test]
fn pages_keep_their_place() {
    let mut prj = with_pages(4);
    let order: Vec<u64> = prj.pages.iter().map(|p| p.id).collect();
    define(&mut prj, "B", "", "").unwrap();
    assign(&mut prj, "B", &[order[1], order[2]]).unwrap();
    set_active(&mut prj, "B", false).unwrap();
    assert_eq!(prj.pages.iter().map(|p| p.id).collect::<Vec<_>>(), vec![order[0], order[3]]);
    set_active(&mut prj, "B", true).unwrap();
    assert_eq!(prj.pages.iter().map(|p| p.id).collect::<Vec<_>>(), order);

    // The first page, and not all pages.
    assign(&mut prj, "B", &[order[0]]).unwrap();
    set_active(&mut prj, "B", false).unwrap();
    assert_eq!(prj.pages[0].id, order[3]);
    set_active(&mut prj, "B", true).unwrap();
    assert_eq!(prj.pages.iter().map(|p| p.id).collect::<Vec<_>>(), order);
    assign(&mut prj, "B", &[order[3]]).unwrap();
    assert!(set_active(&mut prj, "B", false).is_err());
}

#[test]
fn content_of_an_inactive_option_is_parked_on_assignment() {
    let mut prj = demo_project();
    let s = prj.pages[0].symbols.first().map(|s| s.id).or_else(|| prj.pages[1].symbols.first().map(|s| s.id)).unwrap();
    define(&mut prj, "Off", "", "").unwrap();
    set_active(&mut prj, "Off", false).unwrap();
    assign(&mut prj, "Off", &[s]).unwrap();
    assert!(!in_pages(&prj, s));
    assert!(prj.model_options.max_id() >= s);
    prj.next_id = 1;
    prj.fix_next_id();
    assert!(prj.next_id > s);
    assert_eq!(unassign(&mut prj, &[s]), 1);
    assert!(in_pages(&prj, s));
    assert!(assign(&mut prj, "Off", &[999_999]).is_err());
    assert!(assign(&mut prj, "Nope", &[s]).is_err());
}

#[test]
fn options_follow_configurator_conditions() {
    let mut prj = with_pages(2);
    prj.configuration.options.push(ConfigOption { name: "HEATER".into(), choices: vec!["yes".into(), "no".into()], value: "yes".into() });
    assert!(define(&mut prj, "Heater", "", "PUMP=yes").is_err());
    define(&mut prj, "Heater", "", "HEATER=yes").unwrap();
    let page = prj.pages[1].id;
    assign(&mut prj, "Heater", &[page]).unwrap();
    assert!(apply_conditions(&mut prj).unwrap().is_empty());
    prj.configuration.options[0].value = "no".into();
    assert_eq!(apply_conditions(&mut prj).unwrap(), vec![("Heater".to_string(), false)]);
    assert_eq!(prj.pages.len(), 1);
    assert_eq!(page_count(&prj, prj.model_options.get("Heater").unwrap()), 1);
    let text = prj.to_json();
    let back = Project::from_json(&text).unwrap();
    assert_eq!(back.model_options, prj.model_options);
}

#[test]
fn pages_of_two_options_go_back_in_their_order() {
    let names = |prj: &Project| prj.pages.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
    for (off, on) in [(["Y", "X"], ["Y", "X"]), (["X", "Y"], ["X", "Y"]), (["X", "Y"], ["Y", "X"]), (["Y", "X"], ["X", "Y"])] {
        let mut prj = with_pages(4);
        let (b, c) = (prj.pages[1].id, prj.pages[2].id);
        define(&mut prj, "X", "", "").unwrap();
        define(&mut prj, "Y", "", "").unwrap();
        assign(&mut prj, "X", &[b]).unwrap();
        assign(&mut prj, "Y", &[c]).unwrap();
        for o in off {
            set_active(&mut prj, o, false).unwrap();
        }
        for o in on {
            set_active(&mut prj, o, true).unwrap();
        }
        assert_eq!(names(&prj), ["1", "2", "3", "4"], "off {off:?}, on {on:?}");
    }
}

#[test]
fn content_of_an_option_on_a_page_of_another_one() {
    let setup = || {
        let mut prj = demo_project();
        let (page, sym) = (prj.pages[1].id, prj.pages[1].symbols[0].id);
        define(&mut prj, "X", "", "").unwrap();
        define(&mut prj, "Y", "", "").unwrap();
        assign(&mut prj, "X", &[page]).unwrap();
        assign(&mut prj, "Y", &[sym]).unwrap();
        (prj, sym)
    };
    let visible = |prj: &Project, sym: u64| prj.pages.iter().any(|p| p.symbols.iter().any(|s| s.id == sym));
    // The page comes back without the symbol of the inactive option, which follows it.
    let (mut prj, sym) = setup();
    set_active(&mut prj, "X", false).unwrap();
    set_active(&mut prj, "Y", false).unwrap();
    set_active(&mut prj, "X", true).unwrap();
    assert!(!visible(&prj, sym));
    set_active(&mut prj, "Y", true).unwrap();
    assert!(visible(&prj, sym));
    // The symbol goes back into the page while that is parked.
    let (mut prj, sym) = setup();
    set_active(&mut prj, "Y", false).unwrap();
    set_active(&mut prj, "X", false).unwrap();
    set_active(&mut prj, "Y", true).unwrap();
    set_active(&mut prj, "X", true).unwrap();
    assert!(visible(&prj, sym));
}
