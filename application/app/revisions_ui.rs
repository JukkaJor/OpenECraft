//! Revisions window: revisions with their status, the release workflow (submit
//! for review, approve / reject, release) and the changes since the last released
//! revision.

use super::App;
use crate::model::ReportKind;
use crate::revision::{self, Status};
use egui::{Color32, RichText};
use openecraft_i18n::{tr, tr_fmt};

#[derive(Default)]
pub struct RevisionsUi {
    name: String,
    date: String,
    author: String,
    description: String,
    /// Lock all pages when releasing.
    lock: bool,
    /// Note of an approval / rejection.
    note: String,
    /// Approvers as edited (comma separated).
    approvers: String,
    editing_approvers: bool,
}

pub fn status_color(s: Status) -> Color32 {
    match s {
        Status::Draft => Color32::GRAY,
        Status::InReview => Color32::from_rgb(70, 130, 220),
        Status::Approved => Color32::from_rgb(60, 160, 90),
        Status::Rejected => Color32::from_rgb(220, 60, 60),
        Status::Released => Color32::from_rgb(140, 140, 160),
    }
}

/// What a button of the open revision asks for.
enum Action {
    Submit,
    Withdraw,
    Discard,
    Decide(bool),
    Release,
}

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn revisions_window(&mut self, ctx: &egui::Context) {
        if !self.eng.show_revisions {
            return;
        }
        let mut open = true;
        let mut create = false;
        let mut lock = None;
        let mut jump = None;
        let mut overview = false;
        let mut action = None;
        let r = &mut self.revisions_ui;
        if r.name.is_empty() {
            r.name = revision::next_name(&self.prj);
            r.date = self.prj.info.date.clone();
            r.author = self.prj.info.creator.clone();
        }
        let user = self.user_name();
        let locked = self.prj.pages.iter().filter(|p| p.locked).count();
        let open_rev = revision::open_revision(&self.prj);
        egui::Window::new(tr("Revisions")).open(&mut open).default_size([680.0, 560.0]).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("revs").striped(true).num_columns(5).show(ui, |ui| {
                    for h in [tr("Index"), tr("Status"), tr("Date"), tr("Author"), tr("Description")] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    for (i, r) in self.prj.revisions.iter().enumerate() {
                        let s = revision::status(&self.prj, i);
                        ui.label(&r.name);
                        ui.label(RichText::new(s.title()).color(status_color(s)));
                        ui.label(&r.date);
                        ui.label(&r.author);
                        ui.label(&r.description);
                        ui.end_row();
                    }
                });
                if self.prj.revisions.is_empty() {
                    ui.weak(tr("No revision yet. Changes are tracked from the first revision on."));
                }

                ui.separator();
                egui::CollapsingHeader::new(RichText::new(tr("Approval workflow")).strong()).id_salt("workflow").default_open(self.prj.workflow.active()).show(ui, |ui| {
                    let wf = &mut self.prj.workflow;
                    let rui = &mut self.revisions_ui;
                    if !rui.editing_approvers {
                        rui.approvers = wf.approvers.join(", ");
                    }
                    ui.horizontal(|ui| {
                        ui.label(tr("Approvers"));
                        let resp = ui.add(egui::TextEdit::singleline(&mut rui.approvers).hint_text(tr("User names, separated by commas")).desired_width(300.0));
                        if resp.changed() {
                            wf.approvers = rui.approvers.split(',').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
                        }
                        rui.editing_approvers = resp.has_focus();
                    });
                    if wf.active() {
                        ui.horizontal(|ui| {
                            ui.label(tr("Approvals needed"));
                            let n = wf.approvers.len();
                            ui.add(egui::DragValue::new(&mut wf.required).range(0..=n));
                            ui.weak(if wf.required == 0 { tr("0 = all approvers") } else { "" });
                        });
                        ui.weak(tr("New revisions start as drafts, are submitted for review and can be released once approved."));
                    } else {
                        ui.weak(tr("Without approvers a new revision is released directly."));
                    }
                });

                if let Some(i) = open_rev {
                    ui.separator();
                    let rev = &self.prj.revisions[i];
                    let s = revision::status(&self.prj, i);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tr_fmt!("Revision {}", rev.name)).strong());
                        ui.label(RichText::new(s.title()).color(status_color(s)).strong());
                        if rev.round > 0 {
                            ui.weak(tr_fmt!("review round {}", rev.round));
                        }
                    });
                    let decisions = revision::decisions(&self.prj, i);
                    if s != Status::Draft {
                        let approved = decisions.iter().filter(|a| a.approved).count();
                        ui.label(tr_fmt!("{} of {} approvals", approved, self.prj.workflow.needed()));
                        for a in &decisions {
                            let (text, color) = if a.approved { (tr("approved"), Color32::from_rgb(60, 160, 90)) } else { (tr("rejected"), Color32::from_rgb(220, 60, 60)) };
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(text).color(color));
                                ui.label(format!("{} {}", a.user, a.date));
                                if !a.note.is_empty() {
                                    ui.weak(&a.note);
                                }
                            });
                        }
                    }
                    let is_approver = self.prj.workflow.is_approver(&user);
                    let rui = &mut self.revisions_ui;
                    ui.horizontal_wrapped(|ui| match s {
                        Status::Draft => {
                            if self.prj.workflow.active() {
                                if ui.button(tr("Submit for review")).clicked() {
                                    action = Some(Action::Submit);
                                }
                            } else if ui.button(tr("Release")).clicked() {
                                action = Some(Action::Release);
                            }
                            if ui.button(tr("Discard draft")).clicked() {
                                action = Some(Action::Discard);
                            }
                        }
                        _ => {
                            if is_approver {
                                ui.add(egui::TextEdit::singleline(&mut rui.note).hint_text(tr("Note")).desired_width(160.0));
                                if ui.button(tr("✔ Approve")).clicked() {
                                    action = Some(Action::Decide(true));
                                }
                                if ui.button(tr("✖ Reject")).clicked() {
                                    action = Some(Action::Decide(false));
                                }
                            } else {
                                ui.weak(tr_fmt!("Approving needs an approver; you are \"{}\".", user));
                            }
                            if ui.button(tr("Submit again")).on_hover_text(tr("With the current pages; the approvals so far become void")).clicked() {
                                action = Some(Action::Submit);
                            }
                            if ui.button(tr("Withdraw")).clicked() {
                                action = Some(Action::Withdraw);
                            }
                            if s == Status::Approved {
                                ui.checkbox(&mut rui.lock, tr("Lock all pages"));
                                if ui.button(RichText::new(tr("Release")).strong()).clicked() {
                                    action = Some(Action::Release);
                                }
                            }
                        }
                    });
                } else {
                    ui.separator();
                    let rui = &mut self.revisions_ui;
                    ui.label(RichText::new(tr("New revision")).strong());
                    egui::Grid::new("newrev").num_columns(2).show(ui, |ui| {
                        ui.label(tr("Index"));
                        ui.add(egui::TextEdit::singleline(&mut rui.name).desired_width(60.0));
                        ui.end_row();
                        ui.label(tr("Date"));
                        ui.add(egui::TextEdit::singleline(&mut rui.date).hint_text(tr("YYYY-MM-DD")).desired_width(100.0));
                        ui.end_row();
                        ui.label(tr("Author"));
                        ui.text_edit_singleline(&mut rui.author);
                        ui.end_row();
                        ui.label(tr("Description"));
                        ui.text_edit_multiline(&mut rui.description);
                        ui.end_row();
                    });
                    let label = if self.prj.workflow.active() {
                        tr("Create draft")
                    } else {
                        ui.checkbox(&mut rui.lock, tr("Release: lock all pages"));
                        tr("Create revision")
                    };
                    create = ui.add_enabled(!rui.name.trim().is_empty(), egui::Button::new(label)).clicked();
                }

                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.label(tr_fmt!("{} of {} pages locked", locked, self.prj.pages.len()));
                    if ui.button(tr("Lock all")).clicked() {
                        lock = Some(true);
                    }
                    if ui.button(tr("Unlock all")).clicked() {
                        lock = Some(false);
                    }
                    let pg = &mut self.prj.pages[self.page];
                    ui.checkbox(&mut pg.locked, tr_fmt!("Page {} locked", pg.name));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    let since = revision::last_released(&self.prj).map(|r| r.name.clone()).unwrap_or_default();
                    ui.label(RichText::new(if since.is_empty() { tr("Changes").to_string() } else { tr_fmt!("Changes since revision {}", since) }).strong());
                    ui.checkbox(&mut self.editor.show_revision_marks, tr("Revision marks"));
                    overview = ui.button(tr("📊 Revision overview")).clicked();
                });
                for pc in &self.analysis.changes {
                    let title = match pc.page {
                        Some(_) => tr_fmt!("Page {} ({})", pc.name, pc.changes.len()),
                        None => tr_fmt!("Page {} (deleted)", pc.name),
                    };
                    egui::CollapsingHeader::new(title).id_salt(("chg", pc.page_id)).show(ui, |ui| {
                        for c in &pc.changes {
                            let text = format!("{} {} {}", tr(c.what), c.kind.title(), c.label);
                            match pc.page {
                                Some(pi) if c.kind != revision::ChangeKind::Removed => {
                                    if ui.link(text.trim()).clicked() {
                                        jump = Some((pi, if c.what == "Page" { 0 } else { c.id }));
                                    }
                                }
                                _ => {
                                    ui.label(RichText::new(text.trim()).color(Color32::from_rgb(200, 80, 80)));
                                }
                            }
                        }
                    });
                }
                if self.analysis.changes.is_empty() && revision::last_released(&self.prj).is_some() {
                    ui.weak(tr("No changes since the last revision."));
                }
            });
        });
        self.eng.show_revisions = open;
        if create {
            let r = &self.revisions_ui;
            let name = r.name.trim().to_string();
            revision::create(&mut self.prj, &name, &r.date, &r.author, &r.description, r.lock);
            let msg = if self.prj.workflow.active() { tr_fmt!("Draft of revision {} created", name) } else { tr_fmt!("Revision {} created", name) };
            self.notify(ctx, msg);
            self.revisions_ui.name.clear();
            self.revisions_ui.description.clear();
        }
        if let (Some(action), Some(i)) = (action, open_rev) {
            let name = self.prj.revisions[i].name.clone();
            let result = match action {
                Action::Submit => revision::submit(&mut self.prj, i).map(|_| tr_fmt!("Revision {} submitted for review", name)),
                Action::Withdraw => {
                    revision::withdraw(&mut self.prj, i);
                    Ok(tr_fmt!("Revision {} withdrawn", name))
                }
                Action::Discard => {
                    revision::discard(&mut self.prj, i);
                    self.revisions_ui.name.clear();
                    Ok(tr_fmt!("Draft of revision {} discarded", name))
                }
                Action::Decide(approved) => {
                    let note = std::mem::take(&mut self.revisions_ui.note);
                    let date = self.today();
                    revision::decide(&mut self.prj, i, &user, &date, approved, note.trim())
                        .map(|_| if approved { tr_fmt!("Revision {} approved by {}", name, user) } else { tr_fmt!("Revision {} rejected by {}", name, user) })
                }
                Action::Release => revision::release(&mut self.prj, i, self.revisions_ui.lock).map(|_| tr_fmt!("Revision {} released", name)),
            };
            match result {
                Ok(msg) => self.notify(ctx, msg),
                Err(e) => self.notify(ctx, e),
            }
        }
        if let Some(l) = lock {
            for pg in &mut self.prj.pages {
                pg.locked = l;
            }
        }
        if let Some((pg, id)) = jump {
            self.goto(pg, id);
        }
        if overview {
            self.report_kind = ReportKind::RevisionOverview.into();
            self.show_reports = true;
        }
    }
}
