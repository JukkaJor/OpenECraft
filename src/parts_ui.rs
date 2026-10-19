//! Parts management window and part selection dialog.

use crate::model::{natural_cmp, Project};
use crate::parts::{self, FnKind, FunctionTemplate, Part, PartRef, PartsDb, TopGroup, PRODUCT_GROUPS};
use egui::{vec2, Color32, RichText};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Database,
    Project,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    General,
    Commercial,
    Technical,
    Mounting,
    Functions,
    Accessories,
}

/// Requests the application has to carry out (file dialogs, messages).
pub enum PartsAction {
    Import,
    Export { name: String, filter: (&'static str, &'static str), content: String },
    Message(String),
}

struct SelectRequest {
    tag: String,
}

pub struct PartsUi {
    pub open: bool,
    source: Source,
    filter: String,
    selected: Option<String>,
    tab: Tab,
    select: Option<SelectRequest>,
    sel_filter: String,
    sel_matching: bool,
    sel_part: Option<String>,
}

impl Default for PartsUi {
    fn default() -> Self {
        PartsUi {
            open: false,
            source: Source::Database,
            filter: String::new(),
            selected: None,
            tab: Tab::General,
            select: None,
            sel_filter: String::new(),
            sel_matching: true,
            sel_part: None,
        }
    }
}

/// Product group tree. Returns the part number that was double clicked.
fn part_tree(ui: &mut egui::Ui, id: &str, parts: &[&Part], filter: &str, selected: &mut Option<String>) -> Option<String> {
    let mut tree: BTreeMap<TopGroup, BTreeMap<&str, BTreeMap<&str, Vec<&Part>>>> = BTreeMap::new();
    for p in parts.iter().filter(|p| p.matches_text(filter)) {
        let group = if p.group.is_empty() { "(no product group)" } else { p.group.as_str() };
        tree.entry(p.top_group).or_default().entry(group).or_default().entry(p.subgroup.as_str()).or_default().push(p);
    }
    let open = if filter.is_empty() { None } else { Some(true) };
    let mut activated = None;
    let mut leaf = |ui: &mut egui::Ui, list: &mut Vec<&Part>| {
        list.sort_by(|a, b| natural_cmp(&a.number, &b.number));
        for p in list.iter() {
            let is_sel = selected.as_deref() == Some(p.number.as_str());
            let r = ui.selectable_label(is_sel, &p.number).on_hover_text(&p.description);
            if r.clicked() {
                *selected = Some(p.number.clone());
            }
            if r.double_clicked() {
                activated = Some(p.number.clone());
            }
        }
    };
    for (top, groups) in tree.iter_mut() {
        egui::CollapsingHeader::new(RichText::new(top.title()).strong())
            .id_salt((id, top.title()))
            .default_open(true)
            .open(open)
            .show(ui, |ui| {
                for (group, subs) in groups.iter_mut() {
                    egui::CollapsingHeader::new(*group).id_salt((id, top.title(), *group)).open(open).show(ui, |ui| {
                        for (sub, list) in subs.iter_mut() {
                            if sub.is_empty() {
                                leaf(ui, list);
                            } else {
                                egui::CollapsingHeader::new(*sub)
                                    .id_salt((id, top.title(), *group, *sub))
                                    .open(open)
                                    .show(ui, |ui| leaf(ui, list));
                            }
                        }
                    });
                }
            });
    }
    if tree.is_empty() {
        ui.weak("No parts found.");
    }
    activated
}

fn text_row(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(label);
    ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
    ui.end_row();
}

fn num_row(ui: &mut egui::Ui, label: &str, value: &mut f32, suffix: &str) {
    ui.label(label);
    ui.add(egui::DragValue::new(value).speed(0.1).range(0.0..=1.0e7).suffix(suffix));
    ui.end_row();
}

