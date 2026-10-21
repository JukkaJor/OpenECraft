//! PDF export of all pages (A3 landscape): vector graphics, Helvetica text, clickable
//! cross references and report rows, one bookmark per page. The file is plain ASCII
//! (non-ASCII text is written as WinAnsi octal escapes).

use crate::geom::P;
use crate::index::Index;
use crate::model::{Project, PAGE_H, PAGE_W};
use crate::render::{draw_page, Draw, Ink, Opts, VAnchor};
use crate::reports::Analysis;
use crate::symbols::Anchor;
use std::fmt::Write;

/// Points per millimetre.
const K: f32 = 72.0 / 25.4;

/// Glyph widths of Helvetica (1/1000 em) for ASCII 32..=126.
const HELVETICA: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556, 556, 556, 556,
    278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667,
    611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833,
    556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

/// WinAnsi code of a character (`?` if it has none).
fn win_ansi(c: char) -> u8 {
    match c {
        ' '..='~' => c as u8,
        '€' => 0x80,
        '…' => 0x85,
        '–' => 0x96,
        '—' => 0x97,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '\u{a0}'..='\u{ff}' => c as u8,
        _ => b'?',
    }
}

/// PDF string literal content.
fn pdf_string(text: &str) -> String {
    let mut out = String::new();
    for b in text.chars().map(win_ansi) {
        match b {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(b as char);
            }
            32..=126 => out.push(b as char),
            _ => {
                let _ = write!(out, "\\{b:03o}");
            }
        }
    }
    out
}

fn text_width(text: &str, size: f32) -> f32 {
    text.chars()
        .map(|c| match win_ansi(c) {
            b @ 32..=126 => HELVETICA[(b - 32) as usize] as f32,
            _ => 556.0,
        })
        .sum::<f32>()
        * size
        / 1000.0
}

fn rgb(ink: Ink) -> (f32, f32, f32) {
    match ink {
        Ink::Frame | Ink::FrameText => (0.33, 0.33, 0.33),
        Ink::CrossRef => (0.1, 0.24, 0.56),
        Ink::Error => (0.8, 0.0, 0.0),
        Ink::Plate => (0.95, 0.95, 0.95),
        Ink::Fill => (0.86, 0.86, 0.86),
        _ => (0.0, 0.0, 0.0),
    }
}

fn x(v: f32) -> f32 {
    v * K
}

fn y(v: f32) -> f32 {
    (PAGE_H - v) * K
}

/// Collects the content stream and the links of one page.
#[derive(Default)]
pub struct PdfDraw {
    ops: String,
    /// (x0, y0, x1, y1 in PDF units, target page index)
    links: Vec<([f32; 4], usize)>,
}

impl PdfDraw {
    fn stroke(&mut self, width: f32, ink: Ink, dashed: bool) {
        let (r, g, b) = rgb(ink);
        let dash = if dashed { "[4.25 2.83] 0 d" } else { "[] 0 d" };
        let _ = write!(self.ops, "{r:.3} {g:.3} {b:.3} RG {:.3} w {dash} ", width.max(0.1) * K);
    }

    fn fill(&mut self, ink: Ink) {
        let (r, g, b) = rgb(ink);
        let _ = write!(self.ops, "{r:.3} {g:.3} {b:.3} rg ");
    }

    fn path(&mut self, pts: &[P]) {
        for (i, q) in pts.iter().enumerate() {
            let _ = write!(self.ops, "{:.2} {:.2} {} ", x(q.x), y(q.y), if i == 0 { "m" } else { "l" });
        }
    }
}

impl Draw for PdfDraw {
    fn line(&mut self, a: P, b: P, width: f32, ink: Ink, dashed: bool) {
        self.stroke(width, ink, dashed);
        self.path(&[a, b]);
        self.ops.push_str("S\n");
    }

    fn poly(&mut self, pts: &[P], closed: bool, width: f32, ink: Ink, fill: bool) {
        if pts.len() < 2 {
            return;
        }
        if fill {
            self.fill(ink);
        }
        if !fill || width > 0.0 {
            self.stroke(width, ink, false);
        }
        self.path(pts);
        let op = match (fill, width > 0.0, closed) {
            (true, true, _) => "b",
            (true, false, _) => "f",
            (false, _, true) => "s",
            (false, _, false) => "S",
        };
        self.ops.push_str(op);
        self.ops.push('\n');
    }

    fn circle(&mut self, c: P, r: f32, width: f32, ink: Ink, fill: bool) {
        // Four Bézier arcs.
        let (cx, cy, r) = (x(c.x), y(c.y), r * K);
        let k = 0.552_284_8 * r;
        if fill {
            self.fill(ink);
        } else {
            self.stroke(width, ink, false);
        }
        let _ = writeln!(
            self.ops,
            "{:.2} {:.2} m {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c {}",
            cx + r, cy,
            cx + r, cy + k, cx + k, cy + r, cx, cy + r,
            cx - k, cy + r, cx - r, cy + k, cx - r, cy,
            cx - r, cy - k, cx - k, cy - r, cx, cy - r,
            cx + k, cy - r, cx + r, cy - k, cx + r, cy,
            if fill { "f" } else { "s" }
        );
    }

