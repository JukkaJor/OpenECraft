use crate::import::mesh_shells;
use cadmpeg_ir::geometry::{Curve as IrCurve, CurveGeometry, NurbsCurve as IrNurbsCurve, NurbsSurface as IrNurbsSurface, Surface as IrSurface, SurfaceGeometry};
use cadmpeg_ir::math::{Point3 as IrPoint3, Vector3 as IrVector3};
use cadmpeg_ir::topology::{
    Body as IrBody, BodyKind as IrBodyKind, Coedge, Edge as IrEdge, Face as IrFace, Loop as IrLoop, LoopBoundaryRole, Point as IrPoint, Region as IrRegion, Sense as IrSense, Shell as IrShell,
    Vertex as IrVertex,
};
use cadmpeg_ir::units::Units;
use cadmpeg_ir::CadIr;
use monstertruck_io::cadmpeg::{to_bodies, ImportedBody};
use monstertruck_meshing::prelude::*;
use monstertruck_modeling::Surface as MtSurface;
use openecraft_parts::{Mesh3d, MeshBuilder};
use parasolid_core::brep::{map_xb_brep, map_xt_brep, BrepModel, CurveKind, NurbsCurve, NurbsSurface, Sense, SurfaceKind};
use parasolid_core::{
    inspect_xb, inspect_xt, parse_xb, parse_xt, BuiltinProfileRegistry, BuiltinSchemaProvider, DocumentLimits, InspectionLimits, SchemaKey, SchemaProvider, SchemaTypeLookup, TypeDefinition,
};
use std::collections::HashMap;
use std::f64::consts::TAU;

type V3 = [f64; 3];

/// Millimetres per Parasolid length unit (metre).
const MM: f64 = 1000.0;
/// Points on a sampled curve per full turn, and at most along a free-form curve.
const TURN_SAMPLES: usize = 48;
const CURVE_SAMPLES: usize = 64;

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(a: V3, s: f64) -> V3 {
    a.map(|c| c * s)
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn length(a: V3) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: V3) -> V3 {
    let l = length(a);
    if l > 0.0 {
        scale(a, 1.0 / l)
    } else {
        a
    }
}
fn v3(v: parasolid_core::brep::Vector3) -> V3 {
    [v.x, v.y, v.z]
}
fn point(p: V3) -> IrPoint3 {
    IrPoint3::new(p[0], p[1], p[2])
}
fn vector(v: V3) -> IrVector3 {
    IrVector3::new(v[0], v[1], v[2])
}
fn positive(s: Sense) -> bool {
    s != Sense::Negative
}

/// Schemas embedded as differences to the V13 base (`SCH_…_13006`, every Parasolid
/// version since V14): resolved with the reviewed V13 base definitions.
struct EmbeddedBase<'a>(Vec<BuiltinSchemaProvider<'a>>);

impl SchemaProvider for EmbeddedBase<'_> {
    fn contains_schema(&self, schema: &str) -> bool {
        self.0.iter().any(|p| p.contains_schema(schema))
    }
    fn type_definition(&self, schema: &str, node_type: u16) -> Option<&TypeDefinition> {
        self.0.iter().find_map(|p| p.type_definition(schema, node_type))
    }
    fn lookup_type(&self, schema: &str, node_type: u16) -> SchemaTypeLookup<'_> {
        self.0.iter().map(|p| p.lookup_type(schema, node_type)).find(|l| !matches!(l, SchemaTypeLookup::Unknown)).unwrap_or(SchemaTypeLookup::Unknown)
    }
    fn supports_schema_key(&self, key: &SchemaKey) -> bool {
        key.base() == Some("13006")
    }
}

/// The B-rep of a text (`X_T`) or binary (`X_B`) transmit file.
fn read_model(bytes: &[u8]) -> Result<BrepModel, String> {
    let limits = InspectionLimits::default();
    let (text, raw_key) = match inspect_xt(bytes, limits) {
        Ok(h) => (true, h.schema_key),
        Err(text_error) => match inspect_xb(bytes, limits) {
            Ok(h) => (false, h.schema_key),
            Err(_) => return Err(format!("not a Parasolid transmit file: {text_error}")),
        },
    };
    let key = SchemaKey::parse(&raw_key).map_err(|e| e.to_string())?;
    let registry = BuiltinProfileRegistry::compiled().map_err(|e| e.to_string())?;
    let parse = |provider: &dyn Fn(&[u8]) -> Result<BrepModel, parasolid_core::ParseError>| provider(bytes).map_err(|e| format!("Parasolid file: {e}"));
    if let Some(provider) = registry.provider_for_key(&key) {
        return parse(&|b| if text { map_xt_brep(&parse_xt(b, &provider, DocumentLimits::default())?) } else { map_xb_brep(&parse_xb(b, &provider, DocumentLimits::default())?) });
    }
    if key.base() == Some("13006") {
        // The most complete base layouts first.
        let mut profiles: Vec<_> = registry.profiles().filter_map(|p| p.accepted_schema_keys().find(|k| k.provider_schema() == "13006").cloned()).collect();
        profiles.sort_by_key(|k| k.raw() != "SCH_1300000_13006");
        let provider = EmbeddedBase(profiles.iter().filter_map(|k| registry.provider_for_key(k)).collect());
        return parse(&|b| if text { map_xt_brep(&parse_xt(b, &provider, DocumentLimits::default())?) } else { map_xb_brep(&parse_xb(b, &provider, DocumentLimits::default())?) });
    }
    Err(format!("Parasolid schema {raw_key} is not supported"))
}

/// Knot vector with every knot repeated by its multiplicity.
fn expanded(knots: &[f64], multiplicities: &[u16]) -> Vec<f64> {
    knots.iter().zip(multiplicities).flat_map(|(k, &m)| std::iter::repeat_n(*k, m as usize)).collect()
}

/// Index of the knot span of `t` and the non-zero basis functions there.
fn basis(knots: &[f64], degree: usize, count: usize, t: f64) -> (usize, Vec<f64>) {
    let mut span = degree;
    while span + 1 < count && t >= knots[span + 1] {
        span += 1;
    }
    let mut n = vec![0.0; degree + 1];
    let (mut left, mut right) = (vec![0.0; degree + 1], vec![0.0; degree + 1]);
    n[0] = 1.0;
    for j in 1..=degree {
        left[j] = t - knots[span + 1 - j];
        right[j] = knots[span + j] - t;
        let mut saved = 0.0;
        for r in 0..j {
            let denominator = right[r + 1] + left[j - r];
            let temp = if denominator.abs() > 0.0 { n[r] / denominator } else { 0.0 };
            n[r] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }
        n[j] = saved;
    }
    (span, n)
}

