//! Fallback fonts of the operating system for scripts the bundled fonts do not cover
//! (Chinese, Japanese, Korean, …): project texts in these scripts would otherwise show
//! as empty boxes on the desktop. The bundled fonts stay first; the system fonts are
//! only used for glyphs they lack. In the browser the files cannot be read and nothing
//! changes (the browser has its own fallback).

use egui::epaint::text::{FontData, FontInsert, FontPriority, InsertFontFamily};
use egui::FontFamily;

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

/// Adds fonts as fallbacks of the proportional and monospace fonts.
pub fn install(ctx: &egui::Context, fonts: &[(String, Vec<u8>)]) {
    for (path, bytes) in fonts {
        let families = [FontFamily::Proportional, FontFamily::Monospace].into_iter().map(|family| InsertFontFamily { family, priority: FontPriority::Lowest }).collect();
        ctx.add_font(FontInsert { name: format!("system:{path}"), data: FontData::from_owned(bytes.clone()), families });
    }
}

/// Adds the system fonts found as fallbacks of the proportional and monospace fonts;
/// returns how many were added.
pub fn install_system_fallbacks(ctx: &egui::Context) -> usize {
    let found = system_fallbacks();
    install(ctx, &found);
    found.len()
}

#[cfg(test)]
mod tests;
