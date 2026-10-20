//! Files of the editor. Dialogs, the browser file picker and downloads are
//! implemented in `openecraft_ui::io`.

pub use openecraft_ui::io::short_name;
use openecraft_ui::io::FileKind;
use openecraft_ui::part_editor::ModelKind;

/// What a file is opened for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    Project,
    /// Import into the parts database.
    Parts,
    /// 2D / 3D model of a part.
    Model(ModelKind),
}

impl FileKind for Purpose {
    fn filter(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Purpose::Project => ("OpenECraft project", &[PROJECT_EXT, "json"]),
            Purpose::Parts => ("Parts (CSV, JSON)", &["csv", "json", "txt"]),
            Purpose::Model(kind) => kind.filter(),
        }
    }
}

pub type FileIo = openecraft_ui::io::FileIo<Purpose>;
pub type Loaded = openecraft_ui::io::Loaded<Purpose>;

pub const PROJECT_EXT: &str = "oec";
