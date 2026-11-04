//! Import of PDF comments into the review layer: the annotations a reviewer added in
//! Acrobat or another PDF viewer to a PDF export of the project (notes, text boxes,
//! highlights, shapes and their replies) become review comments on the page they were
//! made on; rectangles, ellipses and lines also become markups. Pages are matched by
//! their order, positions by the page size of the PDF. Comments imported before are
//! skipped, so the same PDF can be imported again after more comments were added.
//!
//! The reader understands what PDF viewers write: plain objects, incremental updates
//! and compressed object streams (FlateDecode).

use crate::geom::{p, P};
use crate::model::review::{Markup, MarkupKind, Pen, Reply};
use crate::model::Project;
use crate::review;
use openecraft_i18n::tr;
use std::collections::HashMap;

/// Points per millimetre.
const K: f32 = 72.0 / 25.4;

#[derive(Clone, Debug, PartialEq)]
enum Obj {
    Null,
    Bool(bool),
    Num(f64),
    Str(Vec<u8>),
    Name(String),
    Array(Vec<Obj>),
    Dict(Vec<(String, Obj)>),
    Ref(u32),
}

impl Obj {
    fn get(&self, key: &str) -> Option<&Obj> {
        match self {
            Obj::Dict(d) => d.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn name(&self) -> Option<&str> {
        match self {
            Obj::Name(n) => Some(n),
            _ => None,
        }
    }

    fn num(&self) -> Option<f64> {
        match self {
            Obj::Num(n) => Some(*n),
            _ => None,
        }
    }
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

fn is_delim(c: u8) -> bool {
    matches!(c, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() || c == 0 {
                self.i += 1;
            } else if c == b'%' {
                while self.peek().is_some_and(|c| c != b'\n' && c != b'\r') {
                    self.i += 1;
                }
            } else {
                break;
            }
        }
    }

    fn word(&mut self) -> &[u8] {
        let start = self.i;
        while self.peek().is_some_and(|c| !c.is_ascii_whitespace() && !is_delim(c)) {
            self.i += 1;
        }
        &self.b[start..self.i]
    }

    fn uint(&mut self) -> Option<u32> {
        let w = self.word();
        if w.is_empty() || !w.iter().all(u8::is_ascii_digit) {
            return None;
        }
        std::str::from_utf8(w).ok()?.parse().ok()
    }

    /// The rest of a literal string after its `(`.
    fn literal(&mut self) -> Vec<u8> {
        let mut s = Vec::new();
        let mut level = 1;
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                b'(' => {
                    level += 1;
                    s.push(c);
                }
                b')' => {
                    level -= 1;
                    if level == 0 {
                        break;
                    }
                    s.push(c);
                }
                b'\\' => {
                    let Some(e) = self.peek() else { break };
                    self.i += 1;
                    match e {
                        b'n' => s.push(b'\n'),
                        b'r' => s.push(b'\r'),
                        b't' => s.push(b'\t'),
                        b'b' => s.push(8),
                        b'f' => s.push(12),
                        b'\r' => {
                            if self.peek() == Some(b'\n') {
                                self.i += 1;
                            }
                        }
                        b'\n' => {}
                        b'0'..=b'7' => {
                            let mut v = (e - b'0') as u32;
                            for _ in 0..2 {
                                match self.peek() {
                                    Some(d @ b'0'..=b'7') => {
                                        v = v * 8 + (d - b'0') as u32;
                                        self.i += 1;
                                    }
                                    _ => break,
                                }
                            }
                            s.push(v as u8);
                        }
                        other => s.push(other),
                    }
                }
                _ => s.push(c),
            }
        }
        s
    }

