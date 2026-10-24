//! Manufacturer catalogs shipped with the server (`parts-server/catalogs/<id>.json`):
//! installed into new databases and with `--install-catalogs`.

use openecraft_parts::portal::Catalog;
use openecraft_parts::{add_generated_models, Part};
use serde::Deserialize;

/// User name in the history of parts installed from a bundled catalog.
pub const INSTALLER: &str = "bundled catalog";

/// Catalog id and file content, one entry per file in `catalogs/`.
const FILES: &[(&str, &str)] = &[
    ("abb", include_str!("../catalogs/abb.json")),
    ("beckhoff", include_str!("../catalogs/beckhoff.json")),
    ("hager", include_str!("../catalogs/hager.json")),
    ("mdt", include_str!("../catalogs/mdt.json")),
    ("phoenix_contact", include_str!("../catalogs/phoenix_contact.json")),
    ("siemens", include_str!("../catalogs/siemens.json")),
    ("weidmueller", include_str!("../catalogs/weidmueller.json")),
];

/// A catalog with its parts.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Bundle {
    pub catalog: Catalog,
    pub parts: Vec<Part>,
}

pub fn ids() -> Vec<&'static str> {
    FILES.iter().map(|(id, _)| *id).collect()
}

/// A bundled catalog; its parts refer to it and get generated models where they have none.
pub fn load(id: &str) -> Result<Bundle, String> {
    let (_, text) = FILES.iter().find(|(i, _)| i.eq_ignore_ascii_case(id)).ok_or_else(|| format!("unknown catalog {id} (bundled: {})", ids().join(", ")))?;
    let mut bundle: Bundle = serde_json::from_str(text).map_err(|e| format!("catalog {id}: {e}"))?;
    for p in &mut bundle.parts {
        p.catalog = bundle.catalog.id.clone();
        p.revision = 0;
        p.modified = 0;
        add_generated_models(p);
    }
    Ok(bundle)
}

pub fn all() -> Result<Vec<Bundle>, String> {
    ids().into_iter().map(load).collect()
}

#[cfg(test)]
mod tests;