/// Homogeneous control vertex as (x, y, z, w); rational vertices are stored
/// multiplied through by their weight.
fn homogeneous(v: &[f64]) -> [f64; 4] {
    [v.first().copied().unwrap_or(0.0), v.get(1).copied().unwrap_or(0.0), v.get(2).copied().unwrap_or(0.0), if v.len() >= 4 { v[3] } else { 1.0 }]
}

struct Spline {
    degree: usize,
    knots: Vec<f64>,
    poles: Vec<[f64; 4]>,
}

impl Spline {
    fn new(c: &NurbsCurve) -> Option<Spline> {
        let knots = expanded(&c.knots, &c.knot_multiplicities);
        let poles: Vec<[f64; 4]> = c.control_vertices.iter().map(|v| homogeneous(v)).collect();
        (knots.len() == poles.len() + c.degree as usize + 1 && !poles.is_empty()).then_some(Spline { degree: c.degree as usize, knots, poles })
    }
    fn range(&self) -> (f64, f64) {
        (self.knots[self.degree], self.knots[self.knots.len() - 1 - self.degree])
    }
    fn at(&self, t: f64) -> V3 {
        let (span, n) = basis(&self.knots, self.degree, self.poles.len(), t);
        let mut h = [0.0; 4];
        for (k, b) in n.iter().enumerate() {
            let p = self.poles[span - self.degree + k];
            for c in 0..4 {
                h[c] += b * p[c];
            }
        }
        if h[3].abs() > 0.0 {
            [h[0] / h[3], h[1] / h[3], h[2] / h[3]]
        } else {
            [h[0], h[1], h[2]]
        }
    }
    /// Parameter of the curve point nearest to `p`.
    fn parameter(&self, p: V3) -> f64 {
        let (a, b) = self.range();
        let n = 256;
        let mut best = (f64::INFINITY, a);
        for i in 0..=n {
            let t = a + (b - a) * i as f64 / n as f64;
            let d = length(sub(self.at(t), p));
            if d < best.0 {
                best = (d, t);
            }
        }
        let (mut lo, mut hi) = ((best.1 - (b - a) / n as f64).max(a), (best.1 + (b - a) / n as f64).min(b));
        for _ in 0..40 {
            let (m1, m2) = (lo + (hi - lo) / 3.0, hi - (hi - lo) / 3.0);
            if length(sub(self.at(m1), p)) < length(sub(self.at(m2), p)) {
                hi = m2;
            } else {
                lo = m1;
            }
        }
        (lo + hi) / 2.0
    }
    /// Clamped at both ends (the curve starts and ends at its end poles).
    fn clamped(&self) -> bool {
        let k = &self.knots;
        let d = self.degree;
        k[..=d].iter().all(|x| *x == k[0]) && k[k.len() - 1 - d..].iter().all(|x| *x == k[k.len() - 1])
    }
}

struct SplineSurface {
    u: Spline,
    v: Spline,
    /// Poles u-major: index `i * v_count + j`.
    poles: Vec<[f64; 4]>,
}

impl SplineSurface {
    /// With the poles read u-major (v fastest) or, `transposed`, v-major.
    fn new(s: &NurbsSurface, transposed: bool) -> Option<SplineSurface> {
        let (nu, nv) = (s.u_control_vertex_count as usize, s.v_control_vertex_count as usize);
        let raw: Vec<[f64; 4]> = s.control_vertices.iter().map(|v| homogeneous(v)).collect();
        if raw.len() != nu * nv {
            return None;
        }
        let poles = if transposed { (0..nu * nv).map(|k| raw[(k % nv) * nu + k / nv]).collect() } else { raw };
        let curve = |degree: u16, knots: &[f64], mult: &[u16], count: usize| {
            let knots = expanded(knots, mult);
            (knots.len() == count + degree as usize + 1).then_some(Spline { degree: degree as usize, knots, poles: vec![[0.0; 4]; count] })
        };
        Some(SplineSurface { u: curve(s.u_degree, &s.u_knots, &s.u_knot_multiplicities, nu)?, v: curve(s.v_degree, &s.v_knots, &s.v_knot_multiplicities, nv)?, poles })
    }
    fn at(&self, u: f64, v: f64) -> V3 {
        let nv = self.v.poles.len();
        let (su, bu) = basis(&self.u.knots, self.u.degree, self.u.poles.len(), u);
        let (sv, bv) = basis(&self.v.knots, self.v.degree, nv, v);
        let mut h = [0.0; 4];
        for (i, a) in bu.iter().enumerate() {
            for (j, b) in bv.iter().enumerate() {
                let p = self.poles[(su - self.u.degree + i) * nv + sv - self.v.degree + j];
                for c in 0..4 {
                    h[c] += a * b * p[c];
                }
            }
        }
        if h[3].abs() > 0.0 {
            [h[0] / h[3], h[1] / h[3], h[2] / h[3]]
        } else {
            [h[0], h[1], h[2]]
        }
    }
    /// Mean distance of `points` to a sampled grid of the surface.
    fn misfit(&self, points: &[V3]) -> f64 {
        let ((u0, u1), (v0, v1)) = (self.u.range(), self.v.range());
        let n = 24;
        let grid: Vec<V3> = (0..=n).flat_map(|i| (0..=n).map(move |j| (i, j))).map(|(i, j)| self.at(u0 + (u1 - u0) * i as f64 / n as f64, v0 + (v1 - v0) * j as f64 / n as f64)).collect();
        points.iter().map(|p| grid.iter().map(|g| length(sub(*g, *p))).fold(f64::INFINITY, f64::min)).sum::<f64>() / points.len().max(1) as f64
    }
}

