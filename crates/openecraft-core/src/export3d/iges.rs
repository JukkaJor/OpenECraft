//! IGES 5.3 export: a manifold solid B-rep object (186) per closed mesh – an open
//! shell (514 form 2) for the others – with a vertex list (502), an edge list (504)
//! of lines (110) and a planar face (510, 508, 190) per triangle; colour definitions
//! (314), name properties (406) and a group (402) per enclosure.

use super::*;

/// Hollerith string, ASCII only, short enough for one parameter line.
fn h(s: &str) -> String {
    let s: String = s.chars().map(|c| if c.is_ascii() && !c.is_ascii_control() { c } else { '_' }).take(56).collect();
    format!("{}H{s}", s.len())
}

/// Parameters joined by `,` and ended by `;`, packed into lines of `width` columns
/// without splitting a parameter.
pub(super) fn pack(params: &[String], width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for (i, p) in params.iter().enumerate() {
        let p = format!("{p}{}", if i + 1 == params.len() { ';' } else { ',' });
        if !line.is_empty() && line.len() + p.len() > width {
            lines.push(std::mem::take(&mut line));
        }
        line.push_str(&p);
    }
    lines.push(line);
    lines
}

/// Physically dependent (subordinate 01), visible.
const DEPENDENT: &str = "00010000";
const INDEPENDENT: &str = "00000000";

struct Iges {
    directory: Vec<String>,
    parameters: Vec<String>,
}

impl Iges {
    /// Adds an entity; returns its directory entry pointer.
    fn add(&mut self, kind: u32, form: u32, params: &[String], color: i64, label: &str, status: &str) -> usize {
        let de = self.directory.len() + 1;
        let first = self.parameters.len() + 1;
        let tokens: Vec<String> = std::iter::once(kind.to_string()).chain(params.iter().cloned()).collect();
        for line in pack(&tokens, 64) {
            self.parameters.push(format!("{line:<64}{de:>8}P{:>7}", self.parameters.len() + 1));
        }
        let count = self.parameters.len() + 1 - first;
        let label: String = label.chars().filter(|c| c.is_ascii_graphic()).take(8).collect();
        self.directory.push(format!("{kind:>8}{first:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{status:>8}D{:>7}", 0, 0, 0, 0, 0, 0, de));
        self.directory.push(format!("{kind:>8}{:>8}{color:>8}{count:>8}{form:>8}{:>8}{:>8}{label:>8}{:>8}D{:>7}", 0, "", "", 0, de + 1));
        de
    }

    fn name(&mut self, name: &str) -> usize {
        self.add(406, 15, &["1".into(), h(name)], 0, "NAME", DEPENDENT)
    }
}

/// Lines of a section, 72 columns plus section letter and number.
fn section(lines: Vec<String>, letter: char) -> Vec<String> {
    lines.iter().enumerate().map(|(i, l)| format!("{l:<72}{letter}{:>7}", i + 1)).collect()
}

/// Whether triangles bound a volume: every edge used exactly twice, in opposite directions.
pub(super) fn closed(faces: &[[usize; 3]]) -> bool {
    let mut directed: HashMap<(usize, usize), u32> = HashMap::new();
    for f in faces {
        for k in 0..3 {
            *directed.entry((f[k], f[(k + 1) % 3])).or_default() += 1;
        }
    }
    directed.iter().all(|(&(a, b), &n)| n == 1 && directed.get(&(b, a)) == Some(&1))
}

/// `YYYYMMDD.HHNNSS` from the project date (any text with the year, month and day digits).
pub(super) fn timestamp(date: &str) -> String {
    let digits: String = date.chars().filter(|c| c.is_ascii_digit()).take(8).collect();
    format!("{}.000000", if digits.len() == 8 && digits.starts_with(['1', '2']) { digits.as_str() } else { "20000101" })
}

