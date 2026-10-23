//! IFC import (IFC2x3 / IFC4 / IFC4x3, STEP physical file): the body geometry of all
//! products, placed by their local placements, as one mesh in millimetres.
//!
//! Read are tessellated face sets, faceted B-reps, shell and face based surface
//! models, extruded area solids (rectangle, circle, ellipse and polyline profiles),
//! blocks, cylinders, mapped items and the first operand of Boolean results. Curved
//! B-reps (`IfcAdvancedBrep`) and inner face bounds (holes) are left out.

use crate::graphics::{Mesh3d, MeshBuilder};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Ref(u64),
    Num(f64),
    Str(String),
    Enum(String),
    List(Vec<Value>),
    /// Typed value, e.g. `IFCLENGTHMEASURE(1.)`.
    Typed(String, Box<Value>),
    Null,
}

impl Value {
    fn num(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            Value::Typed(_, v) => v.num(),
            _ => None,
        }
    }
    fn reference(&self) -> Option<u64> {
        match self {
            Value::Ref(r) => Some(*r),
            _ => None,
        }
    }
    fn list(&self) -> &[Value] {
        match self {
            Value::List(l) => l,
            _ => &[],
        }
    }
    fn enumeration(&self) -> &str {
        match self {
            Value::Enum(e) => e,
            _ => "",
        }
    }
    fn text(&self) -> &str {
        match self {
            Value::Str(s) => s,
            _ => "",
        }
    }
}

struct Parser<'a> {
    s: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn skip_space(&mut self) {
        loop {
            while self.pos < self.s.len() && self.s[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.s[self.pos..].starts_with(b"/*") {
                self.pos = self.s[self.pos..].windows(2).position(|w| w == b"*/").map_or(self.s.len(), |i| self.pos + i + 2);
            } else {
                return;
            }
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.skip_space();
        self.s.get(self.pos).copied()
    }
    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("IFC file: '{}' expected at byte {}", c as char, self.pos))
        }
    }
    fn word(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.s.len() && (self.s[self.pos].is_ascii_alphanumeric() || self.s[self.pos] == b'_') {
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.pos]).to_ascii_uppercase()
    }
    fn value(&mut self) -> Result<Value, String> {
        match self.peek().ok_or("IFC file is truncated")? {
            b'#' => {
                self.pos += 1;
                Ok(Value::Ref(self.word().parse().map_err(|_| "IFC file: invalid reference")?))
            }
            b'$' | b'*' => {
                self.pos += 1;
                Ok(Value::Null)
            }
            b'.' => {
                self.pos += 1;
                let e = self.word();
                self.expect(b'.')?;
                Ok(Value::Enum(e))
            }
            b'\'' => {
                self.pos += 1;
                let mut out = Vec::new();
                loop {
                    let c = *self.s.get(self.pos).ok_or("IFC file: unterminated string")?;
                    self.pos += 1;
                    if c == b'\'' {
                        if self.s.get(self.pos) == Some(&b'\'') {
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                    out.push(c);
                }
                Ok(Value::Str(String::from_utf8_lossy(&out).into_owned()))
            }
            b'(' => Ok(Value::List(self.list()?)),
            b'"' => {
                self.pos += 1;
                while self.pos < self.s.len() && self.s[self.pos] != b'"' {
                    self.pos += 1;
                }
                self.pos += 1;
                Ok(Value::Null)
            }
            c if c.is_ascii_alphabetic() => {
                let name = self.word();
                let mut args = self.list()?;
                Ok(Value::Typed(name, Box::new(if args.len() == 1 { args.remove(0) } else { Value::List(args) })))
            }
            _ => {
                let start = self.pos;
                while self.pos < self.s.len() && matches!(self.s[self.pos], b'0'..=b'9' | b'.' | b'-' | b'+' | b'E' | b'e') {
                    self.pos += 1;
                }
                let text = std::str::from_utf8(&self.s[start..self.pos]).unwrap_or_default();
                text.parse().map(Value::Num).map_err(|_| format!("IFC file: unexpected '{}' at byte {start}", self.s[start] as char))
            }
        }
    }
    fn list(&mut self) -> Result<Vec<Value>, String> {
        self.expect(b'(')?;
        let mut items = Vec::new();
        if self.peek() == Some(b')') {
            self.pos += 1;
            return Ok(items);
        }
        loop {
            items.push(self.value()?);
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b')') => {
                    self.pos += 1;
                    return Ok(items);
                }
                _ => return Err(format!("IFC file: ',' or ')' expected at byte {}", self.pos)),
            }
        }
    }
}

