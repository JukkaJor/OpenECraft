//! Language state and the lookup / substitution functions (see the crate docs).

use crate::catalog;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

/// A supported UI language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    De,
    Fr,
    Es,
}

impl Lang {
    pub const ALL: [Lang; 4] = [Lang::En, Lang::De, Lang::Fr, Lang::Es];

    /// ISO 639-1 code, also used for persisting the choice.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::Es => "es",
        }
    }

    /// Name of the language in the language itself.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::Es => "Español",
        }
    }

    /// Language for an ISO code or a locale such as `de_DE.UTF-8` / `fr-CA`.
    pub fn from_code(code: &str) -> Option<Lang> {
        let primary = code.trim().split(['_', '-', '.', '@']).next()?.to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| l.code() == primary)
    }

    fn index(self) -> u8 {
        self as u8
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn lang() -> Lang {
    Lang::ALL[CURRENT.load(Ordering::Relaxed) as usize % Lang::ALL.len()]
}

pub fn set_lang(lang: Lang) {
    CURRENT.store(lang.index(), Ordering::Relaxed);
}

/// Language of the operating system (`LC_ALL`, `LC_MESSAGES`, `LANG`), English if unknown.
pub fn system_lang() -> Lang {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .find_map(|v| Lang::from_code(&v))
        .unwrap_or(Lang::En)
}

/// Start-up language: the stored choice (ISO code), otherwise the language of the
/// operating system if it is supported; a language already set by the platform entry
/// point (browser language) is kept.
pub fn init(stored: Option<&str>) {
    if let Some(l) = stored.and_then(Lang::from_code) {
        set_lang(l);
    } else if system_lang() != Lang::En {
        set_lang(system_lang());
    }
}

/// All catalog entries as `[English, German, French, Spanish]` (for tests of the applications).
pub fn catalog_entries() -> Vec<[&'static str; 4]> {
    catalog::all().map(|(en, de, fr, es)| [en, de, fr, es]).collect()
}

fn table() -> &'static HashMap<&'static str, [&'static str; 3]> {
    static TABLE: OnceLock<HashMap<&'static str, [&'static str; 3]>> = OnceLock::new();
    TABLE.get_or_init(|| catalog::all().map(|(en, de, fr, es)| (en, [de, fr, es])).collect())
}

/// Translation of the English text `en` into the current language; unknown texts are
/// returned unchanged.
pub fn tr(en: &'static str) -> &'static str {
    translate(en, lang())
}

pub fn translate(en: &'static str, lang: Lang) -> &'static str {
    match lang {
        Lang::En => en,
        _ => table().get(en).map(|t| t[lang.index() as usize - 1]).unwrap_or(en),
    }
}

/// Like [`tr`] for texts with placeholders: `{}` is replaced by the arguments in order,
/// `{0}`, `{1}`, … by the argument with that index (translations may reorder them).
pub fn tr_args(en: &'static str, args: &[&dyn Display]) -> String {
    fill(translate(en, lang()), args)
}

/// `tr_fmt!("Saved {}", name)` – translated text with `{}` placeholders filled by the
/// arguments (anything that implements `Display`).
#[macro_export]
macro_rules! tr_fmt {
    ($text:literal $(, $arg:expr)* $(,)?) => {
        $crate::tr_args($text, &[$(&$arg as &dyn ::std::fmt::Display),*])
    };
}

/// Marks an English text in a `const` / `static` table as translatable (the text is
/// translated with [`tr`] where it is shown); keeps it visible to the catalog test.
#[macro_export]
macro_rules! tr_const {
    ($text:literal) => {
        $text
    };
}

fn fill(text: &str, args: &[&dyn Display]) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut next = 0;
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}') else { break };
        let key = &rest[start + 1..start + end];
        let index = if key.is_empty() {
            next += 1;
            Some(next - 1)
        } else {
            key.parse::<usize>().ok()
        };
        out.push_str(&rest[..start]);
        match index.and_then(|i| args.get(i)) {
            Some(arg) => out.push_str(&arg.to_string()),
            None => out.push_str(&rest[start..start + end + 1]),
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests;
