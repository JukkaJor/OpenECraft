//! Report blocks and report pages: a report block generates the pages of one report
//! with a scheme (filter, sort order, grouping) – one page set per strip, plug,
//! cable, device … if wanted – and replaces them when it is generated again. Report
//! pages can also be output into other projects as fixed pages.

use openecraft_i18n::{tr, tr_fmt};
use crate::forms::{self, ReportForm};
use crate::model::*;
use crate::reports::{Analysis, Table};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Rows per report page if the form sets none.
pub const ROWS_PER_PAGE: usize = 40;
/// Rows per page of the terminal line-up (the drawing of the strip takes space).
pub const LINEUP_ROWS_PER_PAGE: usize = 24;

/// What a report page shows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportPart {
    /// Report block that generated the page (output pages: source project and block).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub block: String,
    /// Report scheme; empty: the form of the report.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub scheme: String,
    /// Only the rows of this value of the group column; none: all rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// First row shown.
    pub first: usize,
    /// Fixed content output from another project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frozen: Option<PageTable>,
}

/// A set of report pages generated together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportBlock {
    pub name: String,
    pub kind: ReportKind,
    /// Report scheme; empty: the form of the report.
    pub scheme: String,
    /// New pages for every value of the group column (strip, plug, cable, device …).
    pub per_group: bool,
    /// Groups to generate (`-X*`, several separated by `,`); empty: all.
    pub groups: String,
    /// Structure of the generated pages.
    pub function: String,
    pub location: String,
}

impl Default for ReportBlock {
    fn default() -> Self {
        ReportBlock {
            name: String::new(),
            kind: ReportKind::TerminalDiagram,
            scheme: String::new(),
            per_group: true,
            groups: String::new(),
            function: String::new(),
            location: String::new(),
        }
    }
}

/// One element of the terminal line-up drawing.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LineupBox {
    pub label: String,
    pub width: f32,
    pub jumper: u32,
    /// End bracket or end plate.
    pub accessory: bool,
    pub spare: bool,
}

/// The content of one report page, ready to draw.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PageTable {
    pub title: String,
    pub headers: Vec<String>,
    /// Relative column widths.
    pub widths: Vec<f32>,
    pub rows: Vec<Vec<String>>,
    #[serde(skip)]
    pub targets: Vec<Option<(usize, u64)>>,
    /// Rows the row height is fitted to.
    pub per_page: usize,
    /// Text size of the rows (mm); 0: fitted.
    pub text_size: f32,
    /// Rows of the report that follow this page.
    pub more: usize,
    /// The rows continue on the next page of the report block.
    pub continued: bool,
    /// Terminal line-up drawn above the table.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lineup: Vec<LineupBox>,
    /// Project the page was output from.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub source: String,
}

fn column(table: &Table, name: &str) -> Option<usize> {
    table.headers.iter().position(|h| *h == name)
}

/// The rows of a report filtered and sorted by a form.
pub fn scheme_table(table: &Table, form: Option<&ReportForm>) -> Table {
    let Some(form) = form else { return table.clone() };
    let filters: Vec<(usize, &forms::RowFilter)> = form.filters.iter().filter_map(|f| Some((column(table, &f.column)?, f))).collect();
    // (cells, jump target)
    type Row = (Vec<String>, Option<(usize, u64)>);
    let mut rows: Vec<Row> = table
        .rows
        .iter()
        .zip(&table.targets)
        .filter(|(r, _)| filters.iter().all(|(c, f)| f.passes(r.get(*c).map(String::as_str).unwrap_or(""))))
        .map(|(r, t)| (r.clone(), *t))
        .collect();
    let keys: Vec<(usize, bool)> = form.sort.iter().filter_map(|k| Some((column(table, &k.column)?, k.descending))).collect();
    if !keys.is_empty() {
        rows.sort_by(|a, b| {
            keys.iter().fold(std::cmp::Ordering::Equal, |o, (c, desc)| {
                o.then_with(|| {
                    let ord = natural_cmp(a.0.get(*c).map(String::as_str).unwrap_or(""), b.0.get(*c).map(String::as_str).unwrap_or(""));
                    if *desc { ord.reverse() } else { ord }
                })
            })
        });
    }
    let (rows, targets) = rows.into_iter().unzip();
    Table { headers: table.headers.clone(), widths: table.widths.clone(), rows, targets }
}

