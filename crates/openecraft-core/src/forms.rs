//! Forms: title blocks and report forms. A title block is a list of lines,
//! rectangles and texts with placeholders (`{project}`, `{page}`, `{sheet}` …) that
//! is drawn into the title block area at the bottom of a page. Companies define their
//! own title blocks and share them as files; report forms choose the columns, their
//! order and the rows per report page.

use openecraft_i18n::{tr, tr_const};
use crate::geom::{p, P};
use crate::model::{PageKind, Project, ReportKind};
use crate::params::Text;
use crate::render::{Draw, Ink, VAnchor};
use crate::table::header_matches;
use crate::symbols::Anchor;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;

/// Size of the title block area (origin at its top left corner).
pub const AREA_W: f32 = crate::frame::TITLE_W;
pub const AREA_H: f32 = crate::frame::TITLE_H;

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
    /// Only rows that pass all filters are shown.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<RowFilter>,
    /// Sort order (first key first); empty: the order of the report.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sort: Vec<SortKey>,
    /// Column that groups the rows: report blocks start new pages for every value;
    /// empty: the first column.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub group: String,
    /// Values of the first shown column that repeat the row above are left blank.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub blank_repeats: bool,
    /// Relative column widths by header name (instead of the report's).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub widths: BTreeMap<String, f32>,
    /// Text size of the rows (mm); 0: fitted to the rows per page.
    pub text_size: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterOp {
    #[default]
    Contains,
    NotContains,
    Equals,
    NotEquals,
    StartsWith,
    Empty,
    NotEmpty,
}

impl FilterOp {
    pub const ALL: [FilterOp; 7] =
        [FilterOp::Contains, FilterOp::NotContains, FilterOp::Equals, FilterOp::NotEquals, FilterOp::StartsWith, FilterOp::Empty, FilterOp::NotEmpty];

    pub fn title(self) -> &'static str {
        match self {
            FilterOp::Contains => tr("contains"),
            FilterOp::NotContains => tr("does not contain"),
            FilterOp::Equals => tr("is"),
            FilterOp::NotEquals => tr("is not"),
            FilterOp::StartsWith => tr("starts with"),
            FilterOp::Empty => tr("is empty"),
            FilterOp::NotEmpty => tr("is not empty"),
        }
    }

    /// Needs a value to compare with.
    pub fn has_value(self) -> bool {
        !matches!(self, FilterOp::Empty | FilterOp::NotEmpty)
    }
}

/// Keeps the rows whose cell in `column` passes (case-insensitive).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RowFilter {
    pub column: String,
    pub op: FilterOp,
    pub value: String,
}

impl RowFilter {
    pub fn passes(&self, cell: &str) -> bool {
        let (c, v) = (cell.to_lowercase(), self.value.trim().to_lowercase());
        match self.op {
            FilterOp::Contains => c.contains(&v),
            FilterOp::NotContains => !c.contains(&v),
            FilterOp::Equals => c == v,
            FilterOp::NotEquals => c != v,
            FilterOp::StartsWith => c.starts_with(&v),
            FilterOp::Empty => c.trim().is_empty(),
            FilterOp::NotEmpty => !c.trim().is_empty(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SortKey {
    pub column: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub descending: bool,
}

/// A named report form of one report, e.g. "Terminals for the workshop"; report
/// blocks choose it by name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportScheme {
    pub name: String,
    pub kind: ReportKind,
    pub form: ReportForm,
}

impl Default for ReportScheme {
    fn default() -> Self {
        ReportScheme { name: String::new(), kind: ReportKind::DeviceList, form: ReportForm::default() }
    }
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
    /// Named report forms.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub schemes: Vec<ReportScheme>,
    /// Plot frames defined for the project.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub frames: Vec<crate::frame::PlotFrame>,
    /// Plot frame of pages that do not choose one; empty: A3 landscape.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub default_frame: String,
}

/// File format for exchanging title blocks and report schemes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FormLibrary {
    pub title_blocks: Vec<Form>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub schemes: Vec<ReportScheme>,
}

impl FormLibrary {
    pub fn from_json(text: &str) -> Result<FormLibrary, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }
}

/// Adds the title blocks and report schemes of a library to the project; ones with
/// the same name (schemes: of the same report) are replaced. Returns their numbers.
pub fn merge(prj: &mut Project, lib: FormLibrary) -> (usize, usize) {
    let n = (lib.title_blocks.len(), lib.schemes.len());
    for f in lib.title_blocks {
        match prj.forms.title_blocks.iter_mut().find(|x| x.name == f.name) {
            Some(x) => *x = f,
            None => prj.forms.title_blocks.push(f),
        }
    }
    for s in lib.schemes {
        match prj.forms.schemes.iter_mut().find(|x| x.name == s.name && x.kind == s.kind) {
            Some(x) => *x = s,
            None => prj.forms.schemes.push(s),
        }
    }
    n
}

/// Report form of a report: the scheme of that name, otherwise the report's form.
pub fn report_form<'a>(prj: &'a Project, kind: ReportKind, scheme: &str) -> Option<&'a ReportForm> {
    let named = prj.forms.schemes.iter().find(|s| !scheme.is_empty() && s.kind == kind && s.name == scheme);
    named.map(|s| &s.form).or_else(|| prj.forms.reports.get(&kind))
}

