//! Project archives and version conversion. An archive (`.oecz`) is a ZIP file with
//! the project, a manifest, the embedded documents and images as plain files and
//! optionally a PDF of all pages, so it can be read without OpenECraft. Opening a
//! project file of an older format converts it and reports what was converted.

use openecraft_i18n::{tr, tr_fmt};
use crate::model::{Project, FORMAT};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const ARCHIVE_EXT: &str = "oecz";
const PROJECT_FILE: &str = "project.oec";
const MANIFEST_FILE: &str = "manifest.json";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArchiveOptions {
    pub comment: String,
    /// Archive date (`YYYY-MM-DD`).
    pub date: String,
    /// Adds a PDF of all pages.
    pub pdf: bool,
    /// Leaves out stored baselines and wiring marks.
    pub compact: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Manifest {
    pub generator: String,
    pub format: u32,
    pub name: String,
    pub description: String,
    pub revision: String,
    pub archived: String,
    pub comment: String,
    pub pages: usize,
    pub files: Vec<String>,
}

/// What opening a file converted.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Conversion {
    /// Format of the file.
    pub from: u32,
    pub steps: Vec<String>,
}

impl Conversion {
    pub fn converted(&self) -> bool {
        self.from < FORMAT
    }

    /// Written by a newer version: data this one does not know is lost on saving.
    pub fn newer(&self) -> bool {
        self.from > FORMAT
    }

    /// Note for the user, if there is something to say.
    pub fn message(&self) -> Option<String> {
        if self.newer() {
            Some(tr_fmt!("The file was written by a newer version (format {}): data this version does not know is lost when saving", self.from))
        } else if self.converted() {
            Some(tr_fmt!("Converted from file format {} to {}: {}", self.from, FORMAT, self.steps.join("; ")))
        } else {
            None
        }
    }
}

/// Reads a project file of any format version.
pub fn convert(text: &str) -> Result<(Project, Conversion), String> {
    let v: Value = serde_json::from_str(text).map_err(|e| tr_fmt!("Not a valid project file: {}", e))?;
    let from = v.get("format").and_then(Value::as_u64).unwrap_or(0) as u32;
    let mut steps = Vec::new();
    if from < 1 {
        let parts = v["devices"].as_object().map(|d| d.values().filter(|d| d["part"].as_str().is_some_and(|p| !p.is_empty())).count()).unwrap_or(0);
        if parts > 0 {
            steps.push(tr_fmt!("{} single part assignments converted to part lists", parts));
        }
        let props = v["pages"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|p| p["symbols"].as_array().into_iter().flatten())
            .flat_map(|s| s["props"].as_array().into_iter().flatten())
            .filter(|pp| pp.get("kind").is_some())
            .count();
        if props > 0 {
            steps.push(tr_fmt!("{} placed properties converted to property keys", props));
        }
        steps.push(tr("file format version added").into());
    }
    let prj = Project::from_json(text)?;
    Ok((prj, Conversion { from, steps }))
}

pub fn is_archive(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04")
}

/// Reads a project file or an archive.
pub fn read_project(bytes: &[u8]) -> Result<(Project, Conversion), String> {
    if is_archive(bytes) {
        let (prj, _, conv) = restore(bytes)?;
        Ok((prj, conv))
    } else {
        convert(&String::from_utf8_lossy(bytes))
    }
}

fn file_name(text: &str) -> String {
    let s: String = text.chars().map(|c| if c.is_alphanumeric() || "-_. ".contains(c) { c } else { '_' }).collect();
    let s = s.trim().to_string();
    if s.is_empty() {
        "_".into()
    } else {
        s
    }
}

/// Writes the project into an archive.
pub fn archive(prj: &Project, opts: &ArchiveOptions) -> Vec<u8> {
    let mut prj = prj.clone();
    prj.format = FORMAT;
    if opts.compact {
        prj.baselines.clear();
        prj.wiring_marks.clear();
    }
    let mut zip = ZipWriter::new(&opts.date);
    zip.add(PROJECT_FILE, prj.to_json().as_bytes());
    let mut files = vec![PROJECT_FILE.to_string()];
    let mut used = std::collections::BTreeSet::new();
    let mut unique = |name: String| {
        let mut n = name.clone();
        let mut i = 2;
        while !used.insert(n.clone()) {
            n = format!("{i} {name}");
            i += 1;
        }
        n
    };
    for pg in &prj.pages {
        let docs = pg.document.iter().chain(pg.images.iter().map(|i| &i.file));
        for doc in docs {
            let Ok(bytes) = doc.bytes() else { continue };
            let name = unique(format!("files/{} {}", file_name(&pg.name), file_name(&doc.name)));
            zip.add(&name, &bytes);
            files.push(name);
        }
    }
    if opts.pdf {
        let name = "project.pdf".to_string();
        zip.add(&name, crate::pdf::document(&prj).as_bytes());
        files.push(name);
    }
    let manifest = Manifest {
        generator: format!("OpenECraft {}", env!("CARGO_PKG_VERSION")),
        format: FORMAT,
        name: prj.info.name.clone(),
        description: prj.info.description.clone(),
        revision: prj.info.revision.clone(),
        archived: opts.date.clone(),
        comment: opts.comment.clone(),
        pages: prj.pages.len(),
        files,
    };
    zip.add(MANIFEST_FILE, serde_json::to_string_pretty(&manifest).unwrap_or_default().as_bytes());
    zip.finish()
}

