use super::*;

#[test]
fn layouts_are_saved_by_name_and_replaced() {
    let mut list = Vec::new();
    assert!(save(&mut list, Layout { name: " PLC work ".into(), windows: vec!["plc".into()], ..Default::default() }));
    assert!(!save(&mut list, Layout { name: "  ".into(), ..Default::default() }));
    assert!(save(&mut list, Layout { name: "PLC work".into(), windows: vec!["plc".into(), "reports".into()], ..Default::default() }));
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].windows.len(), 2);
    remove(&mut list, "PLC work");
    assert!(list.is_empty());
}

#[test]
fn applying_opens_the_saved_windows_and_closes_the_others() {
    let (mut a, mut b, mut c) = (true, false, false);
    let layout = Layout { windows: vec!["b".into(), "c".into(), "gone".into()], ..Default::default() };
    {
        let mut flags = [("a", &mut a), ("b", &mut b), ("c", &mut c)];
        apply_windows(&layout, &mut flags);
        assert_eq!(open_windows(&flags), vec!["b".to_string(), "c".to_string()]);
    }
    assert!(!a && b && c);
}
