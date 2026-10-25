use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// A triangle in millimetres.
pub type Tri = [[f32; 3]; 3];

const SCHEMA: &str = "SCH_1300000_13006";
const MODELLER: &str = ": TRANSMIT FILE created by modeller version 1300000";
const HEADER: [&str; 6] = [
    "**ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz**************************",
    "**PARASOLID !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~0123456789**************************",
    "**PART1;APPL=OpenECraft;FORMAT=text;GUISE=transmit;",
    "**PART2;SCH=SCH_1300000_13006;USFLD_SIZE=0;",
    "**PART3;",
    "**END_OF_HEADER*****************************************************************",
];
/// Records of the text format are at most 80 characters long.
const LINE: usize = 80;
/// Positions closer than this (mm) are one vertex.
const WELD: f64 = 1e-4;
/// Neighbouring triangles whose normals differ by less than this (1 - cos) and that
/// lie in one plane (distance in mm) form one face.
const FLAT: f64 = 1e-9;
const PLANE_DISTANCE: f64 = 1e-5;

/// Node types of the V13 schema.
const BODY: u32 = 12;
const SHELL: u32 = 13;
const FACE: u32 = 14;
const LOOP: u32 = 15;
const EDGE: u32 = 16;
const FIN: u32 = 17;
const VERTEX: u32 = 18;
const REGION: u32 = 19;
const POINT: u32 = 29;
const LINE_CURVE: u32 = 30;
const PLANE: u32 = 50;
const POINTER_LIST: u32 = 74;
/// Body types.
const SOLID_BODY: u32 = 1;
const SHEET_BODY: u32 = 3;

type V3 = [f64; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: V3) -> Option<V3> {
    let l = dot(a, a).sqrt();
    (l > 1e-12).then(|| a.map(|c| c / l))
}

/// A body to write: positions in mm and planar faces, each with its normal and its
/// loops (vertex indices in loop direction, the face on the left seen from above).
struct Part {
    solid: bool,
    points: Vec<V3>,
    faces: Vec<Face>,
}

struct Face {
    normal: V3,
    loops: Vec<Vec<usize>>,
}

/// Welds the triangles; returns the positions and the non-degenerate triangles.
fn weld(tris: &[Tri]) -> (Vec<V3>, Vec<[usize; 3]>) {
    let mut points = Vec::new();
    let mut index: HashMap<[i64; 3], usize> = HashMap::new();
    let mut out = Vec::new();
    for t in tris {
        let v = t.map(|p| {
            let p = p.map(f64::from);
            *index.entry(p.map(|c| (c / WELD).round() as i64)).or_insert_with(|| {
                points.push(p);
                points.len() - 1
            })
        });
        let area = cross(sub(points[v[1]], points[v[0]]), sub(points[v[2]], points[v[0]]));
        if v[0] != v[1] && v[1] != v[2] && v[0] != v[2] && dot(area, area) > 1e-18 {
            out.push(v);
        }
    }
    (points, out)
}

