//! EDZ import of OpenECraft: EPLAN Data Portal archives. An EDZ file is a 7-Zip
//! archive with a `manifest.xml` and the items of its packages below `items/<type>/`:
//! EPLAN parts XML (part, connection points, drilling pattern, manufacturer), pictures
//! and macros. The parts are read with [`openecraft_parts::from_eplan_documents`];
//! pictures and macros are returned as files of their parts. EPLAN macros (`.ema`)
//! cannot be shown and are kept as they are.

mod archive;

pub use archive::{import, is_edz, read, Edz, EdzFile};
