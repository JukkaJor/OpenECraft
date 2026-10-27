//! Pre-planning window: the segment tree with the planned power, the selected
//! segment with its structure and planned devices (state against the schematic),
//! templates, page generation from page macros and the CSV / Excel exchange.

use super::App;
use crate::io::Purpose;
use egui::{Color32, RichText};
use openecraft_core::index::Index;
use openecraft_core::macros::MacroKind;
use openecraft_core::preplanning::{self as pp, PlannedDevice, State};
use openecraft_i18n::{tr, tr_fmt};

#[derive(Default)]
pub struct PreplanningUi {
    pub open: bool,
    selected: Option<u64>,
    template: String,
    message: String,
}

fn state_color(s: State) -> Color32 {
    match s {
        State::Planned => Color32::GRAY,
        State::Engineered => crate::style::ok(),
        State::PartDiffers => crate::style::warning(),
    }
}

impl App {
    pub(super) fn preplanning_pane(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let (mut import, mut export, mut generate) = (false, None, false);
        ui.scope(|ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr("➕ Segment")).on_hover_text(tr("At the top level")).clicked() {
                    let n = self.session.prj.preplanning.segments.iter().filter(|s| s.parent == 0).count() + 1;
                    self.preplanning_ui.selected = Some(pp::add_segment(&mut self.session.prj, 0, &tr_fmt!("Segment {}", n)));
                }
                let sel = self.preplanning_ui.selected.filter(|id| pp::segment(&self.session.prj, *id).is_some());
                if ui.add_enabled(sel.is_some(), egui::Button::new(tr("➕ Sub-segment"))).clicked() {
                    if let Some(p) = sel {
                        let n = self.session.prj.preplanning.segments.iter().filter(|s| s.parent == p).count() + 1;
                        self.preplanning_ui.selected = Some(pp::add_segment(&mut self.session.prj, p, &tr_fmt!("Segment {}", n)));
                    }
                }
                if ui.add_enabled(sel.is_some(), egui::Button::new(tr("🗑 Remove segment"))).on_hover_text(tr("With everything below it")).clicked() {
                    if let Some(id) = sel {
                        pp::remove_segment(&mut self.session.prj, id);
                        self.preplanning_ui.selected = None;
                    }
                }
                ui.separator();
                import = ui.button(tr("Import CSV / Excel…")).clicked();
                if ui.button(tr("Export CSV…")).clicked() {
                    export = Some(false);
                }
                if ui.button(tr("Export Excel…")).clicked() {
                    export = Some(true);
                }
                generate = ui.button(tr("Generate pages")).on_hover_text(tr("A schematic page per planned device with a page macro that is not in the schematic yet (selected segment, otherwise all)")).clicked();
            });
            if !self.preplanning_ui.message.is_empty() {
                ui.weak(&self.preplanning_ui.message);
            }
            ui.separator();
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(220.0);
                    self.preplanning_tree(ui);
                });
                ui.separator();
                ui.vertical(|ui| self.preplanning_segment(ui));
            });
        });
        if import {
            self.io.open(ctx, Purpose::Preplanning);
        }
        match export {
            Some(true) => {
                let bytes = openecraft_core::xlsx::write(&[("Preplanning".into(), pp::sheet(&self.session.prj))]);
                self.export_bytes(ctx, "preplanning.xlsx".into(), ("Excel", "xlsx"), &bytes);
            }
            Some(false) => self.export(ctx, "preplanning.csv".into(), ("CSV", "csv"), pp::csv(&self.session.prj)),
            None => {}
        }
        if generate {
            let roots: Vec<u64> = match self.preplanning_ui.selected.filter(|id| pp::segment(&self.session.prj, *id).is_some()) {
                Some(id) => vec![id],
                None => self.session.prj.preplanning.segments.iter().filter(|s| s.parent == 0).map(|s| s.id).collect(),
            };
            let (mut pages, mut missing) = (Vec::new(), Vec::new());
            for r in roots {
                let (p, m) = pp::generate(&mut self.session.prj, r, &self.macro_lib);
                pages.extend(p);
                missing.extend(m);
            }
            self.preplanning_ui.message = if missing.is_empty() {
                tr_fmt!("{} pages generated", pages.len())
            } else {
                tr_fmt!("{} pages generated; page macros not found: {}", pages.len(), missing.join(", "))
            };
        }
    }

    fn preplanning_tree(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(tr("Segments")).strong());
        egui::ScrollArea::vertical().id_salt("pp_tree").max_height(400.0).show(ui, |ui| {
            let rows: Vec<(usize, u64, String, f32, usize)> =
                pp::tree(&self.session.prj).into_iter().map(|(d, s)| (d, s.id, s.name.clone(), pp::power(&self.session.prj, s.id), s.devices.len())).collect();
            if rows.is_empty() {
                ui.weak(tr("No segments yet: add one, or import a CSV / Excel file."));
            }
            for (depth, id, name, power, n) in rows {
                ui.horizontal(|ui| {
                    ui.add_space(16.0 * depth as f32);
                    let text = if power > 0.0 { format!("{name}   ({n} · {power:.1} kW)") } else { format!("{name}   ({n})") };
                    if ui.selectable_label(self.preplanning_ui.selected == Some(id), text).clicked() {
                        self.preplanning_ui.selected = Some(id);
                    }
                });
            }
        });
    }

    fn preplanning_segment(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.preplanning_ui.selected.filter(|id| pp::segment(&self.session.prj, *id).is_some()) else {
            ui.weak(tr("Select a segment."));
            return;
        };
        let idx = Index::build(&self.session.prj);
        let page_macros: Vec<String> = self.macro_lib.macros.iter().filter(|m| m.kind == MacroKind::Page).map(|m| m.name.clone()).collect();
        let prj = &self.session.prj;
        let tags: Vec<(String, State)> = pp::segment(prj, id)
            .map(|s| {
                s.devices
                    .iter()
                    .map(|d| {
                        let t = pp::full_tag(prj, id, d);
                        let st = pp::state(prj, &idx, &t, d);
                        (t, st)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let path = pp::path(&self.session.prj, id);
        let (inh_f, inh_l) = pp::structure(&self.session.prj, id);
        let Some(s) = pp::segment_mut(&mut self.session.prj, id) else { return };
        ui.label(RichText::new(path).strong());
        egui::Grid::new("pp_seg").num_columns(2).show(ui, |ui| {
            ui.label(tr("Name"));
            let mut name = s.name.clone();
            if ui.text_edit_singleline(&mut name).changed() && !name.contains(pp::PATH_SEP) && !name.trim().is_empty() {
                s.name = name;
            }
            ui.end_row();
            ui.label(tr("Description"));
            ui.text_edit_singleline(&mut s.description);
            ui.end_row();
            ui.label(tr("Function (=)"));
            ui.add(egui::TextEdit::singleline(&mut s.function).hint_text(inh_f));
            ui.end_row();
            ui.label(tr("Location (+)"));
            ui.add(egui::TextEdit::singleline(&mut s.location).hint_text(inh_l));
            ui.end_row();
        });
        ui.separator();
        let mut remove = None;
        egui::ScrollArea::both().id_salt("pp_devices").max_height(260.0).show(ui, |ui| {
            egui::Grid::new("pp_dev").striped(true).num_columns(9).spacing([6.0, 3.0]).show(ui, |ui| {
                for h in [tr("Tag"), tr("Kind"), tr("Description"), tr("kW"), tr("Voltage"), tr("Part"), tr("Page macro"), tr("State"), ""] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for (i, d) in s.devices.iter_mut().enumerate() {
                    ui.add(egui::TextEdit::singleline(&mut d.tag).desired_width(60.0).font(egui::TextStyle::Monospace));
                    ui.add(egui::TextEdit::singleline(&mut d.kind).desired_width(70.0));
                    ui.add(egui::TextEdit::singleline(&mut d.description).desired_width(120.0));
                    ui.add(egui::DragValue::new(&mut d.power).speed(0.1).range(0.0..=100000.0));
                    ui.add(egui::TextEdit::singleline(&mut d.voltage).desired_width(55.0));
                    ui.add(egui::TextEdit::singleline(&mut d.part).desired_width(90.0));
                    egui::ComboBox::from_id_salt(("pp_macro", i)).width(110.0).selected_text(d.macro_name.as_str()).show_ui(ui, |ui| {
                        ui.selectable_value(&mut d.macro_name, String::new(), "–");
                        for m in &page_macros {
                            ui.selectable_value(&mut d.macro_name, m.clone(), m);
                        }
                    });
                    match tags.get(i) {
                        Some((t, st)) => ui.label(RichText::new(st.title()).color(state_color(*st))).on_hover_text(t),
                        None => ui.label(""),
                    };
                    if ui.small_button("🗑").clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
        });
        if let Some(i) = remove {
            s.devices.remove(i);
        }
        if ui.button(tr("➕ Device")).clicked() {
            let n = s.devices.len() + 1;
            s.devices.push(PlannedDevice { tag: format!("-M{n}"), ..Default::default() });
        }
        ui.separator();
        let templates: Vec<String> = self.session.prj.preplanning.templates.iter().map(|t| t.name.clone()).collect();
        let (mut apply, mut save) = (false, false);
        ui.horizontal(|ui| {
            ui.label(tr("Template"));
            egui::ComboBox::from_id_salt("pp_template").selected_text(self.preplanning_ui.template.as_str()).show_ui(ui, |ui| {
                for t in &templates {
                    ui.selectable_value(&mut self.preplanning_ui.template, t.clone(), t);
                }
            });
            apply = ui.add_enabled(templates.contains(&self.preplanning_ui.template), egui::Button::new(tr("Apply"))).clicked();
            ui.add(egui::TextEdit::singleline(&mut self.preplanning_ui.template).desired_width(120.0).hint_text(tr("name")));
            save = ui.add_enabled(!self.preplanning_ui.template.trim().is_empty(), egui::Button::new(tr("Save as template"))).clicked();
        });
        let name = self.preplanning_ui.template.clone();
        if apply {
            match pp::apply_template(&mut self.session.prj, id, &name) {
                Ok(n) => self.preplanning_ui.message = tr_fmt!("{} devices added", n),
                Err(e) => self.preplanning_ui.message = e,
            }
        }
        if save {
            let _ = pp::save_template(&mut self.session.prj, id, &name);
        }
    }

    pub(super) fn import_preplanning(&mut self, ctx: &egui::Context, bytes: &[u8]) {
        let rows = if bytes.starts_with(b"PK") {
            match openecraft_core::xlsx::read(bytes) {
                Ok(sheets) => sheets.into_iter().next().map(|(_, r)| r).unwrap_or_default(),
                Err(e) => return self.notify(ctx, e),
            }
        } else {
            openecraft_core::xlsx::rows_of_csv(String::from_utf8_lossy(bytes).trim_start_matches('\u{feff}'))
        };
        match pp::import_rows(&mut self.session.prj, &rows) {
            Ok((s, a, u)) => self.notify(ctx, tr_fmt!("{} segments created, {} devices added, {} updated", s, a, u)),
            Err(e) => self.notify(ctx, e),
        }
    }
}
