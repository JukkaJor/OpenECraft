//! Property database window (definitions, function definitions, placement schemes,
//! values of any object) and the property grid shown in the property editor.

use openecraft_ui::icons::{self, Icon};
use openecraft_i18n::{tr, tr_fmt};
use super::App;
use openecraft_core::functions::{self, FnCategory, FunctionDef};
use crate::io::Purpose;
use openecraft_core::model::*;
use openecraft_core::parts::FnKind;
use openecraft_core::properties::{self, DataType, Group, Library, Object, ObjectClass, Origin, PropDef};
use openecraft_core::props::{self, Scheme};
use openecraft_core::symbols::{self, Anchor};
use egui::RichText;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Definitions,
    Functions,
    Schemes,
    Values,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Target {
    #[default]
    Project,
    Page,
    Device,
    Cable,
}

/// Filter and edit state of one property grid.
#[derive(Default)]
pub struct GridState {
    filter: String,
    /// Also groups that do not fit the function definition.
    all: bool,
    filled: bool,
    /// Value key and text being edited (taken over when the field loses focus).
    editing: Option<(String, String)>,
    error: Option<String>,
}

#[derive(Default)]
pub struct PropertiesUi {
    pub open: bool,
    tab: Tab,
    filter: String,
    class: Option<ObjectClass>,
    group: Option<Group>,
    /// User defined property being edited (new or existing) with its choices as text.
    draft: Option<(PropDef, String)>,
    function: String,
    function_filter: String,
    scheme: String,
    new_scheme: String,
    target: Target,
    target_key: String,
    pub device_grid: GridState,
    pub function_grid: GridState,
    values_grid: GridState,
}

enum Action {
    Import,
    Export,
    Reapply(String),
    Removed(usize),
    Error(String),
}

impl App {
    pub(super) fn property_db_pane(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let mut action = None;
        ui.scope(|ui| {
            let st = &mut self.properties_ui;
            ui.horizontal(|ui| {
                ui.selectable_value(&mut st.tab, Tab::Definitions, tr("Property definitions"));
                ui.selectable_value(&mut st.tab, Tab::Functions, tr("Function definitions"));
                ui.selectable_value(&mut st.tab, Tab::Schemes, tr("Placement schemes"));
                ui.selectable_value(&mut st.tab, Tab::Values, tr("Values"));
                ui.separator();
                if ui.button(tr("Import library…")).on_hover_text(tr("Adds user defined properties, function definitions and schemes of a property library file")).clicked() {
                    action = Some(Action::Import);
                }
                if ui.button(tr("Export library…")).on_hover_text(tr("Saves the project's own properties, function definitions and schemes for other projects")).clicked() {
                    action = Some(Action::Export);
                }
            });
            ui.separator();
            let prj = &mut self.session.prj;
            match st.tab {
                Tab::Definitions => definitions_tab(ui, prj, st, &mut action),
                Tab::Functions => functions_tab(ui, prj, st, &mut action),
                Tab::Schemes => schemes_tab(ui, prj, st, &mut action),
                Tab::Values => values_tab(ui, prj, st),
            }
        });
        match action {
            Some(Action::Import) => self.io.open(ctx, Purpose::PropertyLibrary),
            Some(Action::Export) => {
                let json = self.session.prj.property_db.to_json();
                self.export(ctx, "properties.json".into(), (tr("OpenECraft property library"), "json"), json);
            }
            Some(Action::Reapply(name)) => {
                let n = props::reapply(&mut self.session.prj, &name);
                self.notify(ctx, tr_fmt!("Scheme placed again at {} symbols", n));
            }
            Some(Action::Removed(n)) => self.notify(ctx, tr_fmt!("Property removed with {} values", n)),
            Some(Action::Error(e)) => self.notify(ctx, e),
            None => {}
        }
    }

    pub(super) fn import_property_library(&mut self, ctx: &egui::Context, text: &str) {
        match Library::from_json(text) {
            Ok(lib) => {
                let n = self.session.prj.property_db.merge(lib);
                self.notify(ctx, tr_fmt!("Imported {} entries of the property library", n));
            }
            Err(e) => self.notify_error(ctx, e),
        }
    }
}

// ------------------------------------------------------------------ definitions

