//! Graphical report pages (in the spirit of EPLAN's graphical forms): the terminal
//! diagram draws the strip with a box per terminal, jumper bridges and the targets on
//! both sides; the cable diagram draws each cable with its cores from end to end; the
//! plug diagram draws each plug with its socket and pin halves per pin; the PLC
//! assignment draws each card as a module with inputs coming in from the left and
//! outputs going out to the right.
//! They work from the rows of the report page in the report's own column order.

use super::{Draw, Ink, VAnchor, SYMBOL_W};
use crate::frame::PlotFrame;
use crate::geom::p;
use crate::model::{ReportId, ReportKind};
use crate::report_blocks::PageTable;
use crate::symbols::Anchor;
use openecraft_i18n::{tr, tr_fmt};

/// Height of a row of a graphical report (mm).
const ROW: f32 = 6.0;
const SIZE: f32 = 2.2;

fn cell(row: &[String], i: usize) -> &str {
    row.get(i).map(String::as_str).unwrap_or("")
}

/// Shortens a text to `max` characters (with an ellipsis).
fn fit(s: &str, max: usize) -> String {
    if s.chars().count() > max && max > 1 {
        s.chars().take(max - 1).collect::<String>() + "…"
    } else {
        s.to_string()
    }
}

/// Draws a graphical report page; false if the report has no graphical form.
pub fn draw(d: &mut dyn Draw, kind: ReportId, t: &PageTable, frame: &PlotFrame) -> bool {
    if kind == ReportId::from(ReportKind::TerminalDiagram) {
        terminal_diagram(d, t, frame);
    } else if kind == ReportId::from(ReportKind::CableDiagram) {
        cable_diagram(d, t, frame);
    } else if kind == ReportId::from(ReportKind::PlugDiagram) {
        plug_diagram(d, t, frame);
    } else if kind == ReportId::from(ReportKind::PlcAssignment) {
        plc_diagram(d, t, frame);
    } else {
        return false;
    }
    true
}

