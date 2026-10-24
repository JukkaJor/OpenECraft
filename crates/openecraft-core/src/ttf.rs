//! Minimal TrueType reader and subsetter for embedding a font in PDF files: the glyph
//! of a character (`cmap` formats 4 and 12), advance widths (`hmtx`), the metrics of
//! the font descriptor, and a subset that keeps only the glyph outlines in use (glyph
//! ids stay the same, so a PDF can address glyphs by id with `Identity-H`).

use std::collections::{BTreeSet, HashMap};

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*d.get(at)?, *d.get(at + 1)?]))
}

fn i16_at(d: &[u8], at: usize) -> Option<i16> {
    u16_at(d, at).map(|v| v as i16)
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes([*d.get(at)?, *d.get(at + 1)?, *d.get(at + 2)?, *d.get(at + 3)?]))
}

pub struct Font<'a> {
    data: &'a [u8],
    tables: HashMap<[u8; 4], (usize, usize)>,
    pub units_per_em: u16,
    pub num_glyphs: u16,
    long_loca: bool,
    num_hmetrics: u16,
    /// Offset of the cmap subtable used and its format.
    cmap: (usize, u16),
    pub ascent: i16,
    pub descent: i16,
    /// xMin, yMin, xMax, yMax in font units.
    pub bbox: [i16; 4],
    pub cap_height: i16,
}

impl<'a> Font<'a> {
    pub fn parse(data: &'a [u8]) -> Option<Font<'a>> {
        let n = u16_at(data, 4)? as usize;
        let mut tables = HashMap::new();
        for i in 0..n {
            let rec = 12 + 16 * i;
            let tag: [u8; 4] = data.get(rec..rec + 4)?.try_into().ok()?;
            let (off, len) = (u32_at(data, rec + 8)? as usize, u32_at(data, rec + 12)? as usize);
            if off + len > data.len() {
                return None;
            }
            tables.insert(tag, (off, len));
        }
        let head = tables.get(b"head")?.0;
        let hhea = tables.get(b"hhea")?.0;
        let maxp = tables.get(b"maxp")?.0;
        // A subset has no cmap; its characters are then unknown.
        let cmap = tables.get(b"cmap").map(|t| t.0);
        let mut best: Option<(usize, u16, u8)> = None;
        for i in 0..cmap.and_then(|c| u16_at(data, c + 2)).unwrap_or(0) as usize {
            let cmap = cmap.unwrap_or_default();
            let rec = cmap + 4 + 8 * i;
            let (platform, encoding) = (u16_at(data, rec)?, u16_at(data, rec + 2)?);
            let off = cmap + u32_at(data, rec + 4)? as usize;
            let format = u16_at(data, off)?;
            let rank = match (platform, encoding, format) {
                (3, 10, 12) | (0, 4, 12) => 3,
                (3, 1, 4) | (0, 3, 4) => 2,
                (0, _, 4) => 1,
                _ => 0,
            };
            if rank > 0 && best.is_none_or(|b| rank > b.2) {
                best = Some((off, format, rank));
            }
        }
        let (cmap_off, format, _) = best.unwrap_or((0, 0, 0));
        let os2 = tables.get(b"OS/2").map(|t| t.0);
        let cap_height = os2.and_then(|o| if u16_at(data, o)? >= 2 { i16_at(data, o + 88) } else { None }).unwrap_or(700);
        Some(Font {
            data,
            units_per_em: u16_at(data, head + 18)?,
            num_glyphs: u16_at(data, maxp + 4)?,
            long_loca: i16_at(data, head + 50)? == 1,
            num_hmetrics: u16_at(data, hhea + 34)?,
            cmap: (cmap_off, format),
            ascent: i16_at(data, hhea + 4)?,
            descent: i16_at(data, hhea + 6)?,
            bbox: [i16_at(data, head + 36)?, i16_at(data, head + 38)?, i16_at(data, head + 40)?, i16_at(data, head + 42)?],
            cap_height,
            tables,
        })
    }

    fn table(&self, tag: &[u8; 4]) -> Option<&'a [u8]> {
        self.tables.get(tag).map(|&(o, l)| &self.data[o..o + l])
    }

    /// Glyph id of a character; 0 (the missing glyph) if the font has none.
    pub fn glyph(&self, c: char) -> u16 {
        self.lookup(c as u32).unwrap_or(0)
    }

