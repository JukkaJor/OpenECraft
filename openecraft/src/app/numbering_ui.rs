//! Numbering dialogs: device tags (format, start, scope, preview) and pages.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use openecraft_core::numbering::{self, DeviceNumbering, PageNumbering, DEVICE_FORMATS};
use egui::RichText;

#[derive(Default)]
pub struct NumberingUi {
    pub show_devices: bool,
    pub show_pages: bool,
    devices: DeviceNumbering,
    pages: PageNumbering,
    /// Only the current page instead of the whole project.
    current_page: bool,
    /// Letters filter as typed (`K, Q`).
    letters: String,
}

impl App {
    pub(super) fn numbering_windows(&mut self, ctx: &egui::Context) {
        self.device_numbering_window(ctx);
        self.page_numbering_window(ctx);
    }

    fn device_numbering_window(&mut self, ctx: &egui::Context) {
        if !self.numbering_ui.show_devices {
            return;
        }
        let (window, esc) = self.dialogs.window(ctx, tr("Device numbering"), "device_numbering", None);
        let mut open = !esc;
        let mut apply = false;
        let mut cancel = false;
        let page = self.session.page;
        let st = &mut self.numbering_ui;
        st.devices.pages = if st.current_page { vec![page] } else { Vec::new() };
        st.devices.letters = st.letters.split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect();
        let plan = numbering::plan_devices(&self.session.prj, &st.devices);
        window.open(&mut open).resizable(true).default_width(420.0).show(ctx, |ui| {
            ui.label(tr("Numbers the device tags per structure and identifying letter in schematic order (page, column, top to bottom)"));
            ui.add_space(4.0);
            egui::Grid::new("devnum").num_columns(2).show(ui, |ui| {
                ui.label(tr("Format"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut st.devices.format).desired_width(160.0).font(egui::TextStyle::Monospace));
                    egui::ComboBox::from_id_salt("devfmt").selected_text("…").width(30.0).show_ui(ui, |ui| {
                        for (f, example) in DEVICE_FORMATS {
                            if ui.button(format!("{f}   {example}")).clicked() {
                                st.devices.format = f.to_string();
                            }
                        }
                    });
                });
                ui.end_row();
                ui.label(tr("Start at"));
                ui.add(egui::DragValue::new(&mut st.devices.start).range(0..=100_000));
                ui.end_row();
                ui.label(tr("Step"));
                ui.add(egui::DragValue::new(&mut st.devices.step).range(1..=1000));
                ui.end_row();
                ui.label(tr("Identifying letters"));
                ui.add(egui::TextEdit::singleline(&mut st.letters).desired_width(120.0).hint_text(tr("all, or e.g. K, Q")));
                ui.end_row();
            });
            ui.checkbox(&mut st.current_page, tr("Only devices on the current page"));
            ui.checkbox(&mut st.devices.skip_terminals, tr("Terminal strips, plugs and PLC cards keep their tags"));
            ui.label(RichText::new(tr("Placeholders: {letter} identifying letter, {n} counter, {page} page, {column} / {row} where the main function stands")).weak());
            ui.separator();
            if plan.is_empty() {
                ui.label(tr("All devices are numbered already."));
            } else {
                ui.label(if plan.len() == 1 { tr("1 device gets a new tag").to_string() } else { tr_fmt!("{} devices get a new tag", plan.len()) });
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    egui::Grid::new("devplan").striped(true).num_columns(2).show(ui, |ui| {
                        for (old, new) in &plan {
                            ui.monospace(old);
                            ui.monospace(format!("› {new}"));
                            ui.end_row();
                        }
                    });
                });
            }
            ui.horizontal(|ui| {
                apply = ui.add_enabled(!plan.is_empty(), egui::Button::new(tr("Number devices"))).clicked();
                cancel = ui.button(tr("Cancel")).clicked();
            });
        });
        if apply {
            let opts = self.numbering_ui.devices.clone();
            let n = numbering::renumber_devices(&mut self.session.prj, &opts).len();
            self.notify(ctx, if n == 1 { tr("1 device renamed").to_string() } else { tr_fmt!("{} devices renamed", n) });
        }
        self.numbering_ui.show_devices = open && !cancel;
    }

    fn page_numbering_window(&mut self, ctx: &egui::Context) {
        if !self.numbering_ui.show_pages {
            return;
        }
        let (window, esc) = self.dialogs.window(ctx, tr("Renumber pages"), "renumber_pages", None);
        let mut open = !esc;
        let mut apply = false;
        let mut cancel = false;
        let st = &mut self.numbering_ui;
        let plan = numbering::plan_pages(&self.session.prj, &st.pages);
        let sectioned = !self.session.prj.sections.is_empty();
        window.open(&mut open).resizable(false).show(ctx, |ui| {
            egui::Grid::new("pagenum").num_columns(2).show(ui, |ui| {
                ui.label(tr("Start at"));
                ui.add(egui::DragValue::new(&mut st.pages.start).range(0..=100_000));
                ui.end_row();
                ui.label(tr("Step"));
                ui.add(egui::DragValue::new(&mut st.pages.step).range(1..=1000));
                ui.end_row();
            });
            ui.checkbox(&mut st.pages.per_structure, tr("Start again when the page structure changes"));
            ui.checkbox(&mut st.pages.subpages, tr("Keep sub-pages (3.a, 3.1) with the page before"));
            if sectioned {
                ui.checkbox(&mut st.pages.sections, tr("Number the sections 1, 2, 3 … on every level first"));
                ui.label(RichText::new(tr("In every section counting starts again after the section number (2.3.1).")).weak());
            }
            ui.label(RichText::new(tr("Released pages keep their name.")).weak());
            ui.separator();
            ui.label(if plan.len() == 1 { tr("1 page gets a new name").to_string() } else { tr_fmt!("{} pages get a new name", plan.len()) });
            ui.horizontal(|ui| {
                apply = ui.add_enabled(!plan.is_empty(), egui::Button::new(tr("Renumber pages"))).clicked();
                cancel = ui.button(tr("Cancel")).clicked();
            });
        });
        if apply {
            let opts = self.numbering_ui.pages.clone();
            let n = numbering::renumber_pages(&mut self.session.prj, &opts);
            self.notify(ctx, if n == 1 { tr("1 page renamed").to_string() } else { tr_fmt!("{} pages renamed", n) });
        }
        self.numbering_ui.show_pages = open && !cancel;
    }
}