/// Rows of the terminal diagram (columns: strip, terminal, level, jumper, external,
/// internal, potential, location, function text).
fn terminal_diagram(d: &mut dyn Draw, t: &PageTable, frame: &PlotFrame) {
    let area = frame.area();
    let (x0, x1) = (area.min.x + 7.0, area.max.x - 10.0);
    let top = area.min.y;
    d.text(p(x0, top + 9.0), &t.title, 6.0, Anchor::Left, VAnchor::Middle, Ink::Text);
    let w = x1 - x0;
    // Columns: potential | external targets | strip | internal targets | function text | location.
    let (pot_x, ext_r, box_l) = (x0, x0 + w * 0.36, x0 + w * 0.40);
    let box_r = box_l + 14.0;
    let (int_l, ft_l, loc_r) = (box_r + 4.0, x0 + w * 0.80, x1);
    let mut y = top + 19.0;
    for (x, h, a) in [(pot_x, tr("Potential"), Anchor::Left), (ext_r, tr("External"), Anchor::Right), ((box_l + box_r) / 2.0, tr("Terminal"), Anchor::Center), (int_l, tr("Internal"), Anchor::Left), (ft_l, tr("Function text"), Anchor::Left)] {
        d.text(p(x, y), h, 2.4, a, VAnchor::Middle, Ink::Text);
    }
    y += 4.0;
    d.line(p(x0, y), p(x1, y), 0.3, Ink::Frame, false);
    let mut strip = String::new();
    let mut jumpers: Vec<(String, f32)> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let ext_chars = ((ext_r - pot_x - 16.0) / (SIZE * 0.6)) as usize;
    let int_chars = ((ft_l - int_l - 2.0) / (SIZE * 0.6)) as usize;
    let ft_chars = ((loc_r - ft_l - 16.0) / (SIZE * 0.6)) as usize;
    for (ri, row) in t.rows.iter().enumerate() {
        let target = t.targets.get(ri).copied().flatten();
        let (label, level, jumper) = (cell(row, 1), cell(row, 2), cell(row, 3));
        let (ext, int, pot, loc, ft) = (cell(row, 4), cell(row, 5), cell(row, 6), cell(row, 7), cell(row, 8));
        // Jumper rows are listed below the strip.
        if target.is_none() && !jumper.is_empty() {
            notes.push([label, jumper, ext, int, loc, ft].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join("  "));
            continue;
        }
        if cell(row, 0) != strip {
            strip = cell(row, 0).to_string();
            y += 2.0;
            d.text(p(box_l, y + 2.5), &strip, 3.0, Anchor::Left, VAnchor::Middle, Ink::Text);
            y += 6.0;
        }
        let mid = y + ROW / 2.0;
        let accessory = target.is_none() && ft.is_empty() && !ext.is_empty();
        if accessory {
            // End bracket / end plate: a narrow filled bar with its part.
            d.rect(p(box_l, y + 1.8), p(box_r, y + ROW - 1.8), SYMBOL_W, Ink::Frame, true);
            d.text(p(int_l, mid), &fit(&format!("{label}  {ext}"), int_chars), SIZE, Anchor::Left, VAnchor::Middle, Ink::FrameText);
            y += ROW;
            continue;
        }
        let spare = target.is_none();
        d.rect(p(box_l, y + 0.6), p(box_r, y + ROW - 0.6), SYMBOL_W * 1.5, if spare { Ink::Frame } else { Ink::Line }, false);
        d.text(p((box_l + box_r) / 2.0, mid), label, 2.6, Anchor::Center, VAnchor::Middle, if spare { Ink::FrameText } else { Ink::Text });
        if !level.is_empty() {
            d.text(p(box_r - 0.8, y + 1.6), level, 1.4, Anchor::Right, VAnchor::Middle, Ink::FrameText);
        }
        if !spare {
            d.line(p(ext_r + 1.0, mid), p(box_l, mid), SYMBOL_W, Ink::Line, false);
            d.line(p(box_r, mid), p(int_l - 1.0, mid), SYMBOL_W, Ink::Line, false);
            d.circle(p(box_l, mid), 0.5, 0.0, Ink::Line, true);
            d.circle(p(box_r, mid), 0.5, 0.0, Ink::Line, true);
        }
        d.text(p(ext_r, mid), &fit(ext, ext_chars), SIZE, Anchor::Right, VAnchor::Middle, Ink::Text);
        d.text(p(int_l, mid), &fit(int, int_chars), SIZE, Anchor::Left, VAnchor::Middle, Ink::Text);
        d.text(p(pot_x, mid), &fit(pot, 10), SIZE, Anchor::Left, VAnchor::Middle, Ink::Text);
        d.text(p(ft_l, mid), &fit(ft, ft_chars), SIZE, Anchor::Left, VAnchor::Middle, Ink::Text);
        if !loc.is_empty() {
            d.text(p(loc_r, mid), loc, SIZE, Anchor::Right, VAnchor::Middle, Ink::CrossRef);
            if let Some((page, _)) = target {
                d.link(p(loc_r - 14.0, y), p(loc_r, y + ROW), page);
            }
        }
        if !jumper.is_empty() {
            jumpers.push((jumper.to_string(), mid));
        }
        y += ROW;
    }
    // Jumpers: a bridge beside the boxes from the first to the last terminal.
    let mut numbers: Vec<&String> = jumpers.iter().map(|j| &j.0).collect();
    numbers.sort();
    numbers.dedup();
    for (k, n) in numbers.iter().enumerate() {
        let ys: Vec<f32> = jumpers.iter().filter(|j| &j.0 == *n).map(|j| j.1).collect();
        let bx = box_l - 2.5 - 2.0 * (k % 3) as f32;
        if let (Some(a), Some(b)) = (ys.first(), ys.last()) {
            d.line(p(bx, *a), p(bx, *b), SYMBOL_W * 2.0, Ink::Line, false);
            for yy in &ys {
                d.line(p(bx, *yy), p(box_l, *yy), SYMBOL_W, Ink::Line, false);
                d.circle(p(bx, *yy), 0.5, 0.0, Ink::Line, true);
            }
            d.text(p(bx - 0.8, *a), n, 1.6, Anchor::Right, VAnchor::Middle, Ink::FrameText);
        }
    }
    y += 3.0;
    for n in &notes {
        d.text(p(x0, y + 2.0), &fit(n, ((x1 - x0) / (SIZE * 0.6)) as usize), SIZE, Anchor::Left, VAnchor::Middle, Ink::Text);
        y += 4.5;
    }
    footer(d, t, frame);
}