fn definitions_tab(ui: &mut egui::Ui, prj: &mut Project, st: &mut PropertiesUi, action: &mut Option<Action>) {
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(&mut st.filter).hint_text(tr("Filter")).desired_width(160.0));
        egui::ComboBox::from_id_salt("pdb_class").selected_text(st.class.map(|c| c.title()).unwrap_or(tr("All objects"))).show_ui(ui, |ui| {
            ui.selectable_value(&mut st.class, None, tr("All objects"));
            for c in ObjectClass::ALL {
                ui.selectable_value(&mut st.class, Some(c), c.title());
            }
        });
        egui::ComboBox::from_id_salt("pdb_group").selected_text(st.group.map(|g| g.title()).unwrap_or(tr("All groups"))).show_ui(ui, |ui| {
            ui.selectable_value(&mut st.group, None, tr("All groups"));
            for g in Group::ALL {
                ui.selectable_value(&mut st.group, Some(g), g.title());
            }
        });
        if icons::text_button(ui, Icon::Plus, tr("User defined property")).clicked() {
            let class = st.class.filter(|c| *c != ObjectClass::Part).unwrap_or_default();
            st.draft = Some((PropDef { key: "user.".into(), class, group: Group::User, ..Default::default() }, String::new()));
        }
    });
    let total: usize = properties::all_defs(prj).count();
    let values: usize = properties::all_defs(prj).map(|d| d.value_keys().len()).sum();
    ui.weak(tr_fmt!("{} property definitions with {} values including the indexed ones", total, values));
    if let Some((draft, choices)) = &mut st.draft {
        egui::Frame::group(ui.style()).show(ui, |ui| draft_editor(ui, prj, draft, choices));
        let (draft, choices) = st.draft.clone().unwrap_or_default();
        ui.horizontal(|ui| {
            if ui.button(tr("Save")).clicked() {
                let d = PropDef { choices: choices.split(',').map(|c| c.trim().to_string()).filter(|c| !c.is_empty()).collect(), ..draft };
                match properties::define(prj, d) {
                    Ok(()) => st.draft = None,
                    Err(e) => *action = Some(Action::Error(e)),
                }
            }
            if ui.button(tr("Cancel")).clicked() {
                st.draft = None;
            }
        });
        ui.separator();
    }
    let f = st.filter.to_lowercase();
    let rows: Vec<PropDef> = properties::all_defs(prj)
        .filter(|d| st.class.is_none_or(|c| d.class == c) && st.group.is_none_or(|g| d.group == g))
        .filter(|d| f.is_empty() || d.title().to_lowercase().contains(&f) || d.key.contains(&f) || d.unit.to_lowercase() == f)
        .cloned()
        .collect();
    let mut edit = None;
    let mut remove = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Grid::new("pdb_defs").num_columns(7).striped(true).show(ui, |ui| {
            for h in [tr("Name"), tr("Key"), tr("Object"), tr("Group"), tr("Type"), tr("Unit"), ""] {
                ui.label(RichText::new(h).strong());
            }
            ui.end_row();
            for d in &rows {
                let name = if d.indices > 0 { format!("{} [1…{}]", d.title(), d.indices) } else { d.title() };
                ui.label(name);
                ui.label(RichText::new(&d.key).monospace().small());
                ui.label(d.class.title());
                ui.label(d.group.title());
                let ty = if d.ty == DataType::Choice && !d.choices.is_empty() { format!("{}: {}", d.ty.title(), d.choices.join(", ")) } else { d.ty.title().into() };
                ui.add(egui::Label::new(ty).truncate());
                ui.label(&d.unit);
                ui.horizontal(|ui| {
                    if d.builtin {
                        let kind = if d.read_only { tr("computed") } else if d.field { tr("model") } else { tr("built-in") };
                        ui.weak(kind);
                    } else {
                        if icons::button(ui, Icon::Markup, tr("Edit")).clicked() {
                            edit = Some(d.clone());
                        }
                        if icons::button(ui, Icon::Delete, tr("Delete the property and all its values")).clicked() {
                            remove = Some(d.key.clone());
                        }
                    }
                });
                ui.end_row();
            }
        });
    });
    if let Some(d) = edit {
        let choices = d.choices.join(", ");
        st.draft = Some((d, choices));
    }
    if let Some(key) = remove {
        match properties::undefine(prj, &key) {
            Ok(n) => *action = Some(Action::Removed(n)),
            Err(e) => *action = Some(Action::Error(e)),
        }
    }
}

