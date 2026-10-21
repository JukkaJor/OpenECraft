//! Small 3D vector helpers and an orbit camera, shared by the 3D views.

pub type V3 = [f32; 3];

pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

pub fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt().max(1e-6);
    [a[0] / l, a[1] / l, a[2] / l]
}

pub type Tri = [V3; 3];

/// Bounding box (min, max) of triangles.
pub fn tri_bounds(tris: &[Tri]) -> (V3, V3) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for v in tris.iter().flatten() {
        for k in 0..3 {
            min[k] = min[k].min(v[k]);
            max[k] = max[k].max(v[k]);
        }
    }
    (min, max)
}

fn boxes_overlap(a: (V3, V3), b: (V3, V3), eps: f32) -> bool {
    (0..3).all(|k| a.0[k] < b.1[k] - eps && b.0[k] < a.1[k] - eps)
}

/// True if the segment p0–p1 passes through the inside of the triangle (touching
/// edges, end points and segments lying in the triangle's plane do not count).
pub fn segment_crosses_triangle(p0: V3, p1: V3, t: &Tri) -> bool {
    const EPS: f32 = 1e-4;
    let dir = sub(p1, p0);
    let e1 = sub(t[1], t[0]);
    let e2 = sub(t[2], t[0]);
    let h = cross(dir, e2);
    let a = dot(e1, h);
    if a.abs() < 1e-9 {
        return false;
    }
    let f = 1.0 / a;
    let s = sub(p0, t[0]);
    let u = f * dot(s, h);
    let q = cross(s, e1);
    let v = f * dot(dir, q);
    let along = f * dot(e2, q);
    u > EPS && v > EPS && u + v < 1.0 - EPS && along > 1e-6 && along < 1.0 - 1e-6
}

/// Ray parity test: is `q` inside the closed surface(s) made of `tris`?
pub fn point_inside(q: V3, tris: &[Tri]) -> bool {
    // A slightly skewed ray avoids hitting edges of axis aligned models exactly.
    let far = [q[0] + 1.0e4, q[1] + 170.0, q[2] + 230.0];
    tris.iter().filter(|t| segment_crosses_triangle(q, far, t)).count() % 2 == 1
}

/// True if two triangle surfaces penetrate each other or one lies inside the
/// other. Surfaces that only touch do not count.
pub fn surfaces_intersect(a: &[Tri], b: &[Tri]) -> bool {
    let (ba, bb) = (tri_bounds(a), tri_bounds(b));
    if a.is_empty() || b.is_empty() || !boxes_overlap(ba, bb, 1e-4) {
        return false;
    }
    let edges_cross = |x: &[Tri], y: &[Tri], by: (V3, V3)| {
        x.iter().any(|t| {
            (0..3).any(|i| {
                let (p0, p1) = (t[i], t[(i + 1) % 3]);
                let eb = (
                    [p0[0].min(p1[0]), p0[1].min(p1[1]), p0[2].min(p1[2])],
                    [p0[0].max(p1[0]), p0[1].max(p1[1]), p0[2].max(p1[2])],
                );
                // Quick reject of edges outside the other surface.
                if (0..3).any(|k| eb.1[k] < by.0[k] || eb.0[k] > by.1[k]) {
                    return false;
                }
                y.iter().any(|u| segment_crosses_triangle(p0, p1, u))
            })
        })
    };
    if edges_cross(a, b, bb) || edges_cross(b, a, ba) {
        return true;
    }
    // No crossing edges: one surface may lie completely inside the other.
    let centroid = |t: &Tri| [(t[0][0] + t[1][0] + t[2][0]) / 3.0, (t[0][1] + t[1][1] + t[2][1]) / 3.0, (t[0][2] + t[1][2] + t[2][2]) / 3.0];
    point_inside(centroid(&a[0]), b) || point_inside(centroid(&b[0]), a)
}

/// The 12 triangles of a box (outward winding).
pub fn box_triangles(min: V3, max: V3) -> Vec<Tri> {
    let c = |x: usize, y: usize, z: usize| [if x == 0 { min[0] } else { max[0] }, if y == 0 { min[1] } else { max[1] }, if z == 0 { min[2] } else { max[2] }];
    let quads = [
        [c(0, 0, 0), c(0, 0, 1), c(0, 1, 1), c(0, 1, 0)],
        [c(1, 0, 0), c(1, 1, 0), c(1, 1, 1), c(1, 0, 1)],
        [c(0, 0, 0), c(1, 0, 0), c(1, 0, 1), c(0, 0, 1)],
        [c(0, 1, 0), c(0, 1, 1), c(1, 1, 1), c(1, 1, 0)],
        [c(0, 0, 0), c(0, 1, 0), c(1, 1, 0), c(1, 0, 0)],
        [c(0, 0, 1), c(1, 0, 1), c(1, 1, 1), c(0, 1, 1)],
    ];
    quads.iter().flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]]).collect()
}

/// Camera orbiting around `target` (y up).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub target: V3,
}

impl Orbit {
    /// Eye position and camera axes (right, up, forward).
    pub fn basis(&self) -> (V3, V3, V3, V3) {
        let eye = [
            self.target[0] + self.dist * self.pitch.cos() * self.yaw.sin(),
            self.target[1] + self.dist * self.pitch.sin(),
            self.target[2] + self.dist * self.pitch.cos() * self.yaw.cos(),
        ];
        let fwd = norm(sub(self.target, eye));
        let right = norm(cross(fwd, [0.0, 1.0, 0.0]));
        let up = cross(right, fwd);
        (eye, right, up, fwd)
    }
}

#[cfg(test)]
mod tests;