/// Rows of the cable diagram (columns: cable, core, from, to, potential, wire number,
/// cross-section): every cable a band with its cores.
fn cable_diagram(d: &mut dyn Draw, t: &PageTable, frame: &PlotFrame) {
    let area = frame.area();
    let (x0, x1) = (area.min.x + 7.0, area.max.x - 10.0);
    let top = area.min.y;
    d.text(p(x0, top + 9.0), &t.title, 6.0, Anchor::Left, VAnchor::Middle, Ink::Text);
    let w = x1 - x0;
    let (from_r, cab_l) = (x0 + w * 0.34, x0 + w * 0.40);
    let cab_r = cab_l + w * 0.20;
    let to_l = cab_r + w * 0.06;
    let mut y = top + 19.0;
    for (x, h, a) in [(from_r, tr("From"), Anchor::Right), ((cab_l + cab_r) / 2.0, tr("Cable"), Anchor::Center), (to_l, tr("To"), Anchor::Left)] {
        d.text(p(x, y), h, 2.4, a, VAnchor::Middle, Ink::Text);
    }
    y += 4.0;
    d.line(p(x0, y), p(x1, y), 0.3, Ink::Frame, false);
    let chars_from = ((from_r - x0 - 2.0) / (SIZE * 0.6)) as usize;
    let chars_to = ((x1 - to_l) / (SIZE * 0.6)) as usize;
    let mut i = 0;
    while i < t.rows.len() {
        let tag = cell(&t.rows[i], 0).to_string();
        let n = t.rows[i..].iter().take_while(|r| cell(r, 0) == tag).count();
        let band_top = y + 6.0;
        let band_bottom = band_top + n as f32 * ROW;
        let cs = cell(&t.rows[i], 6);
        d.text(p((cab_l + cab_r) / 2.0, y + 3.0), format!("{tag}  {cs}").trim(), 2.8, Anchor::Center, VAnchor::Middle, Ink::Text);
        d.rect(p(cab_l, band_top), p(cab_r, band_bottom), SYMBOL_W * 2.0, Ink::Line, false);
        for (k, row) in t.rows[i..i + n].iter().enumerate() {
            let mid = band_top + (k as f32 + 0.5) * ROW;
            let (core, from, to, pot, number) = (cell(row, 1), cell(row, 2), cell(row, 3), cell(row, 4), cell(row, 5));
            let reserve = to.is_empty() && pot.is_empty() && number.is_empty();
            d.line(p(from_r + 1.0, mid), p(to_l - 1.0, mid), SYMBOL_W, Ink::Line, reserve);
            // The core designation sits in a gap of the core line inside the band.
            let cx = (cab_l + cab_r) / 2.0;
            d.rect(p(cx - 5.0, mid - 1.6), p(cx + 5.0, mid + 1.6), 0.0, Ink::Paper, true);
            d.text(p(cx, mid), core, SIZE, Anchor::Center, VAnchor::Middle, Ink::Text);
            d.text(p(from_r, mid), &fit(from, chars_from), SIZE, Anchor::Right, VAnchor::Middle, if reserve { Ink::FrameText } else { Ink::Text });
            d.text(p(to_l, mid), &fit(to, chars_to), SIZE, Anchor::Left, VAnchor::Middle, Ink::Text);
            let info = [pot, number].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join("  ");
            if !info.is_empty() {
                d.text(p(cab_r + 1.0, mid - 1.3), &info, 1.5, Anchor::Left, VAnchor::Middle, Ink::FrameText);
            }
            if let Some(Some((page, _))) = t.targets.get(i + k) {
                d.link(p(x0, mid - ROW / 2.0), p(x1, mid + ROW / 2.0), *page);
            }
        }
        y = band_bottom + 4.0;
        i += n;
    }
    footer(d, t, frame);
}