/// Tabbed editor for one part.
fn part_editor(ui: &mut egui::Ui, part: &mut Part, tab: &mut Tab, all_numbers: &[String], duplicate: bool) {
    ui.horizontal_wrapped(|ui| {
        for (t, name) in [
            (Tab::General, "General"),
            (Tab::Commercial, "Prices / other"),
            (Tab::Technical, "Technical data"),
            (Tab::Mounting, "Mounting data"),
            (Tab::Functions, "Function template"),
            (Tab::Accessories, "Accessories"),
        ] {
            ui.selectable_value(tab, t, name);
        }
    });
    ui.separator();
    egui::ScrollArea::vertical().id_salt("part_editor").auto_shrink([false, false]).show(ui, |ui| match tab {
        Tab::General => {
            egui::Grid::new("pg").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Part number");
                ui.vertical(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut part.number).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                    if duplicate {
                        ui.colored_label(Color32::from_rgb(220, 60, 60), "Part number already exists");
                    }
                });
                ui.end_row();
                text_row(ui, "Type number", &mut part.type_number);
                text_row(ui, "Order number", &mut part.order_number);
                text_row(ui, "Manufacturer", &mut part.manufacturer);
                text_row(ui, "Supplier", &mut part.supplier);
                ui.label("Description");
                ui.add(egui::TextEdit::multiline(&mut part.description).desired_rows(2).desired_width(f32::INFINITY));
                ui.end_row();
                ui.label("Product top group");
                egui::ComboBox::from_id_salt("top").selected_text(part.top_group.title()).show_ui(ui, |ui| {
                    for t in TopGroup::ALL {
                        ui.selectable_value(&mut part.top_group, t, t.title());
                    }
                });
                ui.end_row();
                ui.label("Product group");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut part.group).desired_width(200.0));
                    egui::ComboBox::from_id_salt("grp").selected_text("…").width(24.0).show_ui(ui, |ui| {
                        for (g, _) in PRODUCT_GROUPS {
                            if ui.selectable_label(part.group == *g, *g).clicked() {
                                part.group = g.to_string();
                            }
                        }
                    });
                });
                ui.end_row();
                ui.label("Product subgroup");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut part.subgroup).desired_width(200.0));
                    let subs = PRODUCT_GROUPS.iter().find(|(g, _)| *g == part.group).map(|(_, s)| *s).unwrap_or(&[]);
                    if !subs.is_empty() {
                        egui::ComboBox::from_id_salt("sub").selected_text("…").width(24.0).show_ui(ui, |ui| {
                            for s in subs {
                                if ui.selectable_label(part.subgroup == *s, *s).clicked() {
                                    part.subgroup = s.to_string();
                                }
                            }
                        });
                    }
                });
                ui.end_row();
                ui.label("Identifier");
                ui.add(egui::TextEdit::singleline(&mut part.identifier).desired_width(50.0))
                    .on_hover_text("Identifying letter of devices using this part (K, Q, S …)");
                ui.end_row();
                ui.label("Note");
                ui.add(egui::TextEdit::multiline(&mut part.note).desired_rows(2).desired_width(f32::INFINITY));
                ui.end_row();
            });
        }
        Tab::Commercial => {
            egui::Grid::new("pc").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Price");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut part.price).speed(0.1).range(0.0..=1.0e7).fixed_decimals(2));
                    ui.add(egui::TextEdit::singleline(&mut part.currency).desired_width(40.0));
                });
                ui.end_row();
                ui.label("Price unit");
                ui.add(egui::DragValue::new(&mut part.price_unit).range(1..=100000)).on_hover_text("The price refers to this many units");
                ui.end_row();
                ui.label("Packaging quantity");
                ui.add(egui::DragValue::new(&mut part.pack_quantity).range(1..=100000));
                ui.end_row();
                ui.label("Quantity unit");
                ui.add(egui::TextEdit::singleline(&mut part.quantity_unit).desired_width(60.0));
                ui.end_row();
                text_row(ui, "Certificates", &mut part.certificates);
                text_row(ui, "Datasheet (URL)", &mut part.datasheet);
            });
            if part.datasheet.starts_with("http") {
                ui.hyperlink_to("Open datasheet", &part.datasheet);
            }
        }
        Tab::Technical => {
            egui::Grid::new("pt").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                text_row(ui, "Voltage", &mut part.voltage);
                text_row(ui, "Current", &mut part.current);
                text_row(ui, "Power", &mut part.power);
                ui.label("Technical data");
                ui.add(egui::TextEdit::multiline(&mut part.tech_data).desired_rows(5).desired_width(f32::INFINITY));
                ui.end_row();
            });
        }
        Tab::Mounting => {
            egui::Grid::new("pm").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                num_row(ui, "Width", &mut part.width, " mm");
                num_row(ui, "Height", &mut part.height, " mm");
                num_row(ui, "Depth", &mut part.depth, " mm");
                num_row(ui, "Weight", &mut part.weight, " kg");
                text_row(ui, "Mounting", &mut part.mounting);
            });
        }
        Tab::Functions => {
            ui.weak("Functions the part provides. Used for device selection, the contact image (reserve contacts), the project check and to assign connection designations. Designations are listed in symbol connection order, separated by blanks.");
            ui.add_space(4.0);
            let mut remove = None;
            egui::Grid::new("pf").num_columns(4).striped(true).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label(RichText::new("Function").strong());
                ui.label(RichText::new("Connection designations").strong());
                ui.label(RichText::new("Description").strong());
                ui.label("");
                ui.end_row();
                for (i, f) in part.functions.iter_mut().enumerate() {
                    egui::ComboBox::from_id_salt(("fk", i)).selected_text(f.kind.title()).width(170.0).show_ui(ui, |ui| {
                        for k in FnKind::ALL {
                            ui.selectable_value(&mut f.kind, k, k.title());
                        }
                    });
                    ui.add(egui::TextEdit::singleline(&mut f.pins).desired_width(140.0).font(egui::TextStyle::Monospace));
                    ui.add(egui::TextEdit::singleline(&mut f.description).desired_width(160.0));
                    if ui.small_button("🗑").clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
            if let Some(i) = remove {
                part.functions.remove(i);
            }
            if ui.button("➕ Add function").clicked() {
                part.functions.push(FunctionTemplate::new(FnKind::No, ""));
            }
        }
        Tab::Accessories => {
            let mut remove = None;
            for (i, a) in part.accessories.iter().enumerate() {
                ui.horizontal(|ui| {
                    let known = all_numbers.contains(a);
                    ui.label(RichText::new(a).monospace().color(if known { ui.visuals().text_color() } else { Color32::from_rgb(220, 60, 60) }));
                    if ui.small_button("🗑").clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                part.accessories.remove(i);
            }
            let mut add = None;
            egui::ComboBox::from_id_salt("acc").selected_text("➕ Add accessory").width(240.0).show_ui(ui, |ui| {
                for n in all_numbers {
                    if *n != part.number && !part.accessories.contains(n) && ui.selectable_label(false, n).clicked() {
                        add = Some(n.clone());
                    }
                }
            });
            if let Some(n) = add {
                part.accessories.push(n);
            }
        }
    });
}

