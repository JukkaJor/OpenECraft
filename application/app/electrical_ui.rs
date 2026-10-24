//! Electrical calculations: supply and installation conditions, short circuit and
//! selectivity, loads and power supply budgets, safety functions (ISO 13849-1) and
//! the rule libraries / customer rule sets the project is verified against.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use crate::electrical::{Earthing, Insulation, Material};
use crate::io::Purpose;
use crate::model::ReportKind;
use crate::reports::Table;
use crate::rules::{self, Check, Condition, Level, LimitOverrides, Op, RuleSet, Scope};
use crate::safety::{self, Category, Pl, SafetyFunction, Subsystem};
use egui::{Color32, RichText};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Supply,
    ShortCircuit,
    Loads,
    Safety,
    Rules,
}

#[derive(Default)]
pub struct ElectricalUi {
    pub open: bool,
    tab: Tab,
    /// Customer rule set being edited.
    rule_set: Option<usize>,
}

/// A report table with a jump button per row that has a target.
fn table_grid(ui: &mut egui::Ui, id: &str, t: &Table, jump: &mut Option<(usize, u64)>) {
    if t.rows.is_empty() {
        ui.weak(tr("Nothing to show."));
        return;
    }
    egui::ScrollArea::both().id_salt(id).max_height(260.0).auto_shrink([false, true]).show(ui, |ui| {
        egui::Grid::new(id).striped(true).spacing([10.0, 3.0]).show(ui, |ui| {
            ui.label("");
            for h in &t.headers {
                ui.label(RichText::new(*h).strong());
            }
            ui.end_row();
            for (row, target) in t.rows.iter().zip(&t.targets) {
                match target {
                    Some(tg) => {
                        if ui.small_button("➡").on_hover_text(tr("Click to show")).clicked() {
                            *jump = Some(*tg);
                        }
                    }
                    None => {
                        ui.label("");
                    }
                }
                for c in row {
                    ui.label(c);
                }
                ui.end_row();
            }
        });
    });
}

fn combo<T: PartialEq + Copy>(ui: &mut egui::Ui, id: impl std::hash::Hash + std::fmt::Debug, value: &mut T, all: &[T], title: impl Fn(T) -> String) {
    egui::ComboBox::from_id_salt(id).selected_text(title(*value)).show_ui(ui, |ui| {
        for x in all {
            ui.selectable_value(value, *x, title(*x));
        }
    });
}

/// An optional limit: checkbox and value.
fn limit_num(ui: &mut egui::Ui, label: &str, v: &mut Option<f32>, default: f32, suffix: &str, speed: f64) {
    let mut on = v.is_some();
    ui.checkbox(&mut on, label);
    match (on, v.as_mut()) {
        (true, Some(x)) => {
            ui.add(egui::DragValue::new(x).speed(speed).range(0.0..=100000.0).suffix(suffix));
        }
        (true, None) => *v = Some(default),
        (false, _) => {
            *v = None;
            ui.label("");
        }
    }
    ui.end_row();
}

fn limit_flag(ui: &mut egui::Ui, label: &str, v: &mut Option<bool>) {
    let mut on = v.unwrap_or(false);
    ui.checkbox(&mut on, label);
    *v = on.then_some(true);
    ui.label("");
    ui.end_row();
}

fn condition_edit(ui: &mut egui::Ui, id: impl std::hash::Hash + std::fmt::Debug + Copy, c: &mut Condition, scope: Scope) {
    egui::ComboBox::from_id_salt((id, "field")).selected_text(&c.field).width(110.0).show_ui(ui, |ui| {
        for f in scope.fields() {
            ui.selectable_value(&mut c.field, f.to_string(), *f);
        }
    });
    ui.add(egui::TextEdit::singleline(&mut c.field).desired_width(90.0)).on_hover_text(tr("Field; tech:<name> for tech data of parts, prop:<name> for project properties"));
    combo(ui, (id, "op"), &mut c.op, &Op::ALL, |o| o.title().into());
    if c.op.needs_value() {
        ui.add(egui::TextEdit::singleline(&mut c.value).desired_width(110.0));
    }
}

