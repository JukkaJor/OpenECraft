//! The menu bar of the editor: one method per menu.

use super::*;

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(tr("File"), |ui| self.file_menu(ui));
            ui.menu_button(tr("Edit"), |ui| self.edit_menu(ui));
            ui.menu_button(tr("Insert"), |ui| self.insert_menu(ui));
            ui.menu_button(tr("Page"), |ui| self.page_menu(ui));
            ui.menu_button(tr("Project"), |ui| self.project_menu(ui));
            ui.menu_button(tr("Reports"), |ui| self.reports_menu(ui));
            ui.menu_button(tr("Plugins"), |ui| self.plugins_menu(ui));
            ui.menu_button(tr("View"), |ui| self.view_menu(ui));
            ui.menu_button(tr("Help"), |ui| self.help_menu(ui));
        });
    }

    pub(super) fn file_menu(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if ui.button(tr("New project")).clicked() {
            self.replace_project(Project::new_default(), None);
        }
        ui.menu_button(tr("New project from template"), |ui| {
            for (name, desc) in openecraft_core::project_templates::BUILTIN {
                if ui.button(tr(name)).on_hover_text(tr(desc)).clicked() {
                    if let Some(t) = openecraft_core::project_templates::builtin(name) {
                        let prj = openecraft_core::project_templates::from_template(&t, tr("New project"), &openecraft_ui::io::today());
                        self.replace_project(prj, None);
                    }
                    ui.close();
                }
            }
            ui.separator();
            if ui.button(tr("From a template file…")).on_hover_text(tr("Any project file: its settings, forms, structure, potentials and pages start the new project")).clicked() {
                self.io.open(&ctx, Purpose::ProjectTemplate);
                ui.close();
            }
        });
        if ui.button(tr("Open…        Ctrl+O")).clicked() {
            self.io.open(&ctx, Purpose::Project);
        }
        if ui.button(tr("Save as project template…")).on_hover_text(tr("Saves the project without revisions, baselines, review and wiring marks as a template for new projects")).clicked() {
            let t = openecraft_core::project_templates::as_template(&self.session.prj);
            let name = format!("{}_template.oec", self.session.prj.info.name.replace(' ', "_"));
            self.export(&ctx, name, (tr("OpenECraft project template"), "oec"), t.to_json());
        }
        if ui.button(tr("Open demo project")).clicked() {
            self.replace_project(sample::demo_project(), None);
        }
        ui.separator();
        let save_label = if self.remote.shared.is_some() {
            tr("Save to project server   Ctrl+S")
        } else if FileIo::can_save_in_place() {
            tr("Save          Ctrl+S")
        } else {
            tr("Download project   Ctrl+S")
        };
        if ui.button(save_label).clicked() {
            self.save(&ctx, false);
        }
        if self.remote.shared.is_some() && !FileIo::can_save_in_place() && ui.button(tr("Download project")).clicked() {
            self.save(&ctx, true);
        }
        if FileIo::can_save_in_place() && ui.button(tr("Save as…")).clicked() {
            self.save(&ctx, true);
        }
        ui.separator();
        if ui.button(tr("Project server…")).on_hover_text(tr("Open and save shared projects, lock pages")).clicked() {
            self.remote.window = true;
        }
        ui.separator();
        ui.menu_button(tr("Export"), |ui| self.export_menu(ui));
        if !self.session.prj.languages.targets.is_empty() {
            ui.menu_button(tr("Export as PDF in language"), |ui| {
                for l in self.session.prj.languages.all() {
                    if ui.button(translation::language_name(&l)).clicked() {
                        self.export_pdf_in(&ctx, &l);
                        ui.close();
                    }
                }
            });
        }
        if ui.button(tr("Import DXF / DWG as page…")).on_hover_text(tr("An old drawing as graphics on a new schematic page (lines, arcs, texts, blocks)")).clicked() {
            self.io.open(&ctx, Purpose::Drawing);
        }
        if FileIo::has_own_window() {
            ui.separator();
            if ui.button(tr("Quit")).clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    pub(super) fn edit_menu(&mut self, ui: &mut egui::Ui) {
        if ui.add_enabled(self.session.history.can_undo(), egui::Button::new(tr("Undo   Ctrl+Z"))).clicked() {
            self.session.undo();
        }
        if ui.add_enabled(self.session.history.can_redo(), egui::Button::new(tr("Redo   Ctrl+Y"))).clicked() {
            self.session.redo();
        }
        ui.separator();
        let has_sel = !self.editor.selection.is_empty();
        if ui.add_enabled(has_sel, egui::Button::new(tr("Rotate   R"))).clicked() {
            ops::rotate(&self.session.prj.symbol_table(), &mut self.session.prj.pages[self.session.page], &self.editor.selection);
        }
        if ui.add_enabled(has_sel, egui::Button::new(tr("Mirror   M"))).clicked() {
            ops::mirror(&mut self.session.prj.pages[self.session.page], &self.editor.selection);
        }
        if ui.add_enabled(has_sel, egui::Button::new(tr("Delete   Del"))).clicked() {
            ops::delete(&mut self.session.prj.pages[self.session.page], &self.editor.selection);
            self.editor.selection.clear();
        }
        if ui.button(tr("Find…   Ctrl+F")).on_hover_text(tr("Find and replace in the whole project")).clicked() {
            self.find_ui.open = true;
        }
        if ui.button(tr("Select all   Ctrl+A")).clicked() {
            self.editor.selection = ops::all_ids(&self.session.prj.pages[self.session.page]).into_iter().collect();
        }
        ui.label(RichText::new(tr("Copy / paste: Ctrl+C, Ctrl+X, Ctrl+V")).weak());
        ui.separator();
        self.arrange_menu_entries(ui);
        ui.separator();
        self.edit_menu_entries(ui);
        ui.separator();
        if ui.add_enabled(has_sel, egui::Button::new(tr("Save selection as macro…"))).clicked() {
            self.save_as_macro(false);
        }
    }

    pub(super) fn insert_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button(tr("Macro…")).clicked() {
            self.eng.show_macros = true;
        }
        ui.separator();
        if ui.button(tr("Wire   W")).clicked() {
            self.editor.set_tool(Tool::Wire);
        }
        if ui.button(tr("Text   T")).clicked() {
            self.editor.set_tool(Tool::Text);
        }
        if ui
            .button(tr("Path function text"))
            .on_hover_text(tr("Function text of the functions in this column that have none of their own (device list, terminal, plug and PLC diagrams)"))
            .clicked()
        {
            self.editor.set_tool(Tool::Text);
            self.editor.text_proto = Some(openecraft_core::model::TextItem { text: "Function text".into(), path_function: true, ..Default::default() });
        }
        ui.menu_button(tr("Special text"), |ui| {
            ui.label(RichText::new(tr("Special text: project and page properties such as {page} or {prop:Name} are filled in when the page is drawn")).weak());
            for (ph, meaning) in openecraft_core::forms::PLACEHOLDERS.iter().filter(|(ph, _)| *ph != "{{ }}") {
                if ui.button(format!("{}  {}", tr(meaning), ph)).clicked() {
                    self.editor.set_tool(Tool::Text);
                    self.editor.text_proto = Some(openecraft_core::model::TextItem { text: (*ph).to_string(), ..Default::default() });
                }
            }
        });
        if ui.button(tr("Graphic line")).clicked() {
            self.editor.set_tool(Tool::Line);
        }
        if ui.button(tr("Graphic rectangle")).clicked() {
            self.editor.set_tool(Tool::Rect);
        }
        self.insert_menu_entries(ui);
        ui.separator();
        for cat in Category::ALL {
            ui.menu_button(cat.title(), |ui| {
                for def in symbols::library().iter().filter(|d| d.category == cat) {
                    if ui.button(tr(def.name)).clicked() {
                        self.editor.set_tool(Tool::Place(def.key));
                    }
                }
            });
        }
        let user = self.user_symbols();
        if !user.is_empty() {
            ui.menu_button(Category::User.title(), |ui| {
                for def in user {
                    if ui.button(format!("{}  ({})", tr(def.name), def.library)).clicked() {
                        self.editor.set_tool(Tool::Place(def.key));
                    }
                }
            });
        }
    }

    pub(super) fn page_menu(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if ui.button(tr("New schematic page")).clicked() {
            self.add_page(PageKind::Schematic);
        }
        if ui.button(tr("New cover sheet")).clicked() {
            self.add_page(PageKind::Cover);
        }
        if ui.button(tr("New overview page")).on_hover_text(tr("Single-line or block diagram: drawn like a schematic, not evaluated")).clicked() {
            self.add_page(PageKind::Overview);
        }
        if ui.button(tr("New graphics page")).on_hover_text(tr("Free graphics and notes, not evaluated")).clicked() {
            self.add_page(PageKind::Graphic);
        }
        if ui.button(tr("New document page…")).on_hover_text(tr("Embeds a file (PDF, Word, Excel, image …) as a page")).clicked() {
            self.pages_ui.replace_document = None;
            self.io.open(&ctx, Purpose::Document);
        }
        if ui.button(tr("Renumber pages…")).clicked() {
            self.numbering_ui.show_pages = true;
        }
        ui.menu_button(tr("New page from template"), |ui| {
            for t in openecraft_core::page_templates::all(&self.session.prj) {
                if ui.button(openecraft_core::page_templates::display_name(&t.name)).on_hover_text(t.kind.label()).clicked() {
                    self.insert_template(&t);
                }
            }
        });
        ui.menu_button(tr("New report page"), |ui| {
            for (k, title) in self.report_list() {
                if ui.button(title).clicked() {
                    self.add_page(PageKind::Report(k));
                }
            }
        });
        ui.menu_button(tr("New mounting layout page"), |ui| {
            let encs: Vec<(u64, String)> = self.session.prj.layout.enclosures.iter().map(|e| (e.id, e.name.clone())).collect();
            for (id, name) in encs {
                if ui.button(name).clicked() {
                    self.add_page(PageKind::Mounting(id));
                }
            }
            if self.session.prj.layout.enclosures.is_empty() {
                ui.weak(tr("Create an enclosure in the panel layout first."));
            }
        });
        ui.separator();
        if ui.button(tr("Page properties…")).clicked() {
            self.page_props = Some(self.session.page);
        }
        if ui.button(tr("Save page as template…")).clicked() {
            self.open_save_template();
        }
        if ui.button(tr("Save page as macro…")).clicked() {
            self.save_as_macro(true);
        }
        if ui.add_enabled(self.session.page > 0, egui::Button::new(tr("Move page up"))).clicked() {
            self.session.page = ops::move_page(&mut self.session.prj, self.session.page, self.session.page - 1);
        }
        if ui.add_enabled(self.session.page + 1 < self.session.prj.pages.len(), egui::Button::new(tr("Move page down"))).clicked() {
            self.session.page = ops::move_page(&mut self.session.prj, self.session.page, self.session.page + 1);
        }
        if ui.add_enabled(self.session.prj.pages.len() > 1, egui::Button::new(tr("Delete page"))).clicked() {
            match ops::delete_page(&mut self.session.prj, self.session.page) {
                Ok(_) => {
                    self.session.page = self.session.page.min(self.session.prj.pages.len() - 1);
                    self.editor.selection.clear();
                }
                Err(e) => self.notify(&ctx, tr(e)),
            }
        }
    }

    pub(super) fn project_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button(tr("Project properties…")).clicked() {
            self.show_project = true;
        }
        if ui.button(tr("Project management…")).on_hover_text(tr("Project database, batch changes of project properties, subprojects, options, archives and verification")).clicked() {
            self.projects_ui.open = true;
        }
        if ui.button(tr("Parts management…")).clicked() {
            self.parts_ui.open = true;
        }
        if ui.button(tr("Data portal…")).on_hover_text(tr("Search the parts server by catalog, product group and technical data, add parts and check the database for updates")).clicked() {
            self.parts_ui.show_portal = true;
        }
        if ui.button(tr("Structure management…")).on_hover_text(tr("Structure identifiers (IEC 81346), templates and work areas")).clicked() {
            self.structure_ui.open = true;
        }
        if ui.button(tr("Property database…")).on_hover_text(tr("Property definitions, function definitions, placement schemes and the values of all objects")).clicked() {
            self.properties_ui.open = true;
        }
        if ui.button(tr("Cables…")).clicked() {
            self.show_cables = true;
        }
        if ui.button(tr("Potentials…")).clicked() {
            self.eng.show_potentials = true;
        }
        if ui.button(tr("Electrical calculations…")).on_hover_text(tr("Short circuit, selectivity, loads, EMC, functional safety and verification against rule sets")).clicked() {
            self.electrical_ui.open = true;
        }
        if ui.button(tr("Wire numbering…")).clicked() {
            self.show_numbering = true;
        }
        if ui.button(tr("Cross references…")).on_hover_text(tr("Format of cross references, structure of the target page, interruption points, contact images")).clicked() {
            self.show_xref = true;
        }
        if ui.button(tr("Device numbering…")).on_hover_text(tr("Numbers the device tags per structure and identifying letter in schematic order (page, column, top to bottom)")).clicked() {
            self.numbering_ui.show_devices = true;
        }
        if ui.button(tr("Harnesses…")).clicked() {
            self.harness_ui.open = true;
        }
        if ui.button(tr("Field wiring…")).on_hover_text(tr("Plant layout, route network, field cable planning, equipotential bonding and cable accessories")).clicked() {
            self.field_ui.open = true;
        }
        if ui.button(tr("Terminal strips…")).clicked() {
            self.eng.show_strips = true;
        }
        if ui.button(tr("Plugs…")).clicked() {
            self.plugs_ui.open = true;
        }
        if ui.button(tr("Project languages…")).on_hover_text(tr("Translate the project texts; documentation per language")).clicked() {
            self.languages_ui.open = true;
        }
        if ui.button(tr("Fluid and process…")).on_hover_text(tr("Hose defaults and PCT loops of the pre-planning")).clicked() {
            self.process_ui.open = true;
        }
        if ui.button(tr("Single-line / multi-line…")).on_hover_text(tr("Plan devices in the single-line diagram, generate the schematic and keep both in sync")).clicked() {
            self.singleline_ui.open = true;
        }
        if ui.button(tr("Marking…")).clicked() {
            self.eng.show_marking = true;
        }
        if ui.button(tr("PLC overview…")).clicked() {
            self.eng.show_plc = true;
        }
        if ui.button(tr("PLC hardware and bus topology…")).on_hover_text(tr("Stations, racks, modules, bus systems, AutomationML exchange with PLC tools")).clicked() {
            self.plc_ui.open = true;
        }
        if ui.button(tr("Enterprise integration…")).on_hover_text(tr("Exchange with ERP, PDM, PLM, ECAD, MCAD, procurement and wiring machines")).clicked() {
            self.integration_ui.open = true;
        }
        if ui.button(tr("Compare and baselines…")).on_hover_text(tr("Compare with a baseline, a revision or another project file and take differences over")).clicked() {
            self.compare_ui.open = true;
        }
        if ui.button(tr("Change requests…")).clicked() {
            self.requests_ui.open = true;
        }
        if ui.button(tr("Review: comments and markup")).clicked() {
            self.workspace = Workspace::Schematic;
            self.left_tab = LeftTab::Review;
        }
        if ui.button(tr("Revisions…")).on_hover_text(tr("Revisions, approval and release")).clicked() {
            self.eng.show_revisions = true;
        }
        if ui.button(tr("Edit in table…")).on_hover_text(tr("Devices, connections and terminals: filter, fill, find and replace, CSV")).clicked() {
            self.tables_ui.open = true;
        }
        if ui.button(tr("Macros…")).clicked() {
            self.eng.show_macros = true;
        }
        if ui.button(tr("Configurator…")).on_hover_text(tr("Generate pages from page macros by options and rules")).clicked() {
            self.config_ui.open = true;
        }
        ui.separator();
        if ui.button(tr("Symbol editor…")).clicked() {
            self.symed.open = true;
        }
        if ui.button(tr("Title blocks, plot frames and page templates…")).clicked() {
            self.forms_ui.open = true;
        }
        if ui.button(tr("Report blocks…")).clicked() {
            self.report_blocks_ui.open = true;
        }
        if ui.button(tr("Check project")).clicked() {
            self.left_tab = LeftTab::Check;
        }
    }

    pub(super) fn reports_menu(&mut self, ui: &mut egui::Ui) {
        for (k, title) in self.report_list() {
            if ui.button(title).clicked() {
                self.report_kind = k;
                self.show_reports = true;
            }
        }
        ui.separator();
        if ui.button(tr("Report blocks…")).clicked() {
            self.report_blocks_ui.open = true;
        }
        if ui.add_enabled(!self.session.prj.report_blocks.is_empty(), egui::Button::new(tr("⟳ Update report blocks"))).clicked() {
            self.generate_report_blocks(ui.ctx());
        }
    }

    pub(super) fn view_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button(tr("Fit page   F")).clicked() {
            self.editor.fit_pending = true;
        }
        ui.checkbox(&mut self.editor.show_grid, tr("Grid"));
        self.grid_menu(ui);
        ui.checkbox(&mut self.editor.show_pins, tr("Connection points"));
        ui.checkbox(&mut self.editor.show_revision_marks, tr("Revision marks"));
        ui.checkbox(&mut self.session.prj.wire_colors, tr("Wire colours"))
            .on_hover_text(tr("Draw wires in their colour (also in PDF and SVG); saved with the project"));
        ui.separator();
        openecraft_ui::language::menu(ui);
        let mut pref = ui.ctx().options(|o| o.theme_preference);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut pref, egui::ThemePreference::System, tr("System"));
            ui.selectable_value(&mut pref, egui::ThemePreference::Light, tr("Light"));
            ui.selectable_value(&mut pref, egui::ThemePreference::Dark, tr("Dark"));
        });
        if pref != ui.ctx().options(|o| o.theme_preference) {
            ui.ctx().set_theme(pref);
        }
    }

    pub(super) fn help_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button(tr("Shortcuts & usage")).clicked() {
            self.show_help = true;
        }
    }

    /// File › Export: every exporter of the registry (built-in and of plugins).
    pub(super) fn export_menu(&mut self, ui: &mut egui::Ui) {
        let exporters: Vec<openecraft_api::Exporter> = self.plugins.exporters().cloned().collect();
        for e in exporters {
            if ui.button(format!("{}…", text(&e.title))).clicked() {
                let ctx = ui.ctx().clone();
                self.run_exporter(&ctx, &e, None);
                ui.close();
            }
        }
    }
}
