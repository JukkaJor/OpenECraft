//! Excel workbooks (Office Open XML, `.xlsx`): writing tables as sheets (bold, frozen
//! header row with filter, column widths, numbers as numbers) and reading the cells of
//! all sheets as text, for the round trip of tables and reports through a spreadsheet.

use crate::xml::{self, esc, Element};
use crate::zip::{read_zip, ZipWriter};
use openecraft_i18n::tr;
use std::collections::HashMap;
use std::fmt::Write;

/// A sheet: name and rows of cells (the first row is the header).
pub type Sheet = (String, Vec<Vec<String>>);

const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PKG_REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// Column letters of a 0-based index (`0` → `A`, `27` → `AB`).
pub fn column_name(mut i: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'A' + (i % 26) as u8);
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

/// Most rows and columns of a sheet (Excel's limits, `XFD1048576`).
const MAX_ROWS: usize = 1 << 20;
const MAX_COLUMNS: usize = 1 << 14;

/// 0-based column of a cell reference (`B7` → 1); None beyond the last column.
fn column_of(r: &str) -> Option<usize> {
    let letters: Vec<u8> = r.bytes().take_while(u8::is_ascii_alphabetic).map(|b| b.to_ascii_uppercase()).collect();
    if letters.is_empty() || letters.len() > 3 {
        return None;
    }
    Some(letters.iter().fold(0usize, |n, b| n * 26 + (b - b'A' + 1) as usize) - 1).filter(|c| *c < MAX_COLUMNS)
}

/// A cell value written as number: plain decimals without leading zeros or signs that
/// a spreadsheet would change (`007`, `+5`, `1e3` stay texts).
fn is_number(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() || t != s || t.len() > 15 || t.starts_with('+') || t.contains(['e', 'E']) {
        return false;
    }
    let digits = t.strip_prefix('-').unwrap_or(t);
    let int = digits.split('.').next().unwrap_or("");
    if int.len() > 1 && int.starts_with('0') {
        return false;
    }
    digits.split('.').count() <= 2 && !int.is_empty() && digits.chars().all(|c| c.is_ascii_digit() || c == '.') && !digits.ends_with('.')
}

/// Removes characters XML cannot hold.
fn clean(s: &str) -> String {
    s.chars().filter(|c| matches!(c, '\t' | '\n' | '\r') || *c >= ' ').collect()
}

/// Sheet name as Excel allows it (31 characters, no `[]:*?/\`), unique.
fn sheet_name(name: &str, used: &[String]) -> String {
    let base: String = name.chars().map(|c| if "[]:*?/\\".contains(c) { '_' } else { c }).take(28).collect();
    let base = if base.trim().is_empty() { "Sheet".to_string() } else { base };
    let mut n = base.clone();
    let mut k = 2;
    while used.iter().any(|u| u.eq_ignore_ascii_case(&n)) {
        n = format!("{base} {k}");
        k += 1;
    }
    n
}

fn sheet_xml(rows: &[Vec<String>]) -> String {
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"{MAIN}\">");
    if !rows.is_empty() {
        out.push_str("<sheetViews><sheetView workbookViewId=\"0\"><pane ySplit=\"1\" topLeftCell=\"A2\" activePane=\"bottomLeft\" state=\"frozen\"/></sheetView></sheetViews>");
    }
    if cols > 0 {
        out.push_str("<cols>");
        for c in 0..cols {
            let widest = rows.iter().filter_map(|r| r.get(c)).map(|v| v.lines().map(|l| l.chars().count()).max().unwrap_or(0)).max().unwrap_or(0);
            let _ = write!(out, "<col min=\"{0}\" max=\"{0}\" width=\"{1}\" customWidth=\"1\"/>", c + 1, (widest as f32 * 1.1 + 2.0).clamp(6.0, 60.0));
        }
        out.push_str("</cols>");
    }
    out.push_str("<sheetData>");
    for (ri, row) in rows.iter().enumerate() {
        let _ = write!(out, "<row r=\"{}\">", ri + 1);
        for (ci, v) in row.iter().enumerate() {
            if v.is_empty() {
                continue;
            }
            let r = format!("{}{}", column_name(ci), ri + 1);
            let style = if ri == 0 { " s=\"1\"" } else { "" };
            if ri > 0 && is_number(v) {
                let _ = write!(out, "<c r=\"{r}\"{style}><v>{v}</v></c>");
            } else {
                let _ = write!(out, "<c r=\"{r}\"{style} t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>", esc(&clean(v)).replace("&#10;", "\n"));
            }
        }
        out.push_str("</row>");
    }
    out.push_str("</sheetData>");
    if cols > 0 && !rows.is_empty() {
        let _ = write!(out, "<autoFilter ref=\"A1:{}{}\"/>", column_name(cols - 1), rows.len().max(1));
    }
    out.push_str("</worksheet>");
    out
}

