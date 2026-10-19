//! SVG export (single page) and a printable HTML document (all pages).

use crate::geom::P;
use crate::index::Index;
use crate::model::{Project, PAGE_H, PAGE_W};
use crate::render::{draw_page, Draw, Ink, Opts, VAnchor};
use crate::reports::Analysis;
use crate::symbols::Anchor;
use std::fmt::Write;

pub struct SvgDraw {
    out: String,
}

fn color(ink: Ink) -> &'static str {
    match ink {
        Ink::Frame | Ink::FrameText => "#555",
        Ink::CrossRef => "#1a3d8f",
        Ink::Error => "#c00",
        Ink::Wire => "#000",
        _ => "#000",
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

impl SvgDraw {
    fn stroke(&mut self, width: f32, ink: Ink, dashed: bool) -> String {
        let dash = if dashed { " stroke-dasharray=\"1.5 1\"" } else { "" };
        format!("stroke=\"{}\" stroke-width=\"{:.2}\" stroke-linecap=\"round\"{dash}", color(ink), width.max(0.1))
    }
}

impl Draw for SvgDraw {
    fn line(&mut self, a: P, b: P, width: f32, ink: Ink, dashed: bool) {
        let s = self.stroke(width, ink, dashed);
        let _ = writeln!(self.out, "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" {s}/>", a.x, a.y, b.x, b.y);
    }

    fn poly(&mut self, pts: &[P], closed: bool, width: f32, ink: Ink, fill: bool) {
        let tag = if closed { "polygon" } else { "polyline" };
        let pts: Vec<String> = pts.iter().map(|q| format!("{:.2},{:.2}", q.x, q.y)).collect();
        let fill_s = if fill { color(ink) } else { "none" };
        let s = if fill && width <= 0.0 { String::new() } else { self.stroke(width, ink, false) };
        let _ = writeln!(self.out, "<{tag} points=\"{}\" fill=\"{fill_s}\" {s}/>", pts.join(" "));
    }

    fn circle(&mut self, c: P, r: f32, width: f32, ink: Ink, fill: bool) {
        let fill_s = if fill { color(ink) } else { "none" };
        let s = if fill && width <= 0.0 { String::new() } else { self.stroke(width, ink, false) };
        let _ = writeln!(self.out, "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{fill_s}\" {s}/>", c.x, c.y, r);
    }

    fn text(&mut self, pos: P, text: &str, size: f32, h: Anchor, v: VAnchor, ink: Ink) {
        let anchor = match h {
            Anchor::Left => "start",
            Anchor::Center => "middle",
            Anchor::Right => "end",
        };
        // Approximate vertical alignment via dy (dominant-baseline support varies).
        let dy = match v {
            VAnchor::Middle => size * 0.35,
            VAnchor::Bottom => -size * 0.1,
        };
        let _ = writeln!(
            self.out,
            "<text x=\"{:.2}\" y=\"{:.2}\" font-size=\"{:.2}\" text-anchor=\"{anchor}\" fill=\"{}\">{}</text>",
            pos.x,
            pos.y + dy,
            size * 1.25,
            color(ink),
            esc(text)
        );
    }
}

pub fn page_svg(prj: &Project, page_i: usize) -> String {
    let idx = Index::build(prj);
    let an = Analysis::build(prj);
    page_svg_with(prj, &idx, &an, page_i)
}

fn page_svg_with(prj: &Project, idx: &Index, an: &Analysis, page_i: usize) -> String {
    let mut d = SvgDraw { out: String::new() };
    let _ = writeln!(
        d.out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{PAGE_W}mm\" height=\"{PAGE_H}mm\" viewBox=\"0 0 {PAGE_W} {PAGE_H}\" font-family=\"Arial, Helvetica, sans-serif\">"
    );
    let _ = writeln!(d.out, "<rect x=\"0\" y=\"0\" width=\"{PAGE_W}\" height=\"{PAGE_H}\" fill=\"#fff\"/>");
    draw_page(&mut d, prj, idx, an, page_i, &Opts::default());
    d.out.push_str("</svg>\n");
    d.out
}

/// All pages in one HTML file, one page per printed sheet (A3 landscape).
/// Open in a browser and print to PDF.
pub fn print_document(prj: &Project) -> String {
    let idx = Index::build(prj);
    let an = Analysis::build(prj);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{}</title><style>\
         @page {{ size: A3 landscape; margin: 0; }}\
         body {{ margin: 0; background: #888; }}\
         .page {{ width: 420mm; height: 297mm; page-break-after: always; background: #fff; margin: 0 auto 8mm auto; }}\
         @media print {{ body {{ background: none; }} .page {{ margin: 0; }} }}\
         </style></head><body>",
        esc(&prj.info.name)
    );
    for i in 0..prj.pages.len() {
        out.push_str("<div class=\"page\">\n");
        out.push_str(&page_svg_with(prj, &idx, &an, i));
        out.push_str("</div>\n");
    }
    out.push_str("</body></html>\n");
    out
}