/// Column that groups the rows: the form's group column, otherwise the first one.
pub fn group_column(table: &Table, form: Option<&ReportForm>) -> usize {
    form.and_then(|f| column(table, &f.group)).unwrap_or(0)
}

pub fn rows_per_page(kind: ReportKind, form: Option<&ReportForm>) -> usize {
    let default = if kind == ReportKind::TerminalLineup { LINEUP_ROWS_PER_PAGE } else { ROWS_PER_PAGE };
    form.map(|f| f.rows).filter(|r| *r > 0).unwrap_or(default)
}

/// Values of the group column in order of appearance.
pub fn groups(table: &Table, col: usize) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for r in &table.rows {
        let g = r.get(col).cloned().unwrap_or_default();
        if v.last() != Some(&g) && !v.contains(&g) {
            v.push(g);
        }
    }
    v
}

/// `pattern` matches `text`: wildcards `*` and `?`, several patterns separated by `,`
/// or `;`, case-insensitive; an empty pattern matches everything.
pub fn matches(pattern: &str, text: &str) -> bool {
    fn glob(p: &[char], t: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('*') => glob(&p[1..], t) || (!t.is_empty() && glob(p, &t[1..])),
            Some('?') => !t.is_empty() && glob(&p[1..], &t[1..]),
            Some(c) => t.first() == Some(c) && glob(&p[1..], &t[1..]),
        }
    }
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let mut any = false;
    for p in pattern.split([',', ';']).map(str::trim).filter(|p| !p.is_empty()) {
        any = true;
        if glob(&p.to_lowercase().chars().collect::<Vec<_>>(), &text) {
            return true;
        }
    }
    !any
}

/// Content of a report page; `None` for other pages.
pub fn page_table(prj: &Project, an: &Analysis, page_i: usize) -> Option<PageTable> {
    let page = prj.pages.get(page_i)?;
    let PageKind::Report(kind) = page.kind else { return None };
    let part = page.report.clone().unwrap_or_default();
    if let Some(frozen) = part.frozen {
        return Some(frozen);
    }
    let form = forms::report_form(prj, kind, &part.scheme);
    let table = scheme_table(&an.report(prj, kind), form);
    let gc = group_column(&table, form);
    let rows: Vec<usize> = (0..table.rows.len()).filter(|&i| part.group.as_ref().is_none_or(|g| table.rows[i].get(gc) == Some(g))).collect();
    let per_page = rows_per_page(kind, form);
    let shown: Vec<usize> = rows.iter().skip(part.first).take(per_page).copied().collect();
    let more = rows.len().saturating_sub(part.first + shown.len());
    let cols = forms::report_columns(form, &table.headers);
    let width = |c: usize| form.and_then(|f| f.widths.get(table.headers[c])).copied().filter(|w| *w > 0.0).unwrap_or(table.widths[c]);
    let mut out_rows: Vec<Vec<String>> = shown.iter().map(|&i| cols.iter().map(|&c| table.rows[i].get(c).cloned().unwrap_or_default()).collect()).collect();
    if form.is_some_and(|f| f.blank_repeats) {
        for i in (1..out_rows.len()).rev() {
            if !out_rows[i].is_empty() && out_rows[i][0] == out_rows[i - 1][0] {
                out_rows[i][0].clear();
            }
        }
    }
    let base = form.map(|f| f.title.as_str()).filter(|t| !t.is_empty()).unwrap_or(kind.title());
    let title = match part.group.as_deref() {
        Some(g) if !g.is_empty() => format!("{base} {g}"),
        _ => base.to_string(),
    };
    let lineup = if kind == ReportKind::TerminalLineup { lineup_boxes(&table, &shown) } else { Vec::new() };
    Some(PageTable {
        title,
        headers: cols.iter().map(|&c| table.headers[c].to_string()).collect(),
        widths: cols.iter().map(|&c| width(c)).collect(),
        rows: out_rows,
        targets: shown.iter().map(|&i| table.targets[i]).collect(),
        per_page,
        text_size: form.map(|f| f.text_size).unwrap_or(0.0),
        more,
        continued: more > 0 && !part.block.is_empty(),
        lineup,
        source: String::new(),
    })
}