/// Renames a scheme together with the report blocks and pages that use it. Fails for
/// empty names and names the report already has.
pub fn rename_scheme(prj: &mut Project, kind: ReportKind, old: &str, new: &str) -> bool {
    let new = new.trim();
    if new.is_empty() || prj.forms.schemes.iter().any(|s| s.kind == kind && s.name == new) {
        return false;
    }
    let Some(s) = prj.forms.schemes.iter_mut().find(|s| s.kind == kind && s.name == old) else { return false };
    s.name = new.to_string();
    for b in prj.report_blocks.iter_mut().filter(|b| b.kind == kind && b.scheme == old) {
        b.scheme = new.to_string();
    }
    for pg in prj.pages.iter_mut().filter(|p| p.kind == PageKind::Report(kind)) {
        if let Some(r) = pg.report.as_mut().filter(|r| r.scheme == old) {
            r.scheme = new.to_string();
        }
    }
    true
}

/// Names of the schemes of a report.
pub fn scheme_names(prj: &Project, kind: ReportKind) -> Vec<String> {
    prj.forms.schemes.iter().filter(|s| s.kind == kind).map(|s| s.name.clone()).collect()
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

fn structure_description(prj: &Project, aspect: crate::structure::Aspect, id: &str) -> String {
    prj.structures.entry(aspect, id).map(|e| e.description.clone()).unwrap_or_default()
}

/// Value of one placeholder (without braces); `None` for unknown names.
pub fn placeholder(prj: &Project, page_i: usize, name: &str) -> Option<String> {
    let info = &prj.info;
    let page = &prj.pages[page_i];
    let rev = crate::revision::last_released(prj);
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
        "page.installation" => page.installation.clone(),
        "page.product" => page.product.clone(),
        "page.function.description" => structure_description(prj, crate::structure::Aspect::Function, &page.function),
        "page.location.description" => structure_description(prj, crate::structure::Aspect::Location, &page.location),
        "page.type" => match page.kind {
            PageKind::Schematic => tr("Schematic"),
            PageKind::Cover => tr("Cover"),
            PageKind::Report(_) => tr("Report"),
            PageKind::Mounting(_) => tr("Mounting"),
            PageKind::Overview => tr("Overview"),
            PageKind::Graphic => tr("Graphics"),
            PageKind::Document => tr("Document"),
        }
        .into(),
        "sheet" => (page_i + 1).to_string(),
        "sheets" => prj.pages.len().to_string(),
        "rev.name" => rev.map(|r| r.name.clone()).unwrap_or_default(),
        "rev.date" => rev.map(|r| r.date.clone()).unwrap_or_default(),
        "rev.author" => rev.map(|r| r.author.clone()).unwrap_or_default(),
        "rev.description" => rev.map(|r| r.description.clone()).unwrap_or_default(),
        _ => prop_placeholder(prj, page, name.strip_prefix("prop:")?),
    };
    Some(v)
}

