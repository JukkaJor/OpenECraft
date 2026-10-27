//! Project language tools: source and target languages of the project texts and the
//! translation dictionary. Output per language: `export` with `language`.

use super::{optional, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::translation::{self};
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::Value;
use openecraft_api::builtin::languages::{list};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "import_dictionary",
        help: tr_const!("Reads translations from CSV: first column the source line, one column per language code"),
        params: &[
            optional("path", Kind::Str, tr_const!("CSV file")),
            optional("csv", Kind::Str, tr_const!("CSV content instead of a file")),
        ],
        needs_project: true,
        run: import_dictionary,
    },
];

fn import_dictionary(s: &mut Session, a: &Args) -> Result<Value, String> {
    let text = match (a.str("csv"), a.str("path")) {
        (Some(csv), _) => csv.to_string(),
        (None, Some(path)) => std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, openecraft_i18n::io_error(&e)))?,
        (None, None) => return Err(tr_fmt!("Missing parameter: {}", "path")),
    };
    let n = translation::from_csv(s.project_mut()?, &text)?;
    let mut v = list(s.project()?)?;
    v["imported"] = n.into();
    Ok(v)
}

#[cfg(test)]
mod tests;