    fn obj(&mut self, depth: usize) -> Option<Obj> {
        if depth > 64 {
            return None;
        }
        self.skip_ws();
        let c = self.peek()?;
        match c {
            b'<' if self.b.get(self.i + 1) == Some(&b'<') => {
                self.i += 2;
                let mut d = Vec::new();
                loop {
                    self.skip_ws();
                    match self.peek()? {
                        b'>' => {
                            self.i += 2;
                            return Some(Obj::Dict(d));
                        }
                        b'/' => {
                            let Obj::Name(k) = self.obj(depth + 1)? else { return None };
                            let v = self.obj(depth + 1)?;
                            d.push((k, v));
                        }
                        _ => return None,
                    }
                }
            }
            b'<' => {
                self.i += 1;
                let mut hex = Vec::new();
                while let Some(c) = self.peek() {
                    self.i += 1;
                    if c == b'>' {
                        break;
                    }
                    if c.is_ascii_hexdigit() {
                        hex.push(c);
                    }
                }
                if hex.len() % 2 == 1 {
                    hex.push(b'0');
                }
                let val = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
                Some(Obj::Str(hex.chunks(2).map(|h| val(h[0]) * 16 + val(h[1])).collect()))
            }
            b'(' => {
                self.i += 1;
                Some(Obj::Str(self.literal()))
            }
            b'[' => {
                self.i += 1;
                let mut a = Vec::new();
                loop {
                    self.skip_ws();
                    if self.peek()? == b']' {
                        self.i += 1;
                        return Some(Obj::Array(a));
                    }
                    a.push(self.obj(depth + 1)?);
                }
            }
            b'/' => {
                self.i += 1;
                let raw = self.word().to_vec();
                let mut name = Vec::new();
                let mut k = 0;
                while k < raw.len() {
                    if raw[k] == b'#' && k + 2 < raw.len() {
                        let h = std::str::from_utf8(&raw[k + 1..k + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                        if let Some(h) = h {
                            name.push(h);
                            k += 3;
                            continue;
                        }
                    }
                    name.push(raw[k]);
                    k += 1;
                }
                Some(Obj::Name(String::from_utf8_lossy(&name).into_owned()))
            }
            b'0'..=b'9' | b'-' | b'+' | b'.' => {
                let start = self.i;
                let w = self.word().to_vec();
                // An object reference: "12 0 R".
                if w.iter().all(u8::is_ascii_digit) {
                    let back = self.i;
                    self.skip_ws();
                    if self.uint().is_some() {
                        self.skip_ws();
                        if self.peek() == Some(b'R') && self.b.get(self.i + 1).is_none_or(|c| c.is_ascii_whitespace() || is_delim(*c)) {
                            self.i += 1;
                            return std::str::from_utf8(&w).ok()?.parse().ok().map(Obj::Ref);
                        }
                    }
                    self.i = back;
                }
                let n = std::str::from_utf8(&w).ok()?.parse::<f64>().ok();
                if n.is_none() {
                    self.i = start + 1;
                }
                Some(Obj::Num(n.unwrap_or(0.0)))
            }
            _ => {
                let w = self.word();
                if w.is_empty() {
                    self.i += 1;
                    return Some(Obj::Null);
                }
                Some(match w {
                    b"true" => Obj::Bool(true),
                    b"false" => Obj::Bool(false),
                    _ => Obj::Null,
                })
            }
        }
    }
}

/// Most the compressed object streams of a PDF may unpack to, all together: more is
/// left out rather than running out of memory on a crafted file.
const MAX_UNPACKED: usize = 1 << 30;

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    hay.get(from..)?.windows(needle.len()).position(|w| w == needle).map(|i| i + from)
}

/// The objects of a PDF by number; later definitions (incremental updates) win.
#[derive(Default)]
struct Doc {
    objs: HashMap<u32, Obj>,
}

impl Doc {
    fn parse(b: &[u8]) -> Doc {
        let mut doc = Doc::default();
        let mut at = 0;
        let mut left = MAX_UNPACKED;
        while let Some(k) = find(b, b"obj", at) {
            at = k + 3;
            let after_ok = b.get(k + 3).is_none_or(|c| c.is_ascii_whitespace() || is_delim(*c));
            if !after_ok || k == 0 || !b[k - 1].is_ascii_whitespace() {
                continue;
            }
            // Walk back over "<num> <gen> ".
            let mut j = k;
            let mut nums = 0;
            let mut num_start = k;
            while nums < 2 && j > 0 {
                while j > 0 && b[j - 1].is_ascii_whitespace() {
                    j -= 1;
                }
                let end = j;
                while j > 0 && b[j - 1].is_ascii_digit() {
                    j -= 1;
                }
                if j == end {
                    break;
                }
                nums += 1;
                num_start = j;
            }
            if nums < 2 {
                continue;
            }
            let mut head = Parser { b, i: num_start };
            let Some(num) = head.uint() else { continue };
            let mut p = Parser { b, i: k + 3 };
            let Some(obj) = p.obj(0) else { continue };
            p.skip_ws();
            let stream = if b[p.i..].starts_with(b"stream") {
                let mut s = p.i + 6;
                if b.get(s) == Some(&b'\r') {
                    s += 1;
                }
                if b.get(s) == Some(&b'\n') {
                    s += 1;
                }
                let len = obj.get("Length").and_then(Obj::num).map(|l| l as usize).filter(|l| s.checked_add(*l).is_some_and(|e| e <= b.len()));
                let end = match len {
                    Some(l) if find(b, b"endstream", s + l).is_some_and(|e| e - (s + l) <= 4) => s + l,
                    _ => find(b, b"endstream", s).unwrap_or(b.len()),
                };
                at = end;
                Some(b[s..end].to_vec())
            } else {
                None
            };
            if obj.get("Type").and_then(Obj::name) == Some("ObjStm") {
                if let Some(data) = stream.as_deref().and_then(|s| decode(&obj, s, left)) {
                    left -= data.len();
                    doc.read_object_stream(&obj, &data);
                }
                continue;
            }
            doc.objs.insert(num, obj);
        }
        doc
    }

