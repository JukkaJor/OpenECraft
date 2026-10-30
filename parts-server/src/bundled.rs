//! Manufacturer catalogs shipped with OpenECraft ([`openecraft_parts::bundled`]):
//! installed into new databases and with `--install-catalogs`.

pub use openecraft_parts::bundled::{all, ids, load, Bundle};

/// User name in the history of parts installed from a bundled catalog.
pub const INSTALLER: &str = "bundled catalog";
