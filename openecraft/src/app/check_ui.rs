//! The Messages pane (the messages of the project check as a sortable table with
//! filters, accepting, levels) and the check settings (checks switched off, changed
//! levels, accepted messages).

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use openecraft_core::model::natural_cmp;
use openecraft_core::structure;
use openecraft_ui::icons::{self, Icon};
use openecraft_api::params::text;
use openecraft_core::checks;
use openecraft_core::issue::{Issue, Severity};
use openecraft_core::model::checks::Level;
use egui::RichText;

pub struct CheckUi {
    pub show_settings: bool,
    errors: bool,
    warnings: bool,
    notes: bool,
    accepted: bool,
    current_page: bool,
    filter: String,
    /// Column sorted by and descending.
    sort: (usize, bool),
}

impl Default for CheckUi {
    fn default() -> Self {
        CheckUi { show_settings: false, errors: true, warnings: true, notes: true, accepted: false, current_page: false, filter: String::new(), sort: (0, false) }
    }
}

fn level_title(l: Level) -> &'static str {
    match l {
        Level::Error => tr("Error"),
        Level::Warning => tr("Warning"),
        Level::Info => tr("Note"),
        Level::Off => tr("Off"),
    }
}

enum Action {
    Jump(usize, u64),
    Accept(Issue, bool),
    Level(String, Option<Level>),
    Disable(String),
}