    fn read_object_stream(&mut self, dict: &Obj, data: &[u8]) {
        let n = dict.get("N").and_then(Obj::num).unwrap_or(0.0) as usize;
        let first = dict.get("First").and_then(Obj::num).unwrap_or(0.0) as usize;
        let mut head = Parser { b: data, i: 0 };
        let mut entries = Vec::new();
        for _ in 0..n {
            head.skip_ws();
            let Some(num) = head.uint() else { break };
            head.skip_ws();
            let Some(off) = head.uint() else { break };
            entries.push((num, off as usize));
        }
        for (num, off) in entries {
            let mut p = Parser { b: data, i: first.saturating_add(off) };
            if let Some(obj) = p.obj(0) {
                self.objs.insert(num, obj);
            }
        }
    }

    fn resolve<'a>(&'a self, o: &'a Obj) -> &'a Obj {
        let mut o = o;
        for _ in 0..8 {
            match o {
                Obj::Ref(r) => o = self.objs.get(r).unwrap_or(&Obj::Null),
                _ => break,
            }
        }
        o
    }

    fn catalog(&self) -> Option<&Obj> {
        // The catalog with the highest object number is the one of the last update.
        let mut cats: Vec<(&u32, &Obj)> = self.objs.iter().filter(|(_, o)| o.get("Type").and_then(Obj::name) == Some("Catalog")).collect();
        cats.sort_by_key(|(n, _)| **n);
        cats.last().map(|(_, o)| *o)
    }

    /// The page dictionaries in page order.
    fn pages(&self) -> Vec<&Obj> {
        let mut out = Vec::new();
        if let Some(root) = self.catalog().and_then(|c| c.get("Pages")) {
            self.collect_pages(self.resolve(root), &mut out, 0);
        }
        out
    }

    fn collect_pages<'a>(&'a self, node: &'a Obj, out: &mut Vec<&'a Obj>, depth: usize) {
        if depth > 32 {
            return;
        }
        match node.get("Type").and_then(Obj::name) {
            Some("Page") => out.push(node),
            _ => {
                if let Some(Obj::Array(kids)) = node.get("Kids").map(|k| self.resolve(k)) {
                    for k in kids {
                        self.collect_pages(self.resolve(k), out, depth + 1);
                    }
                }
            }
        }
    }
}

