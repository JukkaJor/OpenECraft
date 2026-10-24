//! The navigator on the left: pages, devices, connections, cables, layers, check and
//! review tabs.

use super::*;

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn navigator(&mut self, ui: &mut egui::Ui, idx: &Index) {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.left_tab, LeftTab::Pages, tr("Pages"));
            ui.selectable_value(&mut self.left_tab, LeftTab::Devices, tr("Devices"));
            ui.selectable_value(&mut self.left_tab, LeftTab::Connections, tr("Connections"));
            ui.selectable_value(&mut self.left_tab, LeftTab::Cables, tr("Cables"));
            ui.selectable_value(&mut self.left_tab, LeftTab::Layers, tr("Layers"));
            let n_err = self.session.analysis.issues.iter().filter(|i| i.severity == Severity::Error).count();
            let label = if n_err > 0 { RichText::new(tr_fmt!("Check ({})", n_err)).color(Color32::from_rgb(220, 60, 60)) } else { RichText::new(tr("Check")) };
            ui.selectable_value(&mut self.left_tab, LeftTab::Check, label);
            let open = self.session.prj.review.live_comments().filter(|c| !c.status.is_done()).count();
            let label = if open > 0 { tr_fmt!("Review ({})", open) } else { tr("Review").to_string() };
            ui.selectable_value(&mut self.left_tab, LeftTab::Review, label);
        });
        let areas = &self.session.prj.structures.work_areas;
        if !areas.is_empty() || !self.work_area.is_empty() {
            ui.horizontal(|ui| {
                ui.label(tr("Work area"));
                let shown = if self.work_area.is_empty() { tr("(whole project)").to_string() } else { self.work_area.clone() };
                egui::ComboBox::from_id_salt("work_area").selected_text(shown).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.work_area, String::new(), tr("(whole project)"));
                    for w in areas {
                        ui.selectable_value(&mut self.work_area, w.name.clone(), &w.name).on_hover_text(w.filter.structure());
                    }
                });
            });
        }
        let area = self.session.prj.structures.work_area(&self.work_area).cloned();
        ui.separator();
        match self.left_tab {
            LeftTab::Pages => {
                ui.horizontal(|ui| {
                    if ui.button(tr("+ Page")).on_hover_text(tr("New schematic page after the current one")).clicked() {
                        self.add_page(PageKind::Schematic);
                    }
                    if ui.button("⚙").on_hover_text(tr("Page properties")).clicked() {
                        self.page_props = Some(self.session.page);
                    }
                    if ui.add_enabled(self.session.page > 0, egui::Button::new("⏶")).clicked() {
                        self.session.prj.pages.swap(self.session.page, self.session.page - 1);
                        self.session.page -= 1;
                    }
                    if ui.add_enabled(self.session.page + 1 < self.session.prj.pages.len(), egui::Button::new("⏷")).clicked() {
                        self.session.prj.pages.swap(self.session.page, self.session.page + 1);
                        self.session.page += 1;
                    }
                });
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let mut goto = None;
                    let mut lock_action = None;
                    for (i, pg) in self.session.prj.pages.iter().enumerate() {
                        if area.as_ref().is_some_and(|w| !w.shows_page(pg)) && i != self.session.page {
                            continue;
                        }
                        let icon = match pg.kind {
                            PageKind::Schematic => "🗋",
                            PageKind::Cover => "🏷",
                            PageKind::Report(_) => "📊",
                            PageKind::Mounting(_) => "🗄",
                            PageKind::Overview => "🗺",
                            PageKind::Graphic => "✏",
                            PageKind::Document => "📎",
                        };
                        let mut text = egui::text::LayoutJob::default();
                        let style = egui::TextFormat { color: ui.visuals().text_color(), ..Default::default() };
                        text.append(&format!("{icon} {}   {}", structure::page_full_name(pg), pg.description), 0.0, style.clone());
                        if let Some((mark, color)) = self.page_lock_mark(pg.id) {
                            text.append(&mark, 8.0, egui::TextFormat { color, ..style });
                        }
                        let r = ui.selectable_label(i == self.session.page, text);
                        if r.clicked() {
                            goto = Some(i);
                        }
                        if self.remote.shared.is_some() {
                            r.context_menu(|ui| {
                                if let Some(a) = self.page_lock_menu(ui, pg.id) {
                                    lock_action = Some(a);
                                }
                            });
                        }
                        if r.double_clicked() {
                            self.page_props = Some(i);
                        }
                    }
                    if let Some(i) = goto {
                        self.goto(i, 0);
                    }
                    if let Some((target, release)) = lock_action {
                        if release {
                            self.save_remote(vec![target]);
                        } else {
                            self.remote.lock(vec![target]);
                        }
                    }
                });
            }
            LeftTab::Devices => {
                ui.add(egui::TextEdit::singleline(&mut self.device_filter).hint_text(tr("Filter devices…")));
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let mut jump = None;
                    let f = self.device_filter.to_lowercase();
                    // Devices grouped by structure (=function+location).
                    let mut groups: Vec<(String, Vec<&String>)> = Vec::new();
                    let tags = idx.sorted_tags().into_iter().filter(|t| area.as_ref().is_none_or(|w| w.shows_device(t)));
                    for tag in tags.filter(|t| f.is_empty() || t.to_lowercase().contains(&f)) {
                        let st = Dt::parse(tag).structure();
                        match groups.iter_mut().find(|g| g.0 == st) {
                            Some(g) => g.1.push(tag),
                            None => groups.push((st, vec![tag])),
                        }
                    }
                    groups.sort_by(|a, b| natural_cmp(&a.0, &b.0));
                    let structured = groups.iter().any(|g| !g.0.is_empty());
                    let prj = &self.session.prj;
                    let placing = self.editor.place_tag.clone();
                    let mut menu: Option<(String, drawing_ui::DeviceAction)> = None;
                    let mut device = |ui: &mut egui::Ui, tag: &String| {
                        let dev = &idx.devices[tag];
                        let title = if structured { Dt::parse(tag).name } else { tag.clone() };
                        let title = if placing.as_deref() == Some(tag.as_str()) { format!("{title}  ✏") } else { title };
                        let header = egui::CollapsingHeader::new(RichText::new(title).monospace().strong()).id_salt(tag).show(ui, |ui| {
                            for (kind, locs) in [(tr("Main"), &dev.mains), (tr("Aux"), &dev.auxs), ("", &dev.others)] {
                                for loc in locs {
                                    let inst = prj.pages[loc.page].symbols.iter().find(|s| s.id == loc.sym_id);
                                    let Some(inst) = inst else { continue };
                                    let name = symbols::get(&inst.sym).map(|d| tr(d.name)).unwrap_or("?");
                                    let extra = if inst.label.is_empty() { inst.pins.join("-") } else { inst.label.clone() };
                                    let text = format!("{} {kind} {name} {extra}", loc.reference(prj));
                                    if ui.link(text.trim()).clicked() {
                                        jump = Some((loc.page, loc.sym_id));
                                    }
                                }
                            }
                        });
                        header.header_response.context_menu(|ui| {
                            if let Some(a) = drawing_ui::device_menu(ui, placing.as_deref() == Some(tag.as_str())) {
                                menu = Some((tag.clone(), a));
                            }
                        });
                    };
                    for (st, tags) in &groups {
                        if structured {
                            let title = if st.is_empty() { "(no structure)" } else { st.as_str() };
                            egui::CollapsingHeader::new(RichText::new(title).strong()).id_salt(("structure", st)).default_open(true).show(ui, |ui| {
                                for tag in tags {
                                    device(ui, tag);
                                }
                            });
                        } else {
                            for tag in tags {
                                device(ui, tag);
                            }
                        }
                    }
                    if idx.devices.is_empty() {
                        ui.weak(tr("No devices yet. Pick a symbol from the library on the right and click into the page."));
                    }
                    if let Some((tag, a)) = menu {
                        self.device_action(&tag, a);
                    }
                    if let Some((pg, id)) = jump {
                        self.goto(pg, id);
                    }
                });
            }
            LeftTab::Review => {
                egui::ScrollArea::vertical().id_salt("review_tab").auto_shrink([false, false]).show(ui, |ui| self.review_tab(ui));
            }
            LeftTab::Connections => self.connections_tab(ui),
            LeftTab::Cables => self.cables_tab(ui),
            LeftTab::Layers => self.layers_tab(ui),
            LeftTab::Check => {
                let issues = self.session.analysis.issues.clone();
                let count = |s| issues.iter().filter(|i| i.severity == s).count();
                ui.label(format!(
                    "{} errors, {} warnings, {} notes",
                    count(Severity::Error),
                    count(Severity::Warning),
                    count(Severity::Info)
                ));
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let mut jump = None;
                    for issue in &issues {
                        let (icon, color) = match issue.severity {
                            Severity::Error => ("⛔", Color32::from_rgb(220, 60, 60)),
                            Severity::Warning => ("⚠", Color32::from_rgb(220, 160, 0)),
                            Severity::Info => ("ℹ", ui.visuals().weak_text_color()),
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(icon).color(color));
                            let r = ui.add(egui::Label::new(&issue.message).sense(egui::Sense::click()));
                            if r.on_hover_text(tr("Click to show")).clicked() {
                                jump = issue.target;
                            }
                        });
                    }
                    if issues.is_empty() {
                        ui.label(tr("✔ No problems found."));
                    }
                    if let Some((pg, id)) = jump {
                        self.goto(pg, id);
                    }
                });
            }
        }
    }
}
