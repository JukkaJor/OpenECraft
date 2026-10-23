use super::*;

#[test]
fn the_default_frame_keeps_the_a3_geometry() {
    let f = PlotFrame::default();
    assert_eq!(f.size(), (420.0, 297.0));
    let (b, a) = (f.border(), f.area());
    assert_eq!((b.min, b.max), (p(10.0, 10.0), p(410.0, 287.0)));
    assert_eq!((a.min, a.max), (p(15.0, 15.0), p(410.0, 255.0)));
    assert_eq!(f.title_scale(), 1.0);
    assert_eq!(f.column_of(p(16.0, 50.0)), "1");
    assert_eq!(f.column_of(p(409.0, 50.0)), "10");
    assert_eq!(f.row_of(p(100.0, 16.0)), "A");
    assert_eq!(f.row_of(p(100.0, 254.0)), "F");
    // Outside the area the first / last column counts.
    assert_eq!(f.column_of(p(0.0, 0.0)), "1");
}

#[test]
fn formats_orientation_and_labels() {
    let a4 = find(&Project::default(), "A4 portrait");
    assert_eq!(a4.size(), (210.0, 297.0));
    // The title block shrinks to the narrower border.
    assert!((a4.title_scale() - 190.0 / 400.0).abs() < 1e-4);
    assert!((a4.title_origin().y - (287.0 - 32.0 * a4.title_scale())).abs() < 1e-4);
    let custom = PlotFrame { paper: Paper::Custom, custom: [600.0, 250.0], column_labels: Labels::Letters, row_labels: Labels::Numbers, ..Default::default() };
    assert_eq!(custom.size(), (600.0, 250.0));
    assert_eq!(custom.column_of(p(590.0, 50.0)), "J");
    assert_eq!(custom.row_of(p(50.0, 16.0)), "1");
    assert_eq!(Labels::Letters.label(27), "AB");
    let no_strips = PlotFrame { strips: false, ..Default::default() };
    assert_eq!(no_strips.area().min, p(10.0, 10.0));
    assert_eq!(Paper::from_name("letter"), Some(Paper::Letter));
    assert_eq!(Paper::from_name("custom"), Some(Paper::Custom));
    assert!(a4.summary().contains("210 x 297"));
}

#[test]
fn pages_use_their_frame_or_the_project_default() {
    let mut prj = Project::default();
    assert_eq!(of_page(&prj, 0).size(), (420.0, 297.0));
    prj.forms.default_frame = "A4 landscape".into();
    assert_eq!(of_page(&prj, 0).size(), (297.0, 210.0));
    set(&mut prj, PlotFrame { name: "Wide".into(), paper: Paper::Custom, custom: [800.0, 297.0], columns: 20, ..Default::default() });
    prj.pages[0].frame = "Wide".into();
    assert_eq!(of_page(&prj, 0).columns, 20);
    assert_eq!(names(&prj)[0], "Wide");
    assert!(names(&prj).contains(&"A0 landscape".to_string()));
    // Unknown names fall back to the default frame.
    prj.pages[0].frame = "Gone".into();
    assert_eq!(of_page(&prj, 0).size(), (420.0, 297.0));
    prj.pages[0].name = "3".into();
    prj.pages[0].frame = String::new();
    prj.forms.default_frame = String::new();
    assert_eq!(reference(&prj, 0, p(60.0, 50.0)), "/3.2");
}
