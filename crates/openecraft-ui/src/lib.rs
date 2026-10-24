//! egui components shared by the OpenECraft applications (editor and parts manager).
//!
//! - [`part_editor`] – part tree, tabbed part editor, 2D / 3D model previews
//! - [`client`] – client of the parts server (`openecraft-parts-server`)
//! - [`portal`] – data portal browser (faceted search, files, favorites, update check)
//! - [`io`] – file dialogs on desktop, file picker and downloads in the browser
//! - [`language`] – language menu (UI texts are translated with `openecraft-i18n`)

pub mod client;
pub mod io;
pub mod language;
pub mod part_editor;
pub mod portal;
