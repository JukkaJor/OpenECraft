//! egui components shared by the OpenECraft applications (editor and parts manager).
//!
//! - [`part_editor`] – part tree, tabbed part editor, 2D / 3D model previews
//! - [`client`] – client of the parts server (`openecraft-server`)
//! - [`io`] – file dialogs on desktop, file picker and downloads in the browser

pub mod client;
pub mod io;
pub mod part_editor;
