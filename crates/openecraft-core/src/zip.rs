//! ZIP files (stored or deflated entries, UTF-8 names): written for project archives
//! and Office Open XML files, read back with CRC check.

use openecraft_i18n::{tr, tr_fmt};


pub fn crc32(data: &[u8]) -> u32 {
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
pub fn dos_date(date: &str) -> u16 {
    let mut it = date.split('-').map(|x| x.trim().parse::<u16>().ok());
    match (it.next().flatten(), it.next().flatten(), it.next().flatten()) {
        (Some(y), Some(m), Some(d)) if (1980..2108).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d) => ((y - 1980) << 9) | (m << 5) | d,
        _ => (1 << 5) | 1,
    }
}

pub struct ZipWriter {
    out: Vec<u8>,
    central: Vec<u8>,
    count: u16,
    date: u16,
}

impl ZipWriter {
    pub fn new(date: &str) -> ZipWriter {
        ZipWriter { out: Vec::new(), central: Vec::new(), count: 0, date: dos_date(date) }
    }

    pub fn add(&mut self, name: &str, data: &[u8]) {
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

    pub fn finish(mut self) -> Vec<u8> {
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
pub fn read_zip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
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