/// Read only summary used in the selection dialog.
fn part_summary(ui: &mut egui::Ui, p: &Part) {
    ui.label(RichText::new(&p.number).monospace().strong());
    ui.label(&p.description);
    egui::Grid::new("sum").num_columns(2).show(ui, |ui| {
        for (k, v) in [
            ("Manufacturer", p.manufacturer.clone()),
            ("Type number", p.type_number.clone()),
            ("Order number", p.order_number.clone()),
            ("Product group", format!("{} / {}", p.group, p.subgroup)),
            ("Voltage / current", format!("{} {}", p.voltage, p.current)),
            ("Price", format!("{:.2} {} / {} {}", p.price, p.currency, p.price_unit, p.quantity_unit)),
            ("Dimensions", format!("{} × {} × {} mm", p.width, p.height, p.depth)),
            ("Functions", p.template_summary()),
            ("Accessories", p.accessories.join(", ")),
        ] {
            if !v.trim().is_empty() && v.trim() != "/" {
                ui.label(RichText::new(k).weak());
                ui.label(v);
                ui.end_row();
            }
        }
    });
}

impl PartsUi {
    /// Opens the management window with a part selected.
    pub fn show_part(&mut self, number: &str, in_project: bool) {
        self.open = true;
        self.source = if in_project { Source::Project } else { Source::Database };
        self.selected = Some(number.to_string());
    }