fn draft_editor(ui: &mut egui::Ui, prj: &Project, d: &mut PropDef, choices: &mut String) {
    let exists = prj.property_db.properties.iter().any(|x| x.key == d.key);
    egui::Grid::new("pdb_draft").num_columns(4).show(ui, |ui| {
        ui.label(tr("Key"));
        ui.add_enabled(!exists, egui::TextEdit::singleline(&mut d.key).desired_width(160.0).font(egui::TextStyle::Monospace));
        ui.label(tr("Name"));
        ui.text_edit_singleline(&mut d.name);
        ui.end_row();
        ui.label(tr("Object"));
        egui::ComboBox::from_id_salt("draft_class").selected_text(d.class.title()).show_ui(ui, |ui| {
            for c in ObjectClass::ALL.into_iter().filter(|c| *c != ObjectClass::Part) {
                ui.selectable_value(&mut d.class, c, c.title());
            }
        });
        ui.label(tr("Group"));
        egui::ComboBox::from_id_salt("draft_group").selected_text(d.group.title()).show_ui(ui, |ui| {
            for g in Group::ALL {
                ui.selectable_value(&mut d.group, g, g.title());
            }
        });
        ui.end_row();
        ui.label(tr("Type"));
        egui::ComboBox::from_id_salt("draft_type").selected_text(d.ty.title()).show_ui(ui, |ui| {
            for t in DataType::ALL {
                ui.selectable_value(&mut d.ty, t, t.title());
            }
        });
        ui.label(tr("Unit"));
        ui.add(egui::TextEdit::singleline(&mut d.unit).desired_width(60.0));
        ui.end_row();
        ui.label(tr("Indices"));
        ui.add(egui::DragValue::new(&mut d.indices).range(0..=500)).on_hover_text(tr("0 = a single value; otherwise values [1] … [n]"));
        ui.label(tr("Default"));
        ui.text_edit_singleline(&mut d.default);
        ui.end_row();
        if d.ty == DataType::Choice {
            ui.label(tr("Choices"));
            ui.text_edit_singleline(choices).on_hover_text(tr("Values separated by commas"));
            ui.end_row();
        }
        ui.label(tr("Description"));
        ui.text_edit_singleline(&mut d.description);
        ui.end_row();
    });
}

// ------------------------------------------------------------- function definitions

fn functions_tab(ui: &mut egui::Ui, prj: &mut Project, st: &mut PropertiesUi, action: &mut Option<Action>) {
    let usage = functions::usage(prj);
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        ui.add(egui::TextEdit::singleline(&mut st.function_filter).hint_text(tr("Filter")));
        let f = st.function_filter.to_lowercase();
        egui::ScrollArea::vertical().id_salt("fn_list").auto_shrink([false, false]).show(ui, |ui| {
            for c in FnCategory::ALL {
                let defs: Vec<&FunctionDef> = functions::all(prj).filter(|d| d.category == c && (f.is_empty() || d.title().to_lowercase().contains(&f) || d.key.contains(&f))).collect();
                if defs.is_empty() {
                    continue;
                }
                egui::CollapsingHeader::new(c.title()).id_salt(("fncat", c)).default_open(!f.is_empty()).show(ui, |ui| {
                    for d in defs {
                        let n = usage.get(&d.key).copied().unwrap_or(0);
                        let label = if n > 0 { format!("{} ({n})", d.title()) } else { d.title() };
                        let text = if d.builtin { RichText::new(label) } else { RichText::new(label).strong() };
                        if ui.selectable_label(st.function == d.key, text).clicked() {
                            st.function = d.key.clone();
                        }
                    }
                });
            }
        });
        let ui = &mut cols[1];
        function_details(ui, prj, st, action);
    });
}