/// Drawing elements of the terminal line-up rows (all columns of the report).
fn lineup_boxes(table: &Table, rows: &[usize]) -> Vec<LineupBox> {
    let (Some(element), Some(label), Some(jumper), Some(width)) =
        (column(table, tr("Element")), column(table, tr("Designation")), column(table, tr("Jumper")), column(table, tr("Width (mm)")))
    else {
        return Vec::new();
    };
    rows.iter()
        .map(|&i| {
            let r = &table.rows[i];
            let kind = r[element].as_str();
            LineupBox {
                label: r[label].clone(),
                width: r[width].parse().unwrap_or(crate::strips::TERMINAL_WIDTH),
                jumper: r[jumper].parse().unwrap_or(0),
                accessory: r[label].is_empty(),
                spare: kind == tr("Spare terminal"),
            }
        })
        .collect()
}

/// The pages of a block: (group, first row) per page.
pub fn block_pages(prj: &Project, an: &Analysis, block: &ReportBlock) -> Vec<(Option<String>, usize)> {
    let form = forms::report_form(prj, block.kind, &block.scheme);
    let table = scheme_table(&an.report(prj, block.kind), form);
    let per_page = rows_per_page(block.kind, form);
    let chunks = |n: usize| (0..n.div_ceil(per_page).max(1)).map(move |k| k * per_page);
    if !block.per_group {
        return chunks(table.rows.len()).map(|f| (None, f)).collect();
    }
    let gc = group_column(&table, form);
    let mut out = Vec::new();
    for g in groups(&table, gc).into_iter().filter(|g| matches(&block.groups, g)) {
        let n = table.rows.iter().filter(|r| r.get(gc) == Some(&g)).count();
        out.extend(chunks(n).map(|f| (Some(g.clone()), f)));
    }
    out
}

/// Removes the pages that `mine` selects and returns where they were and their names.
fn take_pages(prj: &mut Project, mine: impl Fn(&Page) -> bool) -> Result<(usize, VecDeque<String>), String> {
    if let Some(pg) = prj.pages.iter().find(|p| mine(p) && p.locked) {
        return Err(tr_fmt!("Page {} is locked", pg.name));
    }
    let at = prj.pages.iter().position(&mine).unwrap_or(prj.pages.len());
    let names = prj.pages.iter().filter(|p| mine(p)).map(|p| p.name.clone()).collect();
    prj.pages.retain(|p| !mine(p));
    Ok((at, names))
}

/// Generates the pages of a report block, replacing the ones it generated before (at
/// their place, with their page names). Returns the number of pages.
pub fn generate(prj: &mut Project, an: &Analysis, block: usize) -> Result<usize, String> {
    let b = prj.report_blocks.get(block).cloned().ok_or_else(|| tr_fmt!("No report block {}", block + 1))?;
    let mine = |p: &Page| p.report.as_ref().is_some_and(|r| r.frozen.is_none() && r.block == b.name);
    let (at, mut names) = take_pages(prj, mine)?;
    let pages = block_pages(prj, an, &b);
    let title = forms::report_form(prj, b.kind, &b.scheme).map(|f| f.title.clone()).filter(|t| !t.is_empty()).unwrap_or_else(|| b.kind.title().to_string());
    for (i, (group, first)) in pages.iter().enumerate() {
        let name = names.pop_front().unwrap_or_else(|| crate::ops::next_page_name(prj));
        let description = match group {
            Some(g) if !g.is_empty() => format!("{title} {g}"),
            _ => title.clone(),
        };
        let id = prj.new_id();
        let mut page = Page::new(id, &name, &description, PageKind::Report(b.kind));
        page.function = b.function.clone();
        page.location = b.location.clone();
        page.report = Some(ReportPart { block: b.name.clone(), scheme: b.scheme.clone(), group: group.clone(), first: *first, frozen: None });
        prj.pages.insert(at + i, page);
    }
    Ok(pages.len())
}

