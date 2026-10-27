//! Project languages window: source and target languages of the project texts, the
//! translation dictionary (edit, collect, CSV for translators) and the export of the
//! documentation per language.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use crate::io::Purpose;
use openecraft_core::translation::{self, language_name, normalize_code};
use egui::RichText;

#[derive(Default)]
pub struct LanguagesUi {
    pub open: bool,
    new_lang: String,
    filter: String,
    /// Show only lines without a translation into this language.
    untranslated: Option<String>,
}

impl App {
    /// PDF of all pages with the project texts and generated texts in `lang`.
    pub(super) fn export_pdf_in(&mut self, ctx: &egui::Context, lang: &str) {
        if let Some(e) = self.plugins.exporter("pdf").cloned() {
            self.run_exporter(ctx, &e, Some(lang));
        }
    }

    pub(super) fn languages_pane(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let (mut collect, mut clean, mut export_csv, mut import_csv, mut pdf) = (false, false, false, false, None);
        let mut remove_lang = None;
        let missing: Vec<(String, usize)> = self.session.prj.languages.targets.iter().map(|l| (l.clone(), translation::missing(&self.session.prj, l).len())).collect();
        let ui_state = &mut self.languages_ui;
        let langs = &mut self.session.prj.languages;
        ui.scope(|ui| {
            ui.weak(tr("Project texts (function texts, page descriptions, free texts, descriptions) are written in the source language; the dictionary translates them line by line. Reports and title blocks follow the output language."));
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Source language"));
                let mut src = langs.source.clone();
                if ui.add(egui::TextEdit::singleline(&mut src).desired_width(50.0)).changed() {
                    if let Some(c) = normalize_code(&src) {
                        langs.source = c;
                    }
                }
                ui.separator();
                ui.label(tr("Target languages"));
                for (l, n) in &missing {
                    let text = if *n > 0 { tr_fmt!("{} ({} open)", language_name(l), n) } else { language_name(l) };
                    ui.label(RichText::new(text).strong());
                    if ui.small_button("✖").on_hover_text(tr("Remove the language (its translations stay in the dictionary)")).clicked() {
                        remove_lang = Some(l.clone());
                    }
                }
                ui.add(egui::TextEdit::singleline(&mut ui_state.new_lang).hint_text(tr("e.g. de, fr, it")).desired_width(90.0));
                let code = normalize_code(&ui_state.new_lang).filter(|c| *c != langs.source && !langs.targets.contains(c));
                if ui.add_enabled(code.is_some(), egui::Button::new(tr("➕ Add"))).on_hover_text(tr("ISO 639-1 language code")).clicked() {
                    langs.targets.extend(code);
                    ui_state.new_lang.clear();
                }
            });
            ui.horizontal_wrapped(|ui| {
                collect = ui.button(tr("Collect texts from the project")).on_hover_text(tr("Adds all lines of the project texts to the dictionary")).clicked();
                clean = ui.button(tr("Remove unused entries")).clicked();
                export_csv = ui.button(tr("Export CSV…")).on_hover_text(tr("Dictionary for translators: one column per language")).clicked();
                import_csv = ui.button(tr("Import CSV…")).clicked();
                ui.separator();
                for l in langs.all() {
                    if ui.button(tr_fmt!("PDF {}", l)).on_hover_text(tr_fmt!("Export all pages as PDF in {}", language_name(&l))).clicked() {
                        pdf = Some(l);
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(tr("Filter"));
                ui.add(egui::TextEdit::singleline(&mut ui_state.filter).desired_width(200.0));
                let shown = ui_state.untranslated.as_deref().map(language_name).unwrap_or_else(|| tr("all lines").to_string());
                egui::ComboBox::from_id_salt("untranslated").selected_text(shown).show_ui(ui, |ui| {
                    ui.selectable_value(&mut ui_state.untranslated, None, tr("all lines"));
                    for l in &langs.targets {
                        ui.selectable_value(&mut ui_state.untranslated, Some(l.clone()), tr_fmt!("not translated into {}", language_name(l)));
                    }
                });
            });
            if langs.targets.is_empty() {
                ui.weak(tr("Add a target language to translate the project texts."));
            } else if langs.dictionary.is_empty() {
                ui.weak(tr("The dictionary is empty: collect the texts from the project."));
            }
            let filter = ui_state.filter.to_lowercase();
            let targets = langs.targets.clone();
            let width = ((ui.available_width() - 40.0) / (targets.len() + 1) as f32).max(120.0);
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("dictionary").striped(true).num_columns(targets.len() + 1).show(ui, |ui| {
                    ui.label(RichText::new(language_name(&langs.source)).strong());
                    for l in &targets {
                        ui.label(RichText::new(language_name(l)).strong());
                    }
                    ui.end_row();
                    for (line, tr_map) in langs.dictionary.iter_mut() {
                        if !filter.is_empty() && !line.to_lowercase().contains(&filter) && !tr_map.values().any(|t| t.to_lowercase().contains(&filter)) {
                            continue;
                        }
                        if let Some(l) = &ui_state.untranslated {
                            if tr_map.get(l).is_some_and(|t| !t.trim().is_empty()) {
                                continue;
                            }
                        }
                        ui.add_sized([width, 18.0], egui::Label::new(line.as_str()).truncate());
                        for l in &targets {
                            let mut value = tr_map.get(l).cloned().unwrap_or_default();
                            if ui.add(egui::TextEdit::singleline(&mut value).desired_width(width)).changed() {
                                if value.is_empty() {
                                    tr_map.remove(l);
                                } else {
                                    tr_map.insert(l.clone(), value);
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
            });
        });
        if let Some(l) = remove_lang {
            self.session.prj.languages.targets.retain(|x| *x != l);
        }
        if collect {
            let n = translation::collect(&mut self.session.prj);
            self.notify(ctx, tr_fmt!("{} new texts in the dictionary", n));
        }
        if clean {
            let n = translation::remove_unused(&mut self.session.prj);
            self.notify(ctx, tr_fmt!("{} unused entries removed", n));
        }
        if export_csv {
            let csv = translation::to_csv(&self.session.prj);
            self.export(ctx, "dictionary.csv".into(), ("CSV", "csv"), csv);
        }
        if import_csv {
            self.io.open(ctx, Purpose::Dictionary);
        }
        if let Some(l) = pdf {
            self.export_pdf_in(ctx, &l);
        }
    }
}