fn function_details(ui: &mut egui::Ui, prj: &mut Project, st: &mut PropertiesUi, action: &mut Option<Action>) {
    let Some(d) = functions::get(prj, &st.function).cloned() else {
        ui.weak(tr("Select a function definition. Project definitions (bold) can be changed and are kept in property libraries."));
        return;
    };
    ui.label(RichText::new(d.title()).strong());
    ui.label(RichText::new(&d.key).monospace().small());
    if d.builtin {
        egui::Grid::new("fn_builtin").num_columns(2).show(ui, |ui| {
            ui.label(tr("Category"));
            ui.label(d.category.title());
            ui.end_row();
            ui.label(tr("Function template"));
            ui.label(tr(d.kind.title()));
            ui.end_row();
            ui.label(tr("Identifying letter"));
            ui.label(&d.letter);
            ui.end_row();
            ui.label(tr("Symbol"));
            ui.label(prj.symbol(&d.symbol).map(|s| tr(s.name)).unwrap_or(""));
            ui.end_row();
        });
        ui.label(RichText::new(tr("Property groups")).strong());
        ui.label(d.category.groups().iter().map(|g| g.title()).collect::<Vec<_>>().join(", "));
        if icons::text_button(ui, Icon::Plus, tr("Derive project definition")).on_hover_text(tr("A project definition based on this one, with its own defaults and scheme")).clicked() {
            let new = functions::derive(prj, &d);
            st.function = new.key.clone();
            if let Err(e) = functions::define(prj, new) {
                *action = Some(Action::Error(e));
            }
        }
        return;
    }
    let schemes: Vec<(String, String)> = props::all_schemes(prj).map(|s| (s.name.clone(), s.title())).collect();
    let pdefs: Vec<(String, Option<PropDef>)> = d.defaults.keys().map(|k| (k.clone(), properties::valid_key(prj, k).cloned())).collect();
    let classes = [ObjectClass::Device, ObjectClass::Function, ObjectClass::Terminal, ObjectClass::PlcPoint];
    let symbol_name = prj.property_db.functions.iter().find(|x| x.key == d.key).and_then(|x| prj.symbol(&x.symbol)).map(|s| tr(s.name)).unwrap_or("");
    let Some(def) = prj.property_db.functions.iter_mut().find(|x| x.key == d.key) else { return };
    egui::Grid::new("fn_edit").num_columns(2).show(ui, |ui| {
        ui.label(tr("Name"));
        ui.text_edit_singleline(&mut def.name);
        ui.end_row();
        ui.label(tr("Category"));
        egui::ComboBox::from_id_salt("fn_cat").selected_text(def.category.title()).show_ui(ui, |ui| {
            for c in FnCategory::ALL {
                ui.selectable_value(&mut def.category, c, c.title());
            }
        });
        ui.end_row();
        ui.label(tr("Function template"));
        egui::ComboBox::from_id_salt("fn_kind").selected_text(tr(def.kind.title())).height(320.0).show_ui(ui, |ui| {
            for k in FnKind::ALL {
                ui.selectable_value(&mut def.kind, k, tr(k.title()));
            }
        });
        ui.end_row();
        ui.label(tr("Identifying letter"));
        ui.add(egui::TextEdit::singleline(&mut def.letter).desired_width(50.0));
        ui.end_row();
        ui.label(tr("Symbol"));
        egui::ComboBox::from_id_salt("fn_sym").selected_text(symbol_name).height(320.0).show_ui(ui, |ui| {
            for s in symbols::library() {
                ui.selectable_value(&mut def.symbol, s.key.to_string(), tr(s.name));
            }
        });
        ui.end_row();
        ui.label(tr("Connection points"));
        ui.text_edit_singleline(&mut def.pins).on_hover_text(tr("Designations separated by blanks; empty: those of the symbol"));
        ui.end_row();
        ui.label(tr("Placement scheme"));
        let shown = schemes.iter().find(|s| s.0 == def.scheme).map(|s| s.1.clone()).unwrap_or_else(|| tr("(none)").into());
        egui::ComboBox::from_id_salt("fn_scheme").selected_text(shown).show_ui(ui, |ui| {
            ui.selectable_value(&mut def.scheme, String::new(), tr("(none)"));
            for (name, title) in &schemes {
                ui.selectable_value(&mut def.scheme, name.clone(), title);
            }
        });
        ui.end_row();
        ui.label(tr("Description"));
        ui.text_edit_singleline(&mut def.description);
        ui.end_row();
    });
    ui.label(RichText::new(tr("Default values")).strong());
    let mut remove = None;
    egui::Grid::new("fn_defaults").num_columns(3).striped(true).show(ui, |ui| {
        for (k, pd) in &pdefs {
            ui.label(pd.as_ref().map(|d| d.title_of(k)).unwrap_or_else(|| k.clone())).on_hover_text(k);
            if let Some(v) = def.defaults.get_mut(k) {
                let r = ui.text_edit_singleline(v);
                if r.lost_focus() {
                    if let Some(norm) = pd.as_ref().and_then(|d| d.check(v).ok()) {
                        *v = norm;
                    }
                }
                if pd.as_ref().is_some_and(|d| d.check(v).is_err()) {
                    r.on_hover_text(tr("Invalid value"));
                }
            }
            if icons::button(ui, Icon::Delete, tr("Remove")).clicked() {
                remove = Some(k.clone());
            }
            ui.end_row();
        }
    });
    if let Some(k) = remove {
        def.defaults.remove(&k);
    }
    let d2 = def.clone();
    if let Some(k) = key_picker(ui, prj, &classes, true, tr("➕ Default value")) {
        if let Some(def) = prj.property_db.functions.iter_mut().find(|f| f.key == d2.key) {
            def.defaults.entry(k).or_default();
        }
    }
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button(tr("Duplicate")).clicked() {
            let new = functions::derive(prj, &d2);
            st.function = new.key.clone();
            if let Err(e) = functions::define(prj, new) {
                *action = Some(Action::Error(e));
            }
        }
        if icons::text_button(ui, Icon::Delete, tr("Delete")).on_hover_text(tr("Symbols using it go back to their built-in definition")).clicked() {
            functions::remove(prj, &d2.key);
            st.function.clear();
        }
    });
}

