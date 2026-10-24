//! Change requests window: requests from inside or outside the project with
//! status, affected pages and devices, the implementing revision and linked
//! comments; exchange with other systems as CSV or JSON.

use super::{App, LeftTab};
use crate::io::Purpose;
use openecraft_core::review::{self, ChangeRequest, Hit, Priority, Record, RequestStatus};
use openecraft_core::structure;
use egui::{Color32, RichText};
use openecraft_i18n::{tr, tr_fmt};

#[derive(Default)]
pub struct RequestsUi {
    pub open: bool,
    selected: Option<u64>,
    /// Also list rejected and closed requests.
    show_done: bool,
    /// Devices as edited (comma separated).
    devices: String,
    devices_for: u64,
}

fn status_color(s: RequestStatus) -> Color32 {
    match s {
        RequestStatus::New => Color32::from_rgb(230, 140, 0),
        RequestStatus::Accepted | RequestStatus::InWork => Color32::from_rgb(70, 130, 220),
        RequestStatus::Implemented => Color32::from_rgb(60, 160, 90),
        RequestStatus::Rejected | RequestStatus::Closed => Color32::GRAY,
    }
}

impl App {
    pub(super) fn requests_window(&mut self, ctx: &egui::Context) {
        if !self.requests_ui.open {
            return;
        }
        let mut open = true;
        let mut new = false;
        let mut import = false;
        let mut export = None;
        egui::Window::new(tr("Change requests")).open(&mut open).default_size([820.0, 520.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                new = ui.button(tr("➕ New request")).clicked();
                import = ui.button(tr("📂 Import…")).on_hover_text(tr("CSV or JSON from another system; known numbers are updated")).clicked();
                if ui.button(tr("⬇ Export CSV")).clicked() {
                    export = Some(false);
                }
                if ui.button(tr("⬇ Export JSON")).clicked() {
                    export = Some(true);
                }
                ui.checkbox(&mut self.requests_ui.show_done, tr("Rejected and closed ones too"));
            });
            ui.separator();
            egui::Panel::left("request_list").resizable(true).default_size(330.0).show(ui, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    for r in review::sorted_requests(&self.session.prj) {
                        if r.status.is_done() && !self.requests_ui.show_done && self.requests_ui.selected != Some(r.id) {
                            continue;
                        }
                        let mut job = egui::text::LayoutJob::default();
                        let normal = egui::TextFormat { color: ui.visuals().text_color(), ..Default::default() };
                        job.append(&format!("{} ", r.number), 0.0, egui::TextFormat { color: ui.visuals().strong_text_color(), ..normal.clone() });
                        job.append(&format!("• {} ", r.status.title()), 0.0, egui::TextFormat { color: status_color(r.status), ..normal.clone() });
                        if r.priority == Priority::High {
                            job.append("! ", 0.0, egui::TextFormat { color: Color32::from_rgb(220, 60, 60), ..normal.clone() });
                        }
                        job.append(&r.title, 0.0, normal);
                        if ui.selectable_label(self.requests_ui.selected == Some(r.id), job).clicked() {
                            self.requests_ui.selected = Some(r.id);
                        }
                    }
                    if self.session.prj.review.live_requests().next().is_none() {
                        ui.weak(tr("No change requests yet."));
                    }
                });
            });
            egui::CentralPanel::default().show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("request_details").auto_shrink([false, false]).show(ui, |ui| {
                    match self.requests_ui.selected.and_then(|id| self.session.prj.review.request(id).cloned()) {
                        Some(r) => self.request_details(ui, r),
                        None => {
                            ui.weak(tr("Select a request."));
                        }
                    }
                });
            });
        });
        self.requests_ui.open &= open;
        if new {
            let (user, date) = (self.user_name(), self.today());
            let id = review::add_request(&mut self.session.prj, tr("New change request"), &user, &date);
            self.requests_ui.selected = Some(id);
        }
        if import {
            self.io.open(ctx, Purpose::ChangeRequests);
        }
        match export {
            Some(true) => self.export(ctx, "change-requests.json".into(), ("JSON", "json"), review::requests_json(&self.session.prj)),
            Some(false) => self.export(ctx, "change-requests.csv".into(), ("CSV", "csv"), review::requests_csv(&self.session.prj)),
            None => {}
        }
    }

    fn request_details(&mut self, ui: &mut egui::Ui, r: ChangeRequest) {
        let mut e = r.clone();
        egui::Grid::new(("request", r.id)).num_columns(2).show(ui, |ui| {
            ui.label(tr("Number"));
            ui.text_edit_singleline(&mut e.number);
            ui.end_row();
            ui.label(tr("Title"));
            ui.add(egui::TextEdit::singleline(&mut e.title).desired_width(f32::INFINITY));
            ui.end_row();
            ui.label(tr("Description"));
            ui.add(egui::TextEdit::multiline(&mut e.description).desired_rows(3).desired_width(f32::INFINITY));
            ui.end_row();
            ui.label(tr("Requester"));
            ui.text_edit_singleline(&mut e.requester);
            ui.end_row();
            ui.label(tr("Origin"));
            ui.add(egui::TextEdit::singleline(&mut e.origin).hint_text(tr("e.g. customer, ERP, PLM")));
            ui.end_row();
            ui.label(tr("Date"));
            ui.add(egui::TextEdit::singleline(&mut e.date).hint_text(tr("YYYY-MM-DD")).desired_width(100.0));
            ui.end_row();
            ui.label(tr("Priority"));
            egui::ComboBox::from_id_salt(("request_priority", r.id)).selected_text(e.priority.title()).show_ui(ui, |ui| {
                for p in Priority::ALL {
                    ui.selectable_value(&mut e.priority, p, p.title());
                }
            });
            ui.end_row();
            ui.label(tr("Status"));
            egui::ComboBox::from_id_salt(("request_status", r.id)).selected_text(RichText::new(e.status.title()).color(status_color(e.status))).show_ui(ui, |ui| {
                for s in RequestStatus::ALL {
                    ui.selectable_value(&mut e.status, s, RichText::new(s.title()).color(status_color(s)));
                }
            });
            ui.end_row();
            ui.label(tr("Revision"));
            let shown = if e.revision.is_empty() { tr("(none)").to_string() } else { e.revision.clone() };
            egui::ComboBox::from_id_salt(("request_revision", r.id)).selected_text(shown).show_ui(ui, |ui| {
                ui.selectable_value(&mut e.revision, String::new(), tr("(none)"));
                let mut names: Vec<String> = self.session.prj.revisions.iter().map(|r| r.name.clone()).collect();
                let next = openecraft_core::revision::next_name(&self.session.prj);
                if !names.contains(&next) {
                    names.push(next);
                }
                for n in names {
                    ui.selectable_value(&mut e.revision, n.clone(), n);
                }
            });
            ui.end_row();
            ui.label(tr("Devices"));
            if self.requests_ui.devices_for != r.id {
                self.requests_ui.devices = r.devices.join(", ");
                self.requests_ui.devices_for = r.id;
            }
            if ui.add(egui::TextEdit::singleline(&mut self.requests_ui.devices).hint_text(tr("Device tags, separated by commas")).desired_width(f32::INFINITY)).changed() {
                e.devices = self.requests_ui.devices.split(',').map(|d| d.trim().to_string()).filter(|d| !d.is_empty()).collect();
            }
            ui.end_row();
        });
        let mut goto = None;
        egui::CollapsingHeader::new(tr_fmt!("Affected pages ({})", e.pages.len())).id_salt(("request_pages", r.id)).show(ui, |ui| {
            for (i, pg) in self.session.prj.pages.iter().enumerate() {
                let mut on = e.pages.contains(&pg.id);
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut on, format!("{} {}", structure::page_full_name(pg), pg.description)).changed() {
                        if on {
                            e.pages.push(pg.id);
                        } else {
                            e.pages.retain(|p| *p != pg.id);
                        }
                    }
                    if on && ui.small_button("➡").on_hover_text(tr("Go to page")).clicked() {
                        goto = Some(i);
                    }
                });
            }
        });
        let comments: Vec<(u64, u64, String)> = self.session.prj.review.live_comments().filter(|c| c.request == r.id).map(|c| (c.id, c.page, format!("#{} {}: {}", self.session.prj.review.comment_number(c.id), c.status.title(), c.text.lines().next().unwrap_or("")))).collect();
        ui.label(RichText::new(tr_fmt!("Comments ({})", comments.len())).strong());
        let mut show_comment = None;
        for (id, page, text) in comments {
            if ui.link(text).clicked() {
                show_comment = Some((id, page));
            }
        }
        let add_comment = ui.button(tr("💬 Comment on the current page")).on_hover_text(tr("Linked to this request")).clicked();
        ui.separator();
        let delete = ui.button(tr("🗑 Delete request")).clicked();

        if delete {
            e.deleted = true;
            self.requests_ui.selected = None;
        }
        if e != r {
            e.bump();
            if let Some(old) = self.session.prj.review.requests.iter_mut().find(|o| o.id == r.id) {
                *old = e;
            }
        }
        if add_comment {
            let page = self.session.prj.pages[self.session.page].id;
            let (user, date) = (self.user_name(), self.today());
            let id = review::add_comment(&mut self.session.prj, page, openecraft_core::geom::p(20.0, 20.0), &user, &date, "");
            if let Some(c) = self.session.prj.review.comment_mut(id) {
                c.request = r.id;
            }
            self.review_ui.selected = Some(Hit::Comment(id));
            self.left_tab = LeftTab::Review;
        }
        if let Some((id, page)) = show_comment {
            if let Some(pi) = self.session.prj.pages.iter().position(|p| p.id == page) {
                self.goto(pi, 0);
            }
            self.review_ui.selected = Some(Hit::Comment(id));
            self.left_tab = LeftTab::Review;
        }
        if let Some(i) = goto {
            self.goto(i, 0);
        }
    }
}
