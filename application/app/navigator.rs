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
            let n_err = self.session.analysis.issues.iter().filter(|i| i.severity == Severity::Error && !i.accepted).count();
            let label = if n_err > 0 { RichText::new(tr_fmt!("Check ({})", n_err)).color(Color32::from_rgb(220, 60, 60)) } else { RichText::new(tr("Check")) };
            ui.selectable_value(&mut self.left_tab, LeftTab::Check, label);
            let open = self.session.prj.review.live_comments().filter(|c| !c.status.is_done()).count();
            let label = if open > 0 { tr_fmt!("Review ({})", open) } else { tr("Review").to_string() };
            ui.selectable_value(&mut self.left_tab, LeftTab::Review, label);
            ui.selectable_value(&mut self.left_tab, LeftTab::Unplaced, tr("Unplaced")).on_hover_text(tr("PLC channels and functions of the assigned parts that are not in the schematic yet"));
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
                    ui.toggle_value(&mut self.page_tree, "🗄").on_hover_text(tr("Group the pages by structure"));
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
                    let mut action = PageAction::None;
                    let shown: Vec<usize> = (0..self.session.prj.pages.len())
                        .filter(|&i| i == self.session.page || area.as_ref().is_none_or(|w| w.shows_page(&self.session.prj.pages[i])))
                        .collect();
                    if self.page_tree {
                        // Groups by structure, in the order they first occur.
                        let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
                        for &i in &shown {
                            let st = structure::page_structure(&self.session.prj.pages[i]);
                            match groups.iter_mut().find(|g| g.0 == st) {
                                Some(g) => g.1.push(i),
                                None => groups.push((st, vec![i])),
                            }
                        }
                        for (st, pages) in groups {
                            let title = if st.is_empty() { tr("(no structure)").to_string() } else { self.structure_title(&st) };
                            egui::CollapsingHeader::new(title).id_salt(("ptree", st.as_str())).default_open(true).show(ui, |ui| {
                                for i in pages {
                                    action = std::mem::replace(&mut action, PageAction::None).or(self.page_row(ui, i));
                                }
                            });
                        }
                    } else {
                        for i in shown {
                            action = std::mem::replace(&mut action, PageAction::None).or(self.page_row(ui, i));
                        }
                    }
                    let (goto, lock_action) = match action {
                        PageAction::Goto(i) => (Some(i), None),
                        PageAction::Properties(i) => {
                            self.page_props = Some(i);
                            (None, None)
                        }
                        PageAction::Lock(t, r) => (None, Some((t, r))),
                        PageAction::None => (None, None),
                    };
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
                                    let name = self.session.prj.symbol(&inst.sym).map(|d| tr(d.name)).unwrap_or("?");
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
            LeftTab::Check => self.check_tab(ui),
            LeftTab::Unplaced => self.unplaced_tab(ui),
        }
    }
}

/// What a row of the page list asks for.
enum PageAction {
    None,
    Goto(usize),
    Properties(usize),
    Lock(openecraft_core::collab::Target, bool),
}

impl PageAction {
    fn or(self, other: PageAction) -> PageAction {
        match self {
            PageAction::None => other,
            a => a,
        }
    }
}

impl App {
    /// `=A1+CAB1  Pump station` (descriptions from the structure management).
    fn structure_title(&self, st: &str) -> String {
        let dt = Dt::parse_structure(st);
        let s = &self.session.prj.structures;
        let desc: Vec<String> = openecraft_core::structure::Aspect::ALL
            .into_iter()
            .filter_map(|a| s.entry(a, dt.get(a)).map(|e| e.description.clone()))
            .filter(|d| !d.is_empty())
            .collect();
        if desc.is_empty() {
            st.to_string()
        } else {
            format!("{st}  {}", desc.join(", "))
        }
    }

    /// One page of the page list.
    fn page_row(&self, ui: &mut egui::Ui, i: usize) -> PageAction {
        let pg = &self.session.prj.pages[i];
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
        let name = if self.page_tree { pg.name.clone() } else { structure::page_full_name(pg) };
        text.append(&format!("{icon} {name}   {}", pg.description), 0.0, style.clone());
        if let Some((mark, color)) = self.page_lock_mark(pg.id) {
            text.append(&mark, 8.0, egui::TextFormat { color, ..style });
        }
        let r = ui.selectable_label(i == self.session.page, text);
        let mut action = PageAction::None;
        if r.clicked() {
            action = PageAction::Goto(i);
        }
        if self.remote.shared.is_some() {
            r.context_menu(|ui| {
                if let Some((t, release)) = self.page_lock_menu(ui, pg.id) {
                    action = PageAction::Lock(t, release);
                }
            });
        }
        if r.double_clicked() {
            action = PageAction::Properties(i);
        }
        action
    }
}

impl App {
    /// Unplaced PLC channels and part functions, grouped by device; a click places one.
    fn unplaced_tab(&mut self, ui: &mut egui::Ui) {
        let prj = &self.session.prj;
        let channels = openecraft_core::unplaced::plc_channels(prj);
        let functions = openecraft_core::unplaced::device_functions(prj, Some(&self.parts_db));
        let mut start = None;
        ui.weak(tr("Drag an entry into the page, or click it and then click into the page."));
        egui::ScrollArea::vertical().id_salt("unplaced").auto_shrink([false, false]).show(ui, |ui| {
            for (title, list) in [(tr("PLC channels"), &channels), (tr("Functions of the parts"), &functions)] {
                egui::CollapsingHeader::new(RichText::new(tr_fmt!("{} ({})", title, list.len())).strong()).id_salt(title).default_open(true).show(ui, |ui| {
                    let mut tags: Vec<&String> = list.iter().map(|u| &u.tag).collect();
                    tags.dedup();
                    for tag in tags {
                        let items: Vec<&openecraft_core::unplaced::Unplaced> = list.iter().filter(|u| &u.tag == tag).collect();
                        egui::CollapsingHeader::new(RichText::new(format!("{tag}  ({})", items.len())).monospace()).id_salt((title, tag)).show(ui, |ui| {
                            for u in items {
                                let payload = crate::editor::PlacePayload { symbol: u.symbol, tag: u.tag.clone(), label: u.label.clone(), pins: u.pins.clone() };
                                let r = ui.dnd_drag_source(ui.make_persistent_id(("unplaced", &u.tag, &u.title)), payload, |ui| ui.link(&u.title).on_hover_text(tr("Click, or drag it into the page")));
                                if r.inner.clicked() {
                                    start = Some(u.clone());
                                }
                            }
                        });
                    }
                    if list.is_empty() {
                        ui.weak(tr("(no entries)"));
                    }
                });
            }
        });
        if let Some(u) = start {
            self.editor.set_tool(Tool::Place(u.symbol));
            self.editor.place_tag = Some(u.tag.clone());
            self.editor.place_once = Some((u.label, u.pins));
        }
    }
}