/// The data of a stream; None for filters other than Flate and for more than `limit`
/// bytes.
fn decode(dict: &Obj, data: &[u8], limit: usize) -> Option<Vec<u8>> {
    let filter = match dict.get("Filter") {
        Some(Obj::Name(n)) => Some(n.as_str()),
        Some(Obj::Array(a)) if a.len() == 1 => a[0].name(),
        None => None,
        _ => return None,
    };
    match filter {
        None => Some(data.to_vec()).filter(|d| d.len() <= limit),
        Some("FlateDecode") => miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(data, limit).ok(),
        _ => None,
    }
}

/// A PDF text string: UTF-16 with byte order mark, UTF-8 with BOM, else PDFDocEncoding
/// (read as Latin-1).
fn text_of(b: &[u8]) -> String {
    let s = if b.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = b[2..].chunks(2).filter(|c| c.len() == 2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&units)
    } else if b.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&b[3..]).into_owned()
    } else {
        b.iter().map(|&c| c as char).collect()
    };
    s.replace("\r\n", "\n").replace('\r', "\n").trim().to_string()
}

/// `D:20261025143000+01'00'` → `2026-10-25`.
fn date_of(s: &str) -> String {
    let d: String = s.trim_start_matches("D:").chars().take(8).collect();
    if d.len() == 8 && d.chars().all(|c| c.is_ascii_digit()) {
        format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..])
    } else {
        String::new()
    }
}

/// A comment (annotation) of a PDF page.
#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    /// Object number (to resolve replies).
    pub id: u32,
    /// Page index in the PDF.
    pub page: usize,
    /// Subtype: Text, FreeText, Highlight, Square, …
    pub kind: String,
    /// Rectangle in millimetres from the top left corner of the page: left, top, right, bottom.
    pub rect: [f32; 4],
    /// Line end points (millimetres) of line annotations.
    pub line: Option<[f32; 4]>,
    pub text: String,
    pub author: String,
    pub date: String,
    /// Object number of the annotation this one replies to.
    pub reply_to: Option<u32>,
}

/// Annotations that are not comments: links, popups of notes, form fields.
const IGNORED: [&str; 4] = ["Link", "Popup", "Widget", "PrinterMark"];

/// The comments of a PDF in page order.
pub fn read(pdf: &[u8]) -> Result<Vec<Annotation>, String> {
    if !pdf.starts_with(b"%PDF") {
        return Err(tr("Not a PDF file").into());
    }
    let doc = Doc::parse(pdf);
    let pages = doc.pages();
    if pages.is_empty() {
        return Err(tr("The PDF has no pages that can be read").into());
    }
    let mut out = Vec::new();
    for (pi, page) in pages.iter().enumerate() {
        let height = match page.get("MediaBox").map(|m| doc.resolve(m)) {
            Some(Obj::Array(a)) if a.len() == 4 => (a[3].num().unwrap_or(0.0) - a[1].num().unwrap_or(0.0)) as f32,
            _ => 842.0,
        };
        let mm = |x: f64, y: f64| (x as f32 / K, (height - y as f32) / K);
        let annots: Vec<(u32, &Obj)> = match page.get("Annots").map(|a| doc.resolve(a)) {
            Some(Obj::Array(a)) => a
                .iter()
                .map(|o| match o {
                    Obj::Ref(r) => (*r, doc.resolve(o)),
                    _ => (0, o),
                })
                .collect(),
            _ => Vec::new(),
        };
        for (id, a) in annots {
            let Some(kind) = a.get("Subtype").and_then(Obj::name) else { continue };
            if IGNORED.contains(&kind) {
                continue;
            }
            let rect = match a.get("Rect").map(|r| doc.resolve(r)) {
                Some(Obj::Array(r)) if r.len() == 4 => {
                    let v: Vec<f64> = r.iter().map(|x| doc.resolve(x).num().unwrap_or(0.0)).collect();
                    let (l, t) = mm(v[0].min(v[2]), v[1].max(v[3]));
                    let (r, b) = mm(v[0].max(v[2]), v[1].min(v[3]));
                    [l, t, r, b]
                }
                _ => continue,
            };
            let line = match a.get("L").map(|l| doc.resolve(l)) {
                Some(Obj::Array(l)) if l.len() == 4 => {
                    let v: Vec<f64> = l.iter().map(|x| x.num().unwrap_or(0.0)).collect();
                    let (x1, y1) = mm(v[0], v[1]);
                    let (x2, y2) = mm(v[2], v[3]);
                    Some([x1, y1, x2, y2])
                }
                _ => None,
            };
            let string = |key: &str| match a.get(key).map(|s| doc.resolve(s)) {
                Some(Obj::Str(s)) => text_of(s),
                _ => String::new(),
            };
            let date = Some(date_of(&string("M"))).filter(|d| !d.is_empty()).unwrap_or_else(|| date_of(&string("CreationDate")));
            let reply_to = match a.get("IRT") {
                Some(Obj::Ref(r)) => Some(*r),
                _ => None,
            };
            out.push(Annotation { id, page: pi, kind: kind.to_string(), rect, line, text: string("Contents"), author: string("T"), date, reply_to });
        }
    }
    Ok(out)
}