/// A NURBS surface for the model, in mm: exact if clamped and not periodic,
/// otherwise resampled as a bilinear net over one period. The pole order that fits
/// the face's vertices is used; true if it is the transposed one.
fn nurbs_surface(s: &NurbsSurface, vertices: &[V3]) -> Option<(IrNurbsSurface, bool)> {
    let straight = SplineSurface::new(s, false);
    let swapped = SplineSurface::new(s, true);
    let (surface, transposed) = match (straight, swapped) {
        (Some(a), Some(b)) if !vertices.is_empty() && b.misfit(vertices) < a.misfit(vertices) * 0.5 => (b, true),
        (Some(a), _) => (a, false),
        (None, Some(b)) => (b, true),
        (None, None) => return None,
    };
    let rational = surface.poles.iter().any(|p| (p[3] - 1.0).abs() > 1e-12);
    if !s.u_periodic && !s.v_periodic && surface.u.clamped() && surface.v.clamped() {
        let control_points = surface.poles.iter().map(|p| point(scale([p[0] / p[3], p[1] / p[3], p[2] / p[3]], MM))).collect();
        let weights = rational.then(|| surface.poles.iter().map(|p| p[3]).collect());
        let ir = IrNurbsSurface {
            u_degree: surface.u.degree as u32,
            v_degree: surface.v.degree as u32,
            u_knots: surface.u.knots.clone(),
            v_knots: surface.v.knots.clone(),
            u_count: surface.u.poles.len() as u32,
            v_count: surface.v.poles.len() as u32,
            control_points,
            weights,
            u_periodic: false,
            v_periodic: false,
        };
        return Some((ir, transposed));
    }
    let ((u0, u1), (v0, v1)) = (surface.u.range(), surface.v.range());
    let n = 24;
    let control_points = (0..=n).flat_map(|i| (0..=n).map(move |j| (i, j))).map(|(i, j)| point(scale(surface.at(u0 + (u1 - u0) * i as f64 / n as f64, v0 + (v1 - v0) * j as f64 / n as f64), MM))).collect();
    let knots: Vec<f64> = std::iter::once(0.0).chain((0..=n).map(|k| k as f64)).chain(std::iter::once(n as f64)).collect();
    let ir = IrNurbsSurface { u_degree: 1, v_degree: 1, u_knots: knots.clone(), v_knots: knots, u_count: n + 1, v_count: n + 1, control_points, weights: None, u_periodic: false, v_periodic: false };
    Some((ir, transposed))
}

/// A Parasolid curve and the geometry it resolves to.
struct Curves<'a> {
    model: &'a BrepModel,
    by_id: HashMap<u32, usize>,
}

/// An edge curve for the model (mm) with the parameter interval of the edge.
type EdgeCurve = (CurveGeometry, Option<[f64; 2]>);

impl Curves<'_> {
    fn get(&self, id: u32) -> Option<&parasolid_core::brep::CurveGeometry> {
        self.by_id.get(&id).map(|&i| &self.model.curves[i])
    }

    /// The curve of an edge from `start` to `end` (metres; equal for a ring edge),
    /// running along the curve when `forward`.
    fn edge_curve(&self, id: Option<u32>, start: V3, end: V3, forward: bool) -> EdgeCurve {
        let Some(curve) = id.and_then(|id| self.get(id)) else { return line(start, end) };
        let forward = forward == positive(curve.sense);
        match &curve.kind {
            CurveKind::Line { .. } => line(start, end),
            CurveKind::Circle { center, normal, x_axis, radius } => {
                let (c, n, x) = (v3(*center), unit(v3(*normal)), unit(v3(*x_axis)));
                let y = cross(n, x);
                let angle = |p: V3| {
                    let d = sub(p, c);
                    dot(d, y).atan2(dot(d, x))
                };
                let (t0, mut t1) = (angle(start), angle(end));
                if forward {
                    while t1 <= t0 + 1e-9 {
                        t1 += TAU;
                    }
                } else {
                    while t1 >= t0 - 1e-9 {
                        t1 -= TAU;
                    }
                }
                (CurveGeometry::Circle { center: point(scale(c, MM)), axis: vector(n), ref_direction: vector(x), radius: radius * MM }, Some([t0, t1]))
            }
            CurveKind::Ellipse { center, normal, x_axis, major_radius, minor_radius } => {
                let (c, n, x) = (v3(*center), unit(v3(*normal)), unit(v3(*x_axis)));
                let y = cross(n, x);
                let (a, b) = (*major_radius, *minor_radius);
                let angle = |p: V3| {
                    let d = sub(p, c);
                    (dot(d, y) / b).atan2(dot(d, x) / a)
                };
                let (t0, mut t1) = (angle(start), angle(end));
                if forward {
                    while t1 <= t0 + 1e-9 {
                        t1 += TAU;
                    }
                } else {
                    while t1 >= t0 - 1e-9 {
                        t1 -= TAU;
                    }
                }
                let steps = ((t1 - t0).abs() / TAU * TURN_SAMPLES as f64).ceil().max(2.0) as usize;
                let points = (0..=steps).map(|k| {
                    let t = t0 + (t1 - t0) * k as f64 / steps as f64;
                    add(c, add(scale(x, a * t.cos()), scale(y, b * t.sin())))
                });
                polyline(points.collect(), start, end)
            }
            CurveKind::Nurbs(n) => match Spline::new(n) {
                Some(s) if !n.periodic && s.clamped() => {
                    let (t0, t1) = (s.parameter(start), s.parameter(end));
                    let control_points = s.poles.iter().map(|p| point(scale([p[0] / p[3], p[1] / p[3], p[2] / p[3]], MM))).collect();
                    let weights = s.poles.iter().any(|p| (p[3] - 1.0).abs() > 1e-12).then(|| s.poles.iter().map(|p| p[3]).collect());
                    let ir = IrNurbsCurve { degree: s.degree as u32, knots: s.knots.clone(), control_points, weights, periodic: false };
                    (CurveGeometry::Nurbs(ir), Some([t0, t1]))
                }
                Some(s) => {
                    let (a, b) = s.range();
                    let (mut t0, mut t1) = (s.parameter(start), s.parameter(end));
                    let ring = length(sub(start, end)) < 1e-9;
                    if ring || (forward && t1 <= t0) {
                        (t0, t1) = if forward { (a, b) } else { (b, a) };
                    }
                    let points = (0..=CURVE_SAMPLES).map(|k| s.at(t0 + (t1 - t0) * k as f64 / CURVE_SAMPLES as f64));
                    polyline(points.collect(), start, end)
                }
                None => line(start, end),
            },
            CurveKind::Intersection { chart_points, .. } if chart_points.len() >= 2 => {
                let chart: Vec<V3> = chart_points.iter().map(|p| v3(*p)).collect();
                let nearest = |p: V3| (0..chart.len()).min_by(|&i, &j| length(sub(chart[i], p)).total_cmp(&length(sub(chart[j], p)))).unwrap_or(0);
                let (i, j) = (nearest(start), nearest(end));
                let mut points: Vec<V3> = if i <= j { chart[i..=j].to_vec() } else { chart[j..=i].iter().rev().copied().collect() };
                if points.len() < 2 {
                    points = vec![start, end];
                }
                polyline(points, start, end)
            }
            // `forward` already tells whether the edge runs along this curve.
            CurveKind::Trimmed { basis_curve, .. } => self.edge_curve(Some(*basis_curve), start, end, forward),
            CurveKind::SurfaceParametric { original_curve: Some(original), .. } => self.edge_curve(Some(*original), start, end, forward),
            _ => line(start, end),
        }
    }
}