/// Renames a report block and its pages. Fails for empty names and names in use.
pub fn rename_block(prj: &mut Project, old: &str, new: &str) -> bool {
    let new = new.trim();
    if new.is_empty() || prj.report_blocks.iter().any(|b| b.name == new) {
        return false;
    }
    let Some(b) = prj.report_blocks.iter_mut().find(|b| b.name == old) else { return false };
    b.name = new.to_string();
    for r in prj.pages.iter_mut().filter_map(|p| p.report.as_mut()).filter(|r| r.frozen.is_none() && r.block == old) {
        r.block = new.to_string();
    }
    true
}

/// Generates all report blocks. Returns the number of pages.
pub fn generate_all(prj: &mut Project) -> Result<usize, String> {
    let an = Analysis::build(prj);
    let mut n = 0;
    for i in 0..prj.report_blocks.len() {
        n += generate(prj, &an, i)?;
    }
    Ok(n)
}

/// Report pages of a project (all, or those of one report block).
pub fn report_pages(prj: &Project, block: Option<&str>) -> Vec<usize> {
    (0..prj.pages.len())
        .filter(|&i| matches!(prj.pages[i].kind, PageKind::Report(_)))
        .filter(|&i| block.is_none_or(|b| prj.pages[i].report.as_ref().is_some_and(|r| r.block == b)))
        .collect()
}

/// Outputs report pages of `src` into `target` as fixed pages (they keep their
/// content when the target is changed). Pages output from the same project before
/// are replaced. Returns the number of pages.
pub fn output_to_project(src: &Project, an: &Analysis, pages: &[usize], target: &mut Project) -> Result<usize, String> {
    let source = src.info.name.clone();
    let from_here = |p: &Page| p.report.as_ref().and_then(|r| r.frozen.as_ref()).is_some_and(|f| f.source == source);
    let (at, mut names) = take_pages(target, from_here)?;
    target.fix_next_id();
    let mut n = 0;
    for &pi in pages {
        let Some(mut table) = page_table(src, an, pi) else { continue };
        table.source = source.clone();
        table.targets.clear();
        table.continued = table.continued || table.more > 0;
        let sp = &src.pages[pi];
        let name = names.pop_front().unwrap_or_else(|| crate::ops::next_page_name(target));
        let id = target.new_id();
        let mut page = Page::new(id, &name, &sp.description, sp.kind);
        page.function = sp.function.clone();
        page.location = sp.location.clone();
        let block = sp.report.as_ref().map(|r| r.block.clone()).unwrap_or_default();
        page.report = Some(ReportPart { block, frozen: Some(table), ..Default::default() });
        target.pages.insert(at + n, page);
        n += 1;
    }
    Ok(n)
}

/// [`output_to_project`] on a project file: returns the changed file content.
pub fn output_to_file(src: &Project, an: &Analysis, pages: &[usize], target: &str) -> Result<(String, usize), String> {
    // Not `Project::from_json`: that registers the symbols of the other project.
    let mut prj: Project = serde_json::from_str(target).map_err(|e| tr_fmt!("Not a valid project file: {}", e))?;
    let n = output_to_project(src, an, pages, &mut prj)?;
    Ok((prj.to_json(), n))
}

#[cfg(test)]
mod tests;
