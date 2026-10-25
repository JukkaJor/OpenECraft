//! Language selection shared by the editor and the parts manager.

use openecraft_i18n::{tr, lang, set_lang, Lang};

/// Key under which the applications persist the language code.
pub const STORAGE_KEY: &str = "language";

/// Menu with all languages (each in its own name); returns true if the language changed.
pub fn menu(ui: &mut egui::Ui) -> bool {
    let before = lang();
    ui.menu_button(tr("Language"), |ui| {
        for l in Lang::ALL {
            if ui.selectable_label(lang() == l, l.native_name()).clicked() {
                set_lang(l);
            }
        }
    });
    let changed = lang() != before;
    if changed {
        crate::fonts::follow_language(ui.ctx());
    }
    changed
}
