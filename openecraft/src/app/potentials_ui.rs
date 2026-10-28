//! Potentials window: potential definitions with wire colour and cross-section, the
//! default of nets without a defined potential, presets. Fields of the wire
//! properties that fall back to these automatic values.

use openecraft_ui::icons::{self, Icon};
use openecraft_i18n::tr;
use super::App;
use openecraft_core::model::ReportKind;
use openecraft_core::potentials::{self, PotentialDef, PotentialKind, Preset, Source, WireProps};
use egui::{Color32, RichText};
use std::collections::HashMap;

/// Small colour swatch of a colour code (nothing for unknown codes).
fn swatch(ui: &mut egui::Ui, code: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    if let Some([r, g, b]) = potentials::color_rgb(code) {
        ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(r, g, b));
        if potentials::normalize_color(code) == "GNYE" {
            let half = egui::Rect::from_min_max(rect.center_top(), rect.right_bottom());
            ui.painter().rect_filled(half, 0.0, Color32::from_rgb(235, 205, 0));
        }
        ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0, ui.visuals().weak_text_color()), egui::StrokeKind::Inside);
    }
}

/// Colour code field with a menu of the IEC 60757 codes; `auto` is the colour used
/// while the field is empty.
fn color_field(ui: &mut egui::Ui, value: &mut String, auto: &str, hint: &str, width: f32) {
    swatch(ui, if value.is_empty() { auto } else { value });
    let resp = ui.add(egui::TextEdit::singleline(value).hint_text(hint).desired_width(width));
    if value.is_empty() && !hint.is_empty() {
        resp.on_hover_text(hint);
    }
    let r = icons::button(ui, Icon::ChevronDown, tr("Colour codes"));
    egui::Popup::menu(&r).show(|ui| {
        for (code, name, _) in potentials::COLORS {
            ui.horizontal(|ui| {
                swatch(ui, code);
                if ui.button(format!("{code}  {}", tr(name))).clicked() {
                    *value = code.to_string();
                    ui.close();
                }
            });
        }
    });
}

/// Colour and cross-section fields of the wire properties: empty fields show the
/// automatic value; ⟲ removes a manual value.
pub(super) fn wire_fields(ui: &mut egui::Ui, color: &mut String, cross_section: &mut String, auto: &WireProps) {
    let hint = |value: &str, source: &Source, fallback: &str| {
        let h = WireProps::hint(value, source);
        if h.is_empty() { fallback.to_string() } else { h }
    };
    ui.label(tr("Colour"));
    ui.horizontal(|ui| {
        let h = hint(&auto.color, &auto.color_source, tr("e.g. BU, BK, GNYE"));
        color_field(ui, color, &auto.color, &h, 120.0);
        if ui.add_enabled_ui(!color.is_empty(), |ui| icons::button(ui, Icon::Undo, tr("Automatic (from the potential definition)"))).inner.clicked() {
            color.clear();
        }
    });
    ui.end_row();
    ui.label(tr("Cross-section"));
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        let h = hint(&auto.cross_section, &auto.cross_section_source, tr("e.g. 1.5 mm²"));
        let resp = ui.add(egui::TextEdit::singleline(cross_section).hint_text(h.as_str()).desired_width(120.0));
        if cross_section.is_empty() {
            resp.on_hover_text(h);
        }
        if ui.add_enabled_ui(!cross_section.is_empty(), |ui| icons::button(ui, Icon::Undo, tr("Automatic (from the potential definition)"))).inner
            .clicked()
        {
            cross_section.clear();
        }
    });
    ui.end_row();
}

impl App {
    pub(super) fn potentials_pane(&mut self, ui: &mut egui::Ui) {
        let mut wires: HashMap<&str, usize> = HashMap::new();
        for net in self.session.analysis.netlist.nets.iter().filter(|n| n.named) {
            *wires.entry(net.name.as_str()).or_default() += net.wires.len();
        }
        let mut remove = None;
        let mut preset = None;
        let mut overview = false;
        let prj = &mut self.session.prj;
        ui.scope(|ui| {
            ui.horizontal_wrapped(|ui| {
                if icons::text_button(ui, Icon::Plus, tr("New potential")).clicked() {
                    prj.potentials.push(PotentialDef { name: format!("P{}", prj.potentials.len() + 1), ..Default::default() });
                }
                ui.menu_button(tr("Load preset"), |ui| {
                    for p in Preset::ALL {
                        if ui.button(p.title()).clicked() {
                            preset = Some(p);
                            ui.close();
                        }
                    }
                });
                if icons::text_button(ui, Icon::Report, tr("Potential overview")).clicked() {
                    overview = true;
                }
                ui.separator();
                ui.checkbox(&mut prj.wire_colors, tr("Wire colours"))
                    .on_hover_text(tr("Draw wires in their colour (also in PDF and SVG); saved with the project"));
            });
            ui.weak(tr("Wires without their own colour or cross-section take them from the potential of their connection; cable cores keep the values of the cable."));
            ui.separator();
            egui::ScrollArea::both().auto_shrink([false, true]).max_height(ui.available_height() - 70.0).show(ui, |ui| {
                egui::Grid::new("potentials").striped(true).num_columns(8).spacing([10.0, 4.0]).show(ui, |ui| {
                    for h in [tr("Potential"), tr("Type"), tr("Voltage"), tr("Colour"), tr("Cross-section"), tr("Description"), tr("Wires"), ""] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    for (i, d) in prj.potentials.iter_mut().enumerate() {
                        ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(70.0));
                        egui::ComboBox::from_id_salt(("potkind", i)).selected_text(d.kind.title()).show_ui(ui, |ui| {
                            for k in PotentialKind::ALL {
                                ui.selectable_value(&mut d.kind, k, k.title());
                            }
                        });
                        ui.add(egui::TextEdit::singleline(&mut d.voltage).hint_text(tr("e.g. 400 V AC")).desired_width(110.0));
                        ui.horizontal(|ui| color_field(ui, &mut d.color, "", "", 60.0));
                        ui.add(egui::TextEdit::singleline(&mut d.cross_section).hint_text(tr("e.g. 1.5 mm²")).desired_width(80.0));
                        ui.add(egui::TextEdit::singleline(&mut d.description).desired_width(180.0));
                        ui.label(wires.get(d.name.as_str()).copied().unwrap_or(0).to_string());
                        if icons::button(ui, Icon::Delete, tr("Delete")).clicked() {
                            remove = Some(i);
                        }
                        ui.end_row();
                    }
                });
            });
            ui.separator();
            ui.label(RichText::new(tr("Connections without a defined potential")).strong());
            ui.horizontal(|ui| {
                ui.label(tr("Colour"));
                color_field(ui, &mut prj.wire_defaults.color, "", "", 60.0);
                ui.label(tr("Cross-section"));
                ui.add(egui::TextEdit::singleline(&mut prj.wire_defaults.cross_section).hint_text(tr("e.g. 1.5 mm²")).desired_width(80.0));
            });
        });
        if let Some(i) = remove {
            self.session.prj.potentials.remove(i);
        }
        if let Some(p) = preset {
            self.session.prj.apply_preset(p);
        }
        if overview {
            self.report_kind = ReportKind::PotentialList.into();
            self.show_reports = true;
        }
    }
}
