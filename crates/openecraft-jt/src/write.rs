use super::*;
use std::collections::HashMap;

type Tri = [[f32; 3]; 3];

/// A coloured triangle mesh, written as tri-strip set shape node.
#[derive(Clone, Copy, Debug)]
pub struct Body<'a> {
    pub name: &'a str,
    pub color: [u8; 3],
    /// Shown see-through (alpha 0.25).
    pub transparent: bool,
    pub triangles: &'a [Tri],
}

/// A named group of bodies (group node).
#[derive(Clone, Debug)]
pub struct Group<'a> {
    pub name: &'a str,
    pub bodies: Vec<Body<'a>>,
}

#[derive(Default)]
pub(crate) struct Buf(pub Vec<u8>);

impl Buf {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn i16(&mut self, v: i16) {
        self.0.extend(v.to_le_bytes());
    }
    fn u16(&mut self, v: u16) {
        self.0.extend(v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend(v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend(v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend(v.to_le_bytes());
    }
    fn guid(&mut self, g: Guid) {
        self.u32(g.0);
        self.u16(g.1);
        self.u16(g.2);
        self.0.extend(g.3);
    }
    /// Multi-byte string: character count, UTF-16 characters.
    fn mb_string(&mut self, s: &str) {
        let chars: Vec<u16> = s.encode_utf16().collect();
        self.i32(chars.len() as i32);
        chars.into_iter().for_each(|c| self.u16(c));
    }
    fn i32_list(&mut self, ids: &[i32]) {
        self.i32(ids.len() as i32);
        ids.iter().for_each(|&i| self.i32(i));
    }
    /// Element: length, object type, base type, object ID, data.
    fn element(&mut self, kind: Guid, base: u8, id: i32, data: &[u8]) {
        self.i32((16 + 1 + 4 + data.len()) as i32);
        self.guid(kind);
        self.u8(base);
        self.i32(id);
        self.0.extend(data);
    }
    fn end_of_elements(&mut self) {
        self.i32(16 + 1);
        self.guid(END_OF_ELEMENTS);
        self.u8(0);
    }
}

/// Bounding box, area, vertex / node / polygon counts, size and compression level
/// shared by partition and shape nodes.
fn shape_stats(b: &mut Buf, tris: &[&Tri], size: usize) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let mut area = 0.0;
    for t in tris {
        for v in t.iter() {
            for k in 0..3 {
                min[k] = min[k].min(v[k]);
                max[k] = max[k].max(v[k]);
            }
        }
        let (u, w) = ([0, 1, 2].map(|k| t[1][k] - t[0][k]), [0, 1, 2].map(|k| t[2][k] - t[0][k]));
        let c = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
        area += (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt() / 2.0;
    }
    if tris.is_empty() {
        (min, max) = ([0.0; 3], [0.0; 3]);
    }
    min.into_iter().chain(max).for_each(|c| b.f32(c));
    b.f32(area);
    let (vertices, polygons) = ((tris.len() * 3) as i32, tris.len() as i32);
    for (lo, hi) in [(vertices, vertices), (1, 1), (polygons, polygons)] {
        b.i32(lo);
        b.i32(hi);
    }
    b.i32(size as i32);
    b.f32(0.0);
}

fn base_node(b: &mut Buf, attributes: &[i32]) {
    b.u32(0);
    b.i32_list(attributes);
}

fn material(color: [u8; 3], transparent: bool) -> Vec<u8> {
    let mut b = Buf::default();
    b.u8(0);
    b.u32(0);
    b.u16(0);
    let [r, g, bl] = color.map(|c| c as f32 / 255.0);
    let alpha = if transparent { 0.25 } else { 1.0 };
    for rgba in [[r * 0.3, g * 0.3, bl * 0.3, alpha], [r, g, bl, alpha], [0.2, 0.2, 0.2, alpha], [0.0, 0.0, 0.0, alpha]] {
        rgba.into_iter().for_each(|c| b.f32(c));
    }
    b.f32(20.0);
    b.0
}

/// Tri-strip set LOD data: every triangle a strip of three vertices with a flat
/// normal, unquantized and zlib compressed.
pub(crate) fn lod_data(tris: &[Tri]) -> Vec<u8> {
    let mut raw = Buf::default();
    raw.i32(tris.len() as i32 + 1);
    (0..=tris.len()).for_each(|i| raw.i32(i as i32 * 3));
    for t in tris {
        let (u, w) = ([0, 1, 2].map(|k| t[1][k] - t[0][k]), [0, 1, 2].map(|k| t[2][k] - t[0][k]));
        let c = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
        let l = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt().max(1e-12);
        for v in t {
            c.iter().for_each(|n| raw.f32(n / l));
            v.iter().for_each(|&q| raw.f32(q));
        }
    }
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&raw.0, 6);
    let mut b = Buf::default();
    b.i16(1);
    b.i16(1);
    b.i32(1);
    b.i32(0);
    b.i32(0);
    b.0.extend([0; 4]);
    b.i32(raw.0.len() as i32);
    b.i32(compressed.len() as i32);
    b.0.extend(compressed);
    b.i16(1);
    b.0
}

/// Deterministic segment IDs.
fn segment_id(n: u32) -> Guid {
    Guid(0x4f45_4a54, 0x0001, (n >> 16) as u16, [0x4f, 0x45, 0x43, 0x52, 0, 0, (n >> 8) as u8, n as u8])
}

struct Lsg {
    graph: Buf,
    atoms: Buf,
    next_id: i32,
    strings: HashMap<String, i32>,
    properties: Vec<(i32, Vec<(i32, i32)>)>,
}

impl Lsg {
    fn id(&mut self) -> i32 {
        self.next_id += 1;
        self.next_id
    }
    fn string(&mut self, s: &str) -> i32 {
        if let Some(&id) = self.strings.get(s) {
            return id;
        }
        let id = self.id();
        let mut b = Buf::default();
        b.u32(0);
        b.mb_string(s);
        self.atoms.element(STRING_PROPERTY_ATOM, BASE_PROPERTY, id, &b.0);
        self.strings.insert(s.to_string(), id);
        id
    }
    fn property(&mut self, object: i32, key: &str, value: i32) {
        let key = self.string(key);
        match self.properties.iter_mut().find(|(o, _)| *o == object) {
            Some((_, pairs)) => pairs.push((key, value)),
            None => self.properties.push((object, vec![(key, value)])),
        }
    }
    fn name(&mut self, object: i32, name: &str) {
        let value = self.string(name);
        self.property(object, "JT_PROP_NAME", value);
    }
}

/// The groups as JT file in millimetres; `name` names the root partition.
pub fn write_jt(name: &str, groups: &[Group]) -> Vec<u8> {
    write_with_unit(name, groups, "Millimeters")
}

/// The groups as JT file; the coordinates are in `unit` (a JT unit name).
pub(crate) fn write_with_unit(name: &str, groups: &[Group], unit: &str) -> Vec<u8> {
    let mut lsg = Lsg { graph: Buf::default(), atoms: Buf::default(), next_id: 0, strings: HashMap::new(), properties: Vec::new() };
    let root = lsg.id();
    let mut lods: Vec<(Guid, Vec<u8>)> = Vec::new();
    let mut group_ids = Vec::new();
    for g in groups {
        let gid = lsg.id();
        let mut shapes = Vec::new();
        for body in g.bodies.iter().filter(|b| !b.triangles.is_empty()) {
            let mid = lsg.id();
            lsg.graph.element(MATERIAL_ATTRIBUTE, BASE_ATTRIBUTE, mid, &material(body.color, body.transparent));
            let sid = lsg.id();
            let lod = lod_data(body.triangles);
            let mut b = Buf::default();
            base_node(&mut b, &[mid]);
            shape_stats(&mut b, &body.triangles.iter().collect::<Vec<_>>(), lod.len());
            b.i32(1);
            b.i32(0);
            b.i32(0);
            b.0.extend([0; 4]);
            lsg.graph.element(TRI_STRIP_SET_SHAPE_NODE, BASE_SHAPE, sid, &b.0);
            lsg.name(sid, body.name);
            let segment = segment_id(lods.len() as u32 + 2);
            let late = lsg.id();
            let mut a = Buf::default();
            a.u32(0);
            a.guid(segment);
            a.i32(SEGMENT_SHAPE_LOD0 as i32);
            lsg.atoms.element(LATE_LOADED_PROPERTY_ATOM, BASE_PROPERTY, late, &a.0);
            lsg.property(sid, "JT_LLPROP_SHAPEIMPL", late);
            let mut seg = Buf::default();
            seg.element(TRI_STRIP_SET_SHAPE_LOD, BASE_SHAPE_LOD, lsg.id(), &lod);
            lods.push((segment, seg.0));
            shapes.push(sid);
        }
        let mut b = Buf::default();
        base_node(&mut b, &[]);
        b.i32_list(&shapes);
        lsg.graph.element(GROUP_NODE, BASE_GROUP, gid, &b.0);
        lsg.name(gid, g.name);
        group_ids.push(gid);
    }
    let all: Vec<&Tri> = groups.iter().flat_map(|g| g.bodies.iter().flat_map(|b| b.triangles.iter())).collect();
    let mut b = Buf::default();
    base_node(&mut b, &[]);
    b.i32_list(&group_ids);
    b.i32(0);
    b.mb_string("");
    shape_stats(&mut b, &all, lods.iter().map(|l| l.1.len()).sum());
    let mut graph = Buf::default();
    graph.element(PARTITION_NODE, BASE_GROUP, root, &b.0);
    graph.0.extend(std::mem::take(&mut lsg.graph.0));
    lsg.name(root, name);
    let units = lsg.string(unit);
    lsg.property(root, "JT_PROP_MEASUREMENT_UNITS", units);

    let mut payload = graph;
    payload.end_of_elements();
    payload.0.extend(std::mem::take(&mut lsg.atoms.0));
    payload.end_of_elements();
    payload.i16(1);
    payload.i32(lsg.properties.len() as i32);
    for (object, pairs) in &lsg.properties {
        payload.i32(*object);
        for &(k, v) in pairs {
            payload.i32(k);
            payload.i32(v);
        }
        payload.i32(0);
    }
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&payload.0, 6);
    let mut lsg_data = Buf::default();
    lsg_data.i32(2);
    lsg_data.i32(compressed.len() as i32 + 1);
    lsg_data.u8(ZLIB);
    lsg_data.0.extend(compressed);

    let lsg_id = segment_id(1);
    let mut segments = vec![(lsg_id, SEGMENT_LSG, lsg_data.0)];
    segments.extend(lods.into_iter().map(|(id, data)| (id, SEGMENT_SHAPE_LOD0, data)));
    let mut out = Buf::default();
    let mut version = format!("{VERSION} ").into_bytes();
    version.resize(80, b' ');
    out.0.extend(version);
    out.u8(0);
    out.i32(0);
    let toc_at = out.0.len();
    out.i32(0);
    out.guid(lsg_id);
    let mut toc = Vec::new();
    for (id, kind, data) in segments {
        let offset = out.0.len();
        let len = SEGMENT_HEADER_LEN + data.len();
        out.guid(id);
        out.i32(kind as i32);
        out.i32(len as i32);
        out.0.extend(data);
        toc.push((id, offset, len, kind));
    }
    let toc_offset = out.0.len() as i32;
    out.0[toc_at..toc_at + 4].copy_from_slice(&toc_offset.to_le_bytes());
    out.i32(toc.len() as i32);
    for (id, offset, len, kind) in toc {
        out.guid(id);
        out.i32(offset as i32);
        out.i32(len as i32);
        out.u32(kind << 24);
    }
    out.0
}

#[cfg(test)]
mod tests;
