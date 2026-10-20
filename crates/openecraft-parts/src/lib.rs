//! Shared parts model of OpenECraft, used by the editor and by the parts server.
//!
//! - [`Part`], [`FunctionTemplate`], [`PartsDb`] … – the part data (EPLAN style
//!   product groups, commercial / technical / mounting data, function templates)
//! - CSV / JSON import and export ([`to_csv`], [`from_csv`], [`import`])
//! - 2D / 3D models of parts with SVG, STL and OBJ import ([`Model2d`], [`Mesh3d`])
//! - the generic example catalog ([`default_catalog`])
//! - the HTTP API types of the parts server ([`api`])

pub mod api;
mod catalog;
mod csv;
mod graphics;
mod part;
mod text;
pub mod vec3;

pub use catalog::default_catalog;
pub use csv::{from_csv, import, to_csv};
pub use graphics::{parse_mesh, parse_obj, parse_stl, parse_svg, Mesh3d, Model2d, Prim2d, MAX_TRIANGLES};
pub use part::*;
pub use text::natural_cmp;