/// Reads the project of an archive.
pub fn restore(bytes: &[u8]) -> Result<(Project, Manifest, Conversion), String> {
    let entries = read_zip(bytes)?;
    let get = |name: &str| entries.iter().find(|(n, _)| n == name).map(|(_, b)| b);
    let text = get(PROJECT_FILE).ok_or_else(|| tr("The archive contains no project").to_string())?;
    let manifest = get(MANIFEST_FILE).and_then(|m| serde_json::from_slice(m).ok()).unwrap_or_default();
    let (prj, conv) = convert(&String::from_utf8_lossy(text))?;
    Ok((prj, manifest, conv))
}

/// Manifest of an archive (without reading the project).
pub fn manifest(bytes: &[u8]) -> Result<Manifest, String> {
    let entries = read_zip(bytes)?;
    let m = entries.iter().find(|(n, _)| n == MANIFEST_FILE).ok_or_else(|| tr("The archive has no manifest").to_string())?;
    serde_json::from_slice(&m.1).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------------- ZIP

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// MS-DOS date of `YYYY-MM-DD` (1980-01-01 if it cannot be read).
fn dos_date(date: &str) -> u16 {
    let mut it = date.split('-').map(|x| x.trim().parse::<u16>().ok());
    match (it.next().flatten(), it.next().flatten(), it.next().flatten()) {
        (Some(y), Some(m), Some(d)) if (1980..2108).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d) => ((y - 1980) << 9) | (m << 5) | d,
        _ => (1 << 5) | 1,
    }
}

struct ZipWriter {
    out: Vec<u8>,
    central: Vec<u8>,
    count: u16,
    date: u16,
}

impl ZipWriter {
    fn new(date: &str) -> ZipWriter {
        ZipWriter { out: Vec::new(), central: Vec::new(), count: 0, date: dos_date(date) }
    }

    fn add(&mut self, name: &str, data: &[u8]) {
        let deflated = miniz_oxide::deflate::compress_to_vec(data, 6);
        let (method, body): (u16, &[u8]) = if deflated.len() < data.len() { (8, &deflated) } else { (0, data) };
        let crc = crc32(data);
        let offset = self.out.len() as u32;
        let header = |sig: u32, central: bool| {
            let mut h = Vec::new();
            h.extend(sig.to_le_bytes());
            if central {
                h.extend(20u16.to_le_bytes());
            }
            h.extend(20u16.to_le_bytes());
            // UTF-8 file names.
            h.extend(0x0800u16.to_le_bytes());
            h.extend(method.to_le_bytes());
            h.extend(0u16.to_le_bytes());
            h.extend(self.date.to_le_bytes());
            h.extend(crc.to_le_bytes());
            h.extend((body.len() as u32).to_le_bytes());
            h.extend((data.len() as u32).to_le_bytes());
            h.extend((name.len() as u16).to_le_bytes());
            h.extend(0u16.to_le_bytes());
            if central {
                h.extend([0u8; 6]);
                h.extend(0u32.to_le_bytes());
                h.extend(offset.to_le_bytes());
            }
            h.extend(name.as_bytes());
            h
        };
        let local = header(0x0403_4b50, false);
        let central = header(0x0201_4b50, true);
        self.out.extend(local);
        self.out.extend(body);
        self.central.extend(central);
        self.count += 1;
    }

    fn finish(mut self) -> Vec<u8> {
        let offset = self.out.len() as u32;
        let size = self.central.len() as u32;
        self.out.append(&mut self.central);
        self.out.extend(0x0605_4b50u32.to_le_bytes());
        self.out.extend([0u8; 4]);
        self.out.extend(self.count.to_le_bytes());
        self.out.extend(self.count.to_le_bytes());
        self.out.extend(size.to_le_bytes());
        self.out.extend(offset.to_le_bytes());
        self.out.extend(0u16.to_le_bytes());
        self.out
    }
}

/// Files of a ZIP archive (stored or deflated).
fn read_zip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let damaged = || tr("The archive is damaged").to_string();
    let u16_at = |i: usize| bytes.get(i..i + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(damaged);
    let u32_at = |i: usize| bytes.get(i..i + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(damaged);
    if bytes.len() < 22 {
        return Err(damaged());
    }
    let eocd = (0..=bytes.len() - 22).rev().find(|&i| bytes[i..i + 4] == [0x50, 0x4b, 0x05, 0x06]).ok_or_else(damaged)?;
    let count = u16_at(eocd + 10)? as usize;
    let mut at = u32_at(eocd + 16)? as usize;
    let mut files = Vec::new();
    for _ in 0..count {
        if u32_at(at)? != 0x0201_4b50 {
            return Err(damaged());
        }
        let method = u16_at(at + 10)?;
        let crc = u32_at(at + 16)?;
        let csize = u32_at(at + 20)? as usize;
        let name_len = u16_at(at + 28)? as usize;
        let extra = u16_at(at + 30)? as usize;
        let comment = u16_at(at + 32)? as usize;
        let local = u32_at(at + 42)? as usize;
        let name = String::from_utf8_lossy(bytes.get(at + 46..at + 46 + name_len).ok_or_else(damaged)?).into_owned();
        at += 46 + name_len + extra + comment;
        if u32_at(local)? != 0x0403_4b50 {
            return Err(damaged());
        }
        let start = local + 30 + u16_at(local + 26)? as usize + u16_at(local + 28)? as usize;
        let body = bytes.get(start..start + csize).ok_or_else(damaged)?;
        let data = match method {
            0 => body.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec(body).map_err(|_| damaged())?,
            _ => return Err(tr_fmt!("{}: compression method not supported", name)),
        };
        if crc32(&data) != crc {
            return Err(damaged());
        }
        files.push((name, data));
    }
    Ok(files)
}

#[cfg(test)]
mod tests;
