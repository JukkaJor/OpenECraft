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
    /// PLC hardware configuration (AutomationML).
    Aml,
    /// Device descriptions of PLC hardware (GSDML, ESI).
    DeviceDescription,
    /// Structure template (JSON) to apply to the project.
    StructureTemplate,
    /// A file to embed as a document page.
    Document,
    /// Page templates (JSON) to add to the project.
    PageTemplates,
}

impl FileKind for Purpose {
    fn filter(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Purpose::Project => (tr("OpenECraft project"), &[PROJECT_EXT, "json"]),
            Purpose::Parts => (tr("Parts (CSV, JSON, BMEcat)"), &["csv", "json", "xml", "txt"]),
            Purpose::Model(kind) => kind.filter(),
            Purpose::Macros => (tr("OpenECraft macros (JSON)"), &["json"]),
            Purpose::PlcTable => ("PLC symbol table (CSV)", &["csv", "txt"]),
            Purpose::Symbols => (tr("OpenECraft symbol library (JSON)"), &["json"]),
            Purpose::Forms => (tr("OpenECraft title blocks (JSON)"), &["json"]),
            Purpose::StructureTemplate => (tr("OpenECraft structure template (JSON)"), &["json"]),
            Purpose::PageTemplates => (tr("OpenECraft page templates (JSON)"), &["json"]),
            Purpose::Document => (tr("Documents"), &["pdf", "docx", "doc", "xlsx", "xls", "pptx", "odt", "ods", "png", "jpg", "jpeg", "txt", "*"]),
            Purpose::Table(_) | Purpose::Dictionary => ("CSV", &["csv", "txt"]),
            Purpose::Dxf => (tr("DXF drawing"), &["dxf"]),
            Purpose::Aml => ("AutomationML", &["aml", "xml"]),
            Purpose::DeviceDescription => (tr("Device description (GSDML, ESI)"), &["xml"]),
        }
    }
}

pub type FileIo = openecraft_ui::io::FileIo<Purpose>;
pub type Loaded = openecraft_ui::io::Loaded<Purpose>;

pub const PROJECT_EXT: &str = "oec";