/// A closed edge curve (start = end) split at its middle: the two halves and the
/// point between them (metres).
fn halves(curve: &EdgeCurve) -> Option<(EdgeCurve, EdgeCurve, V3)> {
    match curve {
        (CurveGeometry::Circle { center, axis, ref_direction, radius }, Some([t0, t1])) => {
            let tm = (t0 + t1) / 2.0;
            let (c, n, x) = ([center.x, center.y, center.z], [axis.x, axis.y, axis.z], [ref_direction.x, ref_direction.y, ref_direction.z]);
            let mid = add(c, add(scale(x, radius * tm.cos()), scale(cross(n, x), radius * tm.sin())));
            Some(((curve.0.clone(), Some([*t0, tm])), (curve.0.clone(), Some([tm, *t1])), scale(mid, 1.0 / MM)))
        }
        (CurveGeometry::Nurbs(_), Some([t0, t1])) => {
            let tm = (t0 + t1) / 2.0;
            let mid = sample(&(curve.0.clone(), Some([tm, tm]))).first().copied()?;
            Some(((curve.0.clone(), Some([*t0, tm])), (curve.0.clone(), Some([tm, *t1])), scale(mid, 1.0 / MM)))
        }
        (CurveGeometry::Nurbs(n), None) if n.degree == 1 && n.control_points.len() >= 3 => {
            let k = n.control_points.len() / 2;
            let part = |points: &[IrPoint3]| {
                let pts: Vec<V3> = points.iter().map(|p| [p.x / MM, p.y / MM, p.z / MM]).collect();
                polyline(pts.clone(), pts[0], pts[pts.len() - 1])
            };
            let m = &n.control_points[k];
            Some((part(&n.control_points[..=k]), part(&n.control_points[k..]), [m.x / MM, m.y / MM, m.z / MM]))
        }
        _ => None,
    }
}

fn line(start: V3, end: V3) -> EdgeCurve {
    let direction = unit(sub(end, start));
    (CurveGeometry::Line { origin: point(scale(start, MM)), direction: vector(if length(direction) > 0.0 { direction } else { [1.0, 0.0, 0.0] }) }, None)
}

/// A curve through sampled points (degree 1), its ends exactly on the vertices.
fn polyline(mut points: Vec<V3>, start: V3, end: V3) -> EdgeCurve {
    if points.len() < 2 {
        return line(start, end);
    }
    let n = points.len();
    points[0] = start;
    points[n - 1] = end;
    let knots: Vec<f64> = std::iter::once(0.0).chain((0..n).map(|k| k as f64)).chain(std::iter::once((n - 1) as f64)).collect();
    let ir = IrNurbsCurve { degree: 1, knots, control_points: points.iter().map(|p| point(scale(*p, MM))).collect(), weights: None, periodic: false };
    (CurveGeometry::Nurbs(ir), None)
}

/// Sampled points of an edge curve (mm), from its start to its end.
fn sample(curve: &EdgeCurve) -> Vec<V3> {
    let p = |q: &IrPoint3| [q.x, q.y, q.z];
    match curve {
        (CurveGeometry::Circle { center, axis, ref_direction, radius }, Some([t0, t1])) => {
            let (c, n, x) = (p(center), [axis.x, axis.y, axis.z], [ref_direction.x, ref_direction.y, ref_direction.z]);
            let y = cross(n, x);
            let steps = ((t1 - t0).abs() / TAU * TURN_SAMPLES as f64).ceil().max(2.0) as usize;
            (0..=steps).map(|k| t0 + (t1 - t0) * k as f64 / steps as f64).map(|t| add(c, add(scale(x, radius * t.cos()), scale(y, radius * t.sin())))).collect()
        }
        (CurveGeometry::Nurbs(n), range) if n.degree == 1 && range.is_none() => n.control_points.iter().map(p).collect(),
        (CurveGeometry::Nurbs(n), Some([t0, t1])) => {
            let poles = n.control_points.iter().enumerate().map(|(i, q)| {
                let w = n.weights.as_ref().map_or(1.0, |w| w[i]);
                [q.x * w, q.y * w, q.z * w, w]
            });
            let s = Spline { degree: n.degree as usize, knots: n.knots.clone(), poles: poles.collect() };
            (0..=CURVE_SAMPLES).map(|k| s.at(t0 + (t1 - t0) * k as f64 / CURVE_SAMPLES as f64)).collect()
        }
        _ => Vec::new(),
    }
}