    fn lookup(&self, c: u32) -> Option<u16> {
        let (off, format) = self.cmap;
        let d = self.data;
        match format {
            4 => {
                if c > 0xffff {
                    return None;
                }
                let segs = u16_at(d, off + 6)? as usize / 2;
                let ends = off + 14;
                let starts = ends + 2 * segs + 2;
                let deltas = starts + 2 * segs;
                let ranges = deltas + 2 * segs;
                for i in 0..segs {
                    let end = u16_at(d, ends + 2 * i)? as u32;
                    if c > end {
                        continue;
                    }
                    let start = u16_at(d, starts + 2 * i)? as u32;
                    if c < start {
                        return None;
                    }
                    let delta = u16_at(d, deltas + 2 * i)?;
                    let range = u16_at(d, ranges + 2 * i)? as usize;
                    if range == 0 {
                        return Some((c as u16).wrapping_add(delta));
                    }
                    let at = ranges + 2 * i + range + 2 * (c - start) as usize;
                    let g = u16_at(d, at)?;
                    return Some(if g == 0 { 0 } else { g.wrapping_add(delta) });
                }
                None
            }
            12 => {
                let groups = u32_at(d, off + 12)? as usize;
                for i in 0..groups {
                    let g = off + 16 + 12 * i;
                    let (start, end) = (u32_at(d, g)?, u32_at(d, g + 4)?);
                    if (start..=end).contains(&c) {
                        return Some((u32_at(d, g + 8)? + c - start) as u16);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Advance width of a glyph in font units.
    pub fn advance(&self, gid: u16) -> u16 {
        let Some(hmtx) = self.tables.get(b"hmtx").map(|t| t.0) else { return 0 };
        let i = gid.min(self.num_hmetrics.saturating_sub(1)) as usize;
        u16_at(self.data, hmtx + 4 * i).unwrap_or(0)
    }

    /// Byte range of a glyph's outline in `glyf`.
    fn glyph_range(&self, gid: u16) -> Option<(usize, usize)> {
        let loca = self.table(b"loca")?;
        let g = gid as usize;
        let (a, b) = if self.long_loca {
            (u32_at(loca, 4 * g)? as usize, u32_at(loca, 4 * g + 4)? as usize)
        } else {
            (u16_at(loca, 2 * g)? as usize * 2, u16_at(loca, 2 * g + 2)? as usize * 2)
        };
        (b >= a).then_some((a, b))
    }

    /// Glyphs a composite glyph is built from.
    fn components(&self, gid: u16) -> Vec<u16> {
        let (Some(glyf), Some((a, b))) = (self.table(b"glyf"), self.glyph_range(gid)) else { return Vec::new() };
        let g = &glyf[a..b.min(glyf.len())];
        if g.len() < 10 || i16_at(g, 0).unwrap_or(0) >= 0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut at = 10;
        while let (Some(flags), Some(id)) = (u16_at(g, at), u16_at(g, at + 2)) {
            out.push(id);
            at += 4 + if flags & 1 != 0 { 4 } else { 2 };
            at += if flags & 8 != 0 {
                2
            } else if flags & 0x40 != 0 {
                4
            } else if flags & 0x80 != 0 {
                8
            } else {
                0
            };
            if flags & 0x20 == 0 {
                break;
            }
        }
        out
    }

    /// The font with only the outlines of `glyphs` (plus glyph 0 and the components of
    /// composite glyphs); all other glyphs are empty. Glyph ids do not change.
    pub fn subset(&self, glyphs: &BTreeSet<u16>) -> Vec<u8> {
        let mut keep: BTreeSet<u16> = glyphs.iter().copied().filter(|g| *g < self.num_glyphs).collect();
        keep.insert(0);
        let mut todo: Vec<u16> = keep.iter().copied().collect();
        while let Some(g) = todo.pop() {
            for c in self.components(g) {
                if c < self.num_glyphs && keep.insert(c) {
                    todo.push(c);
                }
            }
        }
        let glyf_in = self.table(b"glyf").unwrap_or(&[]);
        let mut glyf = Vec::new();
        let mut loca = Vec::new();
        for g in 0..self.num_glyphs {
            loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
            if let (true, Some((a, b))) = (keep.contains(&g), self.glyph_range(g)) {
                glyf.extend_from_slice(&glyf_in[a.min(glyf_in.len())..b.min(glyf_in.len())]);
                while glyf.len() % 4 != 0 {
                    glyf.push(0);
                }
            }
        }
        loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
        let mut head = self.table(b"head").unwrap_or(&[]).to_vec();
        if head.len() >= 52 {
            head[8..12].copy_from_slice(&[0, 0, 0, 0]);
            head[50..52].copy_from_slice(&1u16.to_be_bytes());
        }
        let mut out_tables: Vec<([u8; 4], Vec<u8>)> = Vec::new();
        for tag in [*b"cvt ", *b"fpgm", *b"glyf", *b"head", *b"hhea", *b"hmtx", *b"loca", *b"maxp", *b"prep"] {
            let data = match &tag {
                b"glyf" => glyf.clone(),
                b"loca" => loca.clone(),
                b"head" => head.clone(),
                _ => match self.table(&tag) {
                    Some(t) => t.to_vec(),
                    None => continue,
                },
            };
            out_tables.push((tag, data));
        }
        write_font(out_tables)
    }
}

fn checksum(d: &[u8]) -> u32 {
    d.chunks(4).fold(0u32, |s, c| {
        let mut b = [0u8; 4];
        b[..c.len()].copy_from_slice(c);
        s.wrapping_add(u32::from_be_bytes(b))
    })
}

/// A font file of the tables (sorted by tag), with checksums and the head's
/// checksum adjustment.
fn write_font(tables: Vec<([u8; 4], Vec<u8>)>) -> Vec<u8> {
    let n = tables.len() as u16;
    let mut pow = 1u16;
    let mut log = 0u16;
    while pow * 2 <= n {
        pow *= 2;
        log += 1;
    }
    let mut out = Vec::new();
    out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    for v in [n, pow * 16, log, n * 16 - pow * 16] {
        out.extend_from_slice(&v.to_be_bytes());
    }
    let mut offset = 12 + 16 * tables.len();
    let mut head_at = None;
    for (tag, data) in &tables {
        out.extend_from_slice(tag);
        out.extend_from_slice(&checksum(data).to_be_bytes());
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        if tag == b"head" {
            head_at = Some(offset);
        }
        offset += data.len().div_ceil(4) * 4;
    }
    for (_, data) in &tables {
        out.extend_from_slice(data);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }
    if let Some(h) = head_at.filter(|h| h + 12 <= out.len()) {
        let adjust = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
        out[h + 8..h + 12].copy_from_slice(&adjust.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests;