/// Rows of the plug diagram (columns: plug, pin, socket side, pin side, potential,
/// location, function text): every plug a housing split into its socket and pin half,
/// a row per pin with the targets on both sides.
fn plug_diagram(d: &mut dyn Draw, t: &PageTable, frame: &PlotFrame) {
    let area = frame.area();
    let (x0, x1) = (area.min.x + 7.0, area.max.x - 10.0);
    let top = area.min.y;
    d.text(p(x0, top + 9.0), &t.title, 6.0, Anchor::Left, VAnchor::Middle, Ink::Text);
    let w = x1 - x0;
    let (pot_x, sock_r, house_l) = (x0, x0 + w * 0.34, x0 + w * 0.38);
    let house_r = house_l + 24.0;
    let face = (house_l + house_r) / 2.0;
    let (pin_l, ft_l, loc_r) = (house_r + 4.0, x0 + w * 0.78, x1);
    let mut y = top + 19.0;
    for (x, h, a) in [(sock_r, tr("Socket side"), Anchor::Right), (face, tr("Pin"), Anchor::Center), (pin_l, tr("Pin side"), Anchor::Left), (ft_l, tr("Function text"), Anchor::Left)] {
        d.text(p(x, y), h, 2.4, a, VAnchor::Middle, Ink::Text);
    }
    y += 4.0;
    d.line(p(x0, y), p(x1, y), 0.3, Ink::Frame, false);
    let sock_chars = ((sock_r - pot_x - 16.0) / (SIZE * 0.6)) as usize;
    let pin_chars = ((ft_l - pin_l - 2.0) / (SIZE * 0.6)) as usize;
    let ft_chars = ((loc_r - ft_l - 16.0) / (SIZE * 0.6)) as usize;
    let mut i = 0;
    while i < t.rows.len() {
        let plug = cell(&t.rows[i], 0).to_string();
        let n = t.rows[i..].iter().take_while(|r| cell(r, 0) == plug).count();
        d.text(p(house_l, y + 4.0), &plug, 3.0, Anchor::Left, VAnchor::Middle, Ink::Text);
        let (h_top, h_bottom) = (y + 7.0, y + 7.0 + n as f32 * ROW);
        d.rect(p(house_l, h_top), p(house_r, h_bottom), SYMBOL_W * 2.0, Ink::Line, false);
        d.line(p(face, h_top), p(face, h_bottom), SYMBOL_W, Ink::Frame, true);
        for (k, row) in t.rows[i..i + n].iter().enumerate() {
            let mid = h_top + (k as f32 + 0.5) * ROW;
            let (pin, sock, side, pot, loc, ft) = (cell(row, 1), cell(row, 2), cell(row, 3), cell(row, 4), cell(row, 5), cell(row, 6));
            // Socket: a cup opening towards the mating face; pin: a line ending in it.
            d.line(p(sock_r + 1.0, mid), p(face - 3.0, mid), SYMBOL_W, Ink::Line, sock.is_empty());
            d.poly(&[p(face - 1.2, mid - 1.2), p(face - 2.4, mid - 1.2), p(face - 2.4, mid + 1.2), p(face - 1.2, mid + 1.2)], false, SYMBOL_W, Ink::Line, false);
            d.line(p(face - 0.4, mid), p(pin_l - 1.0, mid), SYMBOL_W, Ink::Line, side.is_empty());
            d.rect(p(face + 2.5, mid - 1.6), p(face + 9.5, mid + 1.6), 0.0, Ink::Paper, true);
            d.text(p(face + 6.0, mid), pin, SIZE, Anchor::Center, VAnchor::Middle, Ink::Text);
            d.text(p(sock_r, mid), &fit(sock, sock_chars), SIZE, Anchor::Right, VAnchor::Middle, Ink::Text);
            d.text(p(pin_l, mid), &fit(side, pin_chars), SIZE, Anchor::Left, VAnchor::Middle, Ink::Text);
            d.text(p(pot_x, mid), &fit(pot, 10), SIZE, Anchor::Left, VAnchor::Middle, Ink::Text);
            d.text(p(ft_l, mid), &fit(ft, ft_chars), SIZE, Anchor::Left, VAnchor::Middle, Ink::Text);
            if !loc.is_empty() {
                d.text(p(loc_r, mid), loc, SIZE, Anchor::Right, VAnchor::Middle, Ink::CrossRef);
            }
            if let Some(Some((page, _))) = t.targets.get(i + k) {
                d.link(p(x0, mid - ROW / 2.0), p(x1, mid + ROW / 2.0), *page);
            }
        }
        y = h_bottom + 4.0;
        i += n;
    }
    footer(d, t, frame);
}