/// `{prop:key}`: a project or page property of the database (shown with its unit),
/// otherwise a free project property by name.
fn prop_placeholder(prj: &Project, page: &crate::model::Page, key: &str) -> String {
    use crate::properties::{self, ObjectClass, Object};
    match properties::valid_key(prj, key).map(|d| d.class) {
        Some(ObjectClass::Project) => properties::display(prj, &Object::Project, key),
        Some(ObjectClass::Page) => properties::display(prj, &Object::Page(page.id), key),
        _ => prj.info.properties.get(key).cloned().unwrap_or_default(),
    }
}

/// Placeholders offered in the title block editor: (placeholder, meaning).
pub const PLACEHOLDERS: [(&str, &str); 28] = [
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
    ("{page.installation}", tr_const!("Installation site of the page")),
    ("{page.product}", tr_const!("Higher-level product of the page")),
    ("{page.function.description}", tr_const!("Description of the page's function (structure management)")),
    ("{page.location.description}", tr_const!("Description of the page's location (structure management)")),
    ("{page.type}", tr_const!("Page type")),
    ("{sheet}", tr_const!("Sheet number")),
    ("{sheets}", tr_const!("Number of sheets")),
    ("{rev.name}", tr_const!("Last revision")),
    ("{rev.date}", tr_const!("Date of the last revision")),
    ("{rev.author}", tr_const!("Author of the last revision")),
    ("{rev.description}", tr_const!("Description of the last revision")),
    ("{prop:Name}", "Project property \"Name\""),
    ("{prop:project.customer}", tr_const!("Project property of the property database (by key)")),
    ("{prop:page.drawing_number}", tr_const!("Page property of the property database (by key)")),
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

/// Draws a title block for a page into the title block area of its plot frame.
pub fn draw(d: &mut dyn Draw, form: &Form, prj: &Project, page_i: usize) {
    let frame = crate::frame::of_page(prj, page_i);
    draw_at(d, form, prj, page_i, frame.title_origin(), frame.title_scale());
}

/// Draws a title block with its top left corner at `o`, scaled by `k`.
pub fn draw_at(d: &mut dyn Draw, form: &Form, prj: &Project, page_i: usize, o: P, k: f32) {
    let at = |q: P| o + q * k;
    for item in &form.items {
        match item {
            FormItem::Line { a, b, width } => d.line(at(*a), at(*b), width * k, Ink::Frame, false),
            FormItem::Rect { a, b, width, fill } => {
                if *fill {
                    d.rect(at(*a), at(*b), 0.0, Ink::Plate, true);
                }
                if *width > 0.0 {
                    d.rect(at(*a), at(*b), width * k, Ink::Frame, false);
                }
            }
            FormItem::Text { pos, text, size, anchor, label, max_width } => {
                let ink = if *label { Ink::FrameText } else { Ink::Text };
                let value = fill(text, prj, page_i);
                for (i, l) in value.lines().enumerate() {
                    let w = text_width(l, *size);
                    let size = if *max_width > 0.0 && w > *max_width { (size * max_width / w).max(1.0) } else { *size };
                    d.text(at(*pos + p(0.0, i as f32 * size * 1.4)), l, size * k, *anchor, VAnchor::Middle, ink);
                }
            }
        }
    }
}

/// Indices of the report columns shown by a form (all if it chooses none or none
/// of its columns exist).
pub fn report_columns(form: Option<&ReportForm>, headers: &[Text]) -> Vec<usize> {
    let chosen: Vec<usize> = form
        .map(|f| f.columns.iter().filter_map(|c| headers.iter().position(|h| header_matches(h, c))).collect())
        .unwrap_or_default();
    if chosen.is_empty() {
        (0..headers.len()).collect()
    } else {
        chosen
    }
}

/// The value a form stores for a header (by key or any of its translations).
pub fn header_value<'a, T>(map: &'a BTreeMap<String, T>, header: &Text) -> Option<&'a T> {
    map.iter().find(|(k, _)| header_matches(header, k)).map(|(_, v)| v)
}

#[cfg(test)]
mod tests;
