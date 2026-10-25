use super::*;
use crate::sample::demo_project;

#[test]
fn names_and_ranges_select_pages_in_project_order() {
    let prj = demo_project();
    let names: Vec<&str> = prj.pages.iter().map(|p| p.name.as_str()).collect();
    assert!(names.len() >= 5, "{names:?}");
    let all = parse(&prj, "").unwrap();
    assert_eq!(all.len(), prj.pages.len());
    let sel = parse(&prj, &format!("{}, {}-{}", names[4], names[1], names[2])).unwrap();
    assert_eq!(sel, vec![1, 2, 4]);
    let back = parse(&prj, &format!("{}-{}", names[2], names[0])).unwrap();
    assert_eq!(back, vec![0, 1, 2], "reversed ranges work");
    assert_eq!(parse(&prj, &format!("{0}; {0}", names[3])).unwrap(), vec![3], "no duplicates");
    assert!(parse(&prj, "999").unwrap_err().contains("999"));
}

#[test]
fn names_with_hyphens_are_found_as_a_whole() {
    let mut prj = demo_project();
    prj.pages[1].name = "2-A".into();
    assert_eq!(parse(&prj, "2-A").unwrap(), vec![1]);
    let n0 = prj.pages[0].name.clone();
    assert_eq!(parse(&prj, &format!("{n0}-2-A")).unwrap(), vec![0, 1]);
}