/// The face normal of a Parasolid analytic surface at a point near it (natural
/// normal, before senses), from its parametric form.
fn natural_normal(kind: &SurfaceKind, p: V3) -> Option<V3> {
    let frame = |axis: V3, x: V3| (unit(axis), unit(x), cross(unit(axis), unit(x)));
    let f: Box<dyn Fn(f64, f64) -> V3> = match kind {
        SurfaceKind::Plane { normal, .. } => return Some(unit(v3(*normal))),
        SurfaceKind::Cylinder { point: o, axis, radius, x_axis } => {
            let (a, x, y) = frame(v3(*axis), v3(*x_axis));
            let (o, r) = (v3(*o), *radius);
            Box::new(move |u, v| add(o, add(scale(add(scale(x, u.cos()), scale(y, u.sin())), r), scale(a, v))))
        }
        SurfaceKind::Cone { point: o, axis, radius, sin_half_angle, cos_half_angle, x_axis } => {
            let (a, x, y) = frame(v3(*axis), v3(*x_axis));
            let (o, r, t) = (v3(*o), *radius, sin_half_angle / cos_half_angle);
            Box::new(move |u, v| add(sub(o, scale(a, v)), scale(add(scale(x, u.cos()), scale(y, u.sin())), r + v * t)))
        }
        SurfaceKind::Sphere { center, radius, axis, x_axis } => {
            let (a, x, y) = frame(v3(*axis), v3(*x_axis));
            let (c, r) = (v3(*center), *radius);
            Box::new(move |u, v| add(c, add(scale(add(scale(x, u.cos()), scale(y, u.sin())), r * v.cos()), scale(a, r * v.sin()))))
        }
        SurfaceKind::Torus { center, axis, major_radius, minor_radius, x_axis } => {
            let (a, x, y) = frame(v3(*axis), v3(*x_axis));
            let (c, ma, mi) = (v3(*center), *major_radius, *minor_radius);
            Box::new(move |u, v| add(c, add(scale(add(scale(x, u.cos()), scale(y, u.sin())), ma + mi * v.cos()), scale(a, mi * v.sin()))))
        }
        _ => return None,
    };
    let (u, v) = match kind {
        SurfaceKind::Cylinder { point: o, axis, x_axis, .. } | SurfaceKind::Cone { point: o, axis, x_axis, .. } => {
            let (a, x, y) = frame(v3(*axis), v3(*x_axis));
            let d = sub(p, v3(*o));
            let along = dot(d, a);
            (dot(d, y).atan2(dot(d, x)), if matches!(kind, SurfaceKind::Cone { .. }) { -along } else { along })
        }
        SurfaceKind::Sphere { center, radius, axis, x_axis } => {
            let (a, x, y) = frame(v3(*axis), v3(*x_axis));
            let d = sub(p, v3(*center));
            (dot(d, y).atan2(dot(d, x)), (dot(d, a) / radius).clamp(-1.0, 1.0).asin())
        }
        SurfaceKind::Torus { center, axis, major_radius, x_axis, .. } => {
            let (a, x, y) = frame(v3(*axis), v3(*x_axis));
            let d = sub(p, v3(*center));
            let radial = length(sub(d, scale(a, dot(d, a))));
            (dot(d, y).atan2(dot(d, x)), dot(d, a).atan2(radial - major_radius))
        }
        _ => return None,
    };
    let h = 1e-5;
    let du = sub(f(u + h, v), f(u - h, v));
    let dv = sub(f(u, v + h), f(u, v - h));
    let n = cross(du, dv);
    (length(n) > 1e-18).then(|| unit(n))
}

/// The surface of a face for the model (mm); None for surfaces only the boundary
/// can stand for (blends, offsets, swept and spun surfaces). The flag is set when
/// the conversion reverses the natural normal.
fn surface_geometry(kind: &SurfaceKind, vertices: &[V3]) -> Option<(SurfaceGeometry, bool)> {
    let pm = |p: parasolid_core::brep::Vector3| point(scale(v3(p), MM));
    let dv = |v: parasolid_core::brep::Vector3| vector(unit(v3(v)));
    Some(match kind {
        SurfaceKind::Plane { point: o, normal, x_axis } => (SurfaceGeometry::Plane { origin: pm(*o), normal: dv(*normal), u_axis: dv(*x_axis) }, false),
        SurfaceKind::Cylinder { point: o, axis, radius, x_axis } => (SurfaceGeometry::Cylinder { origin: pm(*o), axis: dv(*axis), ref_direction: dv(*x_axis), radius: radius * MM }, false),
        // The XT cone opens against its axis.
        SurfaceKind::Cone { point: o, axis, radius, sin_half_angle, cos_half_angle, x_axis } => {
            let axis = vector(scale(unit(v3(*axis)), -1.0));
            (SurfaceGeometry::Cone { origin: pm(*o), axis, ref_direction: dv(*x_axis), radius: radius * MM, ratio: 1.0, half_angle: sin_half_angle.atan2(*cos_half_angle) }, false)
        }
        SurfaceKind::Sphere { center, radius, axis, x_axis } => (SurfaceGeometry::Sphere { center: pm(*center), axis: dv(*axis), ref_direction: dv(*x_axis), radius: radius * MM }, false),
        SurfaceKind::Torus { center, axis, major_radius, minor_radius, x_axis } if *major_radius > 0.0 => {
            (SurfaceGeometry::Torus { center: pm(*center), axis: dv(*axis), ref_direction: dv(*x_axis), major_radius: major_radius * MM, minor_radius: minor_radius * MM }, false)
        }
        SurfaceKind::Nurbs(n) => {
            let (surface, transposed) = nurbs_surface(n, vertices)?;
            (SurfaceGeometry::Nurbs(surface), transposed)
        }
        _ => return None,
    })
}

/// The IR of chosen faces: one sheet body per Parasolid body. Faces whose surface
/// cannot be represented are returned as left over.
struct Builder<'a> {
    model: &'a BrepModel,
    curves: Curves<'a>,
    ir: CadIr,
    vertex_ids: HashMap<u32, String>,
    edge_ids: HashMap<u32, Vec<String>>,
    /// Per IR face: Parasolid face index and whether its normal check must be inverted.
    faces: Vec<(usize, bool)>,
}

struct Index {
    surfaces: HashMap<u32, usize>,
    loops: HashMap<u32, usize>,
    half_edges: HashMap<u32, usize>,
    edges: HashMap<u32, usize>,
    vertices: HashMap<u32, usize>,
    points: HashMap<u32, usize>,
    shells: HashMap<u32, usize>,
    regions: HashMap<u32, usize>,
}

impl Index {
    fn new(m: &BrepModel) -> Index {
        Index {
            surfaces: m.surfaces.iter().enumerate().map(|(i, s)| (s.id, i)).collect(),
            loops: m.loops.iter().enumerate().map(|(i, s)| (s.id, i)).collect(),
            half_edges: m.half_edges.iter().enumerate().map(|(i, s)| (s.id, i)).collect(),
            edges: m.edges.iter().enumerate().map(|(i, s)| (s.id, i)).collect(),
            vertices: m.vertices.iter().enumerate().map(|(i, s)| (s.id, i)).collect(),
            points: m.points.iter().enumerate().map(|(i, s)| (s.id, i)).collect(),
            shells: m.shells.iter().enumerate().map(|(i, s)| (s.id, i)).collect(),
            regions: m.regions.iter().enumerate().map(|(i, s)| (s.id, i)).collect(),
        }
    }

