//! Parasolid XT exchange: triangle meshes written as faceted B-rep bodies in the
//! transmit text format (`.x_t`) of the public XT format reference, schema V13
//! (`SCH_1300000_13006`), which every newer Parasolid version reads.
//!
//! Closed, consistently connected components of a mesh become solid bodies whose
//! coplanar neighbouring triangles are merged into one planar face (a box is six
//! faces); the remaining triangles become one sheet body of separate faces. Lengths
//! are written in metres, the Parasolid unit; reading XT files is
//! `openecraft_step::parse_parasolid`.

mod write;

pub use write::{write_xt, Tri};