// ---------------------------------------------------------------------- schemes

fn schemes_tab(ui: &mut egui::Ui, prj: &mut Project, st: &mut PropertiesUi, action: &mut Option<Action>) {
    let mut users: std::collections::BTreeMap<String, usize> = Default::default();
    for s in prj.pages.iter().flat_map(|p| p.symbols.iter()).filter(|s| !s.scheme.is_empty()) {
        *users.entry(s.scheme.clone()).or_default() += 1;
    }
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut st.new_scheme).hint_text(tr("New scheme")).desired_width(140.0));
            if icons::button(ui, Icon::Plus, tr("New, empty placement scheme")).clicked() {
                match props::define_scheme(prj, Scheme { name: st.new_scheme.trim().into(), ..Default::default() }) {
                    Ok(()) => {
                        st.scheme = st.new_scheme.trim().into();
                        st.new_scheme.clear();
                    }
                    Err(e) => *action = Some(Action::Error(e)),
                }
            }
        });
        egui::ScrollArea::vertical().id_salt("scheme_list").auto_shrink([false, false]).show(ui, |ui| {
            for s in props::all_schemes(prj) {
                let n = users.get(&s.name).copied().unwrap_or(0);
                let mut text = RichText::new(if n > 0 { format!("{} ({n})", s.title()) } else { s.title() });
                if !s.builtin {
                    text = text.strong();
                }
                if ui.selectable_label(st.scheme == s.name, text).clicked() {
                    st.scheme = s.name.clone();
                }
            }
        });
        let ui = &mut cols[1];
        let Some(scheme) = props::scheme(prj, &st.scheme).cloned() else {
            ui.weak(tr("Select a placement scheme. A scheme places properties beside symbols; symbols using it are updated when it changes."));
            return;
        };
        ui.label(RichText::new(scheme.title()).strong());
        let classes = ObjectClass::ALL;
        if scheme.builtin {
            for e in &scheme.entries {
                ui.label(properties::def(prj, &e.key).map(|d| d.title_of(&e.key)).unwrap_or_else(|| e.key.clone()));
            }
            if ui.button(tr("Duplicate")).clicked() {
                let name = tr_fmt!("{} (copy)", scheme.title());
                if props::define_scheme(prj, Scheme { name: name.clone(), builtin: false, ..scheme.clone() }).is_ok() {
                    st.scheme = name;
                }
            }
            return;
        }
        let titles: Vec<String> = scheme.entries.iter().map(|e| properties::def(prj, &e.key).map(|d| d.title_of(&e.key)).unwrap_or_else(|| e.key.clone())).collect();
        let added = key_picker(ui, prj, &classes, false, tr("➕ Property"));
        let Some(s) = prj.property_db.schemes.iter_mut().find(|s| s.name == scheme.name) else { return };
        ui.horizontal(|ui| {
            ui.label(tr("Description"));
            ui.text_edit_singleline(&mut s.description);
        });
        let mut remove = None;
        egui::Grid::new("scheme_entries").num_columns(6).striped(true).show(ui, |ui| {
            for (i, e) in s.entries.iter_mut().enumerate() {
                ui.label(&titles[i]).on_hover_text(&e.key);
                ui.add(egui::DragValue::new(&mut e.offset.x).speed(0.5).prefix("x ").suffix(" mm"));
                ui.add(egui::DragValue::new(&mut e.offset.y).speed(0.5).prefix("y ").suffix(" mm"));
                ui.add(egui::DragValue::new(&mut e.size).range(1.0..=10.0).speed(0.1).suffix(" mm"));
                ui.horizontal(|ui| {
                    anchor_combo(ui, ("scheme_anchor", i), &mut e.anchor);
                    ui.checkbox(&mut e.name, tr("Name")).on_hover_text(tr("Shown as \"Name: value\""));
                });
                if icons::button(ui, Icon::Delete, tr("Remove")).clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
        if let Some(i) = remove {
            s.entries.remove(i);
        }
        if let Some(k) = added {
            let y = s.entries.iter().map(|e| e.offset.y + 2.8).fold(0.0, f32::max);
            s.entries.push(PlacedProp { key: k, offset: openecraft_core::geom::p(0.0, y), ..Default::default() });
        }
        ui.weak(tr("Positions are relative to the lower right corner of the symbol."));
        ui.horizontal(|ui| {
            if ui.button(tr("Place again at all symbols using it")).clicked() {
                *action = Some(Action::Reapply(scheme.name.clone()));
            }
            if icons::text_button(ui, Icon::Delete, tr("Delete")).on_hover_text(tr("Symbols using it keep their placed properties")).clicked() {
                props::remove_scheme(prj, &scheme.name);
                st.scheme.clear();
            }
        });
    });
}

