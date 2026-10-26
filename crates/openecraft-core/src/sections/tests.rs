use super::*;
use crate::model::PageKind;

fn names(prj: &Project) -> Vec<&str> {
    prj.pages.iter().map(|p| p.name.as_str()).collect()
}

/// Cover page 1, section 1 with pages 1.1 and 1.2 and subsection 1.1 with page 1.1.1.
fn project() -> (Project, u64, u64) {
    let mut prj = Project::default();
    prj.pages[0].kind = PageKind::Cover;
    let a = add(&mut prj, 0, "Supply");
    let b = add(&mut prj, a, "Main feed");
    for s in [b, a, a] {
        let id = prj.new_id();
        let mut pg = crate::model::Page::new(id, &next_page_name(&prj, s), "", PageKind::Schematic);
        pg.section = s;
        prj.pages.push(pg);
    }
    sort_pages(&mut prj);
    (prj, a, b)
}

#[test]
fn page_names_include_the_section_numbers() {
    let (prj, a, b) = project();
    assert_eq!(names(&prj), ["1", "1.1", "1.2", "1.1.1"]);
    assert_eq!(full_number(&prj, b), "1.1");
    assert_eq!(title(&prj, a), "1  Supply");
    assert_eq!(local_name(&prj, 3), "1");
    assert_eq!(tree(&prj), [(a, 0), (b, 1)]);
    assert_eq!(next_page_name(&prj, a), "1.3");
    assert_eq!(next_page_name(&prj, 0), "2");
}

#[test]
fn renumbering_a_section_renames_its_pages_and_subsections() {
    let (mut prj, a, b) = project();
    prj.pages[2].locked = true;
    set_number(&mut prj, a, "4").unwrap();
    assert_eq!(names(&prj), ["1", "4.1", "1.2", "4.1.1"]);
    assert!(set_number(&mut prj, b, "").is_err());
    assert!(set_number(&mut prj, b, "2.1").is_err());
    let c = add(&mut prj, a, "");
    assert_eq!(full_number(&prj, c), "4.2");
    assert!(set_number(&mut prj, c, "1").is_err());
}

#[test]
fn moving_pages_and_sections_keeps_the_tree_order() {
    let (mut prj, a, b) = project();
    let c = add(&mut prj, 0, "Controls");
    // Page 1.2 goes to section 2 and keeps its number there.
    let at = move_page(&mut prj, 2, c).unwrap();
    assert_eq!((at, names(&prj)), (3, vec!["1", "1.1", "1.1.1", "2.2"]));
    // Section 1.1 moves to the top level: number 1 is taken there, it becomes 3.
    set_parent(&mut prj, b, 0).unwrap();
    assert_eq!(names(&prj), ["1", "1.1", "2.2", "3.1"]);
    assert!(set_parent(&mut prj, a, a).is_err());
    // Section 3 moves up: it swaps place and number with section 2.
    assert!(move_section(&mut prj, b, true));
    assert_eq!(names(&prj), ["1", "1.1", "2.1", "3.2"]);
    assert!(!move_section(&mut prj, a, true));
}

#[test]
fn deleting_a_section_moves_its_content_to_the_parent() {
    let (mut prj, a, b) = project();
    let c = add(&mut prj, b, "");
    delete(&mut prj, b).unwrap();
    assert_eq!(names(&prj), ["1", "1.1", "1.2", "1.3"]);
    assert_eq!(get(&prj, c).map(|s| s.parent), Some(a));
    assert_eq!(full_number(&prj, c), "1.1");
    delete(&mut prj, a).unwrap();
    assert_eq!(names(&prj), ["1", "2", "3", "4"]);
    assert_eq!(full_number(&prj, c), "1");
    assert!(delete(&mut prj, a).is_err());
}

#[test]
fn renumber_counts_every_level_from_one() {
    let (mut prj, a, b) = project();
    let c = add(&mut prj, 0, "");
    set_number(&mut prj, a, "7").unwrap();
    set_number(&mut prj, b, "B").unwrap();
    set_number(&mut prj, c, "9").unwrap();
    assert_eq!(renumbered(&prj)[&b], "1.1");
    assert_eq!(renumber(&mut prj), 3);
    assert_eq!(names(&prj), ["1", "1.1", "1.2", "1.1.1"]);
    assert_eq!(full_number(&prj, c), "2");
}

#[test]
fn pages_of_unknown_sections_are_top_level_pages() {
    let (mut prj, _, _) = project();
    prj.pages[0].section = 999;
    assert_eq!(of_page(&prj, 0), 0);
    sort_pages(&mut prj);
    assert_eq!(names(&prj)[0], "1");
}