    fn vertex_position(&self, m: &BrepModel, vertex: u32) -> Option<V3> {
        let v = &m.vertices[*self.vertices.get(&vertex)?];
        Some(v3(m.points[*self.points.get(&v.point)?].position))
    }

    /// Start and end of an edge (metres); a ring edge starts and ends at the start of its curve.
    fn edge_ends(&self, m: &BrepModel, curves: &Curves<'_>, edge: usize) -> Option<(V3, V3)> {
        let e = &m.edges[edge];
        match (e.start_vertex, e.end_vertex) {
            (Some(a), Some(b)) => Some((self.vertex_position(m, a)?, self.vertex_position(m, b)?)),
            _ => {
                let start = match &curves.get(e.curve?)?.kind {
                    CurveKind::Circle { center, x_axis, radius, .. } => add(v3(*center), scale(unit(v3(*x_axis)), *radius)),
                    CurveKind::Ellipse { center, x_axis, major_radius, .. } => add(v3(*center), scale(unit(v3(*x_axis)), *major_radius)),
                    CurveKind::Nurbs(n) => {
                        let s = Spline::new(n)?;
                        s.at(s.range().0)
                    }
                    _ => return None,
                };
                Some((start, start))
            }
        }
    }

    /// The vertices of a face (metres), for fitting its surface.
    fn face_vertices(&self, m: &BrepModel, face: usize) -> Vec<V3> {
        let mut out = Vec::new();
        for lp in &m.faces[face].loops {
            let Some(&l) = self.loops.get(lp) else { continue };
            for he in &m.loops[l].half_edges {
                if let Some(p) = self.half_edges.get(he).and_then(|&h| m.half_edges[h].vertex).and_then(|v| self.vertex_position(m, v)) {
                    out.push(p);
                }
            }
        }
        out
    }

    /// The Parasolid body of a face.
    fn body_of(&self, m: &BrepModel, face: usize) -> Option<u32> {
        let shell = &m.shells[*self.shells.get(&m.faces[face].back_shell)?];
        Some(m.regions[*self.regions.get(&shell.region)?].body)
    }
}