fn edges(t: [usize; 3]) -> [(usize, usize); 3] {
    [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Every edge is used twice in opposite directions and the triangles around every
/// vertex form one fan: a closed 2-manifold.
fn closed(tris: &[[usize; 3]]) -> bool {
    let mut directed: HashMap<(usize, usize), usize> = HashMap::new();
    for t in tris {
        for e in edges(*t) {
            *directed.entry(e).or_default() += 1;
        }
    }
    if directed.iter().any(|(&(a, b), &n)| n != 1 || directed.get(&(b, a)) != Some(&1)) {
        return false;
    }
    // The link of a vertex: a → b for every triangle (v, a, b); one cycle per vertex.
    let mut link: HashMap<usize, HashMap<usize, usize>> = HashMap::new();
    for t in tris {
        for k in 0..3 {
            link.entry(t[k]).or_default().insert(t[(k + 1) % 3], t[(k + 2) % 3]);
        }
    }
    link.values().all(|next| {
        let Some(&start) = next.keys().next() else { return true };
        let (mut at, mut steps) = (start, 0);
        loop {
            at = next[&at];
            steps += 1;
            if at == start || steps > next.len() {
                break;
            }
        }
        at == start && steps == next.len()
    })
}

fn normal(points: &[V3], t: [usize; 3]) -> V3 {
    unit(cross(sub(points[t[1]], points[t[0]]), sub(points[t[2]], points[t[0]]))).unwrap_or([0.0, 0.0, 1.0])
}

/// The faces of a closed component: coplanar edge-neighbours merged; a group whose
/// boundary passes a vertex twice stays separate triangles.
fn merged_faces(points: &[V3], tris: &[[usize; 3]]) -> Vec<Face> {
    let mut by_edge: HashMap<(usize, usize), usize> = HashMap::new();
    for (i, t) in tris.iter().enumerate() {
        for e in edges(*t) {
            by_edge.insert(e, i);
        }
    }
    let mut done = vec![false; tris.len()];
    let mut faces = Vec::new();
    for seed in 0..tris.len() {
        if done[seed] {
            continue;
        }
        let n = normal(points, tris[seed]);
        let d = dot(n, points[tris[seed][0]]);
        let mut group = vec![seed];
        done[seed] = true;
        let mut k = 0;
        while k < group.len() {
            for (a, b) in edges(tris[group[k]]) {
                let Some(&j) = by_edge.get(&(b, a)) else { continue };
                let t = tris[j];
                if !done[j] && 1.0 - dot(normal(points, t), n) < FLAT && t.iter().all(|&v| (dot(n, points[v]) - d).abs() < PLANE_DISTANCE) {
                    done[j] = true;
                    group.push(j);
                }
            }
            k += 1;
        }
        let inside: HashSet<(usize, usize)> = group.iter().flat_map(|&i| edges(tris[i])).collect();
        let boundary: Vec<(usize, usize)> = inside.iter().copied().filter(|&(a, b)| !inside.contains(&(b, a))).collect();
        let mut next: HashMap<usize, usize> = HashMap::new();
        if boundary.iter().all(|&(a, b)| next.insert(a, b).is_none()) {
            let mut loops = Vec::new();
            let mut used: HashSet<usize> = HashSet::new();
            let mut starts: Vec<usize> = next.keys().copied().collect();
            starts.sort_unstable();
            for start in starts {
                if used.contains(&start) {
                    continue;
                }
                let mut ring = vec![start];
                used.insert(start);
                let mut at = next[&start];
                while at != start {
                    ring.push(at);
                    used.insert(at);
                    at = next[&at];
                }
                loops.push(ring);
            }
            faces.push(Face { normal: n, loops });
        } else {
            faces.extend(group.iter().map(|&i| Face { normal: normal(points, tris[i]), loops: vec![tris[i].to_vec()] }));
        }
    }
    faces
}

/// The bodies of one mesh: a solid per closed component (its triangles connected by
/// edges used twice), and a sheet of separate triangles for the rest.
fn parts(tris: &[Tri]) -> Vec<Part> {
    let (points, tris) = weld(tris);
    let mut uses: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (i, t) in tris.iter().enumerate() {
        for (a, b) in edges(*t) {
            uses.entry((a.min(b), a.max(b))).or_default().push(i);
        }
    }
    let mut parent: Vec<usize> = (0..tris.len()).collect();
    for list in uses.values().filter(|l| l.len() == 2) {
        let (a, b) = (find(&mut parent, list[0]), find(&mut parent, list[1]));
        parent[a] = b;
    }
    let mut components: Vec<Vec<usize>> = Vec::new();
    let mut slot: HashMap<usize, usize> = HashMap::new();
    for i in 0..tris.len() {
        let root = find(&mut parent, i);
        let k = *slot.entry(root).or_insert_with(|| {
            components.push(Vec::new());
            components.len() - 1
        });
        components[k].push(i);
    }
    let mut out = Vec::new();
    let mut rest = Vec::new();
    for component in components {
        let mut local: Vec<[usize; 3]> = component.iter().map(|&i| tris[i]).collect();
        if !closed(&local) {
            rest.extend(local);
            continue;
        }
        // Normals point out of the solid.
        let volume: f64 = local.iter().map(|t| dot(points[t[0]], cross(points[t[1]], points[t[2]]))).sum();
        if volume < 0.0 {
            local.iter_mut().for_each(|t| t.swap(1, 2));
        }
        let mut remap: HashMap<usize, usize> = HashMap::new();
        let mut own = Vec::new();
        for t in &mut local {
            for v in t.iter_mut() {
                *v = *remap.entry(*v).or_insert_with(|| {
                    own.push(points[*v]);
                    own.len() - 1
                });
            }
        }
        out.push(Part { solid: true, faces: merged_faces(&own, &local), points: own });
    }
    if !rest.is_empty() {
        let own: Vec<V3> = rest.iter().flat_map(|t| t.map(|v| points[v])).collect();
        let faces = rest.iter().enumerate().map(|(i, t)| Face { normal: normal(&points, *t), loops: vec![vec![3 * i, 3 * i + 1, 3 * i + 2]] }).collect();
        out.push(Part { solid: false, points: own, faces });
    }
    out
}

/// Numbers as the text format writes them; lengths are converted to metres by the caller.
fn number(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        "0".into()
    } else if (1e-4..1e15).contains(&v.abs()) {
        format!("{v}")
    } else {
        format!("{v:e}")
    }
}

/// The fields of one node: numbers followed by a space, characters and unset values not.
#[derive(Default)]
struct Fields(String);

impl Fields {
    fn int(mut self, v: impl std::fmt::Display) -> Self {
        let _ = write!(self.0, "{v} ");
        self
    }
    fn real(self, v: f64) -> Self {
        self.int(number(v))
    }
    fn vector(self, v: V3) -> Self {
        self.real(v[0]).real(v[1]).real(v[2])
    }
    fn unset(mut self) -> Self {
        self.0.push('?');
        self
    }
    fn char(mut self, c: char) -> Self {
        self.0.push(c);
        self
    }
}

#[derive(Default)]
struct Writer {
    /// Node index and text, root first.
    nodes: Vec<(usize, String)>,
    last: usize,
}

/// A fin: its loop (none for a dummy fin), forward vertex, edge and sense.
struct FinData {
    index: usize,
    looped: Option<(usize, usize, usize)>,
    vertex: usize,
    edge: usize,
    positive: bool,
}

struct EdgeData {
    index: usize,
    line: usize,
    start: usize,
    end: usize,
    fins: Vec<usize>,
}

impl Writer {
    fn alloc(&mut self) -> usize {
        self.last += 1;
        self.last
    }

    fn node(&mut self, kind: u32, index: usize, fields: Fields) {
        self.nodes.push((index, format!("{kind} {index} {}", fields.0)));
    }

    /// Writes one body; `index` is its node index if it was reserved (the root).
    #[allow(clippy::too_many_lines)]
    fn body(&mut self, part: &Part, index: Option<usize>) -> usize {
        let body = index.unwrap_or_else(|| self.alloc());
        let mut ids = 0;
        let mut id = || {
            ids += 1;
            ids
        };
        let metres = |p: V3| p.map(|c| c / 1000.0);
        let void_region = self.alloc();
        let void_shell = self.alloc();
        let solid = part.solid.then(|| (self.alloc(), self.alloc()));
        let vertices: Vec<usize> = part.points.iter().map(|_| self.alloc()).collect();
        let points: Vec<usize> = part.points.iter().map(|_| self.alloc()).collect();
        let faces: Vec<usize> = part.faces.iter().map(|_| self.alloc()).collect();
        let planes: Vec<usize> = part.faces.iter().map(|_| self.alloc()).collect();
        // Loops, fins and edges.
        let mut loops: Vec<Vec<usize>> = Vec::new();
        let mut fins: Vec<FinData> = Vec::new();
        let mut loop_fins: Vec<Vec<usize>> = Vec::new();
        let mut edges: Vec<EdgeData> = Vec::new();
        let mut edge_of: HashMap<(usize, usize), usize> = HashMap::new();
        for face in &part.faces {
            let mut face_loops = Vec::new();
            for ring in &face.loops {
                let lp = self.alloc();
                face_loops.push(lp);
                let mut ring_fins = Vec::new();
                for k in 0..ring.len() {
                    let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
                    let key = (a.min(b), a.max(b));
                    let e = match edge_of.get(&key) {
                        Some(&e) => e,
                        None => {
                            let (index, line) = (self.alloc(), self.alloc());
                            edges.push(EdgeData { index, line, start: a, end: b, fins: Vec::new() });
                            edge_of.insert(key, edges.len() - 1);
                            edges.len() - 1
                        }
                    };
                    let positive = edges[e].start == a;
                    let index = self.alloc();
                    fins.push(FinData { index, looped: Some((lp, 0, 0)), vertex: b, edge: e, positive });
                    edges[e].fins.push(fins.len() - 1);
                    ring_fins.push(fins.len() - 1);
                }
                let n = ring_fins.len();
                for k in 0..n {
                    let (forward, backward) = (fins[ring_fins[(k + 1) % n]].index, fins[ring_fins[(k + n - 1) % n]].index);
                    fins[ring_fins[k]].looped = Some((lp, forward, backward));
                }
                loop_fins.push(ring_fins);
            }
            loops.push(face_loops);
        }
        // An edge of a sheet's boundary has a dummy fin in place of the missing one.
        for (e, edge) in edges.iter_mut().enumerate() {
            if edge.fins.len() == 1 {
                let index = self.alloc();
                fins.push(FinData { index, looped: None, vertex: edge.start, edge: e, positive: false });
                edge.fins.push(fins.len() - 1);
            }
        }
        // The fins ending at a vertex, dummies last (their chain pointer stays null).
        let mut at_vertex: Vec<Vec<usize>> = vec![Vec::new(); part.points.len()];
        for (i, f) in fins.iter().enumerate().filter(|(_, f)| f.looped.is_some()) {
            at_vertex[f.vertex].push(i);
        }
        for (i, f) in fins.iter().enumerate().filter(|(_, f)| f.looped.is_none()) {
            at_vertex[f.vertex].push(i);
        }
        let mut next_at_vertex = vec![0; fins.len()];
        for chain in &at_vertex {
            for w in chain.windows(2) {
                next_at_vertex[w[0]] = fins[w[1]].index;
            }
        }
        let chained = |list: &[usize], k: usize| (if k + 1 < list.len() { list[k + 1] } else { 0 }, if k > 0 { list[k - 1] } else { 0 });
        let first = |list: &[usize]| list.first().copied().unwrap_or(0);
        let lines: Vec<usize> = edges.iter().map(|e| e.line).collect();
        let edge_nodes: Vec<usize> = edges.iter().map(|e| e.index).collect();

        let (shell, kind) = match solid {
            Some((_, solid_shell)) => (solid_shell, SOLID_BODY),
            None => (void_shell, SHEET_BODY),
        };
        // Regions and shells.
        let void_id = id();
        let void_shell_id = id();
        let mut list = Fields::default().int(void_id).int(0).int(body).int(solid.map_or(0, |s| s.0)).int(0).int(void_shell).char('V');
        self.node(REGION, void_region, list);
        match solid {
            Some((solid_region, solid_shell)) => {
                list = Fields::default().int(id()).int(0).int(body).int(0).int(void_region).int(solid_shell).char('S');
                self.node(REGION, solid_region, list);
                list = Fields::default().int(void_shell_id).int(0).int(0).int(0).int(0).int(0).int(0).int(void_region).int(first(&faces));
                self.node(SHELL, void_shell, list);
                list = Fields::default().int(id()).int(0).int(body).int(0).int(first(&faces)).int(0).int(0).int(solid_region).int(0);
                self.node(SHELL, solid_shell, list);
            }
            None => {
                list = Fields::default().int(void_shell_id).int(0).int(body).int(0).int(first(&faces)).int(0).int(0).int(void_region).int(first(&faces));
                self.node(SHELL, void_shell, list);
            }
        }
        // Faces with their planes and loops.
        for (k, face) in part.faces.iter().enumerate() {
            let (next, previous) = chained(&faces, k);
            let fields = Fields::default().int(id()).int(0).unset().int(next).int(previous).int(first(&loops[k])).int(shell).int(planes[k]).char('+');
            self.node(FACE, faces[k], fields.int(0).int(0).int(next).int(previous).int(void_shell));
            let origin = part.points[face.loops[0][0]];
            let along = sub(part.points[face.loops[0][1]], origin);
            let x_axis = unit(sub(along, face.normal.map(|c| c * dot(along, face.normal)))).unwrap_or([1.0, 0.0, 0.0]);
            let (next, previous) = chained(&planes, k);
            let fields = Fields::default().int(id()).int(0).int(faces[k]).int(next).int(previous).int(0).char('+');
            self.node(PLANE, planes[k], fields.vector(metres(origin)).vector(face.normal).vector(x_axis));
        }
        let mut loop_index = 0;
        for (k, face_loops) in loops.iter().enumerate() {
            for (j, &lp) in face_loops.iter().enumerate() {
                let next = face_loops.get(j + 1).copied().unwrap_or(0);
                let head = fins[loop_fins[loop_index][0]].index;
                self.node(LOOP, lp, Fields::default().int(id()).int(0).int(head).int(faces[k]).int(next));
                loop_index += 1;
            }
        }
        // Fins, edges with their lines, vertices with their points.
        for (i, f) in fins.iter().enumerate() {
            let e = &edges[f.edge];
            let other = e.fins.iter().copied().find(|&j| j != i).map_or(0, |j| fins[j].index);
            let (lp, forward, backward) = f.looped.unwrap_or((0, 0, 0));
            let fields = Fields::default().int(0).int(lp).int(forward).int(backward).int(vertices[f.vertex]).int(other).int(e.index).int(0).int(next_at_vertex[i]);
            self.node(FIN, f.index, fields.char(if f.positive { '+' } else { '-' }));
        }
        for (k, e) in edges.iter().enumerate() {
            let (next, previous) = chained(&edge_nodes, k);
            let primary = e.fins.iter().map(|&j| &fins[j]).find(|f| f.positive).map_or(0, |f| f.index);
            let fields = Fields::default().int(id()).int(0).unset().int(primary).int(previous).int(next).int(e.line).int(0).int(0).int(body);
            self.node(EDGE, e.index, fields);
            let (next, previous) = chained(&lines, k);
            let direction = unit(sub(part.points[e.end], part.points[e.start])).unwrap_or([1.0, 0.0, 0.0]);
            let fields = Fields::default().int(id()).int(0).int(e.index).int(next).int(previous).int(0).char('+');
            self.node(LINE_CURVE, e.line, fields.vector(metres(part.points[e.start])).vector(direction));
        }
        for (k, p) in part.points.iter().enumerate() {
            let (next, previous) = chained(&vertices, k);
            let head = at_vertex[k].first().map_or(0, |&i| fins[i].index);
            let fields = Fields::default().int(id()).int(0).int(head).int(previous).int(next).int(points[k]).unset().int(body);
            self.node(VERTEX, vertices[k], fields);
            let (next, previous) = chained(&points, k);
            self.node(POINT, points[k], Fields::default().int(id()).int(0).int(vertices[k]).int(next).int(previous).vector(metres(*p)));
        }
        // The body has no identifier of its own; it records the highest one.
        let highest = ids;
        let fields = Fields::default().int(highest).int(0).int(0).int(0).int(0).int(0).int(0).real(1000.0).real(1e-8).int(0).int(0).int(0).int(1).int(0).int(kind).int(1).int(shell);
        let fields = fields.int(first(&planes)).int(first(&lines)).int(first(&points)).int(void_region).int(first(&edge_nodes)).int(first(&vertices));
        self.node(BODY, body, fields);
        body
    }
}

/// Splits the stream into records; a separating space at a break starts the next
/// record, as readers drop trailing spaces.
fn records(stream: &str) -> String {
    let mut out = String::new();
    let mut rest = stream;
    while !rest.is_empty() {
        let mut cut = rest.len().min(LINE);
        if cut < rest.len() && rest.as_bytes()[cut - 1] == b' ' {
            cut -= 1;
        }
        out.push_str(&rest[..cut]);
        out.push('\n');
        rest = &rest[cut..];
    }
    out
}

/// Writes meshes (triangles in mm, any orientation) as one Parasolid XT text file:
/// the bodies of every mesh (see the crate documentation), under a pointer list when
/// there is more than one.
pub fn write_xt(meshes: &[&[Tri]]) -> String {
    let parts: Vec<Part> = meshes.iter().flat_map(|m| parts(m)).collect();
    let mut w = Writer::default();
    if parts.len() == 1 {
        let root = w.alloc();
        w.body(&parts[0], Some(root));
    } else {
        let root = w.alloc();
        let bodies: Vec<usize> = parts.iter().map(|p| w.body(p, None)).collect();
        let mut fields = Fields::default().int(bodies.len()).int(0);
        for b in &bodies {
            fields = fields.int(b);
        }
        w.nodes.push((root, format!("{POINTER_LIST} {} {root} {}", bodies.len(), fields.0)));
    }
    w.nodes.sort_by_key(|(index, _)| *index);
    let mut stream = format!("T{} {MODELLER}{} {SCHEMA}0 ", MODELLER.len(), SCHEMA.len());
    for (_, text) in &w.nodes {
        stream.push_str(text);
    }
    stream.push_str("1 0 ");
    let mut out: String = HEADER.iter().map(|l| format!("{l}\n")).collect();
    out.push_str(&records(&stream));
    out
}

#[cfg(test)]
mod tests;
