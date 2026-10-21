//! Forms: title blocks and report forms. A title block is a list of lines,
//! rectangles and texts with placeholders (`{project}`, `{page}`, `{sheet}` …) that
//! is drawn into the title block area at the bottom of a page. Companies define their
//! own title blocks and share them as files; report forms choose the columns, their
//! order and the rows per report page.

use openecraft_i18n::{tr, tr_const};
use crate::geom::{p, P};
use crate::model::{PageKind, Project, ReportKind, FRAME_X0, FRAME_X1, FRAME_Y1, TITLE_Y0};
use crate::render::{Draw, Ink, VAnchor};
use crate::symbols::Anchor;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;

/// Size of the title block area (origin at its top left corner).
pub const AREA_W: f32 = FRAME_X1 - FRAME_X0;
pub const AREA_H: f32 = FRAME_Y1 - TITLE_Y0;

pub const STANDARD: &str = "Standard";
pub const COMPACT: &str = "Compact";
pub const NONE: &str = "None";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FormItem {
    Line {
        a: P,
        b: P,
        width: f32,
    },
    Rect {
        a: P,
        b: P,
        width: f32,
        #[serde(default)]
        fill: bool,
    },
    /// Text with placeholders. `label` texts are drawn in the frame colour;
    /// `max_width` > 0 shrinks long values to fit.
    Text {
        pos: P,
        text: String,
        size: f32,
        anchor: Anchor,
        #[serde(default)]
        label: bool,
        #[serde(default)]
        max_width: f32,
    },
}

impl FormItem {
    pub fn kind_name(&self) -> &'static str {
        match self {
            FormItem::Line { .. } => tr("Line"),
            FormItem::Rect { .. } => tr("Rectangle"),
            FormItem::Text { .. } => tr("Text"),
        }
    }

    /// Distance from `q` (title block coordinates) for picking in the editor.
    pub fn distance(&self, q: P) -> f32 {
        match self {
            FormItem::Line { a, b, .. } => crate::geom::dist_to_segment(q, *a, *b),
            FormItem::Rect { a, b, .. } => {
                let c = [*a, p(b.x, a.y), *b, p(a.x, b.y), *a];
                c.windows(2).map(|s| crate::geom::dist_to_segment(q, s[0], s[1])).fold(f32::INFINITY, f32::min)
            }
            FormItem::Text { pos, text, size, anchor, .. } => {
                let w = text_width(text, *size);
                let x0 = match anchor {
                    Anchor::Left => pos.x,
                    Anchor::Center => pos.x - w / 2.0,
                    Anchor::Right => pos.x - w,
                };
                let dx = (x0 - q.x).max(q.x - (x0 + w)).max(0.0);
                let dy = ((q.y - pos.y).abs() - size * 0.6).max(0.0);
                (dx * dx + dy * dy).sqrt()
            }
        }
    }

    pub fn translate(&mut self, d: P) {
        match self {
            FormItem::Line { a, b, .. } | FormItem::Rect { a, b, .. } => {
                *a = *a + d;
                *b = *b + d;
            }
            FormItem::Text { pos, .. } => *pos = *pos + d,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Form {
    pub name: String,
    pub items: Vec<FormItem>,
}

/// Layout of a report page.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportForm {
    /// Heading of the page; empty: the name of the report.
    pub title: String,
    /// Columns shown, in this order (header names); empty: all columns.
    pub columns: Vec<String>,
    /// Rows per page; 0: the default.
    pub rows: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Forms {
    /// Title blocks defined for the project.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub title_blocks: Vec<Form>,
    /// Title block of pages that do not choose one; empty: the standard one.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub default_title_block: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub reports: BTreeMap<ReportKind, ReportForm>,
}

/// File format for exchanging title blocks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FormLibrary {
    pub title_blocks: Vec<Form>,
}

impl FormLibrary {
    pub fn from_json(text: &str) -> Result<FormLibrary, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }
}

/// Adds the title blocks of a library to the project; ones with the same name are
/// replaced. Returns their number.
pub fn merge(prj: &mut Project, lib: FormLibrary) -> usize {
    let n = lib.title_blocks.len();
    for f in lib.title_blocks {
        match prj.forms.title_blocks.iter_mut().find(|x| x.name == f.name) {
            Some(x) => *x = f,
            None => prj.forms.title_blocks.push(f),
        }
    }
    n
}

fn label(items: &mut Vec<FormItem>, x: f32, y: f32, text: &'static str) {
    items.push(FormItem::Text { pos: p(x + 1.0, y + 1.8), text: tr(text).into(), size: 1.6, anchor: Anchor::Left, label: true, max_width: 0.0 });
}

fn value(items: &mut Vec<FormItem>, x: f32, y: f32, text: &str, size: f32, max_width: f32) {
    items.push(FormItem::Text { pos: p(x + 2.0, y), text: text.into(), size, anchor: Anchor::Left, label: false, max_width });
}

fn line(items: &mut Vec<FormItem>, a: P, b: P, width: f32) {
    items.push(FormItem::Line { a, b, width });
}