impl<'a> Builder<'a> {
    fn new(model: &'a BrepModel) -> Builder<'a> {
        let curves = Curves { model, by_id: model.curves.iter().enumerate().map(|(i, c)| (c.id, i)).collect() };
        Builder { model, curves, ir: CadIr::empty(Units::default()), vertex_ids: HashMap::new(), edge_ids: HashMap::new(), faces: Vec::new() }
    }

    fn vertex(&mut self, key: u32, position: V3) -> String {
        if let Some(id) = self.vertex_ids.get(&key) {
            return id.clone();
        }
        let n = self.ir.model.vertices.len();
        let (vid, pid) = (format!("v{n}"), format!("p{n}"));
        self.ir.model.points.push(IrPoint { id: pid.clone().into(), position: point(scale(position, MM)), source_object: None });
        self.ir.model.vertices.push(IrVertex { id: vid.clone().into(), point: pid.into(), tolerance: None });
        self.vertex_ids.insert(key, vid.clone());
        vid
    }

    fn add_edge(&mut self, (geometry, param_range): EdgeCurve, a: String, b: String) -> String {
        let n = self.ir.model.edges.len();
        let (id, cid) = (format!("e{n}"), format!("c{n}"));
        self.ir.model.curves.push(IrCurve { id: cid.clone().into(), geometry, source_object: None });
        self.ir.model.edges.push(IrEdge { id: id.clone().into(), curve: Some(cid.into()), start: a.into(), end: b.into(), param_range, tolerance: None });
        id
    }

    /// The model edges of an edge, in its direction: closed edges are split in two
    /// (the tessellator cannot trim with an edge that starts where it ends).
    fn edge(&mut self, ix: &Index, edge: usize) -> Option<Vec<String>> {
        let e = &self.model.edges[edge];
        if let Some(ids) = self.edge_ids.get(&e.id) {
            return Some(ids.clone());
        }
        let (start, end) = ix.edge_ends(self.model, &self.curves, edge)?;
        // Ring edges get vertices of their own (keyed beyond the vertex ids).
        let ring = u32::MAX - 2 * e.id.min(u32::MAX / 4);
        let (sv, ev) = (e.start_vertex.unwrap_or(ring), e.end_vertex.unwrap_or(ring));
        let a = self.vertex(sv, start);
        let b = self.vertex(ev, end);
        let curve = self.curves.edge_curve(e.curve, start, end, true);
        let ids = match halves(&curve).filter(|_| sv == ev) {
            Some((first, second, mid)) => {
                let m = self.vertex(ring - 1, mid);
                vec![self.add_edge(first, a, m.clone()), self.add_edge(second, m, b)]
            }
            None => vec![self.add_edge(curve, a, b)],
        };
        self.edge_ids.insert(e.id, ids.clone());
        Some(ids)
    }

    /// Adds a face to `shell`; false if its surface or boundary cannot be represented.
    fn face(&mut self, ix: &Index, face: usize, shell: &str) -> bool {
        let f = &self.model.faces[face];
        let Some(surface) = f.surface.and_then(|s| ix.surfaces.get(&s)).map(|&i| &self.model.surfaces[i]) else { return false };
        let Some((geometry, inverted)) = surface_geometry(&surface.kind, &ix.face_vertices(self.model, face)) else { return false };
        if f.loops.is_empty() {
            return false;
        }
        let n = self.ir.model.faces.len();
        let fid = format!("f{n}");
        let sid = format!("s{n}");
        let mut loops = Vec::new();
        for lp in &f.loops {
            let Some(&l) = ix.loops.get(lp) else { return false };
            let fins: Vec<usize> = self.model.loops[l].half_edges.iter().filter_map(|h| ix.half_edges.get(h).copied()).filter(|&h| self.model.half_edges[h].edge.is_some()).collect();
            // A loop of a single vertex (an apex) is no boundary to trim with.
            if fins.is_empty() {
                continue;
            }
            let lid = format!("{fid}l{}", loops.len());
            // The model edges along the loop, with their senses.
            let mut uses: Vec<(String, IrSense)> = Vec::new();
            for &h in &fins {
                let fin = &self.model.half_edges[h];
                let Some(edge) = fin.edge.and_then(|e| ix.edges.get(&e)).copied() else { return false };
                let Some(mut eids) = self.edge(ix, edge) else { return false };
                let sense = if positive(fin.sense) { IrSense::Forward } else { IrSense::Reversed };
                if sense == IrSense::Reversed {
                    eids.reverse();
                }
                uses.extend(eids.into_iter().map(|e| (e, sense)));
            }
            let ids: Vec<String> = (0..uses.len()).map(|k| format!("{lid}c{k}")).collect();
            for (k, (eid, sense)) in uses.into_iter().enumerate() {
                let (next, previous) = (ids[(k + 1) % ids.len()].clone(), ids[(k + ids.len() - 1) % ids.len()].clone());
                self.ir.model.coedges.push(Coedge {
                    id: ids[k].clone().into(),
                    owner_loop: lid.clone().into(),
                    edge: eid.into(),
                    next: next.into(),
                    previous: previous.into(),
                    radial_next: ids[k].clone().into(),
                    sense,
                    pcurves: Vec::new(),
                    use_curve: None,
                    use_curve_parameter_range: None,
                });
            }
            self.ir.model.loops.push(IrLoop { id: lid.clone().into(), face: fid.clone().into(), boundary_role: LoopBoundaryRole::Unspecified, coedges: ids.into_iter().map(Into::into).collect(), vertex_uses: Vec::new() });
            loops.push(lid);
        }
        if loops.is_empty() {
            return false;
        }
        // The face normal follows the surface normal when both senses agree.
        let along = positive(f.sense) == positive(surface.sense);
        self.ir.model.surfaces.push(IrSurface { id: sid.clone().into(), geometry, source_object: None });
        self.ir.model.faces.push(IrFace {
            id: fid.clone().into(),
            shell: shell.to_string().into(),
            surface: sid.into(),
            sense: if along { IrSense::Forward } else { IrSense::Reversed },
            loops: loops.into_iter().map(Into::into).collect(),
            name: None,
            color: None,
            tolerance: None,
        });
        self.faces.push((face, inverted));
        true
    }

    /// One sheet body with the faces; returns the faces that were left out.
    fn body(&mut self, ix: &Index, faces: &[usize]) -> Vec<usize> {
        let n = self.ir.model.bodies.len();
        let (bid, rid, sid) = (format!("b{n}"), format!("r{n}"), format!("sh{n}"));
        let first = self.ir.model.faces.len();
        let rest: Vec<usize> = faces.iter().copied().filter(|&f| !self.face(ix, f, &sid)).collect();
        let face_ids = self.ir.model.faces[first..].iter().map(|f| f.id.clone()).collect::<Vec<_>>();
        if face_ids.is_empty() {
            return rest;
        }
        self.ir.model.shells.push(IrShell { id: sid.clone().into(), region: rid.clone().into(), faces: face_ids, wire_edges: Vec::new(), free_vertices: Vec::new() });
        self.ir.model.regions.push(IrRegion { id: rid.clone().into(), body: bid.clone().into(), shells: vec![sid.into()] });
        self.ir.model.bodies.push(IrBody { id: bid.into(), kind: IrBodyKind::Sheet, regions: vec![rid.into()], transform: None, name: None, color: None, visible: None });
        rest
    }
}

/// A point of a monstertruck surface inside its parameter range and the surface
/// normal there.
fn sample_normal(surface: &MtSurface) -> Option<(V3, V3)> {
    let (ru, rv) = surface.try_range_tuple();
    let mid = |r: Option<(f64, f64)>| r.map_or(0.0, |(a, b)| (a + b) / 2.0);
    let (u, v) = (mid(ru), mid(rv));
    let p = surface.subs(u, v);
    let n = surface.uder(u, v).cross(surface.vder(u, v));
    let n = [n.x, n.y, n.z];
    (length(n) > 1e-18).then(|| ([p.x / MM, p.y / MM, p.z / MM], unit(n)))
}

/// Triangles of a face from its boundary alone (mm), for surfaces the model cannot
/// carry: each loop sampled along its edges, holes bridged to the outer loop, the
/// polygon ear-clipped in its best-fitting plane.
fn boundary_triangles(model: &BrepModel, ix: &Index, curves: &Curves<'_>, face: usize) -> Vec<[V3; 3]> {
    let mut rings: Vec<Vec<V3>> = Vec::new();
    for lp in &model.faces[face].loops {
        let Some(&l) = ix.loops.get(lp) else { continue };
        let mut ring: Vec<V3> = Vec::new();
        for he in &model.loops[l].half_edges {
            let Some(fin) = ix.half_edges.get(he).map(|&h| &model.half_edges[h]) else { continue };
            let Some(edge) = fin.edge.and_then(|e| ix.edges.get(&e)).copied() else { continue };
            let Some((start, end)) = ix.edge_ends(model, curves, edge) else { continue };
            let curve = curves.edge_curve(model.edges[edge].curve, start, end, true);
            let mut points = sample(&curve);
            if points.is_empty() {
                points = vec![scale(start, MM), scale(end, MM)];
            }
            if !positive(fin.sense) {
                points.reverse();
            }
            points.pop();
            ring.extend(points);
        }
        if ring.len() >= 3 {
            rings.push(ring);
        }
    }
    triangulate(rings)
}

/// Newell normal of a ring (its length is twice the area).
fn newell(ring: &[V3]) -> V3 {
    let mut n = [0.0; 3];
    for k in 0..ring.len() {
        let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
        n = add(n, [(a[1] - b[1]) * (a[2] + b[2]), (a[2] - b[2]) * (a[0] + b[0]), (a[0] - b[0]) * (a[1] + b[1])]);
    }
    n
}

/// Ear-clips rings (the largest is the outer one, the others holes) into triangles
/// wound like the outer ring.
fn triangulate(mut rings: Vec<Vec<V3>>) -> Vec<[V3; 3]> {
    if rings.is_empty() {
        return Vec::new();
    }
    rings.sort_by(|a, b| length(newell(b)).total_cmp(&length(newell(a))));
    let normal = unit(newell(&rings[0]));
    if length(normal) == 0.0 {
        return Vec::new();
    }
    let x = unit(if normal[0].abs() < 0.9 { cross(normal, [1.0, 0.0, 0.0]) } else { cross(normal, [0.0, 1.0, 0.0]) });
    let y = cross(normal, x);
    let flat = |p: V3| [dot(p, x), dot(p, y)];
    // Bridge every hole (wound against the outer ring) at its point furthest along x.
    let mut polygon = rings[0].clone();
    for hole in rings.iter().skip(1) {
        let mut hole = hole.clone();
        if dot(newell(&hole), normal) > 0.0 {
            hole.reverse();
        }
        let h = (0..hole.len()).max_by(|&a, &b| flat(hole[a])[0].total_cmp(&flat(hole[b])[0])).unwrap_or(0);
        let hp = flat(hole[h]);
        let o = (0..polygon.len()).min_by(|&a, &b| {
            let (pa, pb) = (flat(polygon[a]), flat(polygon[b]));
            ((pa[0] - hp[0]).powi(2) + (pa[1] - hp[1]).powi(2)).total_cmp(&((pb[0] - hp[0]).powi(2) + (pb[1] - hp[1]).powi(2)))
        });
        let Some(o) = o else { continue };
        let mut spliced: Vec<V3> = polygon[..=o].to_vec();
        spliced.extend(hole[h..].iter().chain(hole[..=h].iter()).copied());
        spliced.extend(polygon[o..].iter().copied());
        polygon = spliced;
    }
    let pts: Vec<[f64; 2]> = polygon.iter().map(|p| flat(*p)).collect();
    let area2 = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    let inside = |p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]| area2(a, b, p) > 1e-12 && area2(b, c, p) > 1e-12 && area2(c, a, p) > 1e-12;
    let mut idx: Vec<usize> = (0..pts.len()).collect();
    let mut out = Vec::new();
    let mut guard = 0;
    while idx.len() > 3 && guard < 4 * pts.len() + 16 {
        guard += 1;
        let n = idx.len();
        let ear = (0..n).find(|&k| {
            let (a, b, c) = (pts[idx[(k + n - 1) % n]], pts[idx[k]], pts[idx[(k + 1) % n]]);
            area2(a, b, c) > 1e-12 && !idx.iter().any(|&j| j != idx[k] && j != idx[(k + n - 1) % n] && j != idx[(k + 1) % n] && inside(pts[j], a, b, c))
        });
        // A degenerate remainder is clipped anyway rather than left as a hole.
        let k = ear.unwrap_or(0);
        out.push([polygon[idx[(k + n - 1) % n]], polygon[idx[k]], polygon[idx[(k + 1) % n]]]);
        idx.remove(k);
    }
    if idx.len() == 3 {
        out.push([polygon[idx[0]], polygon[idx[1]], polygon[idx[2]]]);
    }
    out.retain(|t| length(cross(sub(t[1], t[0]), sub(t[2], t[0]))) > 1e-12);
    out
}