type Entities = HashMap<u64, (String, Vec<Value>)>;

/// The entity instances of the DATA section; complex instances are left out.
fn parse_entities(bytes: &[u8]) -> Result<Entities, String> {
    let text = String::from_utf8_lossy(bytes);
    if !text.trim_start().starts_with("ISO-10303-21") || !text.contains("IFC") {
        return Err("not an IFC file (ISO-10303-21 with an IFC schema)".into());
    }
    let start = text.find("DATA;").ok_or("IFC file without DATA section")? + 5;
    let mut p = Parser { s: text.as_bytes(), pos: start };
    let mut out = HashMap::new();
    while let Some(b'#') = p.peek() {
        p.pos += 1;
        let id: u64 = p.word().parse().map_err(|_| "IFC file: invalid entity number")?;
        p.expect(b'=')?;
        if p.peek() == Some(b'(') {
            p.list()?;
        } else {
            let name = p.word();
            out.insert(id, (name, p.list()?));
        }
        p.expect(b';')?;
    }
    Ok(out)
}

type V = [f64; 3];

/// Affine transform: columns x, y, z and the origin.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Tf([V; 4]);

const IDENTITY: Tf = Tf([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0; 3]]);

impl Tf {
    fn point(&self, p: V) -> V {
        let m = &self.0;
        [0, 1, 2].map(|k| m[0][k] * p[0] + m[1][k] * p[1] + m[2][k] * p[2] + m[3][k])
    }
    fn vector(&self, p: V) -> V {
        let m = &self.0;
        [0, 1, 2].map(|k| m[0][k] * p[0] + m[1][k] * p[1] + m[2][k] * p[2])
    }
    /// `self` after `inner` (inner coordinates to outer).
    fn then(&self, inner: &Tf) -> Tf {
        Tf([self.vector(inner.0[0]), self.vector(inner.0[1]), self.vector(inner.0[2]), self.point(inner.0[3])])
    }
}

