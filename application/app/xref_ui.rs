//! Cross reference settings: format, structure, interruption points, contact images.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use openecraft_core::model::{InterruptionMode, XrefStructure};
use egui::RichText;

pub fn structure_title(s: XrefStructure) -> &'static str {
    match s {
        XrefStructure::Different => tr("When it differs from the page"),
        XrefStructure::Always => tr("Always"),
        XrefStructure::Never => tr("Never"),
    }
}

pub fn interruption_title(m: InterruptionMode) -> &'static str {
    match m {
        InterruptionMode::All => tr("All other points of the name"),
        InterruptionMode::Star => tr("Star: the first point refers to all, the others to it"),
        InterruptionMode::Pairs => tr("Pairs in schematic order (1-2, 3-4 …)"),
    }
}

impl App {
    pub(super) fn xref_body(&mut self, ui: &mut egui::Ui) {
        let x = &mut self.session.prj.xref;
        ui.scope(|ui| {
            egui::Grid::new("xref").num_columns(2).show(ui, |ui| {
                ui.label(tr("Prefix"));
                ui.add(egui::TextEdit::singleline(&mut x.prefix).desired_width(40.0));
                ui.end_row();
                ui.label(tr("Separator"));
                ui.add(egui::TextEdit::singleline(&mut x.separator).desired_width(40.0));
                ui.end_row();
                ui.label(tr("Row"));
                ui.checkbox(&mut x.row, tr("Show the row after the column"));
                ui.end_row();
            });
            let sample = format!("{}3{}4{}", x.prefix, x.separator, if x.row { format!("{}B", x.separator) } else { String::new() });
            ui.label(RichText::new(tr_fmt!("Example: {}", sample)).monospace());
            ui.separator();
            ui.label(RichText::new(tr("Structure of the target page")).strong());
            for s in [XrefStructure::Different, XrefStructure::Always, XrefStructure::Never] {
                ui.radio_value(&mut x.structure, s, structure_title(s));
            }
            ui.separator();
            ui.label(RichText::new(tr("Interruption points refer to")).strong());
            for m in [InterruptionMode::All, InterruptionMode::Star, InterruptionMode::Pairs] {
                ui.radio_value(&mut x.interruptions, m, interruption_title(m));
            }
            ui.separator();
            let mut image = !x.hide_contact_image;
            if ui.checkbox(&mut image, tr("Contact image below coils")).changed() {
                x.hide_contact_image = !image;
            }
        });
    }
}
