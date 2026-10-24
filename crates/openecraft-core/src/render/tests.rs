use super::*;

#[derive(Default)]
struct Rec {
    rects: Vec<(P, P)>,
    texts: Vec<(P, String, f32, Anchor)>,
}

impl Draw for Rec {
    fn line(&mut self, _: P, _: P, _: f32, _: Ink, _: bool) {}
    fn poly(&mut self, _: &[P], _: bool, _: f32, _: Ink, _: bool) {}
    fn circle(&mut self, _: P, _: f32, _: f32, _: Ink, _: bool) {}
    fn text(&mut self, pos: P, text: &str, size: f32, h: Anchor, _: VAnchor, _: Ink) {
        self.texts.push((pos, text.into(), size, h));
    }
    fn rect(&mut self, a: P, b: P, _: f32, _: Ink, _: bool) {
        self.rects.push((a, b));
    }
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
