use super::*;

#[derive(Default)]
struct Rec {
    rects: Vec<(P, P)>,
    texts: Vec<(P, String, f32, Anchor)>,
    styled: Vec<(P, String, TextStyle)>,
    polys: Vec<(Vec<P>, Ink, bool)>,
    urls: Vec<(P, P, String)>,
}

impl Draw for Rec {
    fn line(&mut self, _: P, _: P, _: f32, _: Ink, _: bool) {}
    fn poly(&mut self, pts: &[P], _: bool, _: f32, ink: Ink, fill: bool) {
        self.polys.push((pts.to_vec(), ink, fill));
    }
    fn circle(&mut self, _: P, _: f32, _: f32, _: Ink, _: bool) {}
    fn text(&mut self, pos: P, text: &str, size: f32, h: Anchor, _: VAnchor, _: Ink) {
        self.texts.push((pos, text.into(), size, h));
    }
    fn styled_text(&mut self, pos: P, text: &str, style: &TextStyle, ink: Ink) {
        self.styled.push((pos, text.into(), *style));
        self.text(pos, text, style.size, style.h, style.v, ink);
    }
    fn rect(&mut self, a: P, b: P, _: f32, _: Ink, _: bool) {
        self.rects.push((a, b));
    }
    fn link_url(&mut self, min: P, max: P, url: &str) {
        self.urls.push((min, max, url.into()));
    }
}

#[test]
fn free_texts_are_drawn_with_style_background_and_link() {
    let mut prj = Project::default();
    prj.info.name = "Pump station".into();
    prj.pages[0].texts.push(TextItem {
        id: 1,
        pos: p(100.0, 100.0),
        text: "{project}\nline 2".into(),
        angle: 90.0,
        bold: true,
        background: true,
        frame: true,
        link: "https://example.org".into(),
        ..Default::default()
    });
    let an = Analysis::with_builtin(&prj);
    let mut d = Rec::default();
    draw_page(&mut d, &prj, &an.index, &an, 0, &Opts::default());
    let mine: Vec<_> = d.styled.iter().filter(|(_, _, st)| st.angle == 90.0).collect();
    assert_eq!(mine.len(), 2);
    assert_eq!(mine[0].1, "Pump station", "placeholders are filled in");
    assert!(mine[0].2.bold && mine[0].2.v == VAnchor::Middle);
    assert!((mine[1].0.x - 104.2).abs() < 0.01 && (mine[1].0.y - 100.0).abs() < 0.01, "second line to the right: {:?}", mine[1].0);
    assert!(d.polys.iter().any(|(pts, ink, fill)| pts.len() == 4 && *ink == Ink::Paper && *fill), "background");
    assert!(d.polys.iter().any(|(pts, ink, fill)| pts.len() == 4 && *ink == Ink::Line && !*fill), "frame");
    assert_eq!(d.urls.len(), 1);
    assert_eq!(d.urls[0].2, "https://example.org");
    let b = text_bounds(&prj, 0, &prj.pages[0].texts[0]);
    assert!((b.min.x - 97.9).abs() < 0.01 && (b.max.x - 106.3).abs() < 0.01, "{b:?}");
    assert!((b.min.y - 77.68).abs() < 0.01 && (b.max.y - 100.0).abs() < 0.01, "{b:?}");
}

#[test]
fn cover_rows_sit_centred_inside_their_box() {
    let mut prj = crate::sample::demo_project();
    prj.info.company = "Schaltanlagenbau Musterstadt GmbH & Co. KG, Werk Nord".into();
    let cover = prj.pages.iter().position(|pg| pg.kind == PageKind::Cover).unwrap();
    let mut d = Rec::default();
    draw_cover(&mut d, &prj, &crate::frame::of_page(&prj, cover));
    let (a, b) = *d.rects.last().unwrap();
    let rows: Vec<_> = d.texts.iter().filter(|t| t.0.y > a.y).collect();
    assert_eq!(rows.len(), 10);
    for (pos, text, size, h) in &rows {
        let w = text_width(text, *size);
        let (x0, x1) = match h {
            Anchor::Left => (pos.x, pos.x + w),
            Anchor::Right => (pos.x - w, pos.x),
            Anchor::Center => (pos.x - w / 2.0, pos.x + w / 2.0),
        };
        assert!(x0 > a.x && x1 < b.x, "{text} sticks out of the box");
        assert!(pos.y - size / 2.0 > a.y && pos.y + size / 2.0 < b.y, "{text} overlaps the box");
    }
    let first = rows.iter().map(|t| t.0.y).fold(f32::INFINITY, f32::min);
    let last = rows.iter().map(|t| t.0.y).fold(f32::NEG_INFINITY, f32::max);
    assert!(((first - a.y) - (b.y - last)).abs() < 0.01);
}

#[test]
fn graphical_terminal_and_cable_diagrams() {
    use crate::forms::ReportForm;
    use crate::model::ReportKind;
    let mut prj = crate::sample::demo_project();
    let data = prj.strips.entry("-X1".into()).or_default();
    data.jumpers.insert("1".into(), 1);
    data.jumpers.insert("2".into(), 1);
    for kind in [ReportKind::TerminalDiagram, ReportKind::CableDiagram] {
        prj.forms.reports.insert(kind.into(), ReportForm { graphic: true, ..Default::default() });
    }
    let at = prj.pages.len();
    crate::ops::add_page(&mut prj, PageKind::Report(ReportKind::TerminalDiagram.into()), at);
    crate::ops::add_page(&mut prj, PageKind::Report(ReportKind::CableDiagram.into()), at + 1);
    let an = Analysis::with_builtin(&prj);
    let table = crate::report_blocks::page_table(&prj, &an, at).unwrap();
    assert!(table.graphic);
    assert_eq!(table.per_page, crate::report_blocks::GRAPHIC_ROWS_PER_PAGE);

    let mut d = Rec::default();
    draw_page(&mut d, &prj, &an.index, &an, at, &Opts::default());
    let texts: Vec<&str> = d.texts.iter().map(|t| t.1.as_str()).collect();
    assert!(texts.contains(&"-X1"), "strip heading");
    for label in ["1", "2", "3", "PE"] {
        assert!(texts.contains(&label), "terminal {label}");
    }
    assert!(texts.iter().any(|t| t.starts_with("Plug-in jumper")), "jumper listed below: {texts:?}");
    // A box per terminal (4) and a bar per end bracket / end plate (4), and the frame.
    assert!(d.rects.len() >= 8, "{} boxes", d.rects.len());
    assert!(!texts.contains(&"External (top)"), "no table header");

    let mut d = Rec::default();
    draw_page(&mut d, &prj, &an.index, &an, at + 1, &Opts::default());
    let texts: Vec<&str> = d.texts.iter().map(|t| t.1.as_str()).collect();
    assert!(texts.iter().any(|t| t.starts_with("-W1")), "cable band heading: {texts:?}");

    // Without the switch the table is drawn.
    prj.forms.reports.clear();
    let an = Analysis::with_builtin(&prj);
    assert!(!crate::report_blocks::page_table(&prj, &an, at).unwrap().graphic);
    assert!(crate::report_blocks::has_graphic(ReportKind::CableDiagram) && !crate::report_blocks::has_graphic(ReportKind::DeviceList));
}
