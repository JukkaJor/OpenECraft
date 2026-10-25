//! Project-wide find and replace window (Edit › Find, Ctrl+F).

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use openecraft_core::search::{self, Hit, Kind, Query};
use egui::RichText;

#[derive(Default)]
pub struct FindUi {
    pub open: bool,
    query: Query,
    with: String,
    hits: Vec<Hit>,
    /// Revision of the history the hits were found in.
    rev: u64,
    searched: bool,
}

pub fn kind_title(k: Kind) -> &'static str {
    match k {
        Kind::Page => tr("Pages"),
        Kind::Device => tr("Devices"),
        Kind::FunctionText => tr("Function texts"),
        Kind::Text => tr("Texts"),
        Kind::WireNumber => tr("Wire numbers"),
        Kind::Terminal => tr("Terminals and pins"),
        Kind::Cable => tr("Cables"),
        Kind::Part => tr("Parts"),
        Kind::Description => tr("Descriptions"),
    }
}

impl App {
    pub(super) fn find_window(&mut self, ctx: &egui::Context) {
        if !self.find_ui.open {
            return;
        }
        let mut open = true;
        let mut search_now = false;
        let mut replace = false;
        let mut jump = None;
        let rev = self.session.history.rev;
        let st = &mut self.find_ui;
        egui::Window::new(tr("Find")).open(&mut open).resizable(true).default_width(460.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut st.query.text).hint_text(tr("Text to find")).desired_width(240.0));
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    search_now = true;
                }
                search_now |= ui.button(tr("Find")).clicked();
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut st.query.case_sensitive, tr("Match case"));
                ui.checkbox(&mut st.query.whole_word, tr("Whole words"));
            });
            ui.horizontal_wrapped(|ui| {
                for k in Kind::ALL {
                    let mut on = st.query.kinds.is_empty() || st.query.kinds.contains(&k);
                    if ui.checkbox(&mut on, kind_title(k)).changed() {
                        if st.query.kinds.is_empty() {
                            st.query.kinds = Kind::ALL.to_vec();
                        }
                        st.query.kinds.retain(|x| *x != k);
                        if on {
                            st.query.kinds.push(k);
                        }
                        if st.query.kinds.len() == Kind::ALL.len() {
                            st.query.kinds.clear();
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut st.with).hint_text(tr("Replace with")).desired_width(240.0));
                replace = ui
                    .add_enabled(!st.query.text.is_empty(), egui::Button::new(tr("Replace all")))
                    .on_hover_text(tr("Replaces in free texts, function texts, page, device and cable descriptions; released pages stay"))
                    .clicked();
            });
            ui.separator();
            if st.searched && st.rev != rev {
                ui.weak(tr("The project changed since the search."));
            }
            if st.searched {
                ui.label(tr_fmt!("{} found", st.hits.len()));
            }
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                egui::Grid::new("hits").striped(true).num_columns(3).show(ui, |ui| {
                    for h in &st.hits {
                        ui.weak(kind_title(h.kind));
                        let page = h.page.map(|p| openecraft_core::structure::page_full_name(&self.session.prj.pages[p])).unwrap_or_default();
                        ui.label(RichText::new(page).monospace());
                        if ui.link(&h.text).clicked() {
                            if let Some(p) = h.page {
                                jump = Some((p, h.item));
                            }
                        }
                        ui.end_row();
                    }
                });
            });
        });
        if replace {
            let q = self.find_ui.query.clone();
            let n = search::replace(&mut self.session.prj, &q, &self.find_ui.with.clone());
            self.notify(ctx, tr_fmt!("{} texts changed", n));
            search_now = true;
        }
        if search_now {
            self.find_ui.hits = search::find(&self.session.prj, &self.find_ui.query);
            self.find_ui.rev = self.session.history.rev;
            self.find_ui.searched = true;
        }
        if let Some((p, id)) = jump {
            self.goto(p, id);
        }
        self.find_ui.open = open;
    }
}
