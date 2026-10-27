//! Configurator window: options with their choices, rules that insert page macros
//! (variant, placeholder values, condition) and generating the pages.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use openecraft_core::config::{self, ConfigOption, Rule};
use openecraft_core::macros::MacroKind;
use egui::RichText;

#[derive(Default)]
pub struct ConfigUi {
    pub open: bool,
}

impl App {
    pub(super) fn config_pane(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let page_macros: Vec<(String, Vec<String>)> =
            self.macro_lib.macros.iter().filter(|m| m.kind == MacroKind::Page).map(|m| (m.name.clone(), m.variant_names())).collect();
        let problems = config::problems(&self.session.prj.configuration, &self.macro_lib);
        let generated = self.session.prj.pages.iter().filter(|p| p.generated).count();
        let mut generate = false;
        let cfg = &mut self.session.prj.configuration;
        ui.scope(|ui| {
            ui.weak(tr("Rules insert page macros when their condition holds. Generating replaces the pages generated before; option values fill {OPTION} placeholders."));
            ui.separator();
            ui.label(RichText::new(tr("Options")).strong());
            let mut remove_option = None;
            egui::Grid::new("cfgoptions").striped(true).num_columns(4).spacing([10.0, 4.0]).show(ui, |ui| {
                for h in [tr("Name"), tr("Choices (separated by commas)"), tr("Value"), ""] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for (i, o) in cfg.options.iter_mut().enumerate() {
                    ui.add(egui::TextEdit::singleline(&mut o.name).desired_width(110.0));
                    let mut choices = o.choices.join(", ");
                    if ui.add(egui::TextEdit::singleline(&mut choices).desired_width(220.0)).changed() {
                        o.choices = choices.split(',').map(|c| c.trim().to_string()).filter(|c| !c.is_empty()).collect();
                    }
                    if o.choices.is_empty() {
                        ui.add(egui::TextEdit::singleline(&mut o.value).desired_width(120.0));
                    } else {
                        egui::ComboBox::from_id_salt(("cfgval", i)).selected_text(o.value.clone()).show_ui(ui, |ui| {
                            for c in &o.choices {
                                ui.selectable_value(&mut o.value, c.clone(), c);
                            }
                        });
                    }
                    if ui.small_button("🗑").clicked() {
                        remove_option = Some(i);
                    }
                    ui.end_row();
                }
            });
            if let Some(i) = remove_option {
                cfg.options.remove(i);
            }
            if ui.button(tr("➕ Option")).clicked() {
                cfg.options.push(ConfigOption { name: format!("OPTION{}", cfg.options.len() + 1), ..Default::default() });
            }
            ui.separator();
            ui.label(RichText::new(tr("Rules")).strong());
            let mut remove_rule = None;
            egui::Grid::new("cfgrules").striped(true).num_columns(6).spacing([10.0, 4.0]).show(ui, |ui| {
                for h in ["", tr("Page macro"), tr("Variant"), tr("Values (KEY=value; …)"), tr("Condition (OPTION=value & …)"), ""] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for (i, r) in cfg.rules.iter_mut().enumerate() {
                    ui.label(format!("{}", i + 1));
                    egui::ComboBox::from_id_salt(("cfgmacro", i)).selected_text(r.macro_name.clone()).width(150.0).show_ui(ui, |ui| {
                        for (name, _) in &page_macros {
                            ui.selectable_value(&mut r.macro_name, name.clone(), name);
                        }
                    });
                    let variants = page_macros.iter().find(|m| m.0 == r.macro_name).map(|m| m.1.clone()).unwrap_or_default();
                    egui::ComboBox::from_id_salt(("cfgvar", i)).selected_text(if r.variant.is_empty() { variants.first().cloned().unwrap_or_default() } else { r.variant.clone() }).width(50.0).show_ui(ui, |ui| {
                        for (vi, v) in variants.iter().enumerate() {
                            let value = if vi == 0 { String::new() } else { v.clone() };
                            ui.selectable_value(&mut r.variant, value, v);
                        }
                    });
                    ui.add(egui::TextEdit::singleline(&mut r.values).hint_text(tr("e.g. NAME=Pump; POWER={POWER}")).desired_width(220.0));
                    ui.add(egui::TextEdit::singleline(&mut r.condition).hint_text(tr("e.g. STARTER=DOL")).desired_width(170.0));
                    if ui.small_button("🗑").clicked() {
                        remove_rule = Some(i);
                    }
                    ui.end_row();
                }
            });
            if let Some(i) = remove_rule {
                cfg.rules.remove(i);
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(!page_macros.is_empty(), egui::Button::new(tr("➕ Rule"))).clicked() {
                    cfg.rules.push(Rule { macro_name: page_macros[0].0.clone(), ..Default::default() });
                }
                if page_macros.is_empty() {
                    ui.weak(tr("The macro library has no page macros (Macros › Save page as macro…)."));
                }
            });
            ui.separator();
            for p in &problems {
                ui.colored_label(crate::style::lock(), p);
            }
            ui.horizontal(|ui| {
                generate = ui.add_enabled(problems.is_empty() && !cfg.rules.is_empty(), egui::Button::new(tr("⚙ Generate pages"))).clicked();
                ui.weak(tr_fmt!("{} generated pages in the project (changes on them are replaced)", generated));
            });
        });
        if generate {
            match config::generate(&mut self.session.prj, &self.macro_lib) {
                Ok(g) => self.notify(ctx, tr_fmt!("{} pages removed, {} pages generated", g.removed, g.pages.len())),
                Err(e) => self.notify(ctx, tr_fmt!("Generation failed: {}", e)),
            }
            self.session.page = self.session.page.min(self.session.prj.pages.len() - 1);
            self.editor.selection.clear();
        }
    }
}