/// The scene as IGES 5.3 (millimetres, z up).
pub fn iges(prj: &Project, file_name: &str) -> String {
    let name = if prj.info.name.is_empty() { "Panel layout" } else { &prj.info.name };
    let mut g = Iges { directory: Vec::new(), parameters: Vec::new() };
    let mut colors: HashMap<[u8; 3], usize> = HashMap::new();
    let mut max_coord: f32 = 0.0;
    for es in scene(prj) {
        let mut members = Vec::new();
        for solid in es.solids.iter().filter(|s| !s.triangles.is_empty()) {
            let tris: Vec<(Tri, [f32; 3])> = z_up(&es, solid).into_iter().filter_map(|t| normal(&t).map(|n| (t, n))).collect();
            let mut index: HashMap<[i64; 3], usize> = HashMap::new();
            let mut vertices: Vec<[f32; 3]> = Vec::new();
            let mut faces: Vec<([usize; 3], [f32; 3], [f32; 3])> = Vec::new();
            for (t, n) in &tris {
                let ids = t.map(|v| {
                    *index.entry(v.map(|c| (c * 1000.0).round() as i64)).or_insert_with(|| {
                        vertices.push(v);
                        vertices.len()
                    })
                });
                if ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2] {
                    faces.push((ids, t[0], *n));
                }
            }
            if faces.is_empty() {
                continue;
            }
            max_coord = vertices.iter().flatten().fold(max_coord, |m, c| m.max(c.abs()));
            let color = *colors.entry(solid.color).or_insert_with(|| {
                let [r, gr, b] = solid.color.map(|c| real(c as f32 / 255.0 * 100.0));
                g.add(314, 0, &[r, gr, b], 0, "COLOR", INDEPENDENT)
            });
            let color = -(color as i64);
            let mut params = vec![vertices.len().to_string()];
            params.extend(vertices.iter().flat_map(|v| v.map(real)));
            let vertex_list = g.add(502, 1, &params, 0, "VERTICES", DEPENDENT);
            let mut edge_index: HashMap<(usize, usize), usize> = HashMap::new();
            let mut edges: Vec<(usize, usize, usize)> = Vec::new();
            for (ids, _, _) in &faces {
                for k in 0..3 {
                    let (a, b) = (ids[k], ids[(k + 1) % 3]);
                    edge_index.entry((a.min(b), a.max(b))).or_insert_with(|| {
                        let (p, q) = (vertices[a.min(b) - 1], vertices[a.max(b) - 1]);
                        let line = g.add(110, 0, &[p, q].map(|v| v.map(real)).concat(), 0, "LINE", DEPENDENT);
                        edges.push((line, a.min(b), a.max(b)));
                        edges.len()
                    });
                }
            }
            let mut params = vec![edges.len().to_string()];
            for &(line, a, b) in &edges {
                params.extend([line, vertex_list, a, vertex_list, b].map(|x| x.to_string()));
            }
            let edge_list = g.add(504, 1, &params, 0, "EDGES", DEPENDENT);
            let mut shell = vec![faces.len().to_string()];
            for (ids, p, n) in &faces {
                let point = g.add(116, 0, &[p[0], p[1], p[2]].map(real).into_iter().chain(["0".to_string()]).collect::<Vec<_>>(), 0, "POINT", DEPENDENT);
                let direction = g.add(123, 0, &n.map(real), 0, "NORMAL", DEPENDENT);
                let plane = g.add(190, 0, &[point.to_string(), direction.to_string()], 0, "PLANE", DEPENDENT);
                let mut params = vec!["3".to_string()];
                for k in 0..3 {
                    let (a, b) = (ids[k], ids[(k + 1) % 3]);
                    let edge = edge_index[&(a.min(b), a.max(b))];
                    params.extend(["0".to_string(), edge_list.to_string(), edge.to_string(), u8::from(a < b).to_string(), "0".to_string()]);
                }
                let lp = g.add(508, 1, &params, 0, "LOOP", DEPENDENT);
                let face = g.add(510, 1, &[plane.to_string(), "1".into(), "1".into(), lp.to_string()], color, "FACE", DEPENDENT);
                shell.extend([face.to_string(), "1".into()]);
            }
            let label = g.name(&solid.name);
            if closed(&faces.iter().map(|f| f.0).collect::<Vec<_>>()) {
                let shell = g.add(514, 1, &shell, color, "SHELL", DEPENDENT);
                members.push(g.add(186, 0, &[shell.to_string(), "1".into(), "0".into(), "0".into(), "1".into(), label.to_string()], color, &solid.name, INDEPENDENT));
            } else {
                // Not a solid (overlapping boxes, open meshes): an open shell of faces.
                shell.extend(["0".into(), "1".into(), label.to_string()]);
                members.push(g.add(514, 2, &shell, color, &solid.name, INDEPENDENT));
            }
        }
        let label = g.name(&es.name);
        let mut params = vec![members.len().to_string()];
        params.extend(members.iter().map(|m| m.to_string()));
        params.extend(["0".into(), "1".into(), label.to_string()]);
        g.add(402, 7, &params, 0, &es.name, INDEPENDENT);
    }
    let stamp = timestamp(&prj.info.date);
    let global = [
        "1H,".to_string(),
        "1H;".into(),
        h(name),
        h(file_name),
        h("OpenECraft"),
        h("OpenECraft"),
        "32".into(),
        "38".into(),
        "6".into(),
        "308".into(),
        "15".into(),
        h(name),
        "1.".into(),
        "2".into(),
        "2HMM".into(),
        "1".into(),
        "1.".into(),
        h(&stamp),
        "0.001".into(),
        real(max_coord.max(1.0)),
        String::new(),
        String::new(),
        "11".into(),
        "0".into(),
        h(&stamp),
    ];
    let start = section(vec!["OpenECraft panel layout".into()], 'S');
    let global = section(pack(&global, 72), 'G');
    let mut out = String::new();
    for line in start.iter().chain(&global).chain(&g.directory).chain(&g.parameters) {
        out.push_str(line);
        out.push('\n');
    }
    let _ = writeln!(out, "S{:>7}G{:>7}D{:>7}P{:>7}{:>40}T{:>7}", start.len(), global.len(), g.directory.len(), g.parameters.len(), "", 1);
    out
}
