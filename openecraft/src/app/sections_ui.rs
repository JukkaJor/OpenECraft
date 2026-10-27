//! Page sections in the editor: the section tree of the Pages navigator (a page is
//! dragged onto a section to move it there) and the section properties window.

use super::*;
use super::navigator::PageAction;
use openecraft_core::sections;

/// Payload of a dragged page row: the page index.
#[derive(Clone, Copy)]
pub(super) struct PageDrag(pub usize);

/// What the section tree or the section properties ask for.
pub(super) enum SectionAction {
    NewPage(u64),
    NewSubsection(u64),
    Properties(u64),
    Move(u64, bool),
    Delete(u64),
    MovePage(usize, u64),
    Number(u64, String),
    Parent(u64, u64),
}

/// The Section row of the page properties: the section chosen instead of `current`.
pub(super) fn section_row(ui: &mut egui::Ui, choices: &[(u64, String)], current: u64, enabled: bool) -> Option<u64> {
    let mut out = None;
    ui.label(tr("Section"));
    let shown = choices.iter().find(|(id, _)| *id == current).map(|(_, t)| t.trim().to_string()).unwrap_or_default();
    ui.add_enabled_ui(enabled, |ui| {
        egui::ComboBox::from_id_salt("pgsection").selected_text(shown).show_ui(ui, |ui| {
            for (id, title) in choices {
                if ui.selectable_label(*id == current, title).clicked() && *id != current {
                    out = Some(*id);
                }
            }
        })
    })
    .response
    .on_hover_text(tr("The page name gets the section number"));
    ui.end_row();
    out
}

impl App {
    /// The pages of the top level, then the sections with their pages and subsections.
    pub(super) fn section_tree(&self, ui: &mut egui::Ui, shown: &[usize]) -> PageAction {
        let prj = &self.session.prj;
        let mut action = PageAction::None;
        for &i in shown.iter().filter(|&&i| sections::of_page(prj, i) == 0) {
            action = action.or(self.page_row(ui, i));
        }
        for id in sections::children(prj, 0) {
            action = action.or(self.section_node(ui, id, shown));
        }
        action
    }

    fn section_node(&self, ui: &mut egui::Ui, id: u64, shown: &[usize]) -> PageAction {
        let prj = &self.session.prj;
        let header = egui::CollapsingHeader::new(RichText::new(sections::title(prj, id)).strong()).id_salt(("section", id)).default_open(true).show(ui, |ui| {
            let mut action = PageAction::None;
            for &i in shown.iter().filter(|&&i| sections::of_page(prj, i) == id) {
                action = action.or(self.page_row(ui, i));
            }
            for c in sections::children(prj, id) {
                action = action.or(self.section_node(ui, c, shown));
            }
            action
        });
        let mut action = header.body_returned.unwrap_or(PageAction::None);
        let r = &header.header_response;
        if r.dnd_hover_payload::<PageDrag>().is_some() {
            ui.painter().rect_stroke(r.rect, 2.0, ui.visuals().selection.stroke, egui::StrokeKind::Inside);
        }
        if let Some(p) = r.dnd_release_payload::<PageDrag>() {
            action = action.or(PageAction::Section(SectionAction::MovePage(p.0, id)));
        }
        let siblings = sections::get(prj, id).map(|s| sections::children(prj, s.parent)).unwrap_or_default();
        let (first, last) = (siblings.first() == Some(&id), siblings.last() == Some(&id));
        r.context_menu(|ui| {
            let mut pick = |ui: &mut egui::Ui, enabled: bool, text: &str, a: SectionAction| {
                if ui.add_enabled(enabled, egui::Button::new(text)).clicked() {
                    action = std::mem::replace(&mut action, PageAction::None).or(PageAction::Section(a));
                    ui.close();
                }
            };
            pick(ui, true, tr("New page in this section"), SectionAction::NewPage(id));
            pick(ui, true, tr("New subsection"), SectionAction::NewSubsection(id));
            pick(ui, true, tr("Section properties…"), SectionAction::Properties(id));
            ui.separator();
            pick(ui, !first, tr("Move up"), SectionAction::Move(id, true));
            pick(ui, !last, tr("Move down"), SectionAction::Move(id, false));
            pick(ui, true, tr("Delete section"), SectionAction::Delete(id));
        });
        action
    }

    /// The sections a page can move to: (id, indented title); 0 is the top level.
    pub(super) fn section_choices(&self) -> Vec<(u64, String)> {
        let prj = &self.session.prj;
        let mut out = vec![(0, tr("(top level)").to_string())];
        out.extend(sections::tree(prj).into_iter().map(|(id, depth)| (id, format!("{}{}", "    ".repeat(depth), sections::title(prj, id)))));
        out
    }

