use super::*;
use openecraft_parts::{Mesh3d, MeshBuilder};

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    big_endian: bool,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8], big_endian: bool) -> Reader<'a> {
        Reader { data, pos: 0, big_endian }
    }
    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let b = self.data.get(self.pos..self.pos + N).ok_or("JT file is truncated")?;
        self.pos += N;
        let mut a: [u8; N] = b.try_into().map_err(|_| "JT file is truncated")?;
        if self.big_endian {
            a.reverse();
        }
        Ok(a)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let b = self.data.get(self.pos..self.pos.checked_add(n).ok_or("JT file is truncated")?).ok_or("JT file is truncated")?;
        self.pos += n;
        Ok(b)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.bytes::<1>()?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.bytes()?))
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.bytes()?))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.bytes()?))
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.bytes()?))
    }
    fn len(&mut self) -> Result<usize, String> {
        usize::try_from(self.i32()?).map_err(|_| "invalid length in JT file".to_string())
    }
    fn guid(&mut self) -> Result<Guid, String> {
        let (a, b, c) = (self.u32()?, self.u16()?, self.u16()?);
        let d: [u8; 8] = self.take(8)?.try_into().map_err(|_| "JT file is truncated")?;
        Ok(Guid(a, b, c, d))
    }
    fn mb_string(&mut self) -> Result<String, String> {
        let n = self.len()?;
        let chars = (0..n).map(|_| self.u16()).collect::<Result<Vec<u16>, String>>()?;
        Ok(String::from_utf16_lossy(&chars))
    }
}

fn inflate(data: &[u8]) -> Result<Vec<u8>, String> {
    miniz_oxide::inflate::decompress_to_vec_zlib(data).map_err(|e| format!("damaged compressed data in JT file: {e:?}"))
}

/// Millimetres per unit of the `JT_PROP_MEASUREMENT_UNITS` property of the scene
/// graph segment; 1 without one.
fn unit_mm(r: &mut Reader) -> Result<f64, String> {
    r.pos += SEGMENT_HEADER_LEN;
    let payload = if r.i32()? == 2 {
        let n = r.len()?;
        if r.u8()? != ZLIB {
            return Err("unsupported compression in JT file".into());
        }
        inflate(r.take(n.saturating_sub(1))?)?
    } else {
        r.data[r.pos..].to_vec()
    };
    let mut e = Reader::new(&payload, r.big_endian);
    let mut ends = 0;
    let mut strings = Vec::new();
    while ends < 2 && e.pos < payload.len() {
        let len = e.len()?;
        let start = e.pos;
        let kind = e.guid()?;
        if kind == END_OF_ELEMENTS {
            ends += 1;
        } else if kind == STRING_PROPERTY_ATOM {
            e.u8()?;
            e.i32()?;
            e.u32()?;
            strings.push(e.mb_string()?);
        }
        e.pos = start + len;
    }
    let unit = strings.iter().find_map(|s| match s.to_ascii_lowercase().as_str() {
        "millimeters" | "millimetres" => Some(1.0),
        "centimeters" | "centimetres" => Some(10.0),
        "meters" | "metres" => Some(1000.0),
        "inches" => Some(25.4),
        "feet" => Some(304.8),
        _ => None,
    });
    Ok(unit.unwrap_or(1.0))
}

/// Adds the triangles of a tri-strip set shape LOD element.
fn read_lod(r: &mut Reader, unit: f64, mesh: &mut MeshBuilder) -> Result<(), String> {
    r.pos += SEGMENT_HEADER_LEN;
    r.len()?;
    let kind = r.guid()?;
    if kind != TRI_STRIP_SET_SHAPE_LOD {
        return Err("the JT file contains shape data other than tri-strip sets (not supported)".into());
    }
    r.u8()?;
    r.i32()?;
    r.take(4)?;
    let normal_binding = r.i32()?;
    let (texture_binding, color_binding) = (r.i32()?, r.i32()?);
    let quantization = r.take(4)?;
    if quantization.iter().any(|&q| q != 0) {
        return Err("the JT file contains quantized vertex data (not supported)".into());
    }
    let size = r.len()?;
    let compressed = r.len()?;
    let raw = if compressed > 0 { inflate(r.take(compressed)?)? } else { r.take(size)?.to_vec() };
    let mut v = Reader::new(&raw, r.big_endian);
    let count = v.len()?;
    let starts = (0..count).map(|_| v.len()).collect::<Result<Vec<usize>, String>>()?;
    let floats = 3 + [normal_binding, color_binding].iter().filter(|&&b| b != 0).count() * 3 + usize::from(texture_binding != 0) * 2;
    let vertices = (raw.len() - v.pos) / (floats * 4);
    let mut pos = Vec::with_capacity(vertices);
    for _ in 0..vertices {
        let f = (0..floats).map(|_| v.f32()).collect::<Result<Vec<f32>, String>>()?;
        pos.push([0, 1, 2].map(|k| (f[floats - 3 + k] as f64 * unit) as f32));
    }
    for w in starts.windows(2) {
        let strip = pos.get(w[0]..w[1]).ok_or("invalid triangle strip in JT file")?;
        for k in 0..strip.len().saturating_sub(2) {
            let (a, b, c) = if k % 2 == 0 { (k, k + 1, k + 2) } else { (k + 1, k, k + 2) };
            mesh.triangle(strip[a], strip[b], strip[c])?;
        }
    }
    Ok(())
}

/// Reads the meshes of a JT file into one mesh in mm.
pub fn parse_jt(bytes: &[u8]) -> Result<Mesh3d, String> {
    if bytes.len() < HEADER_LEN {
        return Err("not a JT file".into());
    }
    let version = String::from_utf8_lossy(&bytes[..80]).to_string();
    let number = version.strip_prefix("Version ").and_then(|v| v.split_whitespace().next()).ok_or("not a JT file")?;
    if !version.contains("JT") {
        return Err("not a JT file".into());
    }
    let major: u32 = number.split('.').next().and_then(|m| m.parse().ok()).unwrap_or(0);
    if major != 8 {
        return Err(format!("JT {number} files are not supported (only JT 8 tri-strip meshes)"));
    }
    let big_endian = bytes.get(80) == Some(&1);
    let mut r = Reader::new(bytes, big_endian);
    r.pos = 85;
    r.pos = r.len()?;
    let entries = r.len()?;
    let mut toc = Vec::new();
    for _ in 0..entries {
        r.guid()?;
        let (offset, _len, attributes) = (r.len()?, r.len()?, r.u32()?);
        toc.push((offset, attributes >> 24));
    }
    let unit = match toc.iter().find(|t| t.1 == SEGMENT_LSG) {
        Some(&(offset, _)) => unit_mm(&mut Reader { data: bytes, pos: offset, big_endian })?,
        None => 1.0,
    };
    let mut mesh = MeshBuilder::new();
    let lods: Vec<usize> = toc.iter().filter(|t| t.1 == SEGMENT_SHAPE_LOD0).map(|t| t.0).collect();
    if lods.is_empty() {
        return Err("the JT file contains no triangle meshes (B-rep only JT files are not supported)".into());
    }
    for offset in lods {
        read_lod(&mut Reader { data: bytes, pos: offset, big_endian }, unit, &mut mesh)?;
    }
    mesh.finish()
}

#[cfg(test)]
mod tests;