fn cross(a: V, b: V) -> V {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn unit(a: V) -> V {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if l < 1e-12 {
        a
    } else {
        a.map(|c| c / l)
    }
}

struct Ifc {
    e: Entities,
    mesh: MeshBuilder,
    scale: f64,
    triangles: usize,
}

impl Ifc {
    fn get(&self, v: &Value) -> Option<(&str, &[Value])> {
        self.e.get(&v.reference()?).map(|(n, a)| (n.as_str(), a.as_slice()))
    }

    fn point(&self, v: &Value) -> Option<V> {
        let (_, args) = self.get(v)?;
        let c = args.first()?.list();
        Some([c.first()?.num()?, c.get(1)?.num()?, c.get(2).and_then(Value::num).unwrap_or(0.0)])
    }

    fn direction(&self, v: &Value) -> Option<V> {
        self.point(v).map(unit)
    }

    /// IfcAxis2Placement3D / 2D, IfcCartesianTransformationOperator.
    fn placement(&self, v: &Value) -> Tf {
        let Some((name, args)) = self.get(v) else {
            return IDENTITY;
        };
        let arg = |i: usize| args.get(i).unwrap_or(&Value::Null);
        match name {
            "IFCAXIS2PLACEMENT3D" | "IFCAXIS2PLACEMENT2D" => {
                let origin = self.point(arg(0)).unwrap_or([0.0; 3]);
                let (z, x) = if name == "IFCAXIS2PLACEMENT2D" {
                    ([0.0, 0.0, 1.0], self.direction(arg(1)).unwrap_or([1.0, 0.0, 0.0]))
                } else {
                    (self.direction(arg(1)).unwrap_or([0.0, 0.0, 1.0]), self.direction(arg(2)).unwrap_or([1.0, 0.0, 0.0]))
                };
                let y = unit(cross(z, x));
                Tf([unit(cross(y, z)), y, z, origin])
            }
            n if n.starts_with("IFCCARTESIANTRANSFORMATIONOPERATOR") => {
                let origin = self.point(arg(2)).unwrap_or([0.0; 3]);
                let s = arg(3).num().unwrap_or(1.0);
                let x = self.direction(arg(0)).unwrap_or([1.0, 0.0, 0.0]);
                let z = self.direction(arg(4)).unwrap_or([0.0, 0.0, 1.0]);
                let y = self.direction(arg(1)).unwrap_or_else(|| unit(cross(z, x)));
                let (sy, sz) = if n.contains("NONUNIFORM") { (arg(5).num().unwrap_or(s), arg(6).num().unwrap_or(s)) } else { (s, s) };
                Tf([x.map(|c| c * s), y.map(|c| c * sy), z.map(|c| c * sz), origin])
            }
            _ => IDENTITY,
        }
    }

    /// IfcLocalPlacement chain to the world.
    fn object_placement(&self, v: &Value, depth: usize) -> Tf {
        match self.get(v) {
            Some(("IFCLOCALPLACEMENT", args)) if depth < 64 => {
                let parent = self.object_placement(args.first().unwrap_or(&Value::Null), depth + 1);
                parent.then(&self.placement(args.get(1).unwrap_or(&Value::Null)))
            }
            _ => IDENTITY,
        }
    }

    fn triangle(&mut self, tf: &Tf, t: [V; 3]) -> Result<(), String> {
        let [a, b, c] = t.map(|p| tf.point(p).map(|c| (c * self.scale) as f32));
        self.triangles += 1;
        self.mesh.triangle(a, b, c)
    }

    /// Triangulates a planar polygon (ear clipping in its dominant plane).
    fn polygon(&mut self, tf: &Tf, pts: &[V]) -> Result<(), String> {
        let mut pts: Vec<V> = pts.to_vec();
        pts.dedup();
        if pts.len() > 2 && pts.first() == pts.last() {
            pts.pop();
        }
        if pts.len() < 3 {
            return Ok(());
        }
        let mut n = [0.0; 3];
        for i in 0..pts.len() {
            let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
            n[0] += (p[1] - q[1]) * (p[2] + q[2]);
            n[1] += (p[2] - q[2]) * (p[0] + q[0]);
            n[2] += (p[0] - q[0]) * (p[1] + q[1]);
        }
        let axis = (0..3).max_by(|&a, &b| n[a].abs().total_cmp(&n[b].abs())).unwrap_or(2);
        let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
        let flat: Vec<[f64; 2]> = pts.iter().map(|p| if n[axis] >= 0.0 { [p[u], p[v]] } else { [p[v], p[u]] }).collect();
        for [a, b, c] in ear_clip(&flat) {
            self.triangle(tf, [pts[a], pts[b], pts[c]])?;
        }
        Ok(())
    }

    fn coord_list(&self, v: &Value) -> Vec<V> {
        let Some((_, args)) = self.get(v) else {
            return Vec::new();
        };
        let list = args.first().map(Value::list).unwrap_or_default();
        list.iter().map(|c| c.list()).map(|c| [0, 1, 2].map(|k| c.get(k).and_then(Value::num).unwrap_or(0.0))).collect()
    }

    /// A closed 2D profile as polygon (in its position).
    fn profile(&self, v: &Value) -> Vec<V> {
        let Some((name, args)) = self.get(v) else {
            return Vec::new();
        };
        let arg = |i: usize| args.get(i).unwrap_or(&Value::Null);
        let tf = self.placement(arg(2));
        let ellipse = |a: f64, b: f64| -> Vec<V> {
            (0..32).map(|i| i as f64 / 32.0 * std::f64::consts::TAU).map(|t| tf.point([a * t.cos(), b * t.sin(), 0.0])).collect()
        };
        match name {
            "IFCRECTANGLEPROFILEDEF" | "IFCRECTANGLEHOLLOWPROFILEDEF" | "IFCROUNDEDRECTANGLEPROFILEDEF" => {
                let (x, y) = (arg(3).num().unwrap_or(0.0) / 2.0, arg(4).num().unwrap_or(0.0) / 2.0);
                [[-x, -y, 0.0], [x, -y, 0.0], [x, y, 0.0], [-x, y, 0.0]].map(|p| tf.point(p)).to_vec()
            }
            "IFCCIRCLEPROFILEDEF" | "IFCCIRCLEHOLLOWPROFILEDEF" => {
                let r = arg(3).num().unwrap_or(0.0);
                ellipse(r, r)
            }
            "IFCELLIPSEPROFILEDEF" => ellipse(arg(3).num().unwrap_or(0.0), arg(4).num().unwrap_or(0.0)),
            "IFCARBITRARYCLOSEDPROFILEDEF" | "IFCARBITRARYPROFILEDEFWITHVOIDS" => self.curve_points(arg(2)),
            _ => Vec::new(),
        }
    }

    /// Points of a polyline or indexed poly curve (arcs as their points).
    fn curve_points(&self, v: &Value) -> Vec<V> {
        match self.get(v) {
            Some(("IFCPOLYLINE", args)) => args.first().map(Value::list).unwrap_or_default().iter().filter_map(|p| self.point(p)).collect(),
            Some(("IFCINDEXEDPOLYCURVE", args)) => self.coord_list(args.first().unwrap_or(&Value::Null)),
            _ => Vec::new(),
        }
    }

    fn extrusion(&mut self, tf: &Tf, profile: &[V], dir: V, depth: f64) -> Result<(), String> {
        if profile.len() < 3 {
            return Ok(());
        }
        let d = dir.map(|c| c * depth);
        let top: Vec<V> = profile.iter().map(|p| [p[0] + d[0], p[1] + d[1], p[2] + d[2]]).collect();
        let mut bottom = profile.to_vec();
        bottom.reverse();
        self.polygon(tf, &bottom)?;
        self.polygon(tf, &top)?;
        for i in 0..profile.len() {
            let j = (i + 1) % profile.len();
            self.polygon(tf, &[profile[i], profile[j], top[j], top[i]])?;
        }
        Ok(())
    }

    fn faces(&mut self, tf: &Tf, faces: &[Value]) -> Result<(), String> {
        for f in faces {
            let Some(("IFCFACE" | "IFCFACESURFACE", args)) = self.get(f) else {
                continue;
            };
            let bounds = args.first().map(Value::list).unwrap_or_default();
            let outer = bounds.iter().find(|b| matches!(self.get(b), Some(("IFCFACEOUTERBOUND", _)))).or(bounds.first());
            let Some((_, bound)) = outer.and_then(|b| self.get(b)) else {
                continue;
            };
            let reversed = bound.get(1).map(Value::enumeration) == Some("F");
            let Some(("IFCPOLYLOOP", lp)) = bound.first().and_then(|l| self.get(l)) else {
                continue;
            };
            let mut pts: Vec<V> = lp.first().map(Value::list).unwrap_or_default().iter().filter_map(|p| self.point(p)).collect();
            if reversed {
                pts.reverse();
            }
            self.polygon(tf, &pts)?;
        }
        Ok(())
    }

    fn shells(&mut self, tf: &Tf, shells: &[Value]) -> Result<(), String> {
        for s in shells {
            if let Some((_, args)) = self.get(s) {
                let faces = args.first().map(Value::list).unwrap_or_default().to_vec();
                self.faces(tf, &faces)?;
            }
        }
        Ok(())
    }

    /// Adds a representation item.
    fn item(&mut self, tf: &Tf, v: &Value, depth: usize) -> Result<(), String> {
        let Some((name, args)) = self.get(v) else {
            return Ok(());
        };
        if depth > 32 {
            return Ok(());
        }
        let args = args.to_vec();
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        match name {
            "IFCTRIANGULATEDFACESET" | "IFCTRIANGULATEDIRREGULARNETWORK" => {
                let pts = self.coord_list(&arg(0));
                let pn: Vec<usize> = arg(4).list().iter().filter_map(Value::num).map(|n| n as usize).collect();
                let at = |i: usize| -> Option<V> { pts.get(if pn.is_empty() { i } else { *pn.get(i.checked_sub(1)?)? }.checked_sub(1)?).copied() };
                for t in arg(3).list() {
                    let idx: Vec<usize> = t.list().iter().filter_map(Value::num).map(|n| n as usize).collect();
                    if let [a, b, c] = idx[..] {
                        if let (Some(a), Some(b), Some(c)) = (at(a), at(b), at(c)) {
                            self.triangle(tf, [a, b, c])?;
                        }
                    }
                }
            }
            "IFCPOLYGONALFACESET" => {
                let pts = self.coord_list(&arg(0));
                let pn: Vec<usize> = arg(3).list().iter().filter_map(Value::num).map(|n| n as usize).collect();
                for f in arg(2).list() {
                    let Some((_, face)) = self.get(f) else {
                        continue;
                    };
                    let poly: Vec<V> = face
                        .first()
                        .map(Value::list)
                        .unwrap_or_default()
                        .iter()
                        .filter_map(Value::num)
                        .filter_map(|n| {
                            let i = n as usize;
                            pts.get(if pn.is_empty() { i } else { *pn.get(i.checked_sub(1)?)? }.checked_sub(1)?).copied()
                        })
                        .collect();
                    self.polygon(tf, &poly)?;
                }
            }
            "IFCFACETEDBREP" | "IFCFACETEDBREPWITHVOIDS" => self.shells(tf, &[arg(0)])?,
            "IFCCLOSEDSHELL" | "IFCOPENSHELL" | "IFCCONNECTEDFACESET" => self.shells(tf, std::slice::from_ref(v))?,
            "IFCSHELLBASEDSURFACEMODEL" | "IFCFACEBASEDSURFACEMODEL" => self.shells(tf, arg(0).list())?,
            "IFCEXTRUDEDAREASOLID" | "IFCEXTRUDEDAREASOLIDTAPERED" => {
                let inner = tf.then(&self.placement(&arg(1)));
                let profile = self.profile(&arg(0));
                let dir = self.direction(&arg(2)).unwrap_or([0.0, 0.0, 1.0]);
                self.extrusion(&inner, &profile, dir, arg(3).num().unwrap_or(0.0))?;
            }
            "IFCBLOCK" => {
                let inner = tf.then(&self.placement(&arg(0)));
                let (x, y, z) = (arg(1).num().unwrap_or(0.0), arg(2).num().unwrap_or(0.0), arg(3).num().unwrap_or(0.0));
                self.extrusion(&inner, &[[0.0, 0.0, 0.0], [x, 0.0, 0.0], [x, y, 0.0], [0.0, y, 0.0]], [0.0, 0.0, 1.0], z)?;
            }
            "IFCRIGHTCIRCULARCYLINDER" => {
                let inner = tf.then(&self.placement(&arg(0)));
                let (h, r) = (arg(1).num().unwrap_or(0.0), arg(2).num().unwrap_or(0.0));
                let circle: Vec<V> = (0..32).map(|i| i as f64 / 32.0 * std::f64::consts::TAU).map(|t| [r * t.cos(), r * t.sin(), 0.0]).collect();
                self.extrusion(&inner, &circle, [0.0, 0.0, 1.0], h)?;
            }
            "IFCBOOLEANRESULT" | "IFCBOOLEANCLIPPINGRESULT" => {
                self.item(tf, &arg(1), depth + 1)?;
                if arg(0).enumeration() == "UNION" {
                    self.item(tf, &arg(2), depth + 1)?;
                }
            }
            "IFCCSGSOLID" => self.item(tf, &arg(0), depth + 1)?,
            "IFCMAPPEDITEM" => {
                if let Some((_, map)) = self.get(&arg(0)) {
                    let map = map.to_vec();
                    let target = tf.then(&self.placement(&arg(1)));
                    let inner = target.then(&self.placement(map.first().unwrap_or(&Value::Null)));
                    if let Some((_, rep)) = self.get(map.get(1).unwrap_or(&Value::Null)) {
                        for it in rep.get(3).map(Value::list).unwrap_or_default().to_vec() {
                            self.item(&inner, &it, depth + 1)?;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Millimetres per length unit of the project.
    fn length_unit(&self) -> f64 {
        let prefix = |p: &str| match p {
            "MILLI" => 1.0,
            "CENTI" => 10.0,
            "DECI" => 100.0,
            "KILO" => 1e6,
            "MICRO" => 1e-3,
            _ => 1000.0,
        };
        let units: Vec<&Value> = match self.e.values().find(|(n, _)| n == "IFCPROJECT").and_then(|(_, a)| self.get(a.get(8)?)) {
            Some((_, a)) => a.first().map(Value::list).unwrap_or_default().iter().collect(),
            None => Vec::new(),
        };
        let candidates: Vec<(&str, &[Value])> = if units.is_empty() {
            self.e.values().map(|(n, a)| (n.as_str(), a.as_slice())).collect()
        } else {
            units.iter().filter_map(|u| self.get(u)).collect()
        };
        for (name, args) in candidates {
            if args.get(1).map(Value::enumeration) != Some("LENGTHUNIT") {
                continue;
            }
            match name {
                "IFCSIUNIT" => return prefix(args.get(2).map(Value::enumeration).unwrap_or_default()),
                "IFCCONVERSIONBASEDUNIT" => {
                    let factor = self.get(args.get(3).unwrap_or(&Value::Null)).and_then(|(_, m)| {
                        let base = self.get(m.get(1)?).map_or(1000.0, |(_, si)| prefix(si.get(2).map(Value::enumeration).unwrap_or_default()));
                        Some(m.first()?.num()? * base)
                    });
                    let named = match args.get(2).map(Value::text).unwrap_or_default().to_ascii_uppercase().as_str() {
                        "INCH" => Some(25.4),
                        "FOOT" => Some(304.8),
                        _ => None,
                    };
                    if let Some(f) = factor.or(named) {
                        return f;
                    }
                }
                _ => {}
            }
        }
        1.0
    }
}

/// Products that are no physical objects.
const SKIPPED: [&str; 7] = ["IFCOPENINGELEMENT", "IFCSPACE", "IFCSITE", "IFCVIRTUALELEMENT", "IFCANNOTATION", "IFCGRID", "IFCOPENINGSTANDARDCASE"];

/// Reads the body geometry of an IFC file into a mesh in mm.
pub fn parse_ifc(bytes: &[u8]) -> Result<Mesh3d, String> {
    let e = parse_entities(bytes)?;
    let mut ifc = Ifc { e, mesh: MeshBuilder::new(), scale: 1.0, triangles: 0 };
    ifc.scale = ifc.length_unit();
    let mut products: Vec<(u64, Value, Value)> = ifc
        .e
        .iter()
        .filter(|(_, (name, _))| !SKIPPED.contains(&name.as_str()))
        .filter_map(|(&id, (_, args))| {
            let shape = args.get(6)?;
            matches!(ifc.get(shape), Some(("IFCPRODUCTDEFINITIONSHAPE", _))).then(|| (id, args[5].clone(), shape.clone()))
        })
        .collect();
    products.sort_by_key(|p| p.0);
    for (_, placement, shape) in products {
        let tf = ifc.object_placement(&placement, 0);
        let reps: Vec<Value> = ifc.get(&shape).and_then(|(_, a)| a.get(2)).map(Value::list).unwrap_or_default().to_vec();
        let reps: Vec<(String, Vec<Value>)> = reps
            .iter()
            .filter_map(|r| ifc.get(r))
            .map(|(_, a)| (a.get(1).map(Value::text).unwrap_or_default().to_string(), a.get(3).map(Value::list).unwrap_or_default().to_vec()))
            .collect();
        let body = reps.iter().any(|r| r.0.eq_ignore_ascii_case("Body"));
        for (identifier, items) in reps {
            let wanted = if body { identifier.eq_ignore_ascii_case("Body") } else { matches!(identifier.to_ascii_lowercase().as_str(), "" | "facetation" | "surface") };
            if wanted {
                for item in items {
                    ifc.item(&tf, &item, 0)?;
                }
            }
        }
    }
    if ifc.triangles == 0 {
        return Err("the IFC file contains no supported body geometry (tessellated, faceted or extruded)".into());
    }
    ifc.mesh.finish()
}

/// Ear clipping of a simple polygon; counter-clockwise or clockwise.
fn ear_clip(p: &[[f64; 2]]) -> Vec<[usize; 3]> {
    let n = p.len();
    let area: f64 = (0..n).map(|i| p[i][0] * p[(i + 1) % n][1] - p[(i + 1) % n][0] * p[i][1]).sum();
    let mut idx: Vec<usize> = (0..n).collect();
    if area < 0.0 {
        idx.reverse();
    }
    let cross = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    let mut out = Vec::with_capacity(n.saturating_sub(2));
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let ear = (0..m).find(|&i| {
            let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            cross(p[a], p[b], p[c]) > 1e-12
                && idx.iter().all(|&q| q == a || q == b || q == c || cross(p[a], p[b], p[q]) < 0.0 || cross(p[b], p[c], p[q]) < 0.0 || cross(p[c], p[a], p[q]) < 0.0)
        });
        let i = ear.unwrap_or(0);
        let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
        out.push(if area < 0.0 { [c, b, a] } else { [a, b, c] });
        idx.remove(i);
    }
    if idx.len() == 3 {
        out.push(if area < 0.0 { [idx[2], idx[1], idx[0]] } else { [idx[0], idx[1], idx[2]] });
    }
    out
}

#[cfg(test)]
mod tests;