/// The built-in title blocks.
pub fn builtin() -> Vec<Form> {
    let (w, h) = (AREA_W, AREA_H);
    let ym = h / 2.0;
    let mut std = Vec::new();
    line(&mut std, p(0.0, 0.0), p(w, 0.0), 0.5);
    for x in [80.0, 140.0, 200.0, 320.0, 360.0] {
        line(&mut std, p(x, 0.0), p(x, h), 0.25);
    }
    for (x0, x1) in [(80.0, 140.0), (140.0, 200.0), (320.0, 360.0), (360.0, w)] {
        line(&mut std, p(x0, ym), p(x1, ym), 0.25);
    }
    label(&mut std, 0.0, 0.0, "Company");
    value(&mut std, 0.0, ym + 2.0, "{company}", 4.0, 76.0);
    label(&mut std, 80.0, 0.0, "Date");
    value(&mut std, 80.0, 9.0, "{date}", 2.5, 56.0);
    label(&mut std, 80.0, ym, "Creator");
    value(&mut std, 80.0, ym + 9.0, "{creator}", 2.5, 56.0);
    label(&mut std, 140.0, 0.0, "Revision");
    value(&mut std, 140.0, 9.0, "{revision}", 2.5, 56.0);
    label(&mut std, 140.0, ym, "Software");
    value(&mut std, 140.0, ym + 9.0, "{software}", 2.5, 56.0);
    label(&mut std, 200.0, 0.0, "Project / page description");
    value(&mut std, 200.0, 9.0, "{project}", 3.5, 116.0);
    value(&mut std, 200.0, 21.0, "{page.description}", 3.0, 116.0);
    label(&mut std, 320.0, 0.0, "Page");
    value(&mut std, 320.0, 9.5, "{page}", 4.0, 36.0);
    label(&mut std, 320.0, ym, "Type");
    value(&mut std, 320.0, ym + 9.0, "{page.type}", 2.5, 36.0);
    label(&mut std, 360.0, 0.0, "Sheet");
    value(&mut std, 360.0, 9.5, "{sheet} / {sheets}", 3.5, 36.0);
    label(&mut std, 360.0, ym, "Project");
    value(&mut std, 360.0, ym + 9.0, "{project}", 2.2, 36.0);

    let mut compact = Vec::new();
    line(&mut compact, p(0.0, 0.0), p(w, 0.0), 0.5);
    for x in [120.0, 300.0, 350.0] {
        line(&mut compact, p(x, 0.0), p(x, h), 0.25);
    }
    value(&mut compact, 0.0, 10.0, "{company}", 3.5, 115.0);
    value(&mut compact, 0.0, 22.0, "{project}", 3.0, 115.0);
    value(&mut compact, 120.0, 10.0, "{page.description}", 3.5, 175.0);
    value(&mut compact, 120.0, 22.0, "{page.structure}", 2.5, 175.0);
    label(&mut compact, 300.0, 0.0, "Revision / date");
    value(&mut compact, 300.0, 12.0, "{revision}", 3.0, 46.0);
    value(&mut compact, 300.0, 22.0, "{date}", 2.2, 46.0);
    label(&mut compact, 350.0, 0.0, "Page");
    value(&mut compact, 350.0, 12.0, "{page}", 4.0, 46.0);
    value(&mut compact, 350.0, 22.0, "{sheet} / {sheets}", 2.5, 46.0);

    vec![
        Form { name: STANDARD.into(), items: std },
        Form { name: COMPACT.into(), items: compact },
        Form { name: NONE.into(), items: Vec::new() },
    ]
}

/// Names of all title blocks of a project (custom ones first).
pub fn names(prj: &Project) -> Vec<String> {
    let mut v: Vec<String> = prj.forms.title_blocks.iter().map(|f| f.name.clone()).collect();
    for f in builtin() {
        if !v.contains(&f.name) {
            v.push(f.name);
        }
    }
    v
}

/// A title block by name: the project's own ones take precedence over the built-in
/// ones; unknown names give the standard title block.
pub fn find<'a>(prj: &'a Project, name: &str) -> Cow<'a, Form> {
    if let Some(f) = prj.forms.title_blocks.iter().find(|f| f.name == name) {
        return Cow::Borrowed(f);
    }
    let mut all = builtin();
    let i = all.iter().position(|f| f.name == name).unwrap_or(0);
    Cow::Owned(all.swap_remove(i))
}

/// Title block used on a page.
pub fn title_block(prj: &Project, page_i: usize) -> Cow<'_, Form> {
    let own = &prj.pages[page_i].title_block;
    let name = if !own.is_empty() { own } else { &prj.forms.default_title_block };
    find(prj, if name.is_empty() { STANDARD } else { name })
}

