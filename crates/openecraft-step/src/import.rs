use cadmpeg_ir::products::{AssemblyGraph, PrototypeReference};
use cadmpeg_ir::topology::BodyKind;
use cadmpeg_ir::transform::Transform;
use cadmpeg_ir::CadIr;
use monstertruck_io::cadmpeg::{to_bodies, ImportedBody, ImportedSheet};
use monstertruck_meshing::prelude::*;
use monstertruck_modeling::cgmath::Transform as _;
use monstertruck_topology::compress::CompressedShell;
use openecraft_parts::{Mesh3d, MeshBuilder, MAX_TRIANGLES};
use std::collections::HashMap;

/// Chord tolerance of the tessellation relative to the size of a shell; coarser
/// steps are tried when the mesh gets too large.
const TOLERANCES: [f64; 4] = [0.002, 0.006, 0.02, 0.06];

/// The shells of a converted body.
pub(crate) fn body_shells(body: ImportedBody) -> Vec<ImportedSheet> {
    match body {
        ImportedBody::Solid(solid) => solid.boundaries,
        ImportedBody::Sheet(sheet) => vec![sheet],
    }
}

fn matrix(t: &Transform) -> Matrix4 {
    let r = &t.rows;
    Matrix4::new(
        r[0][0], r[1][0], r[2][0], r[3][0], r[0][1], r[1][1], r[2][1], r[3][1], r[0][2], r[1][2], r[2][2], r[3][2], r[0][3], r[1][3], r[2][3], r[3][3],
    )
}

/// All shells of the solids and surface models with their placement: once per use
/// in the assembly structure; bodies of no product where they are.
fn placed_shells(ir: &CadIr) -> Result<Vec<(ImportedSheet, Matrix4)>, String> {
    let bodies = to_bodies(ir, "STEP").map_err(|e| format!("STEP file: {e}"))?;
    let mut uses: HashMap<_, Vec<Matrix4>> = HashMap::new();
    if let Ok(graph) = AssemblyGraph::new(&ir.model.occurrences) {
        for occurrence in &ir.model.occurrences {
            let PrototypeReference::Local { definition } = &occurrence.prototype else { continue };
            let Some(product) = ir.model.product_definitions.iter().find(|p| &p.id == definition) else { continue };
            let placement = graph.resolved_transform(&occurrence.id).map_or(Matrix4::identity(), |t| matrix(&t));
            for body in &product.bodies {
                uses.entry(body).or_default().push(placement);
            }
        }
    }
    let mut out = Vec::new();
    for (body, imported) in ir.model.bodies.iter().zip(bodies) {
        let placements = uses.get(&body.id).cloned().unwrap_or_else(|| vec![Matrix4::identity()]);
        for shell in body_shells(imported) {
            out.extend(placements.iter().map(|&m| (shell.clone(), m)));
        }
    }
    Ok(out)
}

fn shell_size<C, S>(shell: &CompressedShell<Point3, C, S>) -> f64 {
    let bdd: BoundingBox<Point3> = shell.vertices.iter().collect();
    bdd.diameter()
}

fn tessellate<C, S>(shells: &[(CompressedShell<Point3, C, S>, Matrix4)], relative: f64, unit: f64) -> Result<Mesh3d, String>
where
    CompressedShell<Point3, C, S>: RobustMeshableShape,
    <CompressedShell<Point3, C, S> as RobustMeshableShape>::MeshedShape: MeshedShape,
{
    let mut b = MeshBuilder::new();
    for (shell, matrix) in shells {
        let tolerance = f64::max(shell_size(shell) * relative, TOLERANCE);
        let mesh = shell.robust_triangulation(tolerance).to_polygon();
        let pos: Vec<[f32; 3]> = mesh
            .positions()
            .iter()
            .map(|p| {
                let q = matrix.transform_point(*p);
                [(q.x * unit) as f32, (q.y * unit) as f32, (q.z * unit) as f32]
            })
            .collect();
        for face in mesh.face_iter() {
            for k in 1..face.len().saturating_sub(1) {
                b.triangle(pos[face[0].pos], pos[face[k].pos], pos[face[k + 1].pos])?;
            }
        }
    }
    b.finish()
}

/// Tessellates placed shells into a mesh in mm (`unit`: mm per model unit). Fine
/// details are tessellated more coarsely if the mesh would exceed [`MAX_TRIANGLES`].
pub(crate) fn mesh_shells<C, S>(shells: &[(CompressedShell<Point3, C, S>, Matrix4)], unit: f64) -> Result<Mesh3d, String>
where
    CompressedShell<Point3, C, S>: RobustMeshableShape,
    <CompressedShell<Point3, C, S> as RobustMeshableShape>::MeshedShape: MeshedShape,
{
    let mut last = String::new();
    for relative in TOLERANCES {
        match tessellate(shells, relative, unit) {
            Ok(mesh) => return Ok(mesh),
            Err(e) if e.contains(&MAX_TRIANGLES.to_string()) => last = e,
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

/// Reads a STEP file into a mesh in mm. Curves (wire bodies) are left out.
pub fn parse_step(bytes: &[u8]) -> Result<Mesh3d, String> {
    let mut ir = monstertruck_io::cadmpeg::step::decode_to_ir(&mut std::io::Cursor::new(bytes))
        .map_err(|e| format!("not a readable STEP file: {e}"))?;
    ir.model.bodies.retain(|b| matches!(b.kind, BodyKind::Solid | BodyKind::Sheet));
    if ir.model.bodies.is_empty() {
        return Err("the STEP file contains no solids or surface models".into());
    }
    // cadmpeg converts lengths to millimetres while decoding.
    mesh_shells(&placed_shells(&ir)?, 1.0)
}

#[cfg(test)]
mod tests;
