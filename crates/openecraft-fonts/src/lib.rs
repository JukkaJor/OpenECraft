//! Fonts for the scripts the bundled egui fonts lack: subsets of Noto Sans SC, JP and
//! KR (SIL Open Font License, `fonts/OFL.txt`) with the characters of the Chinese,
//! Japanese and Korean translations, written by `subset.py`. They have TrueType
//! outlines, so the PDF export can embed them, too.

mod order;

pub use order::*;

/// A bundled font.
#[derive(Debug, PartialEq)]
pub struct Bundled {
    /// ISO code of the language the font is made for.
    pub lang: &'static str,
    pub name: &'static str,
    pub data: &'static [u8],
}

pub const CJK: [Bundled; 3] = [
    Bundled { lang: "zh", name: "Noto Sans SC", data: include_bytes!("../fonts/NotoSansSC-Subset.ttf") },
    Bundled { lang: "ja", name: "Noto Sans JP", data: include_bytes!("../fonts/NotoSansJP-Subset.ttf") },
    Bundled { lang: "ko", name: "Noto Sans KR", data: include_bytes!("../fonts/NotoSansKR-Subset.ttf") },
];