fn result_color(ok: bool) -> Color32 {
    if ok {
        Color32::from_rgb(50, 170, 80)
    } else {
        Color32::from_rgb(220, 60, 60)
    }
}

impl App {
    pub(super) fn electrical_window(&mut self, ctx: &egui::Context) {
        if !self.electrical_ui.open {
            return;
        }
        let mut open = true;
        let (mut jump, mut report) = (None, None);
        egui::Window::new(tr("Electrical calculations and verification")).open(&mut open).default_size([980.0, 560.0]).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let tab = &mut self.electrical_ui.tab;
                ui.selectable_value(tab, Tab::Supply, tr("Supply and conditions"));
                ui.selectable_value(tab, Tab::ShortCircuit, tr("Short circuit and selectivity"));
                ui.selectable_value(tab, Tab::Loads, tr("Loads"));
                ui.selectable_value(tab, Tab::Safety, tr("Functional safety"));
                ui.selectable_value(tab, Tab::Rules, tr("Rule sets and verification"));
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("electrical_tab").auto_shrink([false, false]).show(ui, |ui| match self.electrical_ui.tab {
                Tab::Supply => self.supply_tab(ui),
                Tab::ShortCircuit => self.short_circuit_tab(ui, &mut jump, &mut report),
                Tab::Loads => self.loads_tab(ui, &mut jump, &mut report),
                Tab::Safety => self.safety_tab(ui, &mut report),
                Tab::Rules => self.rules_tab(ctx, ui, &mut jump, &mut report),
            });
        });
        self.electrical_ui.open = open;
        if let Some(k) = report {
            self.report_kind = k;
            self.show_reports = true;
        }
        if let Some((page, id)) = jump {
            self.goto(page, id);
        }
    }

    fn supply_tab(&mut self, ui: &mut egui::Ui) {
        let el = &mut self.prj.electrical;
        ui.label(RichText::new(tr("Supply")).strong());
        egui::Grid::new("el_supply").num_columns(2).show(ui, |ui| {
            let s = &mut el.supply;
            ui.label(tr("Voltage (line to line)"));
            ui.add(egui::DragValue::new(&mut s.voltage).speed(1.0).range(1.0..=1000.0).suffix(" V"));
            ui.end_row();
            ui.label(tr("Frequency"));
            ui.add(egui::DragValue::new(&mut s.frequency).speed(1.0).range(0.0..=400.0).suffix(" Hz"));
            ui.end_row();
            ui.label(tr("Earthing system"));
            combo(ui, "el_earthing", &mut s.earthing, &Earthing::ALL, |e| e.title().into());
            ui.end_row();
            ui.label(tr("Highest short-circuit current"));
            ui.add(egui::DragValue::new(&mut s.ik_max).speed(0.1).range(0.1..=200.0).suffix(" kA")).on_hover_text(tr("Three-phase short-circuit current at the supply point"));
            ui.end_row();
            ui.label(tr("Lowest fault current"));
            ui.add(egui::DragValue::new(&mut s.ik_min).speed(0.01).range(0.0..=200.0).suffix(" kA")).on_hover_text(tr("Line to PE at the supply point; 0 = from the highest short-circuit current"));
            ui.end_row();
            ui.label(tr("Rated current of the supply"));
            ui.add(egui::DragValue::new(&mut s.rated_current).speed(1.0).range(0.0..=10000.0).suffix(" A")).on_hover_text(tr("0 = not limited"));
            ui.end_row();
        });
        ui.separator();
        ui.label(RichText::new(tr("Installation conditions of the conductors")).strong());
        egui::Grid::new("el_conditions").num_columns(2).show(ui, |ui| {
            let c = &mut el.conditions;
            ui.label(tr("Ambient temperature"));
            ui.add(egui::DragValue::new(&mut c.ambient).speed(1.0).range(-40.0..=85.0).suffix(" °C"));
            ui.end_row();
            ui.label(tr("Loaded circuits bunched together"));
            ui.add(egui::DragValue::new(&mut c.grouped).speed(0.1).range(1..=50));
            ui.end_row();
            ui.label(tr("Conductor material"));
            combo(ui, "el_material", &mut c.material, &Material::ALL, |m| m.title().into());
            ui.end_row();
            ui.label(tr("Insulation"));
            combo(ui, "el_insulation", &mut c.insulation, &Insulation::ALL, |m| m.title().into());
            ui.end_row();
            ui.label(tr("Diversity factor"));
            ui.add(egui::DragValue::new(&mut el.diversity).speed(0.01).range(0.0..=1.0)).on_hover_text(tr("0 = rated diversity factor of IEC 61439-1, table 101"));
            ui.end_row();
        });
        ui.weak(tr_fmt!("Current-carrying capacity: {} × IEC 60204-1, table 6", format!("{:.2}", el.conditions.factor())));
        ui.separator();
        let l = rules::limits(&self.prj);
        ui.label(RichText::new(tr("Limits in effect")).strong());
        let yes = |b: bool| if b { tr("yes") } else { tr("no") };
        egui::Grid::new("el_limits").num_columns(2).striped(true).show(ui, |ui| {
            for (label, value) in [
                (tr("Voltage drop"), format!("{} %", l.voltage_drop)),
                (tr("Disconnection of final circuits"), format!("{} s", l.disconnect_final)),
                (tr("Disconnection of distribution circuits"), format!("{} s", l.disconnect_distribution)),
                (tr("Smallest cross-section of power circuits"), format!("{} mm²", l.min_cross_section_power)),
                (tr("Smallest cross-section of control circuits"), format!("{} mm²", l.min_cross_section_control)),
                (tr("Load reserve"), format!("{} %", l.load_reserve)),
                (tr("Highest phase unbalance"), if l.max_unbalance > 0.0 { format!("{} %", l.max_unbalance) } else { "–".into() }),
                (tr("Total selectivity"), yes(l.total_selectivity).into()),
                (tr("Shielded drive cables"), yes(l.shield_drive_cables).into()),
                (tr("Shielded analogue cables"), yes(l.shield_analog_cables).into()),
                (tr("Surge protection"), yes(l.surge_protection).into()),
            ] {
                ui.label(label);
                ui.label(value);
                ui.end_row();
            }
        });
    }

    fn short_circuit_tab(&mut self, ui: &mut egui::Ui, jump: &mut Option<(usize, u64)>, report: &mut Option<ReportKind>) {
        ui.weak(tr("Breaking capacity against the short-circuit current at each protective device of the mains, disconnection at the lowest fault current at the end of its circuit, short-circuit withstand of the cables and selectivity with the device upstream."));
        for (k, id) in [(ReportKind::ShortCircuit, "el_sc"), (ReportKind::Selectivity, "el_sel")] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(k.title()).strong());
                if ui.small_button(tr("📊 Report")).clicked() {
                    *report = Some(k);
                }
            });
            if let Some(t) = self.analysis.reports.get(&k) {
                table_grid(ui, id, t, jump);
            }
            ui.add_space(6.0);
        }
    }

    fn loads_tab(&mut self, ui: &mut egui::Ui, jump: &mut Option<(usize, u64)>, report: &mut Option<ReportKind>) {
        ui.weak(tr("Power and current of the loads of the mains (motors, drives, heaters, power supplies, transformers), the load of each protective device, the demand with the diversity factor and the load budget of the power supplies."));
        for (k, id) in [(ReportKind::LoadCalculation, "el_loads"), (ReportKind::PowerSupplyBudget, "el_psu")] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(k.title()).strong());
                if ui.small_button(tr("📊 Report")).clicked() {
                    *report = Some(k);
                }
            });
            if let Some(t) = self.analysis.reports.get(&k) {
                table_grid(ui, id, t, jump);
            }
            ui.add_space(6.0);
        }
    }

    fn safety_tab(&mut self, ui: &mut egui::Ui, report: &mut Option<ReportKind>) {
        ui.weak(tr("Safety functions of ISO 13849-1: each subsystem (input, logic, output) with the devices of a channel, its category, diagnostic coverage and operations per year. Devices count with PL / PFHd, MTTFd or B10d of their parts."));
        ui.horizontal(|ui| {
            if ui.button(tr("➕ Safety function")).clicked() {
                let n = self.prj.electrical.safety.len() + 1;
                let sub = |name: &str| Subsystem { name: name.into(), category: Category::Three, dc: 90.0, operations: 10000.0, ccf: true, ..Default::default() };
                self.prj.electrical.safety.push(SafetyFunction {
                    name: format!("SF{n}"),
                    required: Pl::D,
                    subsystems: vec![sub(tr("Input")), sub(tr("Logic")), sub(tr("Output"))],
                    ..Default::default()
                });
            }
            if ui.button(tr("📊 Report")).clicked() {
                *report = Some(ReportKind::SafetyFunctions);
            }
        });
        let evaluations: Vec<safety::Evaluation> = self.prj.electrical.safety.iter().map(|f| safety::evaluate(&self.prj, f)).collect();
        let mut remove = None;
        for (fi, (f, e)) in self.prj.electrical.safety.iter_mut().zip(&evaluations).enumerate() {
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut f.name).desired_width(160.0));
                ui.add(egui::TextEdit::singleline(&mut f.description).hint_text(tr("Description")).desired_width(240.0));
                ui.label(tr("Required"));
                combo(ui, ("sf_req", fi), &mut f.required, &Pl::ALL, |p| safety::pl_text(Some(p)));
                ui.label(RichText::new(tr_fmt!("reached: PL {}", safety::pl_text(e.achieved))).color(result_color(e.is_met())));
                if ui.small_button("🗑").on_hover_text(tr("Delete")).clicked() {
                    remove = Some(fi);
                }
            });
            let mut remove_sub = None;
            egui::Grid::new(("sf_subs", fi)).striped(true).spacing([8.0, 3.0]).show(ui, |ui| {
                for h in [tr("Subsystem"), tr("Devices"), tr("Category"), tr("DCavg (%)"), tr("Operations / year"), tr("MTTFd (years)"), tr("CCF"), tr("PL"), ""] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for (si, (s, r)) in f.subsystems.iter_mut().zip(&e.subsystems).enumerate() {
                    ui.add(egui::TextEdit::singleline(&mut s.name).desired_width(90.0));
                    let mut text = s.devices.join(" ");
                    if ui.add(egui::TextEdit::singleline(&mut text).hint_text("-S1 -K1").desired_width(140.0)).changed() {
                        s.devices = text.split_whitespace().map(String::from).collect();
                    }
                    combo(ui, ("sf_cat", fi, si), &mut s.category, &Category::ALL, |c| c.title().into());
                    ui.add(egui::DragValue::new(&mut s.dc).speed(1.0).range(0.0..=100.0));
                    ui.add(egui::DragValue::new(&mut s.operations).speed(100.0).range(0.0..=1e8));
                    ui.add(egui::DragValue::new(&mut s.mttfd).speed(1.0).range(0.0..=2500.0)).on_hover_text(tr("0 = from the devices"));
                    ui.checkbox(&mut s.ccf, "").on_hover_text(tr("Measures against common cause failure (at least 65 points)"));
                    let text = r.mttfd.map(|m| format!("{} · MTTFd {m:.0}", safety::pl_text(r.pl))).unwrap_or_else(|| safety::pl_text(r.pl));
                    let label = ui.label(RichText::new(text).color(result_color(r.problems.is_empty() && r.pl.is_some())));
                    if !r.problems.is_empty() {
                        label.on_hover_text(r.problems.join("\n"));
                    }
                    if ui.small_button("🗑").on_hover_text(tr("Delete")).clicked() {
                        remove_sub = Some(si);
                    }
                    ui.end_row();
                }
            });
            if let Some(si) = remove_sub {
                f.subsystems.remove(si);
            }
            if ui.small_button(tr("➕ Subsystem")).clicked() {
                f.subsystems.push(Subsystem { name: tr_fmt!("Subsystem {}", f.subsystems.len() + 1), category: Category::Three, dc: 90.0, ccf: true, ..Default::default() });
            }
        }
        if let Some(i) = remove {
            self.prj.electrical.safety.remove(i);
        }
    }

    fn rules_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, jump: &mut Option<(usize, u64)>, report: &mut Option<ReportKind>) {
        ui.label(RichText::new(tr("Rule libraries")).strong());
        let standards = &mut self.prj.electrical.standards;
        for lib in rules::library() {
            let mut on = standards.contains(&lib.id);
            ui.horizontal_wrapped(|ui| {
                if ui.checkbox(&mut on, RichText::new(&lib.name).strong()).changed() {
                    standards.retain(|s| *s != lib.id);
                    if on {
                        standards.push(lib.id.clone());
                    }
                }
                ui.label(&lib.description);
                ui.weak(&lib.source);
            });
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Customer rule sets")).strong());
            if ui.button(tr("➕ Rule set")).clicked() {
                let n = self.prj.electrical.rule_sets.len() + 1;
                self.prj.electrical.rule_sets.push(RuleSet { name: tr_fmt!("Rule set {}", n), active: true, ..Default::default() });
                self.electrical_ui.rule_set = Some(self.prj.electrical.rule_sets.len() - 1);
            }
            if ui.button(tr("📂 Import…")).clicked() {
                self.io.open(ctx, Purpose::RuleSets);
            }
            if ui.add_enabled(!self.prj.electrical.rule_sets.is_empty(), egui::Button::new(tr("💾 Export…"))).clicked() {
                let json = rules::export(&self.prj.electrical.rule_sets);
                self.export(ctx, "rule_sets.json".into(), (tr("OpenECraft rule sets"), "json"), json);
            }
        });
        let mut remove = None;
        for (i, r) in self.prj.electrical.rule_sets.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.checkbox(&mut r.active, "").on_hover_text(tr("Verify the project against it"));
                let selected = self.electrical_ui.rule_set == Some(i);
                if ui.selectable_label(selected, if r.name.is_empty() { "?" } else { r.name.as_str() }).clicked() {
                    self.electrical_ui.rule_set = if selected { None } else { Some(i) };
                }
                ui.weak(tr_fmt!("{} checks", r.checks.len()));
                ui.weak(&r.source);
                if ui.small_button("🗑").on_hover_text(tr("Delete")).clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            self.prj.electrical.rule_sets.remove(i);
            self.electrical_ui.rule_set = None;
        }
        if let Some(r) = self.electrical_ui.rule_set.and_then(|i| self.prj.electrical.rule_sets.get_mut(i)) {
            ui.separator();
            rule_set_editor(ui, r);
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new(ReportKind::Verification.title()).strong());
            if ui.small_button(tr("📊 Report")).clicked() {
                *report = Some(ReportKind::Verification);
            }
        });
        if let Some(t) = self.analysis.reports.get(&ReportKind::Verification) {
            table_grid(ui, "el_verify", t, jump);
        }
    }

    /// Customer rule sets from a file: replace those of the same name.
    pub(super) fn import_rule_sets(&mut self, ctx: &egui::Context, text: &str) {
        match rules::import(text) {
            Ok(sets) => {
                let n = sets.len();
                for s in sets {
                    match self.prj.electrical.rule_sets.iter_mut().find(|r| r.name == s.name) {
                        Some(r) => *r = s,
                        None => self.prj.electrical.rule_sets.push(s),
                    }
                }
                self.notify(ctx, tr_fmt!("Imported {} rule sets", n));
            }
            Err(e) => self.notify(ctx, e),
        }
    }
}

