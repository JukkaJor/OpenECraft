//! Files of the editor. Dialogs, the browser file picker and downloads are
//! implemented in `openecraft_ui::io`.

pub use openecraft_ui::io::short_name;
use openecraft_i18n::tr;
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
    /// Macro library (JSON) to merge.
    Macros,
    /// PLC symbol table (CSV).
    PlcTable,
    /// Symbol library (JSON) to merge into the project.
    Symbols,
    /// Title blocks (JSON) to add to the project.
    Forms,
    /// A drawing to import as a page.
    Dxf,
    /// A table (CSV) changed in a spreadsheet.
    Table(crate::tables::TableKind),
    /// Translation dictionary (CSV) of the project texts.
    Dictionary,
    /// Another project that gets report pages of this one.
    OutputTarget,
}

impl FileKind for Purpose {
    fn filter(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Purpose::Project | Purpose::OutputTarget => (tr("OpenECraft project"), &[PROJECT_EXT, "json"]),
            Purpose::Parts => (tr("Parts (CSV, JSON, BMEcat)"), &["csv", "json", "xml", "txt"]),
            Purpose::Model(kind) => kind.filter(),
            Purpose::Macros => (tr("OpenECraft macros (JSON)"), &["json"]),
            Purpose::PlcTable => ("PLC symbol table (CSV)", &["csv", "txt"]),
            Purpose::Symbols => (tr("OpenECraft symbol library (JSON)"), &["json"]),
            Purpose::Forms => (tr("OpenECraft title blocks and report schemes (JSON)"), &["json"]),
            Purpose::Table(_) | Purpose::Dictionary => ("CSV", &["csv", "txt"]),
            Purpose::Dxf => (tr("DXF drawing"), &["dxf"]),
        }
    }
}

pub type FileIo = openecraft_ui::io::FileIo<Purpose>;
pub type Loaded = openecraft_ui::io::Loaded<Purpose>;

pub const PROJECT_EXT: &str = "oec";