/// Value of one placeholder (without braces); `None` for unknown names.
pub fn placeholder(prj: &Project, page_i: usize, name: &str) -> Option<String> {
    let info = &prj.info;
    let page = &prj.pages[page_i];
    let rev = prj.revisions.last();
    let v = match name {
        "project" => info.name.clone(),
        "project.description" => info.description.lines().collect::<Vec<_>>().join(" "),
        "company" => info.company.clone(),
        "creator" => info.creator.clone(),
        "date" => info.date.clone(),
        "revision" => info.revision.clone(),
        "software" => "OpenECraft".into(),
        "page" => crate::structure::page_full_name(page),
        "page.name" => page.name.clone(),
        "page.description" => page.description.clone(),
        "page.structure" => crate::structure::page_structure(page),
        "page.function" => page.function.clone(),
        "page.location" => page.location.clone(),
        "page.type" => match page.kind {
            PageKind::Schematic => tr("Schematic"),
            PageKind::Cover => tr("Cover"),
            PageKind::Report(_) => tr("Report"),
            PageKind::Mounting(_) => tr("Mounting"),
        }
        .into(),
        "sheet" => (page_i + 1).to_string(),
        "sheets" => prj.pages.len().to_string(),
        "rev.name" => rev.map(|r| r.name.clone()).unwrap_or_default(),
        "rev.date" => rev.map(|r| r.date.clone()).unwrap_or_default(),
        "rev.author" => rev.map(|r| r.author.clone()).unwrap_or_default(),
        "rev.description" => rev.map(|r| r.description.clone()).unwrap_or_default(),
        _ => info.properties.get(name.strip_prefix("prop:")?).cloned().unwrap_or_default(),
    };
    Some(v)
}

/// Placeholders offered in the title block editor: (placeholder, meaning).
pub const PLACEHOLDERS: [(&str, &str); 22] = [
    ("{project}", tr_const!("Project name")),
    ("{project.description}", tr_const!("Project description")),
    ("{company}", tr_const!("Company")),
    ("{creator}", tr_const!("Created by")),
    ("{date}", tr_const!("Date")),
    ("{revision}", tr_const!("Project revision")),
    ("{software}", tr_const!("OpenECraft")),
    ("{page}", tr_const!("Full page name (=A1+CAB1/3)")),
    ("{page.name}", tr_const!("Page number")),
    ("{page.description}", tr_const!("Page description")),
    ("{page.structure}", tr_const!("Page structure (=A1+CAB1)")),
    ("{page.function}", tr_const!("Function of the page")),
    ("{page.location}", tr_const!("Location of the page")),
    ("{page.type}", tr_const!("Page type")),
    ("{sheet}", tr_const!("Sheet number")),
    ("{sheets}", tr_const!("Number of sheets")),
    ("{rev.name}", tr_const!("Last revision")),
    ("{rev.date}", tr_const!("Date of the last revision")),
    ("{rev.author}", tr_const!("Author of the last revision")),
    ("{rev.description}", tr_const!("Description of the last revision")),
    ("{prop:Name}", "Project property \"Name\""),
    ("{{ }}", tr_const!("Literal braces")),
];

/// Replaces the placeholders of a text; unknown ones stay as they are.
pub fn fill(text: &str, prj: &Project, page_i: usize) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        if rest.starts_with("{{") || rest.starts_with("}}") {
            out.push_str(&rest[..1]);
            i += 2;
            continue;
        }
        if let (true, Some(end)) = (rest.starts_with('{'), rest.find('}')) {
            match placeholder(prj, page_i, rest[1..end].trim()) {
                Some(v) => out.push_str(&v),
                None => out.push_str(&rest[..=end]),
            }
            i += end + 1;
            continue;
        }
        let c = rest.chars().next().unwrap_or(' ');
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Estimated width of a text (same estimate as the canvas hit tests).
pub fn text_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.62
}

/// Draws a title block for a page.
pub fn draw(d: &mut dyn Draw, form: &Form, prj: &Project, page_i: usize) {
    let o = p(FRAME_X0, TITLE_Y0);
    for item in &form.items {
        match item {
            FormItem::Line { a, b, width } => d.line(o + *a, o + *b, *width, Ink::Frame, false),
            FormItem::Rect { a, b, width, fill } => {
                if *fill {
                    d.rect(o + *a, o + *b, 0.0, Ink::Plate, true);
                }
                if *width > 0.0 {
                    d.rect(o + *a, o + *b, *width, Ink::Frame, false);
                }
            }
            FormItem::Text { pos, text, size, anchor, label, max_width } => {
                let ink = if *label { Ink::FrameText } else { Ink::Text };
                let value = fill(text, prj, page_i);
                for (i, l) in value.lines().enumerate() {
                    let w = text_width(l, *size);
                    let size = if *max_width > 0.0 && w > *max_width { (size * max_width / w).max(1.0) } else { *size };
                    d.text(o + *pos + p(0.0, i as f32 * size * 1.4), l, size, *anchor, VAnchor::Middle, ink);
                }
            }
        }
    }
}

/// Indices of the report columns shown by a form (all if it chooses none or none
/// of its columns exist).
pub fn report_columns(form: Option<&ReportForm>, headers: &[&str]) -> Vec<usize> {
    let chosen: Vec<usize> = form
        .map(|f| f.columns.iter().filter_map(|c| headers.iter().position(|h| h == c)).collect())
        .unwrap_or_default();
    if chosen.is_empty() {
        (0..headers.len()).collect()
    } else {
        chosen
    }
}

#[cfg(test)]
mod tests;