fn rule_set_editor(ui: &mut egui::Ui, r: &mut RuleSet) {
    egui::Grid::new("rs_head").num_columns(2).show(ui, |ui| {
        ui.label(tr("Name"));
        ui.add(egui::TextEdit::singleline(&mut r.name).desired_width(240.0));
        ui.end_row();
        ui.label(tr("Source"));
        ui.add(egui::TextEdit::singleline(&mut r.source).hint_text(tr("Customer, plant or standard")).desired_width(240.0));
        ui.end_row();
        ui.label(tr("Description"));
        ui.add(egui::TextEdit::singleline(&mut r.description).desired_width(400.0));
        ui.end_row();
    });
    ui.collapsing(tr("Limits"), |ui| {
        let l: &mut LimitOverrides = &mut r.limits;
        egui::Grid::new("rs_limits").num_columns(2).show(ui, |ui| {
            limit_num(ui, tr("Voltage drop"), &mut l.voltage_drop, 5.0, " %", 0.1);
            limit_num(ui, tr("Disconnection of final circuits"), &mut l.disconnect_final, 0.4, " s", 0.01);
            limit_num(ui, tr("Disconnection of distribution circuits"), &mut l.disconnect_distribution, 5.0, " s", 0.1);
            limit_num(ui, tr("Smallest cross-section of power circuits"), &mut l.min_cross_section_power, 1.5, " mm²", 0.05);
            limit_num(ui, tr("Smallest cross-section of control circuits"), &mut l.min_cross_section_control, 0.5, " mm²", 0.05);
            limit_num(ui, tr("Load reserve"), &mut l.load_reserve, 20.0, " %", 1.0);
            limit_num(ui, tr("Highest phase unbalance"), &mut l.max_unbalance, 20.0, " %", 1.0);
            limit_flag(ui, tr("Total selectivity"), &mut l.total_selectivity);
            limit_flag(ui, tr("Shielded drive cables"), &mut l.shield_drive_cables);
            limit_flag(ui, tr("Shielded analogue cables"), &mut l.shield_analog_cables);
            limit_flag(ui, tr("Surge protection"), &mut l.surge_protection);
        });
    });
    ui.label(RichText::new(tr("Checks")).strong());
    ui.weak(tr("Every item of the scope that meets the conditions must meet the requirement."));
    let mut remove = None;
    for (ci, c) in r.checks.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                combo(ui, ("ck_scope", ci), &mut c.scope, &Scope::ALL, |s| s.title().into());
                combo(ui, ("ck_level", ci), &mut c.level, &Level::ALL, |l| l.title().into());
                ui.add(egui::TextEdit::singleline(&mut c.message).hint_text(tr("Message")).desired_width(320.0));
                if ui.small_button("🗑").on_hover_text(tr("Delete")).clicked() {
                    remove = Some(ci);
                }
            });
            let scope = c.scope;
            let mut remove_when = None;
            for (wi, w) in c.when.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(tr("if"));
                    condition_edit(ui, ("ck_when", ci, wi), w, scope);
                    if ui.small_button("✖").clicked() {
                        remove_when = Some(wi);
                    }
                });
            }
            if let Some(wi) = remove_when {
                c.when.remove(wi);
            }
            ui.horizontal(|ui| {
                ui.label(tr("then"));
                condition_edit(ui, ("ck_req", ci), &mut c.require, scope);
                if ui.small_button(tr("➕ Condition")).clicked() {
                    c.when.push(Condition { field: scope.fields()[0].into(), ..Default::default() });
                }
            });
        });
    }
    if let Some(ci) = remove {
        r.checks.remove(ci);
    }
    if ui.button(tr("➕ Check")).clicked() {
        r.checks.push(Check { require: Condition { field: "description".into(), ..Default::default() }, ..Default::default() });
    }
}