pub(super) fn anchor_combo(ui: &mut egui::Ui, id: impl std::hash::Hash + std::fmt::Debug, a: &mut Anchor) {
    let title = |a: Anchor| match a {
        Anchor::Left => tr("Left"),
        Anchor::Center => tr("Centre"),
        Anchor::Right => tr("Right"),
    };
    egui::ComboBox::from_id_salt(id).selected_text(title(*a)).width(70.0).show_ui(ui, |ui| {
        for x in [Anchor::Left, Anchor::Center, Anchor::Right] {
            ui.selectable_value(a, x, title(x));
        }
    });
}

// ----------------------------------------------------------------------- values

fn values_tab(ui: &mut egui::Ui, prj: &mut Project, st: &mut PropertiesUi) {
    let stats = properties::statistics(prj);
    if !stats.is_empty() {
        ui.weak(stats.iter().map(|(c, n)| format!("{}: {n}", c.title())).collect::<Vec<_>>().join("   "));
    }
    let mut obj = None;
    ui.horizontal_wrapped(|ui| {
        for (t, name) in [(Target::Project, tr("Project")), (Target::Page, tr("Page")), (Target::Device, tr("Device")), (Target::Cable, tr("Cable"))] {
            if ui.selectable_value(&mut st.target, t, name).changed() {
                st.target_key.clear();
            }
        }
        let items: Vec<(String, String)> = match st.target {
            Target::Project => Vec::new(),
            Target::Page => prj.pages.iter().map(|p| (p.id.to_string(), format!("{} {}", p.name, p.description))).collect(),
            Target::Device => prj.all_tags().into_iter().map(|t| (t.clone(), t)).collect(),
            Target::Cable => prj.cables.iter().map(|c| (c.tag.clone(), c.tag.clone())).collect(),
        };
        if st.target != Target::Project {
            if st.target_key.is_empty() {
                if let Some(first) = items.first() {
                    st.target_key = first.0.clone();
                }
            }
            let shown = items.iter().find(|i| i.0 == st.target_key).map(|i| i.1.clone()).unwrap_or_default();
            egui::ComboBox::from_id_salt("values_obj").selected_text(shown).height(360.0).show_ui(ui, |ui| {
                for (k, t) in &items {
                    ui.selectable_value(&mut st.target_key, k.clone(), t);
                }
            });
        }
        obj = match st.target {
            Target::Project => Some(Object::Project),
            Target::Page => st.target_key.parse().ok().map(Object::Page),
            Target::Device if !st.target_key.is_empty() => Some(Object::Device(st.target_key.clone())),
            Target::Cable if !st.target_key.is_empty() => Some(Object::Cable(st.target_key.clone())),
            _ => None,
        };
    });
    ui.separator();
    match obj {
        Some(obj) if obj.exists(prj) => {
            egui::ScrollArea::vertical().id_salt("values_grid").auto_shrink([false, false]).show(ui, |ui| property_grid(ui, prj, &obj, &mut st.values_grid, "values"));
        }
        _ => {
            ui.weak(tr("Nothing to show"));
        }
    }
}

