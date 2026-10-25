//! Fonts for scripts the default egui fonts do not cover. The bundled Chinese,
//! Japanese and Korean fonts (`openecraft_fonts`) hold the characters of the
//! translations; fallback fonts of the operating system cover project texts in these
//! scripts, which would otherwise show as empty boxes on the desktop. The default fonts
//! stay first; the others are only used for glyphs they lack, in the order of
//! `openecraft_fonts::fallback_order`. In the browser the system files cannot be read.

use egui::epaint::text::{FontData, FontDefinitions};
use egui::FontFamily;
use openecraft_fonts::Fallback;
use openecraft_i18n::Lang;
use std::sync::{Arc, OnceLock};

/// Font files tried in this order (Windows, macOS, Linux).
pub const CANDIDATES: &[&str] = &[
    "C:/Windows/Fonts/msyh.ttc",
    "C:/Windows/Fonts/YuGothR.ttc",
    "C:/Windows/Fonts/malgun.ttf",
    "C:/Windows/Fonts/simsun.ttc",
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "/System/Library/Fonts/AppleSDGothicNeo.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
    "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
];

/// Most fallback fonts loaded (they are large).
pub const MAX_FALLBACKS: usize = 2;

/// The first `max` fonts `read` can load: (path, file content).
pub fn load_first(paths: &[&str], max: usize, read: impl Fn(&str) -> Option<Vec<u8>>) -> Vec<(String, Vec<u8>)> {
    paths.iter().filter_map(|p| read(p).filter(|b| b.len() > 1024).map(|b| (p.to_string(), b))).take(max).collect()
}

/// The system fallback fonts found: (path, file content).
pub fn system_fallbacks() -> Vec<(String, Vec<u8>)> {
    load_first(CANDIDATES, MAX_FALLBACKS, |p| std::fs::read(p).ok())
}

/// The default fonts followed by the bundled and the `system` fonts for the UI language
/// `lang`, for the proportional and the monospace family.
pub fn definitions(lang: Lang, system: &[(String, Arc<FontData>)]) -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    let mut names = Vec::new();
    for font in openecraft_fonts::fallback_order(lang.code(), system) {
        let (name, data) = match font {
            Fallback::Bundled(f) => (format!("bundled:{}", f.name), Arc::new(FontData::from_static(f.data))),
            Fallback::System((path, data)) => (format!("system:{path}"), data.clone()),
        };
        defs.font_data.insert(name.clone(), data);
        names.push(name);
    }
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        defs.families.entry(family).or_default().extend(names.iter().cloned());
    }
    defs
}

/// The system fonts installed last, kept to reorder the fonts when the language changes.
static SYSTEM: OnceLock<Vec<(String, Arc<FontData>)>> = OnceLock::new();

/// Installs the bundled fonts and the `system` fonts as fallbacks of the default fonts.
pub fn install(ctx: &egui::Context, system: &[(String, Vec<u8>)]) {
    let system = SYSTEM.get_or_init(|| system.iter().map(|(p, b)| (p.clone(), Arc::new(FontData::from_owned(b.clone())))).collect());
    ctx.set_fonts(definitions(openecraft_i18n::lang(), system));
}

/// Puts the bundled font of the current language first (after a language change).
pub fn follow_language(ctx: &egui::Context) {
    ctx.set_fonts(definitions(openecraft_i18n::lang(), SYSTEM.get().map(Vec::as_slice).unwrap_or_default()));
}

/// Installs the bundled fonts and the system fonts found; returns how many system
/// fonts were added.
pub fn install_system_fallbacks(ctx: &egui::Context) -> usize {
    let found = system_fallbacks();
    install(ctx, &found);
    found.len()
}

#[cfg(test)]
mod tests;
