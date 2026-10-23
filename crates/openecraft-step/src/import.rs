use monstertruck_assembly::assy::{EdgeEntity, NodeEntity};
use monstertruck_io::step::load::step_geometry::{Curve3D, Surface};
use monstertruck_io::step::load::{convert::*, *};
use monstertruck_meshing::prelude::*;
use monstertruck_topology::compress::CompressedShell;
use openecraft_parts::{Mesh3d, MeshBuilder, MAX_TRIANGLES};

type Shell = CompressedShell<Point3, Curve3D, Surface>;

/// Chord tolerance of the tessellation relative to the size of a shell; coarser
/// steps are tried when the mesh gets too large.
const TOLERANCES: [f64; 4] = [0.002, 0.006, 0.02, 0.06];

/// Millimetres per length unit of a STEP file (`SI_UNIT(.MILLI.,.METRE.)`,
/// `SI_UNIT($,.METRE.)`, inch, foot …); 1 if it declares none.
pub fn length_unit_mm(text: &str) -> f64 {
    let upper = text.to_ascii_uppercase();
    for record in upper.split(';').filter(|r| r.contains("LENGTH_UNIT")) {
        let compact: String = record.chars().filter(|c| !c.is_whitespace()).collect();
        if compact.contains("CONVERSION_BASED_UNIT") {
            if compact.contains("'INCH'") {
                return 25.4;
            }
            if compact.contains("'FOOT'") {
                return 304.8;
            }
            continue;
        }
        if let Some(si) = compact.split("SI_UNIT(").nth(1) {
            let prefix = si.split(',').next().unwrap_or_default();
            return match prefix {
                ".MILLI." => 1.0,
                ".CENTI." => 10.0,
                ".DECI." => 100.0,
                ".MICRO." => 0.001,
                ".KILO." => 1_000_000.0,
                _ => 1000.0,
            };
        }
    }
    1.0
}

/// Shells of a solid or surface model representation.
fn shells_of(table: &Table, idx: u64) -> Vec<Shell> {
    if let Some(solid) = table.manifold_solid_brep.get(&idx) {
        table.to_compressed_solid(solid).map(|s| s.boundaries).unwrap_or_default()
    } else if let Some(model) = table.shell_based_surface_model.get(&idx) {
        table.to_compressed_shells(model).unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// All shells with their placement: through the assembly structure, or – for files
/// without one – every solid and surface model where it is.
fn placed_shells(table: &Table) -> Vec<(Shell, Matrix4)> {
    let mut out = Vec::new();
    if let Ok(assy) = table.step_assy() {
        let shapes = assy.par_map(
            |node: &ProductEntity| NodeEntity { shape: node.shape.clone(), attrs: () },
            |edge: &AssembleEntity| EdgeEntity { matrix: Matrix4::try_from(&edge.matrix).unwrap_or(Matrix4::identity()), attrs: () },
        );
        for top in shapes.top_nodes() {
            for path in shapes.paths_iter(top.index()) {
                let matrix = path.matrix();
                for &idx in path.terminal_node().shape() {
                    out.extend(shells_of(table, idx).into_iter().map(|s| (s, matrix)));
                }
            }
        }
    }
    if out.is_empty() {
        let mut ids: Vec<u64> = table.manifold_solid_brep.keys().chain(table.shell_based_surface_model.keys()).copied().collect();
        ids.sort_unstable();
        for idx in ids {
            out.extend(shells_of(table, idx).into_iter().map(|s| (s, Matrix4::identity())));
        }
    }
    out
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

/// Reads a STEP file into a mesh in mm.
pub fn parse_step(bytes: &[u8]) -> Result<Mesh3d, String> {
    let table = Table::from_step_bytes(bytes).map_err(|e| format!("not a readable STEP file: {e}"))?;
    let unit = length_unit_mm(&String::from_utf8_lossy(bytes));
    let shells = placed_shells(&table);
    if shells.is_empty() {
        return Err("the STEP file contains no solids or surface models".into());
    }
    mesh_shells(&shells, unit)
}

#[cfg(test)]
mod tests;