/// Rows of the PLC assignment (columns: card, address, type, function text, connected
/// to, location): every card a module with its channels; inputs come in from the
/// device on the left, outputs go out to the device on the right.
fn plc_diagram(d: &mut dyn Draw, t: &PageTable, frame: &PlotFrame) {
    let area = frame.area();
    let (x0, x1) = (area.min.x + 7.0, area.max.x - 10.0);
    let top = area.min.y;
    d.text(p(x0, top + 9.0), &t.title, 6.0, Anchor::Left, VAnchor::Middle, Ink::Text);
    let w = x1 - x0;
    let (in_r, mod_l) = (x0 + w * 0.30, x0 + w * 0.36);
    let mod_r = mod_l + 30.0;
    let (out_l, ft_l, loc_r) = (mod_r + 6.0, x0 + w * 0.66, x1);
    let mut y = top + 19.0;
    for (x, h, a) in [(in_r, tr("Inputs from"), Anchor::Right), ((mod_l + mod_r) / 2.0, tr("PLC card"), Anchor::Center), (out_l, tr("Outputs to"), Anchor::Left), (ft_l, tr("Function text"), Anchor::Left)] {
        d.text(p(x, y), h, 2.4, a, VAnchor::Middle, Ink::Text);
    }
    y += 4.0;
    d.line(p(x0, y), p(x1, y), 0.3, Ink::Frame, false);
    let side_chars = ((in_r - x0 - 2.0) / (SIZE * 0.6)) as usize;
    let out_chars = ((ft_l - out_l - 2.0) / (SIZE * 0.6)) as usize;
    let ft_chars = ((loc_r - ft_l - 16.0) / (SIZE * 0.6)) as usize;
    let mut i = 0;
    while i < t.rows.len() {
        let card = cell(&t.rows[i], 0).to_string();
        let n = t.rows[i..].iter().take_while(|r| cell(r, 0) == card).count();
        d.text(p(mod_l, y + 4.0), &card, 3.0, Anchor::Left, VAnchor::Middle, Ink::Text);
        let (m_top, m_bottom) = (y + 7.0, y + 7.0 + n as f32 * ROW);
        d.rect(p(mod_l, m_top), p(mod_r, m_bottom), SYMBOL_W * 2.0, Ink::Line, false);
        for (k, row) in t.rows[i..i + n].iter().enumerate() {
            let mid = m_top + (k as f32 + 0.5) * ROW;
            let (addr, kind, ft, to, loc) = (cell(row, 1), cell(row, 2), cell(row, 3), cell(row, 4), cell(row, 5));
            let input = !kind.to_ascii_uppercase().contains('O') && !kind.to_ascii_uppercase().starts_with('Q');
            d.text(p((mod_l + mod_r) / 2.0, mid), &format!("{addr}  {kind}"), SIZE, Anchor::Center, VAnchor::Middle, Ink::Text);
            let open = to.is_empty();
            if input {
                d.line(p(in_r + 1.0, mid), p(mod_l, mid), SYMBOL_W, Ink::Line, open);
                d.poly(&[p(mod_l - 1.8, mid - 0.9), p(mod_l, mid), p(mod_l - 1.8, mid + 0.9)], true, 0.0, Ink::Line, true);
                d.text(p(in_r, mid), &fit(to, side_chars), SIZE, Anchor::Right, VAnchor::Middle, Ink::Text);
            } else {
                d.line(p(mod_r, mid), p(out_l - 1.0, mid), SYMBOL_W, Ink::Line, open);
                d.poly(&[p(out_l - 2.8, mid - 0.9), p(out_l - 1.0, mid), p(out_l - 2.8, mid + 0.9)], true, 0.0, Ink::Line, true);
                d.text(p(out_l, mid), &fit(to, out_chars), SIZE, Anchor::Left, VAnchor::Middle, Ink::Text);
            }
            d.text(p(ft_l, mid), &fit(ft, ft_chars), SIZE, Anchor::Left, VAnchor::Middle, Ink::Text);
            if !loc.is_empty() {
                d.text(p(loc_r, mid), loc, SIZE, Anchor::Right, VAnchor::Middle, Ink::CrossRef);
            }
            if let Some(Some((page, _))) = t.targets.get(i + k) {
                d.link(p(x0, mid - ROW / 2.0), p(x1, mid + ROW / 2.0), *page);
            }
        }
        y = m_bottom + 4.0;
        i += n;
    }
    footer(d, t, frame);
}

/// The note under a report page: empty, continued, or more rows.
fn footer(d: &mut dyn Draw, t: &PageTable, frame: &PlotFrame) {
    let area = frame.area();
    let note = if t.rows.is_empty() {
        Some(tr("(no entries)").to_string())
    } else if t.continued {
        Some(tr("Continued on the next page").to_string())
    } else if t.more > 0 {
        Some(tr_fmt!("… {} more rows – see Reports window / CSV export", t.more))
    } else {
        None
    };
    if let Some(n) = note {
        d.text(p(area.min.x + 7.0, area.max.y - 10.0), &n, 2.4, Anchor::Left, VAnchor::Middle, Ink::FrameText);
    }
}
