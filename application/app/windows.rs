//! The windows of the main application (reports, project and page properties, parts,
//! help) and the keyboard shortcuts.

use super::*;
use crate::shortcuts::{Action, Shortcut};

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn windows(&mut self, ctx: &egui::Context) {
        self.shortcuts_window(ctx);
        self.export_form_window(ctx);
        self.reports_window(ctx);
        self.project_properties_window(ctx);
        self.parts_management_and_part_selection_window(ctx);
        self.page_properties_window(ctx);
        if self.show_help {
            let mut open = true;
            egui::Window::new(tr("OpenECraft – usage")).open(&mut open).default_width(520.0).show(ctx, |ui| {
                ui.label(RichText::new(tr("Workflow")).strong());
                ui.label(tr("1. Pick a symbol in the library (right) and click into the page to place it. Device tags (-K1, -S2 …) are assigned automatically."));
                ui.label(tr("2. Select a coil, then place contacts: they are assigned to that coil and numbered (13/14, 21/22 …). Cross references and the contact image update live."));
                ui.label(tr("3. Draw wires with the wire tool (W). Wires end automatically on connection points. T-junctions get a connection dot."));
                ui.label(tr("4. Potentials (L1, +24V …) and interruption points name the nets across pages. Select a wire to highlight its potential."));
                ui.label(tr("5. Assign parts in the property panel; generate reports (device list, BOM, terminal diagram, connection list, PLC list) and insert them as report pages."));
                ui.label(tr("6. Structure: give pages a function (=A1) and location (+CAB1) in the page properties. Devices placed there get it (=A1+CAB1-K1); on the page only the differing part is shown."));
                ui.label(tr("7. Cables: define them in Project › Cables…, select wires and use Assign to cable. Wire numbers: Project › Wire numbering…, or type a number in the wire properties."));
                ui.label(tr("8. Macros: select items › Edit › Save selection as macro; insert them from Insert › Macro…. Placeholders {NAME} are filled from value sets."));
                ui.label(tr("9. Project › Terminal strips / PLC overview / Revisions: strip order, jumpers and spares; PLC addresses and symbol tables; revisions with change marks and released (locked) pages."));
                ui.label(tr("10. Project › Symbol editor: draw your own symbols (or copy built-in ones) with connection points and function; they are saved in the project and can be exchanged as library files."));
                ui.label(tr("11. Project › Title blocks and report forms: own title blocks with placeholders ({project}, {page}, {prop:Name} …) and the columns of report pages."));
                ui.label(tr("Reports › Report blocks: generate report pages per strip, plug, cable or device with a named scheme (filter, sort order, grouping) and update them; output report pages into another project."));
                ui.label(tr("12. Panel layout: busbars and cutouts; the enclosure properties export the drilling pattern (NC data as DXF / DWG / CSV). The production wire list contains stripping lengths and ferrules."));
                ui.label(tr("13. Harnesses: Project › Harnesses…; select the wires between two devices › Assign to harness, or per connection in the wire properties. Harness data report with lengths, connectors and bundle diameter, KBL export."));
                ui.label(tr("14. Door: switch the panel front view to Door and place command / signal devices there; their door cutouts are generated. Collisions are checked with the parts' 3D models."));
                ui.label(tr("15. Connection definition points on a wire set number, colour, cross-section and core of the whole connection; a cable definition line across wires makes them cores of a cable. Auto-connect joins facing connection points, Route (A) leads wires around symbols."));
                ui.label(tr("16. Graphics: circles, ellipses, arcs, polygons, dimensions and images on layers (navigator › Layers: visible, locked, printed). Drag the tag of a selected symbol to place it; further properties are placed in its properties."));
                ui.separator();
                ui.label(RichText::new(tr("Shortcuts")).strong());
                egui::Grid::new("keys").striped(true).show(ui, |ui| {
                    let sc = &self.editor.shortcuts;
                    let configured: Vec<(String, &str)> = Action::ALL.iter().map(|a| (sc.text(*a), a.title())).filter(|(k, _)| !k.is_empty()).collect();
                    let fixed: Vec<(String, &str)> = [
                        (tr("Esc"), tr("Cancel / select tool / clear selection")),
                        (tr("Del"), tr("Delete selection")),
                        (tr("Arrows"), tr("Move selection by one grid step")),
                        (tr("Ctrl+C / X / V"), tr("Copy / cut / paste (devices get new tags)")),
                        (tr("Wheel / right or middle drag"), tr("Zoom / pan")),
                        (tr("Shift + click / drag"), tr("Add to selection")),
                    ]
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect();
                    for (k, v) in configured.into_iter().chain(fixed) {
                        ui.label(RichText::new(k).monospace());
                        ui.label(v);
                        ui.end_row();
                    }
                });
            });
            self.show_help = open;
        }
    }

    /// Reports.
    pub(super) fn reports_window(&mut self, ctx: &egui::Context) {
    if self.show_reports {
        let mut open = true;
        let mut jump = None;
        let mut export = None;
        let mut export_xlsx = None;
        let mut add_page = None;
        egui::Window::new(tr("Reports")).open(&mut open).default_size([900.0, 520.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (k, title) in self.report_list() {
                    ui.selectable_value(&mut self.report_kind, k, title);
                }
            });
            ui.separator();
            let kind = self.report_kind;
            ui.horizontal(|ui| {
                if ui.button(tr("⬇ Export CSV")).clicked() {
                    export = Some(kind);
                }
                if ui.button(tr("⬇ Export Excel")).clicked() {
                    export_xlsx = Some(kind);
                }
                if ui.button(tr("➕ Insert as report page")).clicked() {
                    add_page = Some(kind);
                }
                if ui.button(tr("Form…")).on_hover_text(tr("Columns, filter, sort order and rows of the report pages")).clicked() {
                    self.forms_ui.open_report(kind);
                }
                if ui.button(tr("Report blocks…")).on_hover_text(tr("Generate report pages per strip, plug, cable or device")).clicked() {
                    self.report_blocks_ui.open = true;
                }
            });
            let Some(table) = self.session.analysis.report(&self.session.prj, kind) else { return };
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("report").striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                    for h in table.header_texts() {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    for (row, target) in table.rows.iter().zip(&table.targets) {
                        for (i, cell) in row.iter().enumerate() {
                            if i == 0 && target.is_some() {
                                if ui.link(cell).clicked() {
                                    jump = *target;
                                }
                            } else {
                                ui.label(cell);
                            }
                        }
                        ui.end_row();
                    }
                });
                if table.rows.is_empty() {
                    ui.weak(tr("(no entries)"));
                }
            });
        });
        self.show_reports = open;
        if let Some((pg, id)) = jump {
            self.goto(pg, id);
        }
        if let Some(kind) = export_xlsx {
            if let Some(t) = self.session.analysis.report(&self.session.prj, kind) {
                let title = self.report_title(kind);
                let bytes = openecraft_core::xlsx::write(&[(title.clone(), t.sheet_rows())]);
                let name = format!("{}.xlsx", title.replace(' ', "_").to_lowercase());
                self.export_bytes(ctx, name, ("Excel", "xlsx"), &bytes);
            }
        }
        if let Some(kind) = export {
            if let Some(t) = self.session.analysis.report(&self.session.prj, kind) {
                let csv = t.to_csv();
                let name = format!("{}.csv", self.report_title(kind).replace(' ', "_").to_lowercase());
                self.export(ctx, name, ("CSV", "csv"), csv);
            }
        }
        if let Some(kind) = add_page {
            self.add_page(PageKind::Report(kind));
        }
    }

    }

    /// Project properties.
    pub(super) fn project_properties_window(&mut self, ctx: &egui::Context) {
    if self.show_project {
        let mut open = true;
        let prj_tb_names = openecraft_core::forms::names(&self.session.prj);
        egui::Window::new(tr("Project properties")).open(&mut open).resizable(false).show(ctx, |ui| {
            let info = &mut self.session.prj.info;
            egui::Grid::new("prj").num_columns(2).show(ui, |ui| {
                ui.label(tr("Project name"));
                ui.text_edit_singleline(&mut info.name);
                ui.end_row();
                ui.label(tr("Description"));
                ui.text_edit_multiline(&mut info.description);
                ui.end_row();
                ui.label(tr("Company"));
                ui.text_edit_singleline(&mut info.company);
                ui.end_row();
                ui.label(tr("Created by"));
                ui.text_edit_singleline(&mut info.creator);
                ui.end_row();
                ui.label(tr("Date"));
                ui.text_edit_singleline(&mut info.date);
                ui.end_row();
                ui.label(tr("Revision"));
                ui.text_edit_singleline(&mut info.revision);
                ui.end_row();
                ui.label(tr("Title block"));
                let forms = &mut self.session.prj.forms;
                let current = if forms.default_title_block.is_empty() { openecraft_core::forms::STANDARD.to_string() } else { forms.default_title_block.clone() };
                egui::ComboBox::from_id_salt("prjtb").selected_text(&current).show_ui(ui, |ui| {
                    for n in prj_tb_names.iter().cloned() {
                        if ui.selectable_label(n == current, &n).clicked() {
                            forms.default_title_block = if n == openecraft_core::forms::STANDARD { String::new() } else { n };
                        }
                    }
                });
                ui.end_row();
            });
            ui.separator();
            ui.label(RichText::new(tr("Project properties")).strong()).on_hover_text(tr("Shown in title blocks with {prop:Name}"));
            let props = &mut self.session.prj.info.properties;
            let mut remove = None;
            let mut rename = None;
            egui::Grid::new("prjprops").num_columns(3).show(ui, |ui| {
                for (k, v) in props.iter_mut() {
                    let mut key = k.clone();
                    if ui.add(egui::TextEdit::singleline(&mut key).desired_width(120.0)).changed() {
                        rename = Some((k.clone(), key));
                    }
                    ui.add(egui::TextEdit::singleline(v).desired_width(200.0));
                    if ui.small_button("🗑").clicked() {
                        remove = Some(k.clone());
                    }
                    ui.end_row();
                }
            });
            if let Some((old, new)) = rename {
                if !new.is_empty() && !props.contains_key(&new) {
                    if let Some(v) = props.remove(&old) {
                        props.insert(new, v);
                    }
                }
            }
            if let Some(k) = remove {
                props.remove(&k);
            }
            if ui.button(tr("➕ Property")).clicked() {
                let mut n = props.len() + 1;
                while props.contains_key(&format!("Property{n}")) {
                    n += 1;
                }
                props.insert(format!("Property{n}"), String::new());
            }
        });
        self.show_project = open;
    }

    }

    /// Parts management and part selection.
    pub(super) fn parts_management_and_part_selection_window(&mut self, ctx: &egui::Context) {
    let mut actions = self.parts_ui.manager(ctx, &mut self.parts_db, &mut self.session.prj);
    actions.extend(self.parts_ui.portal(ctx, &mut self.parts_db));
    self.parts_actions(ctx, actions);
    self.parts_ui.selector(ctx, &self.parts_db, &mut self.session.prj);

    self.cables_window(ctx);
    self.rename_device_window(ctx);
    self.numbering_window(ctx);
    self.numbering_windows(ctx);
    self.check_settings_window(ctx);
    self.arrange_windows(ctx);
    self.xref_window(ctx);
    self.find_window(ctx);
    self.engineering_windows(ctx);
    self.compare_window(ctx);
    self.requests_window(ctx);
    self.symbol_editor_window(ctx);
    self.forms_window(ctx);
    self.report_blocks_window(ctx);
    self.harness_window(ctx);
    self.field_window(ctx);
    self.plugs_window(ctx);
    self.plc_hw_window(ctx);
    self.integration_window(ctx);
    self.process_window(ctx);
    self.electrical_window(ctx);
    self.singleline_window(ctx);
    self.languages_window(ctx);
    self.structure_window(ctx);
    self.properties_window(ctx);
    self.projects_window(ctx);
    self.save_template_window(ctx);
    self.tables_window(ctx);
    self.config_window(ctx);
    self.plugins_windows(ctx);

    }

    /// Page properties.
    pub(super) fn page_properties_window(&mut self, ctx: &egui::Context) {
    if let Some(pi) = self.page_props {
        let mut open = true;
        let mut apply_structure = None;
        let mut document_action = None;
        if pi < self.session.prj.pages.len() {
            let tb_names = openecraft_core::forms::names(&self.session.prj);
            let frame_names = openecraft_core::frame::names(&self.session.prj);
            let page_aspects = self.session.prj.structures.page_aspects();
            let structure_entries = self.session.prj.structures.entries.clone();
            let reports = self.report_list();
            egui::Window::new(tr("Page properties")).open(&mut open).resizable(false).show(ctx, |ui| {
                let pg = &mut self.session.prj.pages[pi];
                egui::Grid::new("pgprops").num_columns(2).show(ui, |ui| {
                    ui.label(tr("Page name / number"));
                    ui.text_edit_singleline(&mut pg.name);
                    ui.end_row();
                    ui.label(tr("Description"));
                    ui.text_edit_singleline(&mut pg.description);
                    ui.end_row();
                    ui.label(tr("Page type"));
                    egui::ComboBox::from_id_salt("pgkind").selected_text(pg.kind.label()).show_ui(ui, |ui| {
                        ui.selectable_value(&mut pg.kind, PageKind::Schematic, PageKind::Schematic.label());
                        ui.selectable_value(&mut pg.kind, PageKind::Overview, PageKind::Overview.label());
                        ui.selectable_value(&mut pg.kind, PageKind::Graphic, PageKind::Graphic.label());
                        ui.selectable_value(&mut pg.kind, PageKind::Document, PageKind::Document.label());
                        ui.selectable_value(&mut pg.kind, PageKind::Cover, PageKind::Cover.label());
                        for (k, title) in &reports {
                            ui.selectable_value(&mut pg.kind, PageKind::Report(*k), tr_fmt!("Report: {}", title));
                        }
                        for e in &self.session.prj.layout.enclosures {
                            ui.selectable_value(&mut pg.kind, PageKind::Mounting(e.id), tr_fmt!("Mounting layout {}", e.name));
                        }
                    });
                    ui.end_row();
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Title block"));
                    let shown = if pg.title_block.is_empty() { "(project default)".to_string() } else { pg.title_block.clone() };
                    egui::ComboBox::from_id_salt("pgtb").selected_text(shown).show_ui(ui, |ui| {
                        ui.selectable_value(&mut pg.title_block, String::new(), tr("(project default)"));
                        for n in &tb_names {
                            ui.selectable_value(&mut pg.title_block, n.clone(), n);
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Plot frame"));
                    let shown = if pg.frame.is_empty() { tr("(project default)").to_string() } else { openecraft_core::frame::display_name(&pg.frame) };
                    egui::ComboBox::from_id_salt("pgframe").selected_text(shown).show_ui(ui, |ui| {
                        ui.selectable_value(&mut pg.frame, String::new(), tr("(project default)"));
                        for n in &frame_names {
                            ui.selectable_value(&mut pg.frame, n.clone(), openecraft_core::frame::display_name(n));
                        }
                    });
                });
                if pg.kind == PageKind::Document {
                    ui.horizontal(|ui| {
                        match &pg.document {
                            Some(doc) => {
                                ui.label(RichText::new(&doc.name).strong());
                                ui.weak(format!("{}, {}", doc.kind().title(), openecraft_core::document::size_text(doc.size())));
                                if ui.button(tr("Save file…")).clicked() {
                                    document_action = Some(false);
                                }
                            }
                            None => {
                                ui.weak(tr("No document embedded"));
                            }
                        }
                        if ui.button(tr("Choose file…")).on_hover_text(tr("Embeds the file in the project (replaces the current one)")).clicked() {
                            document_action = Some(true);
                        }
                    });
                }
                if matches!(pg.kind, PageKind::Overview | PageKind::Graphic) {
                    ui.weak(tr("Symbols and wires on this page are drawn but not evaluated (no devices, connections or reports)."));
                } else if !pg.kind.is_drawing() && !pg.is_empty() {
                    ui.colored_label(Color32::from_rgb(220, 140, 0), tr("This page contains schematic content which is not evaluated on non-schematic pages."));
                }
                if pg.generated {
                    ui.checkbox(&mut pg.generated, tr("Generated by the configurator"))
                        .on_hover_text(tr("Replaced when the configurator generates again; switch off to keep the page"));
                }
                ui.separator();
                ui.label(RichText::new(tr("Structure (IEC 81346)")).strong());
                let key = ui.scope_id().with(("page_structure", pi));
                let (mut dt, mut move_devices): (Dt, bool) = ui.data_mut(|d| d.get_temp(key)).unwrap_or_else(|| (Dt::of_page(pg), true));
                let current = Dt::of_page(pg);
                egui::Grid::new("pgstruct").num_columns(3).show(ui, |ui| {
                    for a in Aspect::ALL.into_iter().filter(|a| page_aspects.contains(a) || !current.get(*a).is_empty()) {
                        ui.label(format!("{}  {}", a.title(), a.prefix()));
                        let hint = match a {
                            Aspect::Function => tr("e.g. A1, PUMP.2"),
                            Aspect::Installation => tr("e.g. HALL1, PLANT2"),
                            Aspect::Location => tr("e.g. CAB1, FIELD"),
                            Aspect::Product => tr("e.g. U1, U1-A2"),
                        };
                        ui.add(egui::TextEdit::singleline(dt.get_mut(a)).hint_text(hint).desired_width(140.0));
                        let defined: Vec<_> = structure_entries.iter().filter(|e| e.aspect == a).collect();
                        ui.add_enabled_ui(!defined.is_empty(), |ui| {
                            ui.menu_button("⏷", |ui| {
                                for e in defined {
                                    if ui.button(format!("{}  {}", a.format(&e.id), e.description)).clicked() {
                                        *dt.get_mut(a) = e.id.clone();
                                        ui.close();
                                    }
                                }
                            })
                            .response
                            .on_hover_text(tr("Identifiers of the structure management"));
                        });
                        ui.end_row();
                    }
                });
                ui.checkbox(&mut move_devices, tr("Move the devices of this page into the new structure"))
                    .on_hover_text(tr("Renames the devices (all their functions, also on other pages)"));
                let mut cleaned = Page::default();
                dt.apply_to(&mut cleaned);
                let changed = Dt::of_page(&cleaned) != current;
                if ui.add_enabled(changed, egui::Button::new(tr("Apply structure"))).clicked() {
                    apply_structure = Some((dt.clone(), move_devices));
                }
                ui.weak(tr("Devices placed on the page get its structure; their tags are shown without it."));
                ui.data_mut(|d| d.insert_temp(key, (dt, move_devices)));
            });
        } else {
            open = false;
        }
        match document_action {
            Some(true) => {
                self.pages_ui.replace_document = Some(pi);
                self.io.open(ctx, Purpose::Document);
            }
            Some(false) => self.save_document(ctx, pi),
            None => {}
        }
        if let Some((dt, move_devices)) = apply_structure {
            ops::set_page_structure(&mut self.session.prj, pi, &dt, move_devices);
        }
        if !open {
            self.page_props = None;
        }
    }

    }

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.recording.is_some() {
            self.record_shortcut(ctx);
            return;
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let sc = self.editor.shortcuts.clone();
        if ctx.input_mut(|i| sc.consume(i, Action::Find)) {
            self.find_ui.open = true;
        }
        let (undo, redo, save, open, pgup, pgdn, w, t) = ctx.input_mut(|i| {
            (
                sc.consume(i, Action::Undo),
                sc.consume(i, Action::Redo),
                sc.consume(i, Action::Save),
                sc.consume(i, Action::Open),
                sc.pressed(i, Action::PreviousPage),
                sc.pressed(i, Action::NextPage),
                sc.pressed(i, Action::WireTool),
                sc.pressed(i, Action::TextTool),
            )
        });
        if undo {
            self.editor.cancel();
            self.session.undo();
        }
        if redo {
            self.editor.cancel();
            self.session.redo();
        }
        if save {
            self.save(ctx, false);
        }
        if open {
            self.io.open(ctx, Purpose::Project);
        }
        if self.workspace != Workspace::Schematic {
            return;
        }
        if pgup && self.session.page > 0 {
            self.goto(self.session.page - 1, 0);
        }
        if pgdn && self.session.page + 1 < self.session.prj.pages.len() {
            self.goto(self.session.page + 1, 0);
        }
        if w {
            self.editor.set_tool(Tool::Wire);
        }
        if t {
            self.editor.set_tool(Tool::Text);
        }
    }

    /// Takes the next key press (with modifiers) as the shortcut of the action being
    /// recorded; Esc cancels. Key presses do not reach the editor meanwhile.
    fn record_shortcut(&mut self, ctx: &egui::Context) {
        let Some(action) = self.recording else { return };
        let pressed = ctx.input_mut(|i| {
            let key = i.events.iter().find_map(|e| match e {
                egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                _ => None,
            });
            i.events.retain(|e| !matches!(e, egui::Event::Key { .. } | egui::Event::Text(_) | egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_)));
            key
        });
        match pressed {
            Some((Key::Escape, m)) if m.is_none() => self.recording = None,
            Some((key, m)) => {
                let s = Shortcut { command: m.command, shift: m.shift, alt: m.alt, key };
                self.editor.shortcuts.set(action, vec![s]);
                self.recording = None;
            }
            None => {}
        }
    }

    /// View › Keyboard shortcuts: the actions with their keys, record, clear, reset.
    pub(super) fn shortcuts_window(&mut self, ctx: &egui::Context) {
        if !self.show_shortcuts {
            self.recording = None;
            return;
        }
        let mut open = true;
        let conflicts = self.editor.shortcuts.conflicts();
        egui::Window::new(tr("Keyboard shortcuts")).open(&mut open).resizable(false).show(ctx, |ui| {
            ui.weak(tr("Record: press the new key with its modifiers (Esc cancels). Esc, Delete, the arrow keys and copy / paste stay fixed."));
            egui::Grid::new("shortcut_grid").num_columns(4).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                for a in Action::ALL {
                    ui.label(a.title());
                    let text = self.editor.shortcuts.text(a);
                    let clash = conflicts.iter().any(|(_, v)| v.contains(&a));
                    let shown = if self.recording == Some(a) {
                        RichText::new(tr("Press a key…")).italics()
                    } else if text.is_empty() {
                        RichText::new("–").weak()
                    } else {
                        RichText::new(text).monospace()
                    };
                    ui.label(if clash { shown.color(Color32::from_rgb(220, 60, 60)) } else { shown });
                    ui.horizontal(|ui| {
                        if ui.small_button(tr("Record")).clicked() {
                            self.recording = Some(a);
                        }
                        if ui.small_button(tr("Clear")).on_hover_text(tr("No shortcut")).clicked() {
                            self.editor.shortcuts.set(a, Vec::new());
                        }
                        if ui.add_enabled(!self.editor.shortcuts.is_default(a), egui::Button::new(tr("Default")).small()).clicked() {
                            self.editor.shortcuts.reset(a);
                        }
                    });
                    ui.end_row();
                }
            });
            for (key, actions) in &conflicts {
                let names: Vec<&str> = actions.iter().map(|a| a.title()).collect();
                ui.colored_label(Color32::from_rgb(220, 60, 60), tr_fmt!("{} is used by: {}", key.text(), names.join(", ")));
            }
            ui.separator();
            if ui.button(tr("Reset all to the defaults")).clicked() {
                self.editor.shortcuts.reset_all();
            }
        });
        self.show_shortcuts = open;
    }
}