    /// Carries out a section action; the current page stays the same page.
    pub(super) fn section_action(&mut self, ctx: &egui::Context, a: SectionAction) {
        let prj = &mut self.session.prj;
        let id_of = |prj: &Project, i: Option<usize>| i.and_then(|i| prj.pages.get(i)).map(|p| p.id);
        let (current, props) = (id_of(prj, Some(self.session.page)), id_of(prj, self.page_props));
        let result = match a {
            SectionAction::NewPage(id) => {
                let at = ops::add_page_in(prj, PageKind::Schematic, id);
                self.goto(at, 0);
                return;
            }
            SectionAction::NewSubsection(id) => {
                self.section_props = Some(sections::add(prj, id, ""));
                Ok(())
            }
            SectionAction::Properties(id) => {
                self.section_props = Some(id);
                Ok(())
            }
            SectionAction::Move(id, up) => {
                sections::move_section(prj, id, up);
                Ok(())
            }
            SectionAction::Delete(id) => sections::delete(prj, id),
            SectionAction::MovePage(page, id) => sections::move_page(prj, page, id).map(|_| ()),
            SectionAction::Number(id, n) => sections::set_number(prj, id, &n),
            SectionAction::Parent(id, parent) => sections::set_parent(prj, id, parent),
        };
        let prj = &self.session.prj;
        let index = |id: Option<u64>| id.and_then(|id| prj.pages.iter().position(|p| p.id == id));
        if let Some(i) = index(current) {
            self.session.page = i;
        }
        self.page_props = index(props);
        if let Err(e) = result {
            self.notify(ctx, tr(e));
        }
    }

    /// Adds a section beside the one of the current page with a first page in it, and
    /// opens its properties.
    pub(super) fn new_section(&mut self) {
        let prj = &mut self.session.prj;
        let parent = sections::get(prj, sections::of_page(prj, self.session.page)).map_or(0, |s| s.parent);
        let id = sections::add(prj, parent, "");
        let at = ops::add_page_in(prj, PageKind::Schematic, id);
        self.goto(at, 0);
        self.section_props = Some(id);
    }

    /// Number, description and parent of a section.
    pub(super) fn section_properties_window(&mut self, ctx: &egui::Context) {
        let Some(id) = self.section_props else { return };
        let Some(s) = sections::get(&self.session.prj, id).cloned() else {
            self.section_props = None;
            return;
        };
        let choices: Vec<(u64, String)> = {
            let below = sections::subtree(&self.session.prj, id);
            self.section_choices().into_iter().filter(|(c, _)| !below.contains(c)).collect()
        };
        let mut open = true;
        let mut action = None;
        egui::Window::new(tr("Section properties")).open(&mut open).resizable(false).show(ctx, |ui| {
            let prj = &mut self.session.prj;
            let key = ui.make_persistent_id(("section_number", id));
            let mut number: String = ui.data_mut(|d| d.get_temp(key)).unwrap_or_else(|| s.number.clone());
            egui::Grid::new("secprops").num_columns(2).show(ui, |ui| {
                ui.label(tr("Number"));
                ui.horizontal(|ui| {
                    let r = ui.add(egui::TextEdit::singleline(&mut number).desired_width(80.0));
                    let changed = number.trim() != s.number;
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.add_enabled(changed, egui::Button::new(tr("Apply"))).on_hover_text(tr("The names of its pages follow")).clicked() || (changed && enter) {
                        action = Some(SectionAction::Number(id, number.clone()));
                    }
                });
                ui.end_row();
                ui.label(tr("Description"));
                if let Some(sec) = prj.sections.iter_mut().find(|x| x.id == id) {
                    ui.text_edit_singleline(&mut sec.description);
                }
                ui.end_row();
                ui.label(tr("Parent section"));
                let parent = sections::get(prj, s.parent).map_or(0, |p| p.id);
                let shown = choices.iter().find(|(c, _)| *c == parent).map(|(_, t)| t.trim().to_string()).unwrap_or_default();
                egui::ComboBox::from_id_salt("secparent").selected_text(shown).show_ui(ui, |ui| {
                    for (c, title) in &choices {
                        if ui.selectable_label(*c == parent, title).clicked() && *c != parent {
                            action = Some(SectionAction::Parent(id, *c));
                        }
                    }
                });
                ui.end_row();
            });
            let pages = prj.pages.iter().filter(|p| p.section == id).count();
            ui.weak(tr_fmt!("{} pages; their names start with {}", pages, sections::prefix(prj, id)));
            ui.separator();
            if ui.button(tr("Delete section")).on_hover_text(tr("Its pages and subsections go to the section above")).clicked() {
                action = Some(SectionAction::Delete(id));
            }
            if action.is_some() {
                ui.data_mut(|d| d.remove::<String>(key));
            } else {
                ui.data_mut(|d| d.insert_temp(key, number));
            }
        });
        if let Some(a) = action {
            self.section_action(ctx, a);
        }
        if !open {
            self.section_props = None;
        }
    }
}
