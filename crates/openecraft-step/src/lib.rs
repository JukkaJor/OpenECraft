//! STEP import (ISO 10303-21, AP203 / AP214 / AP242) and IGES import (5.3): the
//! solids and surface models of a file, placed by its assembly structure and
//! tessellated into a part mesh in millimetres. Reading and tessellating the B-rep
//! is done by `monstertruck` (IGES decoding by its `cadmpeg` backend).

mod iges;
mod import;

pub use iges::parse_iges;
pub use import::{length_unit_mm, parse_step};
