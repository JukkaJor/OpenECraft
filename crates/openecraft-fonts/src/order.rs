//! The order in which the fonts after the default ones are tried.

use crate::{Bundled, CJK};

/// A font after the default ones.
#[derive(Debug, PartialEq)]
pub enum Fallback<T> {
    Bundled(&'static Bundled),
    System(T),
}

/// The fonts tried for characters the default fonts lack, for the UI language `lang`:
/// the bundled font of the language, so that all its texts use one font (Chinese and
/// Japanese share characters drawn differently); then the `system` fonts, complete
/// enough for project texts; then the other bundled fonts.
pub fn fallback_order<T>(lang: &str, system: impl IntoIterator<Item = T>) -> Vec<Fallback<T>> {
    let own = CJK.iter().filter(|f| f.lang == lang).map(Fallback::Bundled);
    let others = CJK.iter().filter(|f| f.lang != lang).map(Fallback::Bundled);
    own.chain(system.into_iter().map(Fallback::System)).chain(others).collect()
}

#[cfg(test)]
mod tests;
