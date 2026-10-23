//! Review: comments with status and replies, redline / greenline markup (the
//! "Review" tab of the navigator and the review tools of the toolbar).

use super::{App, LeftTab};
use crate::editor::{ReviewInput, Tool};
use crate::geom::P;
use crate::review::{self, Hit, MarkupKind, Pen, Record, Reply, Status};
use crate::structure;
use egui::{Color32, RichText};
use openecraft_i18n::{tr, tr_fmt};

#[derive(Default)]
pub struct ReviewUi {
    pub selected: Option<Hit>,
    /// List the comments of all pages, not only of the current one.
    all_pages: bool,
    /// Also list resolved and rejected comments.
    show_done: bool,
    pub pen: Pen,
    reply: String,
    /// Text of the next text markup.
    markup_text: String,
}

pub fn status_color(s: Status) -> Color32 {
    match s {
        Status::Open => Color32::from_rgb(230, 140, 0),
        Status::InProgress => Color32::from_rgb(70, 130, 220),
        Status::Resolved => Color32::from_rgb(60, 160, 90),
        Status::Rejected => Color32::GRAY,
    }
}

impl App {
    /// Name for comments, markup and approvals: the project server user, otherwise
    /// the project's creator.
    pub(super) fn user_name(&self) -> String {
        let user = self.remote.user.trim();
        if user.is_empty() {
            self.prj.info.creator.clone()
        } else {
            user.to_string()
        }
    }

    pub(super) fn today(&self) -> String {
        openecraft_ui::io::today()
    }

    /// Adds what a review tool drew on the canvas.
    pub(super) fn review_input(&mut self, input: ReviewInput) {
        let page = self.prj.pages[self.page].id;
        let (user, date) = (self.user_name(), self.today());
        let hit = match input {
            ReviewInput::Comment(at) => {
                self.editor.set_tool(Tool::Select);
                Hit::Comment(review::add_comment(&mut self.prj, page, at, &user, &date, ""))
            }
            ReviewInput::Markup(kind, pen, a, b) => {
                let text = if kind == MarkupKind::Text && self.review_ui.markup_text.trim().is_empty() {
                    tr("Text").to_string()
                } else {
                    self.review_ui.markup_text.clone()
                };
                let text = if kind == MarkupKind::Text { text } else { String::new() };
                Hit::Markup(review::add_markup(&mut self.prj, review::Markup { page, kind, pen, a, b, text, author: user, date, ..Default::default() }))
            }
        };
        self.review_ui.selected = Some(hit);
        self.left_tab = LeftTab::Review;
    }

    /// Selects the comment or markup at a click of the select tool.
    pub(super) fn review_pick(&mut self, at: P) {
        if !self.editor.review_layer {
            return;
        }
        let page = self.prj.pages[self.page].id;
        let tolerance = 6.0 / self.editor.zoom;
        self.review_ui.selected = review::hit(&self.prj, page, at, tolerance);
        if self.review_ui.selected.is_some() {
            self.editor.selection.clear();
            self.left_tab = LeftTab::Review;
        }
    }

    /// Comment and markup tools of the toolbar.
    pub(super) fn review_tools(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        let tool = self.editor.tool.clone();
        if ui.selectable_label(tool == Tool::Comment, tr("💬 Comment")).on_hover_text(tr("Place a comment on the page")).clicked() {
            self.editor.set_tool(Tool::Comment);
        }
        let pen = self.review_ui.pen;
        egui::ComboBox::from_id_salt("markup_pen")
            .selected_text(RichText::new(pen.title()).color(pen_color(pen)))
            .width(90.0)
            .show_ui(ui, |ui| {
                for p in [Pen::Red, Pen::Green] {
                    ui.selectable_value(&mut self.review_ui.pen, p, RichText::new(p.title()).color(pen_color(p)));
                }
            })
            .response
            .on_hover_text(tr("Redline: to be changed · Greenline: changed as built"));
        for kind in MarkupKind::ALL {
            let t = Tool::Markup(kind, self.review_ui.pen);
            if ui.selectable_label(tool == t, RichText::new(kind.title()).color(pen_color(self.review_ui.pen))).clicked() {
                self.editor.set_tool(t);
            }
        }
        // Switching the pen keeps the markup tool.
        if let Tool::Markup(kind, p) = tool {
            if p != self.review_ui.pen {
                self.editor.set_tool(Tool::Markup(kind, self.review_ui.pen));
            }
        }
    }