/// What an import did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Imported {
    pub comments: usize,
    pub replies: usize,
    pub markups: usize,
    /// Comments imported before.
    pub skipped: usize,
    /// Comments on PDF pages the project does not have.
    pub outside: usize,
}

/// Imports the comments of a PDF export of the project into the review layer.
/// `author` / `date` are used where the PDF has none.
pub fn import(prj: &mut Project, pdf: &[u8], author: &str, date: &str) -> Result<Imported, String> {
    let annots = read(pdf)?;
    let mut res = Imported::default();
    let mut comment_of: HashMap<u32, u64> = HashMap::new();
    let (tops, replies): (Vec<&Annotation>, Vec<&Annotation>) = annots.iter().partition(|a| a.reply_to.is_none());
    for a in tops {
        let Some(page) = prj.pages.get(a.page).map(|pg| pg.id) else {
            res.outside += 1;
            continue;
        };
        let pos = p(a.rect[0], a.rect[1]);
        let who = if a.author.is_empty() { author } else { &a.author };
        let when = if a.date.is_empty() { date } else { &a.date };
        let text = if a.text.is_empty() { a.kind.clone() } else { a.text.clone() };
        let known = prj.review.live_comments().find(|c| c.page == page && c.text == text && c.author == who && dist(c.pos, pos) < 1.0).map(|c| c.id);
        if let Some(id) = known {
            comment_of.insert(a.id, id);
            res.skipped += 1;
            continue;
        }
        let id = review::add_comment(prj, page, pos, who, when, &text);
        comment_of.insert(a.id, id);
        res.comments += 1;
        let shape = match a.kind.as_str() {
            "Square" | "Circle" | "Polygon" => Some((MarkupKind::Rect, p(a.rect[0], a.rect[1]), p(a.rect[2], a.rect[3]))),
            "Line" => a.line.map(|l| (MarkupKind::Line, p(l[0], l[1]), p(l[2], l[3]))),
            _ => None,
        };
        if let Some((kind, from, to)) = shape {
            review::add_markup(prj, Markup { page, kind, pen: Pen::Red, a: from, b: to, author: who.into(), date: when.into(), ..Default::default() });
            res.markups += 1;
        }
    }
    for a in replies {
        // A reply to a reply belongs to the same comment.
        let mut target = a.reply_to;
        let mut hops = 0;
        while let Some(t) = target {
            if comment_of.contains_key(&t) || hops > 16 {
                break;
            }
            target = annots.iter().find(|x| x.id == t).and_then(|x| x.reply_to);
            hops += 1;
        }
        let Some(id) = target.and_then(|t| comment_of.get(&t)).copied() else { continue };
        let who = if a.author.is_empty() { author.to_string() } else { a.author.clone() };
        let when = if a.date.is_empty() { date.to_string() } else { a.date.clone() };
        let Some(c) = prj.review.comment_mut(id) else { continue };
        if a.text.is_empty() || c.replies.iter().any(|r| r.text == a.text && r.author == who) {
            continue;
        }
        c.replies.push(Reply { author: who, date: when, text: a.text.clone() });
        c.rev += 1;
        res.replies += 1;
    }
    Ok(res)
}

fn dist(a: P, b: P) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

#[cfg(test)]
mod tests;