/// A workbook with one sheet per table.
pub fn write(sheets: &[Sheet]) -> Vec<u8> {
    let mut names: Vec<String> = Vec::new();
    for (name, _) in sheets {
        let n = sheet_name(name, &names);
        names.push(n);
    }
    let mut z = ZipWriter::new("2026-01-01");
    let mut types = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>");
    for i in 0..sheets.len() {
        let _ = write!(types, "<Override PartName=\"/xl/worksheets/sheet{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>", i + 1);
    }
    types.push_str("</Types>");
    z.add("[Content_Types].xml", types.as_bytes());
    z.add("_rels/.rels", format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"{PKG_REL}\"><Relationship Id=\"rId1\" Type=\"{REL}/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>").as_bytes());
    let mut wb = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<workbook xmlns=\"{MAIN}\" xmlns:r=\"{REL}\"><sheets>");
    let mut rels = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"{PKG_REL}\">");
    for (i, n) in names.iter().enumerate() {
        let _ = write!(wb, "<sheet name=\"{}\" sheetId=\"{}\" r:id=\"rId{}\"/>", esc(n), i + 1, i + 1);
        let _ = write!(rels, "<Relationship Id=\"rId{}\" Type=\"{REL}/worksheet\" Target=\"worksheets/sheet{}.xml\"/>", i + 1, i + 1);
    }
    let _ = write!(rels, "<Relationship Id=\"rId{}\" Type=\"{REL}/styles\" Target=\"styles.xml\"/></Relationships>", sheets.len() + 1);
    wb.push_str("</sheets></workbook>");
    z.add("xl/workbook.xml", wb.as_bytes());
    z.add("xl/_rels/workbook.xml.rels", rels.as_bytes());
    let styles = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<styleSheet xmlns=\"{MAIN}\"><fonts count=\"2\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font><font><b/><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts><fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"2\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/><xf numFmtId=\"0\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/></cellXfs></styleSheet>"
    );
    z.add("xl/styles.xml", styles.as_bytes());
    for (i, (_, rows)) in sheets.iter().enumerate() {
        z.add(&format!("xl/worksheets/sheet{}.xml", i + 1), sheet_xml(rows).as_bytes());
    }
    z.finish()
}

/// Text of a string item (`<si>`: `<t>` or rich text runs `<r><t>`).
fn item_text(e: &Element) -> String {
    match e.child("t") {
        Some(t) => t.text.clone(),
        None => e.children("r").filter_map(|r| r.child("t")).map(|t| t.text.as_str()).collect(),
    }
}

/// Number written back as it was typed (`2.0` → `2`).
fn number_text(v: &str) -> String {
    match v.parse::<f64>() {
        Ok(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", f as i64),
        Ok(f) => {
            let s = format!("{f}");
            if s.len() <= v.len() { s } else { v.to_string() }
        }
        Err(_) => v.to_string(),
    }
}

/// The sheets of a workbook with their cells as text (gaps filled with empty cells).
pub fn read(bytes: &[u8]) -> Result<Vec<Sheet>, String> {
    let files: HashMap<String, Vec<u8>> = read_zip(bytes)?.into_iter().collect();
    let text = |name: &str| files.get(name).map(|b| String::from_utf8_lossy(b).into_owned());
    let not_xlsx = || tr("Not an Excel workbook (.xlsx)").to_string();
    let wb = xml::parse(&text("xl/workbook.xml").ok_or_else(not_xlsx)?)?;
    let rels = text("xl/_rels/workbook.xml.rels").map(|t| xml::parse(&t)).transpose()?;
    let target = |id: &str| -> Option<String> {
        let r = rels.as_ref()?.children("Relationship").find(|r| r.attr("Id") == Some(id))?;
        let t = r.attr("Target")?;
        Some(t.strip_prefix('/').map(String::from).unwrap_or_else(|| format!("xl/{t}")))
    };
    let shared: Vec<String> = match text("xl/sharedStrings.xml") {
        Some(t) => xml::parse(&t)?.children("si").map(item_text).collect(),
        None => Vec::new(),
    };
    let mut out = Vec::new();
    for (i, s) in wb.descendants("sheet").into_iter().enumerate() {
        let name = s.attr("name").unwrap_or_default().to_string();
        let path = s.attr("id").and_then(target).unwrap_or_else(|| format!("xl/worksheets/sheet{}.xml", i + 1));
        let Some(sheet) = text(&path) else { continue };
        let sheet = xml::parse(&sheet)?;
        let mut rows: Vec<Vec<String>> = Vec::new();
        for row in sheet.descendants("row") {
            // Row numbers out of range count as missing (the row after the last one).
            let r = row.attr("r").and_then(|r| r.parse::<usize>().ok()).filter(|r| (1..=MAX_ROWS).contains(r)).unwrap_or(rows.len() + 1);
            while rows.len() < r {
                rows.push(Vec::new());
            }
            let cells = &mut rows[r - 1];
            for c in row.children("c") {
                let col = c.attr("r").and_then(column_of).unwrap_or(cells.len());
                let v = c.child("v").map(|v| v.text.as_str()).unwrap_or("");
                let value = match c.attr("t") {
                    Some("s") => v.parse::<usize>().ok().and_then(|i| shared.get(i).cloned()).unwrap_or_default(),
                    Some("inlineStr") => c.child("is").map(item_text).unwrap_or_default(),
                    Some("b") => if v == "1" { "TRUE".into() } else { "FALSE".into() },
                    Some("str") | Some("e") => v.to_string(),
                    _ => number_text(v),
                };
                while cells.len() <= col {
                    cells.push(String::new());
                }
                cells[col] = value;
            }
        }
        out.push((name, rows));
    }
    Ok(out)
}

/// The rows of a workbook's first sheet as CSV text (`;` separated, like the CSV
/// exports), so every CSV import also takes `.xlsx` files.
pub fn first_sheet_csv(bytes: &[u8]) -> Result<String, String> {
    let sheets = read(bytes)?;
    let (_, rows) = sheets.into_iter().next().ok_or_else(|| tr("The workbook has no sheet").to_string())?;
    let esc = |s: &str| if s.contains([';', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
    let mut out = String::new();
    for r in rows {
        out.push_str(&r.iter().map(|c| esc(c)).collect::<Vec<_>>().join(";"));
        out.push('\n');
    }
    Ok(out)
}

/// A CSV text (`;` separated, first row header) as rows for [`write`].
pub fn rows_of_csv(text: &str) -> Vec<Vec<String>> {
    openecraft_parts::parse_csv(text.trim_start_matches('\u{feff}'))
}

#[cfg(test)]
mod tests;