    /// The "Review" tab of the navigator.
    pub(super) fn review_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.editor.review_layer, tr("Show on the pages"));
            ui.checkbox(&mut self.review_ui.all_pages, tr("All pages"));
            ui.checkbox(&mut self.review_ui.show_done, tr("Done ones too"));
        });
        if let Tool::Markup(MarkupKind::Text, _) = self.editor.tool {
            ui.add(egui::TextEdit::multiline(&mut self.review_ui.markup_text).hint_text(tr("Markup text")).desired_rows(2));
        }
        let page_id = self.prj.pages[self.page].id;
        let open = self.prj.review.live_comments().filter(|c| !c.status.is_done()).count();
        ui.label(RichText::new(tr_fmt!("Comments ({} open)", open)).strong());
        let mut goto = None;
        egui::ScrollArea::vertical().id_salt("review_list").max_height(220.0).auto_shrink([false, true]).show(ui, |ui| {
            for c in self.prj.review.live_comments() {
                if !self.review_ui.all_pages && c.page != page_id || !self.review_ui.show_done && c.status.is_done() && self.review_ui.selected != Some(Hit::Comment(c.id)) {
                    continue;
                }
                let n = self.prj.review.comment_number(c.id);
                let page = self.prj.pages.iter().position(|p| p.id == c.page);
                let page_name = page.map(|i| structure::page_full_name(&self.prj.pages[i])).unwrap_or_default();
                let first = c.text.lines().next().unwrap_or("");
                let mut job = egui::text::LayoutJob::default();
                let normal = egui::TextFormat { color: ui.visuals().text_color(), ..Default::default() };
                job.append(&format!("#{n} "), 0.0, normal.clone());
                job.append(&format!("• {} ", c.status.title()), 0.0, egui::TextFormat { color: status_color(c.status), ..normal.clone() });
                job.append(&format!("{page_name} · {}: {first}", c.author), 0.0, normal);
                if ui.selectable_label(self.review_ui.selected == Some(Hit::Comment(c.id)), job).clicked() {
                    goto = Some((page, Hit::Comment(c.id)));
                }
            }
        });
        if let Some((page, hit)) = goto {
            if let Some(pi) = page {
                if pi != self.page {
                    self.goto(pi, 0);
                }
            }
            self.review_ui.selected = Some(hit);
        }
        if let Some(Hit::Comment(id)) = self.review_ui.selected {
            ui.separator();
            self.comment_details(ui, id);
        }

        ui.separator();
        ui.label(RichText::new(tr("Markup on this page")).strong());
        let mut remove = None;
        let markups: Vec<review::Markup> = self.prj.review.live_markups().filter(|m| m.page == page_id).cloned().collect();
        if markups.is_empty() {
            ui.weak(tr("None – draw it with the redline / greenline tools of the toolbar."));
        }
        for m in markups {
            let mut edited = m.clone();
            ui.horizontal(|ui| {
                let label = RichText::new(format!("{} {}", m.pen.title(), m.kind.title())).color(pen_color(m.pen));
                if ui.selectable_label(self.review_ui.selected == Some(Hit::Markup(m.id)), label).on_hover_text(format!("{} {}", m.author, m.date)).clicked() {
                    self.review_ui.selected = Some(Hit::Markup(m.id));
                }
                ui.checkbox(&mut edited.done, tr("taken over")).on_hover_text(tr("Taken into the drawing: drawn faded"));
                if ui.small_button("🗑").on_hover_text(tr("Delete")).clicked() {
                    remove = Some(m.id);
                }
            });
            if m.kind == MarkupKind::Text && self.review_ui.selected == Some(Hit::Markup(m.id)) {
                ui.add(egui::TextEdit::multiline(&mut edited.text).desired_rows(2));
            }
            if edited != m {
                self.update_markup(edited);
            }
        }
        if let Some(id) = remove {
            if let Some(m) = self.prj.review.markups.iter().find(|m| m.id == id).cloned() {
                self.update_markup(review::Markup { deleted: true, ..m });
            }
            self.review_ui.selected = None;
        }
    }

    fn update_markup(&mut self, mut m: review::Markup) {
        m.bump();
        if let Some(old) = self.prj.review.markups.iter_mut().find(|o| o.id == m.id) {
            *old = m;
        }
    }

    fn comment_details(&mut self, ui: &mut egui::Ui, id: u64) {
        let Some(c) = self.prj.review.comment(id).cloned() else {
            self.review_ui.selected = None;
            return;
        };
        let mut edited = c.clone();
        ui.label(RichText::new(tr_fmt!("Comment #{} – {} {}", self.prj.review.comment_number(id), c.author, c.date)).strong());
        ui.add(egui::TextEdit::multiline(&mut edited.text).hint_text(tr("What should be checked or changed?")).desired_rows(3).desired_width(f32::INFINITY));
        ui.horizontal(|ui| {
            ui.label(tr("Status"));
            egui::ComboBox::from_id_salt(("comment_status", id)).selected_text(RichText::new(edited.status.title()).color(status_color(edited.status))).show_ui(ui, |ui| {
                for s in Status::ALL {
                    ui.selectable_value(&mut edited.status, s, RichText::new(s.title()).color(status_color(s)));
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label(tr("Change request"));
            let shown = self.prj.review.request(edited.request).map(|r| format!("{} {}", r.number, r.title)).unwrap_or_else(|| tr("(none)").to_string());
            egui::ComboBox::from_id_salt(("comment_request", id)).selected_text(shown).width(160.0).show_ui(ui, |ui| {
                ui.selectable_value(&mut edited.request, 0, tr("(none)"));
                for r in review::sorted_requests(&self.prj) {
                    ui.selectable_value(&mut edited.request, r.id, format!("{} {}", r.number, r.title));
                }
            });
        });
        for r in &c.replies {
            ui.label(RichText::new(format!("{} {}", r.author, r.date)).weak());
            ui.label(&r.text);
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.review_ui.reply).hint_text(tr("Reply…")).desired_width(150.0));
            if ui.add_enabled(!self.review_ui.reply.trim().is_empty(), egui::Button::new(tr("Reply"))).clicked() {
                let text = std::mem::take(&mut self.review_ui.reply);
                edited.replies.push(Reply { author: self.user_name(), date: self.today(), text: text.trim().to_string() });
            }
        });
        if ui.button(tr("🗑 Delete comment")).clicked() {
            edited.deleted = true;
            self.review_ui.selected = None;
        }
        if edited != c {
            edited.bump();
            if let Some(old) = self.prj.review.comments.iter_mut().find(|o| o.id == id) {
                *old = edited;
            }
        }
    }
}

fn pen_color(pen: Pen) -> Color32 {
    match pen {
        Pen::Red => Color32::from_rgb(220, 40, 40),
        Pen::Green => Color32::from_rgb(30, 160, 70),
    }
}
