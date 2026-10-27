use crate::import::{body_shells, mesh_shells};
use cadmpeg_ir::codec::{CodecEntry, DecodeOptions};
use cadmpeg_ir::topology::BodyKind;
use monstertruck_io::cadmpeg::to_bodies;
use monstertruck_meshing::prelude::*;
use openecraft_parts::Mesh3d;

/// Reads an IGES 5.3 file (solids, trimmed and untrimmed surfaces) into a mesh in mm.
/// Curves (wire bodies) are left out.
pub fn parse_iges(bytes: &[u8]) -> Result<Mesh3d, String> {
    let decoded = cadmpeg_codec_iges::IgesCodec
        .decode(&mut std::io::Cursor::new(bytes), &DecodeOptions::default())
        .map_err(|e| format!("not a readable IGES file: {e}"))?;
    let mut ir = decoded.ir;
    ir.model.bodies.retain(|b| matches!(b.kind, BodyKind::Solid | BodyKind::Sheet));
    if ir.model.bodies.is_empty() {
        return Err("the IGES file contains no solids or surfaces".into());
    }
    let bodies = to_bodies(&ir, "IGES").map_err(|e| format!("IGES file: {e}"))?;
    let shells: Vec<_> = bodies.into_iter().flat_map(body_shells).map(|shell| (shell, Matrix4::identity())).collect();
    // cadmpeg converts lengths to millimetres while decoding.
    mesh_shells(&shells, 1.0)
}

#[cfg(test)]
mod tests;