impl App {
    /// The Messages pane: the messages of the project check as a table with filters;
    /// a click on a column sorts, a click on a row shows the object, right click for more.
    #[allow(clippy::too_many_lines)]
    pub(super) fn check_tab(&mut self, ui: &mut egui::Ui) {
        let issues = self.session.analysis.issues.clone();
        let open = |s| issues.iter().filter(|i| i.severity == s && !i.accepted).count();
        let accepted = issues.iter().filter(|i| i.accepted).count();
        let colors = crate::style::colors(self.dark);
        let mut export = false;
        let mut settings = false;
        {
            let st = &mut self.check_ui;
            ui.horizontal_wrapped(|ui| {
                let e = RichText::new(tr_fmt!("{} errors", open(Severity::Error))).color(colors.error);
                ui.toggle_value(&mut st.errors, e).on_hover_text(tr("Errors"));
                let w = RichText::new(tr_fmt!("{} warnings", open(Severity::Warning))).color(colors.warning);
                ui.toggle_value(&mut st.warnings, w).on_hover_text(tr("Warnings"));
                ui.toggle_value(&mut st.notes, tr_fmt!("{} notes", open(Severity::Info))).on_hover_text(tr("Notes"));
                ui.toggle_value(&mut st.accepted, tr_fmt!("✔ {}", accepted)).on_hover_text(tr("Show the messages accepted as checked"));
                ui.toggle_value(&mut st.current_page, tr("This page")).on_hover_text(tr("Only messages of the current page"));
                ui.add(egui::TextEdit::singleline(&mut st.filter).hint_text(tr("Filter")).desired_width(140.0));
                settings = icons::button(ui, Icon::Settings, tr("Check settings: checks, message levels, accepted messages")).clicked();
                export = icons::button(ui, Icon::Download, tr("Export the messages as CSV")).clicked();
            });
        }
        if settings {
            self.check_ui.show_settings = true;
        }
        if export {
            let csv = checks::issues_csv(&self.session.prj, &issues);
            self.export(ui.ctx(), "check.csv".into(), ("CSV", "csv"), csv);
        }
        let st = &self.check_ui;
        let page = self.session.page;
        let filter = st.filter.to_lowercase();
        let titles: Vec<(String, String)> = self.plugins.checks().map(|c| (c.name.clone(), text(&c.title).to_string())).collect();
        let prj = &self.session.prj;
        // Row: issue, page name, object (tag of the symbol), check title.
        let mut rows: Vec<(&Issue, String, String, String)> = issues
            .iter()
            .filter(|i| match i.severity {
                Severity::Error => st.errors,
                Severity::Warning => st.warnings,
                Severity::Info => st.notes,
            })
            .filter(|i| st.accepted || !i.accepted)
            .filter(|i| !st.current_page || i.target.is_some_and(|t| t.0 == page))
            .map(|i| {
                let (pg, obj) = match i.target {
                    Some((p, id)) => {
                        let name = prj.pages.get(p).map(structure::page_full_name).unwrap_or_default();
                        let obj = prj.pages.get(p).and_then(|pg| pg.symbols.iter().find(|s| s.id == id)).map(|s| s.tag.clone()).unwrap_or_default();
                        (name, obj)
                    }
                    None => (String::new(), String::new()),
                };
                let check = titles.iter().find(|t| t.0 == i.check).map_or(i.check.clone(), |t| t.1.clone());
                (i, pg, obj, check)
            })
            .filter(|(i, pg, obj, check)| filter.is_empty() || [i.message().to_lowercase(), i.code(), pg.to_lowercase(), obj.to_lowercase(), check.to_lowercase()].iter().any(|t| t.contains(&filter)))
            .collect();
        let sort = st.sort;
        rows.sort_by(|a, b| {
            let o = match sort.0 {
                1 => a.0.code().cmp(&b.0.code()),
                2 => a.0.message().cmp(&b.0.message()),
                3 => natural_cmp(&a.1, &b.1),
                4 => natural_cmp(&a.2, &b.2),
                5 => a.3.cmp(&b.3),
                _ => (a.0.severity as u8).cmp(&(b.0.severity as u8)),
            };
            if sort.1 { o.reverse() } else { o }
        });
        let mut action = None;
        let mut sort_by = None;
        egui::ScrollArea::both().id_salt("messages").auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("messages_table").striped(true).num_columns(6).spacing([12.0, 3.0]).show(ui, |ui| {
                for (c, title) in [(0, ""), (1, tr("Code")), (2, tr("Message")), (3, tr("Page")), (4, tr("Object")), (5, tr("Check"))] {
                    let arrow = if sort.0 == c { if sort.1 { " ⏶" } else { " ⏷" } } else { "" };
                    if ui.add(egui::Button::new(RichText::new(format!("{title}{arrow}")).strong()).frame(false)).clicked() {
                        sort_by = Some(c);
                    }
                }
                ui.end_row();
                for (issue, pg, obj, check) in &rows {
                    let (icon, color) = match issue.severity {
                        Severity::Error => (Icon::Error, colors.error),
                        Severity::Warning => (Icon::Warning, colors.warning),
                        Severity::Info => (Icon::Info, ui.visuals().weak_text_color()),
                    };
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                    icons::paint(ui.painter(), rect, if issue.accepted { Icon::Check } else { icon }, color);
                    ui.monospace(issue.code());
                    let msg = if issue.accepted { RichText::new(issue.message()).weak() } else { RichText::new(issue.message()) };
                    let r = ui.add(egui::Label::new(msg).sense(egui::Sense::click()));
                    ui.label(pg);
                    ui.monospace(obj);
                    ui.weak(check);
                    ui.end_row();
                    let r = r.on_hover_text(tr("Click to show, right click for more"));
                    if r.clicked() {
                        if let Some((pg, id)) = issue.target {
                            action = Some(Action::Jump(pg, id));
                        }
                    }
                    r.context_menu(|ui| {
                        if issue.target.is_some() && ui.button(tr("Show")).clicked() {
                            if let Some((pg, id)) = issue.target {
                                action = Some(Action::Jump(pg, id));
                            }
                            ui.close();
                        }
                        let label = if issue.accepted { tr("Take the acceptance back") } else { tr("Accept as checked") };
                        if ui.button(label).clicked() {
                            action = Some(Action::Accept((*issue).clone(), !issue.accepted));
                            ui.close();
                        }
                        ui.menu_button(tr("Level of this kind of message"), |ui| {
                            for l in [Level::Error, Level::Warning, Level::Info, Level::Off] {
                                if ui.button(level_title(l)).clicked() {
                                    action = Some(Action::Level(issue.code(), Some(l)));
                                    ui.close();
                                }
                            }
                            if ui.button(tr("As defined by the check")).clicked() {
                                action = Some(Action::Level(issue.code(), None));
                                ui.close();
                            }
                        });
                        if !issue.check.is_empty() && ui.button(tr_fmt!("Switch off the check \"{}\"", check)).clicked() {
                            action = Some(Action::Disable(issue.check.clone()));
                            ui.close();
                        }
                    });
                }
            });
            if rows.is_empty() {
                ui.label(tr("✔ No problems found."));
            }
        });
        if let Some(c) = sort_by {
            let st = &mut self.check_ui;
            st.sort = if st.sort.0 == c { (c, !st.sort.1) } else { (c, false) };
        }
        match action {
            Some(Action::Jump(pg, id)) => self.goto(pg, id),
            Some(Action::Accept(issue, on)) => {
                let (user, date) = (self.user_name(), self.today());
                checks::accept(&mut self.session.prj, &issue, on, "", &user, &date);
            }
            Some(Action::Level(code, Some(l))) => {
                self.session.prj.check_settings.levels.insert(code, l);
            }
            Some(Action::Level(code, None)) => {
                self.session.prj.check_settings.levels.remove(&code);
            }
            Some(Action::Disable(name)) => self.session.prj.check_settings.disabled.push(name),
            None => {}
        }
    }

    pub(super) fn check_settings_body(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let checks: Vec<(String, String)> = self.plugins.checks().map(|c| (c.name.clone(), text(&c.title).to_string())).collect();
        let stale: Vec<String> = checks::stale_accepted(&self.session.prj, &self.session.analysis.issues).into_iter().map(|a| a.id.clone()).collect();
        let (mut export, mut import) = (false, false);
        let s = &mut self.session.prj.check_settings;
        ui.scope(|ui| {
            ui.horizontal(|ui| {
                export = ui.button(tr("Export check scheme…")).on_hover_text(tr("Checks switched off and message levels as a file for other projects")).clicked();
                import = ui.button(tr("Import check scheme…")).clicked();
            });
            egui::CollapsingHeader::new(tr("Checks")).default_open(true).show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("checks").max_height(220.0).show(ui, |ui| {
                    for (name, title) in &checks {
                        let mut on = !s.disabled.contains(name);
                        if ui.checkbox(&mut on, title).on_hover_text(name).changed() {
                            s.disabled.retain(|d| d != name);
                            if !on {
                                s.disabled.push(name.clone());
                            }
                        }
                    }
                });
            });
            egui::CollapsingHeader::new(tr_fmt!("Changed message levels ({})", s.levels.len())).show(ui, |ui| {
                let mut remove = None;
                for (code, level) in &s.levels {
                    ui.horizontal(|ui| {
                        ui.monospace(code);
                        ui.label(level_title(*level));
                        if ui.small_button("🗑").on_hover_text(tr("As defined by the check")).clicked() {
                            remove = Some(code.clone());
                        }
                    });
                }
                if let Some(c) = remove {
                    s.levels.remove(&c);
                }
                if s.levels.is_empty() {
                    ui.weak(tr("Right click a message in the Check tab to change its level."));
                }
            });
            egui::CollapsingHeader::new(tr_fmt!("Accepted messages ({})", s.accepted.len())).show(ui, |ui| {
                let mut remove = None;
                egui::ScrollArea::vertical().id_salt("accepted").max_height(220.0).show(ui, |ui| {
                    for (i, a) in s.accepted.iter_mut().enumerate() {
                        ui.horizontal_wrapped(|ui| {
                            if ui.small_button("🗑").on_hover_text(tr("Take the acceptance back")).clicked() {
                                remove = Some(i);
                            }
                            let gone = stale.contains(&a.id);
                            let msg = if gone { RichText::new(&a.message).weak().strikethrough() } else { RichText::new(&a.message) };
                            ui.label(msg).on_hover_text(if gone { tr("No longer reported") } else { tr("Still reported") });
                            ui.weak(format!("{} {}", a.user, a.date));
                        });
                        ui.add(egui::TextEdit::singleline(&mut a.note).hint_text(tr("Note")).desired_width(f32::INFINITY));
                    }
                });
                if let Some(i) = remove {
                    s.accepted.remove(i);
                }
                if !stale.is_empty() && ui.button(tr_fmt!("Remove the {} messages no longer reported", stale.len())).clicked() {
                    s.accepted.retain(|a| !stale.contains(&a.id));
                }
            });
        });
        if export {
            self.export(ctx, "check_scheme.json".into(), (tr("Check scheme (JSON)"), "json"), checks::scheme_json(&self.session.prj));
        }
        if import {
            self.io.open(ctx, crate::io::Purpose::CheckScheme);
        }
    }
}