// ------------------------------------------------------------- property grid

/// All properties of an object, grouped, with editors for the changeable ones.
pub(super) fn property_grid(ui: &mut egui::Ui, prj: &mut Project, obj: &Object, st: &mut GridState, salt: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(&mut st.filter).hint_text(tr("Filter")).desired_width(110.0));
        ui.checkbox(&mut st.filled, tr("With values only"));
        if properties::function_def(prj, obj).is_some() {
            ui.checkbox(&mut st.all, tr("All groups")).on_hover_text(tr("Also the groups that do not fit the function definition"));
        }
    });
    if let Some(e) = &st.error {
        ui.colored_label(crate::style::error(), e);
    }
    let f = st.filter.to_lowercase();
    let keys: Vec<(String, PropDef)> = properties::keys_of(prj, obj, !st.all, st.filled)
        .into_iter()
        .filter(|(k, d)| f.is_empty() || d.title_of(k).to_lowercase().contains(&f) || k.contains(&f))
        .map(|(k, d)| (k, d.clone()))
        .collect();
    for g in Group::ALL {
        let rows: Vec<&(String, PropDef)> = keys.iter().filter(|(_, d)| d.group == g).collect();
        if rows.is_empty() {
            continue;
        }
        let open = !f.is_empty() || st.filled || matches!(g, Group::General);
        egui::CollapsingHeader::new(format!("{} ({})", g.title(), rows.len())).id_salt((salt, g)).open(if !f.is_empty() || st.filled { Some(true) } else { None }).default_open(open).show(ui, |ui| {
            egui::Grid::new((salt, "grid", g)).num_columns(2).striped(true).show(ui, |ui| {
                for (k, d) in rows {
                    property_row(ui, prj, obj, st, k, d);
                    ui.end_row();
                }
            });
        });
    }
    let other = properties::undefined_values(prj, obj);
    if !other.is_empty() {
        egui::CollapsingHeader::new(tr_fmt!("Other values ({})", other.len())).id_salt((salt, "other")).show(ui, |ui| {
            ui.weak(tr("Values without a property definition (free project properties, removed definitions)"));
            for (k, v) in other {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&k).monospace());
                    ui.label(v);
                });
            }
        });
    }
}

/// Width of the value column of the property grid (egui shrinks grid columns otherwise).
const VALUE_WIDTH: f32 = 200.0;

fn property_row(ui: &mut egui::Ui, prj: &mut Project, obj: &Object, st: &mut GridState, k: &str, d: &PropDef) {
    let name = d.title_of(k);
    let hover = format!("{k}\n{}{}", d.ty.title(), if d.unit.is_empty() { String::new() } else { format!(", {}", d.unit) });
    ui.label(&name).on_hover_text(&hover);
    let origin = properties::origin(prj, obj, k);
    if d.read_only || d.class == ObjectClass::Part {
        let v = properties::display(prj, obj, k);
        ui.horizontal(|ui| {
            ui.set_min_width(VALUE_WIDTH);
            if v.is_empty() {
                ui.weak("–");
            } else {
                ui.add(egui::Label::new(v).truncate());
            }
        });
        return;
    }
    let own = if d.field { properties::get(prj, obj, k) } else { properties::stored(prj, obj, k).unwrap_or_default().to_string() };
    let inherited = if own.is_empty() && !d.field { properties::display(prj, obj, k) } else { String::new() };
    let mut commit = None;
    ui.horizontal(|ui| {
        ui.set_min_width(VALUE_WIDTH);
        match d.ty {
            DataType::Bool | DataType::Choice => {
                let mut v = own.clone();
                let shown = if v.is_empty() { inherited.clone() } else { d.format(&v) };
                let shown = if v.is_empty() && !shown.is_empty() { RichText::new(shown).weak() } else { RichText::new(shown) };
                let options: Vec<(String, String)> = if d.ty == DataType::Bool {
                    vec![("true".into(), tr("Yes").into()), ("false".into(), tr("No").into())]
                } else {
                    d.choices.iter().map(|c| (c.clone(), c.clone())).collect()
                };
                egui::ComboBox::from_id_salt(("pval", k)).selected_text(shown).width(150.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut v, String::new(), "–");
                    for (value, title) in &options {
                        ui.selectable_value(&mut v, value.clone(), title);
                    }
                });
                if v != own {
                    commit = Some(v);
                }
            }
            _ => {
                let mut text = match &st.editing {
                    Some((ek, t)) if ek == k => t.clone(),
                    _ => own.clone(),
                };
                let hint = if inherited.is_empty() { RichText::new(&d.unit) } else { RichText::new(&inherited) };
                let edit = if d.ty == DataType::MultiLine {
                    egui::TextEdit::multiline(&mut text).desired_rows(2)
                } else {
                    egui::TextEdit::singleline(&mut text)
                };
                let r = ui.add(edit.hint_text(hint).desired_width(150.0));
                let r = match origin {
                    Origin::FunctionDefinition => r.on_hover_text(tr("From the function definition")),
                    Origin::Default => r.on_hover_text(tr("Default (part data or definition)")),
                    _ => r,
                };
                if r.changed() {
                    st.editing = Some((k.to_string(), text.clone()));
                }
                if r.lost_focus() {
                    if let Some((_, t)) = st.editing.take_if(|(ek, _)| ek == k) {
                        if t != own {
                            commit = Some(t);
                        }
                    }
                }
            }
        }
        if !d.unit.is_empty() && !matches!(d.ty, DataType::Bool | DataType::Choice) {
            ui.weak(&d.unit);
        }
        if origin == Origin::Stored && !d.field && icons::button(ui, Icon::Delete, tr("Remove the value")).clicked() {
            commit = Some(String::new());
        }
    });
    if let Some(v) = commit {
        st.error = properties::set(prj, obj, k, &v).err();
    }
}