    /// Opens the part selection dialog for a device.
    pub fn select_for(&mut self, tag: &str) {
        self.select = Some(SelectRequest { tag: tag.to_string() });
        self.sel_part = None;
    }

    pub fn manager(&mut self, ctx: &egui::Context, db: &mut PartsDb, prj: &mut Project) -> Vec<PartsAction> {
        let mut actions = Vec::new();
        if !self.open {
            return actions;
        }
        let mut open = true;
        egui::Window::new("Parts management").open(&mut open).default_size([1000.0, 620.0]).show(ctx, |ui| {
            // ---------------------------------------------------------- toolbar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.source, Source::Database, "🗄 Parts database");
                ui.selectable_value(&mut self.source, Source::Project, format!("📁 Project parts ({})", prj.parts.len()));
                ui.separator();
                let list: &mut Vec<Part> = match self.source {
                    Source::Database => &mut db.parts,
                    Source::Project => &mut prj.parts,
                };
                if ui.button("➕ New").clicked() {
                    let number = parts::unique_number(list, "NEW.PART");
                    list.push(Part { number: number.clone(), manufacturer: "NEW".into(), ..Default::default() });
                    self.selected = Some(number);
                    self.tab = Tab::General;
                }
                let sel_idx = self.selected.as_ref().and_then(|n| list.iter().position(|p| &p.number == n));
                if ui.add_enabled(sel_idx.is_some(), egui::Button::new("⎘ Copy")).clicked() {
                    let mut p = list[sel_idx.unwrap()].clone();
                    p.number = parts::unique_number(list, &format!("{}_COPY", p.number));
                    self.selected = Some(p.number.clone());
                    list.push(p);
                }
                if ui.add_enabled(sel_idx.is_some(), egui::Button::new("🗑 Delete")).clicked() {
                    list.remove(sel_idx.unwrap());
                    self.selected = None;
                }
                ui.separator();
                ui.menu_button("Extras", |ui| {
                    if ui.button("Import parts (CSV / JSON)…").clicked() {
                        actions.push(PartsAction::Import);
                    }
                    let src = match self.source {
                        Source::Database => db.parts.clone(),
                        Source::Project => prj.parts.clone(),
                    };
                    let base = if self.source == Source::Database { "parts_database" } else { "project_parts" };
                    if ui.button("Export as CSV…").clicked() {
                        actions.push(PartsAction::Export { name: format!("{base}.csv"), filter: ("CSV", "csv"), content: parts::to_csv(&src) });
                    }
                    if ui.button("Export as JSON…").clicked() {
                        let json = serde_json::to_string_pretty(&PartsDb { parts: src }).unwrap_or_default();
                        actions.push(PartsAction::Export { name: format!("{base}.json"), filter: ("JSON", "json"), content: json });
                    }
                    ui.separator();
                    if ui.button("Synchronize project parts from database").on_hover_text("Updates the project parts with the current database data").clicked() {
                        let n = parts::sync_project_from_db(prj, db);
                        actions.push(PartsAction::Message(format!("{n} project parts updated")));
                    }
                    if ui.button("Transfer project parts to database").on_hover_text("Adds parts that only exist in the project to the database").clicked() {
                        let n = parts::transfer_project_to_db(prj, db);
                        actions.push(PartsAction::Message(format!("{n} parts added to the database")));
                    }
                    if ui.button("Remove unused project parts").clicked() {
                        let n = parts::purge_unused(prj);
                        actions.push(PartsAction::Message(format!("{n} unused project parts removed")));
                    }
                    ui.separator();
                    if ui.button("Add missing parts of the example catalog").clicked() {
                        let mut n = 0;
                        for p in parts::default_catalog() {
                            if db.get(&p.number).is_none() {
                                db.parts.push(p);
                                n += 1;
                            }
                        }
                        actions.push(PartsAction::Message(format!("{n} catalog parts added")));
                    }
                });
            });
            ui.separator();

            // ---------------------------------------------------------- body
            let numbers: Vec<String> = db.parts.iter().chain(prj.parts.iter()).map(|p| p.number.clone()).collect();
            let list: &mut Vec<Part> = match self.source {
                Source::Database => &mut db.parts,
                Source::Project => &mut prj.parts,
            };
            let h = ui.available_height();
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(vec2(300.0, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filter (number, type, manufacturer, text …)"));
                    ui.label(RichText::new(format!("{} parts", list.len())).weak());
                    egui::ScrollArea::vertical().id_salt("ptree").auto_shrink([false, false]).show(ui, |ui| {
                        let refs: Vec<&Part> = list.iter().collect();
                        part_tree(ui, "mgr", &refs, &self.filter, &mut self.selected);
                    });
                });
                ui.separator();
                ui.vertical(|ui| {
                    let idx = self.selected.as_ref().and_then(|n| list.iter().position(|p| &p.number == n));
                    match idx {
                        Some(i) => {
                            let duplicate = list.iter().filter(|p| p.number == list[i].number).count() > 1;
                            let before = list[i].number.clone();
                            part_editor(ui, &mut list[i], &mut self.tab, &numbers, duplicate);
                            if list[i].number != before {
                                self.selected = Some(list[i].number.clone());
                            }
                        }
                        None => {
                            ui.weak("Select a part in the tree or create a new one.");
                            ui.add_space(8.0);
                            match self.source {
                                Source::Database => ui.label("The parts database is shared by all projects. Parts are copied into a project when they are assigned to a device."),
                                Source::Project => ui.label("Project parts are the copies stored in this project; they are used for the bill of materials and the project check."),
                            };
                        }
                    }
                });
            });
        });
        self.open = open;
        actions
    }

    /// Part selection dialog for a device. Adds the chosen part to the device.
    pub fn selector(&mut self, ctx: &egui::Context, db: &PartsDb, prj: &mut Project) {
        let Some(req) = &self.select else { return };
        let tag = req.tag.clone();
        let mut open = true;
        let mut chosen: Option<String> = None;
        // Database parts plus parts that exist only in the project.
        let mut all: Vec<Part> = db.parts.clone();
        for p in &prj.parts {
            if db.get(&p.number).is_none() {
                all.push(p.clone());
            }
        }
        let placed = parts::placed_functions(prj, &tag);
        let main = parts::main_kind(prj, &tag);
        egui::Window::new(format!("Part selection – {tag}")).open(&mut open).default_size([760.0, 480.0]).collapsible(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.sel_filter).hint_text("Filter…").desired_width(220.0));
                ui.checkbox(&mut self.sel_matching, "Device selection")
                    .on_hover_text("Only parts whose function template provides the device's main function");
            });
            ui.separator();
            let shown: Vec<&Part> = all
                .iter()
                .filter(|p| !self.sel_matching || placed.is_empty() || parts::fits_device(p, &placed, main))
                .collect();
            let h = ui.available_height() - 36.0;
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(vec2(300.0, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    egui::ScrollArea::vertical().id_salt("seltree").auto_shrink([false, false]).show(ui, |ui| {
                        if let Some(n) = part_tree(ui, "sel", &shown, &self.sel_filter, &mut self.sel_part) {
                            chosen = Some(n);
                        }
                    });
                });
                ui.separator();
                ui.vertical(|ui| {
                    match self.sel_part.as_ref().and_then(|n| all.iter().find(|p| &p.number == n)) {
                        Some(p) => part_summary(ui, p),
                        None => {
                            ui.weak("Select a part. Double click adds it to the device.");
                        }
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_enabled(self.sel_part.is_some(), egui::Button::new("✔ Add to device")).clicked() {
                    chosen = self.sel_part.clone();
                }
                if ui.button("Cancel").clicked() {
                    self.select = None;
                }
            });
        });
        if let Some(number) = chosen {
            if db.get(&number).is_some() {
                parts::ensure_in_project(prj, db, &number);
            }
            let dev = prj.devices.entry(tag).or_default();
            match dev.parts.iter_mut().find(|r| r.number == number) {
                Some(r) => r.count += 1,
                None => dev.parts.push(PartRef { number, count: 1 }),
            }
            self.select = None;
        }
        if !open {
            self.select = None;
        }
    }
}
