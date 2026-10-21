//! Multi-language support: the UI texts are written in English in the source and
//! translated at run time through [`tr`] / [`tr_args`]. English, German, French and
//! Spanish are mandatory (see `AGENTS.md`); every text passed to `tr` must have an
//! entry in the catalog (`catalog/*.tsv`), which `translate/tests.rs` verifies.

mod catalog;
mod translate;

pub use translate::*;