/// Menu that offers the property keys of some object classes; returns the chosen key.
/// `editable`: only properties that can have values (function definition defaults).
pub(super) fn key_picker(ui: &mut egui::Ui, prj: &Project, classes: &[ObjectClass], editable: bool, label: &str) -> Option<String> {
    let mut picked = None;
    // Stays open while the filter and index fields are used.
    let config = egui::containers::menu::MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
    egui::containers::menu::MenuButton::new(label).config(config).ui(ui, |ui| {
        let id = ui.scope_id().with("key_filter");
        let mut f: String = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
        ui.add(egui::TextEdit::singleline(&mut f).hint_text(tr("Filter")).desired_width(200.0));
        let fl = f.to_lowercase();
        egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
            for c in classes {
                let defs: Vec<&PropDef> = properties::all_defs(prj).filter(|d| d.class == *c && (!editable || (!d.read_only && !d.field)) && (fl.is_empty() || d.title().to_lowercase().contains(&fl) || d.key.contains(&fl))).collect();
                if defs.is_empty() {
                    continue;
                }
                ui.label(RichText::new(c.title()).strong());
                for d in defs {
                    if d.indices == 0 {
                        if ui.button(d.title()).on_hover_text(&d.key).clicked() {
                            picked = Some(d.key.clone());
                        }
                        continue;
                    }
                    ui.horizontal(|ui| {
                        let iid = id.with(&d.key);
                        let mut i: u16 = ui.data_mut(|x| x.get_temp(iid)).unwrap_or(1);
                        ui.add(egui::DragValue::new(&mut i).range(1..=d.indices));
                        if ui.button(d.title()).on_hover_text(&d.key).clicked() {
                            picked = Some(format!("{}[{i}]", d.key));
                        }
                        ui.data_mut(|x| x.insert_temp(iid, i));
                    });
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(id, f));
        if picked.is_some() {
            ui.close();
        }
    });
    picked
}

/// Function definition of a symbol, with the definitions it can take.
pub(super) fn function_def_combo(ui: &mut egui::Ui, prj: &mut Project, symbol_id: u64) {
    let Some((_, s)) = properties::find_symbol(prj, symbol_id) else { return };
    let current = functions::of_symbol(prj, s).map(|d| (d.key.clone(), d.title())).unwrap_or_default();
    let options: Vec<(String, String, bool)> = functions::fitting(prj, s).into_iter().map(|d| (d.key.clone(), d.title(), d.builtin)).collect();
    let mut chosen = None;
    egui::ComboBox::from_id_salt("fn_def").selected_text(current.1).width(170.0).show_ui(ui, |ui| {
        for (key, title, builtin) in &options {
            let text = if *builtin { RichText::new(title) } else { RichText::new(title).strong() };
            if ui.selectable_label(*key == current.0, text).clicked() {
                chosen = Some(key.clone());
            }
        }
    });
    if let Some(k) = chosen {
        let _ = functions::assign(prj, symbol_id, &k);
    }
}
