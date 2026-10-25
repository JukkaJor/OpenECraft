//! Configurator tools: options, rules and generating pages from a macro library file.

use super::{optional, param, Args, Kind, Tool};
use crate::session::Session;
use openecraft_core::config::{self};
use openecraft_core::preplanning;
use openecraft_core::macros::MacroLibrary;
use openecraft_i18n::{tr_const, tr_fmt};
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "generate_pages",
        help: tr_const!("Replaces the pages generated before by the page macros of the rules whose condition holds"),
        params: &[param("macros", Kind::Str, tr_const!("Macro library file (JSON, exported by the editor)"))],
        needs_project: true,
        run: generate_pages,
    },
    Tool {
        name: "export_check_scheme",
        help: tr_const!("Writes the check scheme of the project (checks switched off, message levels) to a file"),
        params: &[param("path", Kind::Str, tr_const!("Output file (JSON)"))],
        needs_project: true,
        run: export_check_scheme,
    },
    Tool {
        name: "import_check_scheme",
        help: tr_const!("Takes the checks switched off and the message levels of a check scheme file"),
        params: &[param("path", Kind::Str, tr_const!("Check scheme file (JSON)"))],
        needs_project: true,
        run: import_check_scheme,
    },
    Tool {
        name: "update_macro_boxes",
        help: tr_const!("Replaces the content of the macro boxes whose macro changed in a macro library file"),
        params: &[param("macros", Kind::Str, tr_const!("Macro library file (JSON, exported by the editor)"))],
        needs_project: true,
        run: update_macro_boxes,
    },
    Tool {
        name: "import_preplanning",
        help: tr_const!("Imports pre-planning segments and planned devices from CSV or Excel (columns Segment, Tag, Kind, Description, Power (kW), Voltage, Part, Macro)"),
        params: &[param("path", Kind::Str, tr_const!("CSV or Excel file"))],
        needs_project: true,
        run: import_preplanning,
    },
    Tool {
        name: "generate_preplanning_pages",
        help: tr_const!("Generates a schematic page per planned device that has a page macro and is not in the schematic yet"),
        params: &[
            param("macros", Kind::Str, tr_const!("Macro library file (JSON, exported by the editor)")),
            optional("segment", Kind::Str, tr_const!("Segment path (default: all segments)")),
        ],
        needs_project: true,
        run: generate_preplanning_pages,
    },
];

fn export_check_scheme(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("path");
    std::fs::write(path, openecraft_core::checks::scheme_json(s.project()?)).map_err(|e| tr_fmt!("Cannot write {}: {}", path, e))?;
    Ok(json!({ "path": path }))
}

fn import_check_scheme(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("path");
    let text = std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let prj = s.project_mut()?;
    openecraft_core::checks::apply_scheme(prj, &text)?;
    Ok(json!({ "disabled": prj.check_settings.disabled, "levels": prj.check_settings.levels.len() }))
}

fn update_macro_boxes(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("macros");
    let text = std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let lib = MacroLibrary::from_json(&text).map_err(|e| tr_fmt!("Not a macro file: {}", e))?;
    let r = openecraft_core::macro_boxes::update(s.project_mut()?, &lib);
    Ok(json!({ "updated": r.updated, "locked": r.locked, "macros_not_found": r.missing }))
}

fn import_preplanning(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("path");
    let bytes = std::fs::read(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let rows = if bytes.starts_with(b"PK") {
        openecraft_core::xlsx::read(&bytes)?.into_iter().next().map(|(_, rows)| rows).unwrap_or_default()
    } else {
        openecraft_core::xlsx::rows_of_csv(String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}'))
    };
    let (segments, added, updated) = preplanning::import_rows(s.project_mut()?, &rows)?;
    Ok(json!({ "segments_created": segments, "devices_added": added, "devices_updated": updated }))
}

fn generate_preplanning_pages(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("macros");
    let text = std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let lib = MacroLibrary::from_json(&text).map_err(|e| tr_fmt!("Not a macro file: {}", e))?;
    let prj = s.project_mut()?;
    let roots: Vec<u64> = match a.str("segment") {
        Some(p) => {
            let want = p.trim().trim_matches('/');
            vec![prj.preplanning.segments.iter().map(|x| x.id).find(|id| preplanning::path(prj, *id) == want).ok_or_else(|| tr_fmt!("No segment {}", p))?]
        }
        None => prj.preplanning.segments.iter().filter(|x| x.parent == 0).map(|x| x.id).collect(),
    };
    let (mut pages, mut missing) = (Vec::new(), Vec::new());
    for r in roots {
        let (p, m) = preplanning::generate(prj, r, &lib);
        pages.extend(p);
        missing.extend(m.into_iter().filter(|x| !missing.contains(x)).collect::<Vec<_>>());
    }
    Ok(json!({ "pages": pages, "macros_not_found": missing }))
}

fn generate_pages(s: &mut Session, a: &Args) -> Result<Value, String> {
    let path = a.req("macros");
    let text = std::fs::read_to_string(path).map_err(|e| tr_fmt!("Cannot read {}: {}", path, e))?;
    let lib = MacroLibrary::from_json(&text).map_err(|e| tr_fmt!("Not a macro file: {}", e))?;
    let g = config::generate(s.project_mut()?, &lib)?;
    Ok(json!({ "removed": g.removed, "pages": g.pages }))
}

#[cfg(test)]
mod tests;
