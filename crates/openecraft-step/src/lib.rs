//! STEP import (ISO 10303-21, AP203 / AP214 / AP242): the solids and surface models
//! of a STEP file, placed by its assembly structure and tessellated into a part mesh
//! in millimetres. Reading and tessellating the B-rep is done by `monstertruck`.

mod import;

pub use import::{length_unit_mm, parse_step};