    fn text(&mut self, pos: P, text: &str, size: f32, h: Anchor, v: VAnchor, ink: Ink) {
        if text.is_empty() {
            return;
        }
        // Same metrics as the SVG export.
        let fs = size * 1.25 * K;
        let w = text_width(text, fs);
        let x0 = match h {
            Anchor::Left => x(pos.x),
            Anchor::Center => x(pos.x) - w / 2.0,
            Anchor::Right => x(pos.x) - w,
        };
        let base = match v {
            VAnchor::Middle => pos.y + size * 0.35,
            VAnchor::Bottom => pos.y - size * 0.1,
        };
        self.fill(ink);
        let _ = writeln!(self.ops, "BT /F1 {fs:.2} Tf {x0:.2} {:.2} Td ({}) Tj ET", y(base), pdf_string(text));
    }

    fn link(&mut self, min: P, max: P, page: usize) {
        self.links.push(([x(min.x), y(max.y), x(max.x), y(min.y)], page));
    }
}

/// Builds the PDF objects and the cross reference table.
struct Writer {
    out: String,
    offsets: Vec<usize>,
}

impl Writer {
    fn obj(&mut self, n: usize, body: &str) {
        if self.offsets.len() <= n {
            self.offsets.resize(n + 1, 0);
        }
        self.offsets[n] = self.out.len();
        let _ = write!(self.out, "{n} 0 obj\n{body}\nendobj\n");
    }
}

/// All pages of the project as one PDF document.
pub fn document(prj: &Project) -> String {
    let idx = Index::build(prj);
    let an = Analysis::build(prj);
    let pages: Vec<PdfDraw> = (0..prj.pages.len())
        .map(|i| {
            let mut d = PdfDraw::default();
            draw_page(&mut d, prj, &idx, &an, i, &Opts::default());
            d
        })
        .collect();

    // Object numbers: 1 catalog, 2 page tree, 3 font, 4 outline root, 5 info, then
    // per page: page, content, bookmark; then the link annotations.
    let n = pages.len();
    let page_obj = |i: usize| 6 + 3 * i;
    let mut next = 6 + 3 * n;
    let mut w = Writer { out: String::from("%PDF-1.4\n"), offsets: vec![0] };
    w.obj(1, "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R /PageMode /UseOutlines >>");
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", page_obj(i))).collect();
    w.obj(2, &format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")));
    w.obj(3, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
    if n > 0 {
        w.obj(4, &format!("<< /Type /Outlines /First {} 0 R /Last {} 0 R /Count {n} >>", page_obj(0) + 2, page_obj(n - 1) + 2));
    } else {
        w.obj(4, "<< /Type /Outlines /Count 0 >>");
    }
    w.obj(5, &format!("<< /Title ({}) /Producer (OpenECraft) >>", pdf_string(&prj.info.name)));
    let (width, height) = (PAGE_W * K, PAGE_H * K);
    for (i, d) in pages.iter().enumerate() {
        let mut annots = Vec::new();
        for (rect, target) in &d.links {
            if *target >= n || *target == i {
                continue;
            }
            let body = format!(
                "<< /Type /Annot /Subtype /Link /Rect [{:.2} {:.2} {:.2} {:.2}] /Border [0 0 0] /Dest [{} 0 R /Fit] >>",
                rect[0], rect[1], rect[2], rect[3], page_obj(*target)
            );
            w.obj(next, &body);
            annots.push(format!("{next} 0 R"));
            next += 1;
        }
        let po = page_obj(i);
        w.obj(
            po,
            &format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width:.2} {height:.2}] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R /Annots [{}] >>",
                po + 1,
                annots.join(" ")
            ),
        );
        w.obj(po + 1, &format!("<< /Length {} >>\nstream\n{}endstream", d.ops.len(), d.ops));
        let pg = &prj.pages[i];
        let title = format!("{} {}", crate::structure::page_full_name(pg), pg.description).trim().to_string();
        let mut bookmark = format!("<< /Title ({}) /Parent 4 0 R /Dest [{po} 0 R /Fit]", pdf_string(&title));
        if i > 0 {
            let _ = write!(bookmark, " /Prev {} 0 R", page_obj(i - 1) + 2);
        }
        if i + 1 < n {
            let _ = write!(bookmark, " /Next {} 0 R", page_obj(i + 1) + 2);
        }
        bookmark.push_str(" >>");
        w.obj(po + 2, &bookmark);
    }
    let xref = w.out.len();
    let count = w.offsets.len();
    let _ = writeln!(w.out, "xref\n0 {count}\n0000000000 65535 f ");
    for off in &w.offsets[1..] {
        let _ = writeln!(w.out, "{off:010} 00000 n ");
    }
    let _ = write!(w.out, "trailer\n<< /Size {count} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{xref}\n%%EOF\n");
    w.out
}

#[cfg(test)]
mod tests;