/// Tessellates the converted bodies after aligning each face with the Parasolid
/// face normal.
fn mesh_bodies(builder: &Builder<'_>, ix: &Index) -> Result<Mesh3d, String> {
    let bodies = to_bodies(&builder.ir, "Parasolid").map_err(|e| e.to_string())?;
    let mut shells = Vec::new();
    let mut k = 0;
    for body in bodies {
        let mut shell = match body {
            ImportedBody::Sheet(sheet) => sheet,
            ImportedBody::Solid(solid) => solid.boundaries.into_iter().next().ok_or("empty solid")?,
        };
        for face in &mut shell.faces {
            let Some(&(ps_face, inverted)) = builder.faces[..].get(k) else { break };
            k += 1;
            let f = &builder.model.faces[ps_face];
            let Some(surface) = f.surface.and_then(|s| ix.surfaces.get(&s)).map(|&i| &builder.model.surfaces[i]) else { continue };
            let mut wanted = positive(f.sense) == positive(surface.sense);
            if inverted {
                wanted = !wanted;
            }
            let check = sample_normal(&face.surface).and_then(|(p, n)| natural_normal(&surface.kind, p).map(|m| dot(m, n)));
            // The converted surface's normal runs against the XT one: flip the face.
            face.orientation = match check {
                Some(d) if d < 0.0 => !wanted,
                _ => wanted,
            };
        }
        shells.push((shell, Matrix4::identity()));
    }
    mesh_shells(&shells, 1.0)
}

/// Reads a Parasolid transmit file (`.x_t` text or `.x_b` binary, V13 or newer with
/// embedded schema, or a schema `parasolid-core` knows) into a mesh in mm. Planes,
/// cylinders, cones, spheres, tori and B-surfaces are tessellated exactly; faces on
/// blend, offset, swept or spun surfaces are filled from their boundary.
pub fn parse_parasolid(bytes: &[u8]) -> Result<Mesh3d, String> {
    let model = read_model(bytes)?;
    let ix = Index::new(&model);
    let mut by_body: Vec<(u32, Vec<usize>)> = Vec::new();
    for face in 0..model.faces.len() {
        let body = ix.body_of(&model, face).unwrap_or(u32::MAX);
        match by_body.iter_mut().find(|(b, _)| *b == body) {
            Some((_, list)) => list.push(face),
            None => by_body.push((body, vec![face])),
        }
    }
    if by_body.is_empty() {
        return Err("the Parasolid file contains no faces".into());
    }
    let mut builder = Builder::new(&model);
    let mut rest = Vec::new();
    for (_, faces) in &by_body {
        rest.extend(builder.body(&ix, faces));
    }
    let mut meshes = Vec::new();
    if !builder.ir.model.bodies.is_empty() {
        match mesh_bodies(&builder, &ix) {
            Ok(mesh) => meshes.push(mesh),
            // One face the tessellator refuses must not lose the others: one at a time.
            Err(_) => {
                for &(face, _) in &builder.faces {
                    let mut single = Builder::new(&model);
                    let left = single.body(&ix, &[face]);
                    match mesh_bodies(&single, &ix) {
                        Ok(mesh) if left.is_empty() => meshes.push(mesh),
                        _ => rest.push(face),
                    }
                }
            }
        }
    }
    let mut b = MeshBuilder::new();
    for mesh in &meshes {
        for t in &mesh.triangles {
            b.triangle(mesh.vertices[t[0] as usize], mesh.vertices[t[1] as usize], mesh.vertices[t[2] as usize])?;
        }
    }
    for face in rest {
        for t in boundary_triangles(&model, &ix, &builder.curves, face) {
            let f = |p: V3| p.map(|c| c as f32);
            b.triangle(f(t[0]), f(t[1]), f(t[2]))?;
        }
    }
    b.finish()
}

#[cfg(test)]
mod tests;
