//! Fluid and process: hose properties of fluid wires, the hose defaults, the fluid
//! netlist, fluid device groups, the links to the electrical devices and the PCT
//! loops of the pre-planning with their engineering status.

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use openecraft_core::electrofluid::{self, DeviceGroup, GroupKind};
use openecraft_core::fluid::{self, LineType, Medium};
use openecraft_core::model::{ReportId, ReportKind};
use openecraft_core::process::{self, PctLoop, Status};
use openecraft_core::symbols::Category;
use egui::{Color32, RichText};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Hoses,
    Groups,
    ElectroFluid,
    Loops,
}

#[derive(Default)]
pub struct ProcessUi {
    pub open: bool,
    tab: Tab,
    /// Result of the last synchronization.
    message: String,
}

fn medium_combo(ui: &mut egui::Ui, id: &str, value: &mut Option<Medium>, default: Medium) {
    let shown = value.map(|m| m.title().to_string()).unwrap_or_else(|| tr_fmt!("{} (default)", default.title()));
    egui::ComboBox::from_id_salt(id).selected_text(shown).show_ui(ui, |ui| {
        ui.selectable_value(value, None, tr_fmt!("{} (default)", default.title()));
        for m in Medium::ALL {
            ui.selectable_value(value, Some(m), m.title());
        }
    });
}

fn status_color(s: Status) -> Color32 {
    match s {
        Status::Planned => Color32::GRAY,
        Status::InProgress => Color32::from_rgb(230, 150, 30),
        Status::Engineered => Color32::from_rgb(50, 170, 80),
    }
}

fn tags_edit(ui: &mut egui::Ui, tags: &mut Vec<String>, hint: &str, width: f32) {
    let mut text = tags.join(" ");
    if ui.add(egui::TextEdit::singleline(&mut text).hint_text(hint).desired_width(width)).changed() {
        *tags = text.split_whitespace().map(String::from).collect();
    }
}

impl App {
    /// Properties of a wire of a fluid net (hose / pipe); they apply to the whole hose.
    pub(super) fn hose_properties(&mut self, ui: &mut egui::Ui, id: u64) {
        let Some(net) = self.session.analysis.open.net_of_wire(id).cloned() else { return };
        let (medium, size, color, length) = openecraft_core::fluid::props_of(&self.session.prj, &net);
        ui.label(RichText::new(tr_fmt!("Hose / pipe {}", net.name)).strong());
        let hoses: Vec<fluid::Hose> = fluid::hoses(&self.session.prj, &self.session.analysis.open).into_iter().filter(|h| h.net == net.name).collect();
        if !hoses.is_empty() {
            let lines: Vec<String> = hoses.iter().map(|h| format!("{} – {}", h.from, h.to)).collect();
            ui.label(RichText::new(lines.join("\n")).monospace());
        }
        if let Some(line) = fluid::fluid_nets(&self.session.prj, &self.session.analysis.open).into_iter().find(|n| n.net == net.name).map(|n| n.line) {
            ui.weak(line.title());
        }
        ui.separator();
        let number_now = openecraft_core::wirenum::number_of(&self.session.prj, &net);
        let mut number = number_now.clone();
        let first = openecraft_core::wirenum::wires_of(&self.session.prj, &net).first().map(|w| (w.medium, w.cross_section.clone(), w.color.clone(), w.length));
        let (mut m, mut s, mut c, mut l) = first.clone().unwrap_or((None, String::new(), String::new(), 0.0));
        let config_now = fluid::config_of(&self.session.prj, &net);
        let mut cfg = config_now.clone();
        let defaults = self.session.prj.fluid.clone();
        egui::Grid::new("hoseprops").num_columns(2).show(ui, |ui| {
            ui.label(tr("Hose number"));
            ui.add(egui::TextEdit::singleline(&mut number).hint_text(tr("not numbered")).desired_width(90.0));
            ui.end_row();
            ui.label(tr("Type"));
            egui::ComboBox::from_id_salt("hline").selected_text(cfg.line.title()).show_ui(ui, |ui| {
                for t in LineType::ALL {
                    ui.selectable_value(&mut cfg.line, t, t.title());
                }
            });
            ui.end_row();
            ui.label(tr("Medium"));
            medium_combo(ui, "hmedium", &mut m, defaults.medium);
            ui.end_row();
            ui.label(tr("Nominal size"));
            ui.add(egui::TextEdit::singleline(&mut s).hint_text(&defaults.size).desired_width(90.0)).on_hover_text(tr("Outer diameter (e.g. 8 mm) or DN"));
            ui.end_row();
            ui.label(tr("Colour"));
            ui.add(egui::TextEdit::singleline(&mut c).hint_text(&defaults.color).desired_width(90.0));
            ui.end_row();
            ui.label(tr("Length"));
            ui.add(egui::DragValue::new(&mut l).speed(0.05).range(0.0..=1000.0).suffix(" m"));
            ui.end_row();
            ui.label(tr("Part"));
            ui.add(egui::TextEdit::singleline(&mut cfg.part).desired_width(110.0)).on_hover_text(tr("Hose / tube / pipe, sold by the metre"));
            ui.end_row();
            ui.label(tr("Fitting at the start"));
            ui.add(egui::TextEdit::singleline(&mut cfg.fitting_a).hint_text("QS-1/8-8").desired_width(110.0));
            ui.end_row();
            ui.label(tr("Fitting at the end"));
            ui.add(egui::TextEdit::singleline(&mut cfg.fitting_b).hint_text("QS-1/8-8").desired_width(110.0));
            ui.end_row();
            ui.label(tr("Max. pressure"));
            ui.add(egui::DragValue::new(&mut cfg.pressure).speed(0.5).range(0.0..=1000.0).suffix(" bar"));
            ui.end_row();
            ui.label(tr("Min. bend radius"));
            ui.add(egui::DragValue::new(&mut cfg.bend_radius).speed(1.0).range(0.0..=5000.0).suffix(" mm"));
            ui.end_row();
        });
        ui.weak(tr_fmt!("Resulting: {} · {} · {} · {} m", medium.title(), size, color, format!("{length:.2}")));
        for h in &hoses {
            if let Some(r) = &h.routed {
                ui.weak(tr_fmt!("Routed in {}: {} m, {} bends", r.enclosure, format!("{:.2}", r.length), r.bends));
            }
        }
        if first != Some((m, s.clone(), c.clone(), l)) || cfg != config_now {
            let hose = (!cfg.is_default()).then(|| cfg.clone());
            for (pi, wid) in &net.wires {
                if let Some(w) = self.session.prj.pages[*pi].wires.iter_mut().find(|w| w.id == *wid) {
                    (w.medium, w.cross_section, w.color, w.length) = (m, s.trim().to_string(), c.trim().to_string(), l);
                    w.hose = hose.clone();
                }
            }
        }
        if number != number_now {
            openecraft_core::wirenum::set(&mut self.session.prj, &net, number.trim(), true);
        }
    }

    pub(super) fn process_window(&mut self, ctx: &egui::Context) {
        if !self.process_ui.open {
            return;
        }
        let mut open = true;
        let (mut jump, mut report) = (None, None);
        egui::Window::new(tr("Fluid and process")).open(&mut open).default_size([900.0, 460.0]).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let tab = &mut self.process_ui.tab;
                ui.selectable_value(tab, Tab::Hoses, tr("Hoses and pipes"));
                ui.selectable_value(tab, Tab::Groups, tr("Device groups"));
                ui.selectable_value(tab, Tab::ElectroFluid, tr("Electro-fluid"));
                ui.selectable_value(tab, Tab::Loops, tr("PCT loops (pre-planning)"));
            });
            ui.separator();
            match self.process_ui.tab {
                Tab::Hoses => self.hoses_tab(ui, &mut jump, &mut report),
                Tab::Groups => self.groups_tab(ui, &mut report),
                Tab::ElectroFluid => self.electro_fluid_tab(ui, &mut jump, &mut report),
                Tab::Loops => self.loops_tab(ui, &mut jump, &mut report),
            }
        });
        self.process_ui.open = open;
        if let Some(k) = report {
            self.report_kind = k;
            self.show_reports = true;
        }
        if let Some((page, id)) = jump {
            self.goto(page, id);
        }
    }

    fn hoses_tab(&mut self, ui: &mut egui::Ui, jump: &mut Option<(usize, u64)>, report: &mut Option<ReportId>) {
        ui.weak(tr("Wires between the ports of fluid power and P&ID symbols are hoses / pipes: their own nets, numbered with the wire numbering, listed in the hose list."));
        let f = &mut self.session.prj.fluid;
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Default medium"));
            let mut m = Some(f.medium);
            egui::ComboBox::from_id_salt("fmedium").selected_text(f.medium.title()).show_ui(ui, |ui| {
                for x in Medium::ALL {
                    ui.selectable_value(&mut m, Some(x), x.title());
                }
            });
            f.medium = m.unwrap_or_default();
            ui.label(tr("Nominal size"));
            ui.add(egui::TextEdit::singleline(&mut f.size).desired_width(70.0));
            ui.label(tr("Colour"));
            ui.add(egui::TextEdit::singleline(&mut f.color).desired_width(50.0));
            ui.label(tr("Number prefix"));
            ui.add(egui::TextEdit::singleline(&mut f.prefix).desired_width(40.0));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Operating pressure"));
            ui.add(egui::DragValue::new(&mut f.pressure).speed(0.5).range(0.0..=1000.0).suffix(" bar")).on_hover_text(tr("Hoses rated below it are errors; 0 = not checked"));
            ui.label(tr("Cutting allowance per end"));
            ui.add(egui::DragValue::new(&mut f.allowance).speed(1.0).range(-500.0..=500.0).suffix(" mm")).on_hover_text(tr("Added to both ends of the length for the cut length (insertion depth)"));
        });
        ui.horizontal_wrapped(|ui| {
            for (k, label) in [
                (ReportKind::HoseList, tr("📊 Hose list")),
                (ReportKind::HoseConfiguration, tr("📊 Hose configuration")),
                (ReportKind::TubingList, tr("📊 Tubing / piping list")),
                (ReportKind::FluidNetlist, tr("📊 Fluid netlist")),
            ] {
                if ui.button(label).clicked() {
                    *report = Some(k.into());
                }
            }
        });
        ui.separator();
        ui.label(RichText::new(tr("Fluid netlist")).strong());
        let nets = fluid::fluid_nets(&self.session.prj, &self.session.analysis.open);
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("fluidnets").striped(true).num_columns(6).spacing([10.0, 4.0]).show(ui, |ui| {
                for h in [tr("Net"), tr("Hose no."), tr("Line"), tr("Medium"), tr("Nominal size"), tr("Ports")] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for n in &nets {
                    ui.label(&n.net);
                    ui.label(&n.number);
                    ui.label(n.line.title());
                    ui.label(n.medium.title());
                    ui.label(&n.size);
                    let ports: Vec<String> = n.ports.iter().map(|p| format!("{} ({})", p.designation, p.kind.title())).collect();
                    if ui.link(ports.join(", ")).clicked() {
                        *jump = n.ports.first().map(|p| (p.pin.page, self.session.prj.pages[p.pin.page].symbols[p.pin.sym].id));
                    }
                    ui.end_row();
                }
            });
        });
    }

    fn groups_tab(&mut self, ui: &mut egui::Ui, report: &mut Option<ReportId>) {
        let mut remove = None;
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("➕ Group")).clicked() {
                let used = |t: &str| self.session.prj.fluid.groups.iter().any(|g| g.tag == t) || self.session.prj.all_tags().iter().any(|x| x == t);
                let tag = (1..).map(|n| format!("-KH{n}")).find(|t| !used(t)).unwrap_or_default();
                self.session.prj.fluid.groups.push(DeviceGroup { tag, ..Default::default() });
            }
            if ui.button(tr("📊 Fluid device groups")).clicked() {
                *report = Some(ReportKind::FluidDeviceGroups.into());
            }
        });
        ui.weak(tr("Valve terminals, service units and manifolds: their fluid devices in slot order, separated by blanks."));
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("fluidgroups").striped(true).num_columns(7).spacing([10.0, 4.0]).show(ui, |ui| {
                for h in [tr("Group"), tr("Kind"), tr("Description"), tr("Places"), tr("Devices"), tr("Part"), ""] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for (i, g) in self.session.prj.fluid.groups.iter_mut().enumerate() {
                    ui.add(egui::TextEdit::singleline(&mut g.tag).desired_width(60.0));
                    egui::ComboBox::from_id_salt(("gkind", i)).selected_text(g.kind.title()).show_ui(ui, |ui| {
                        for k in GroupKind::ALL {
                            ui.selectable_value(&mut g.kind, k, k.title());
                        }
                    });
                    ui.add(egui::TextEdit::singleline(&mut g.description).desired_width(160.0));
                    ui.add(egui::DragValue::new(&mut g.slots).range(0..=128)).on_hover_text(tr("0 = not limited"));
                    tags_edit(ui, &mut g.members, "-QM1 -QM2", 200.0);
                    ui.add(egui::TextEdit::singleline(&mut g.part).desired_width(100.0));
                    if ui.small_button("🗑").on_hover_text(tr("Delete group")).clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
        });
        if let Some(i) = remove {
            self.session.prj.fluid.groups.remove(i);
        }
    }

    fn electro_fluid_tab(&mut self, ui: &mut egui::Ui, jump: &mut Option<(usize, u64)>, report: &mut Option<ReportId>) {
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("🔄 Synchronize")).on_hover_text(tr("Links solenoid valves with the coil of the same number (-QM3 › -MB3) and copies function texts")).clicked() {
                let s = electrofluid::synchronize(&mut self.session.prj);
                self.process_ui.message = tr_fmt!("{} links created, {} function texts copied", s.linked.len(), s.texts);
            }
            if ui.button(tr("📊 Electro-fluid cross reference")).clicked() {
                *report = Some(ReportKind::ElectroFluid.into());
            }
            if !self.process_ui.message.is_empty() {
                ui.weak(&self.process_ui.message);
            }
        });
        ui.weak(tr("Electrical devices that actuate or monitor a fluid device (solenoid coils, position and pressure switches); both schematics show cross references to each other."));
        let idx = openecraft_core::index::Index::build(&self.session.prj);
        let rows: Vec<(String, String, usize, (usize, u64))> = idx
            .sorted_tags()
            .into_iter()
            .filter_map(|t| {
                let (def, _, loc) = electrofluid::main_of(&self.session.prj, &idx, t).filter(|(d, _, _)| d.category == Category::Fluid)?;
                Some((t.clone(), tr(def.name).to_string(), electrofluid::solenoids(def), (loc.page, loc.sym_id)))
            })
            .collect();
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("electrofluid").striped(true).num_columns(4).spacing([10.0, 4.0]).show(ui, |ui| {
                for h in [tr("Fluid device"), tr("Function"), tr("Electrical devices"), tr("Status")] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for (tag, function, solenoids, target) in rows {
                    if ui.link(&tag).clicked() {
                        *jump = Some(target);
                    }
                    ui.label(function);
                    let mut els = electrofluid::linked(&self.session.prj, &tag).to_vec();
                    let before = els.clone();
                    tags_edit(ui, &mut els, "-MB1", 160.0);
                    if els != before {
                        if els.is_empty() {
                            self.session.prj.fluid.links.remove(&tag);
                        } else {
                            self.session.prj.fluid.links.insert(tag.clone(), els.clone());
                        }
                    }
                    let missing: Vec<&String> = els.iter().filter(|e| !idx.devices.contains_key(*e)).collect();
                    if solenoids > 0 && els.is_empty() {
                        ui.colored_label(Color32::from_rgb(230, 150, 30), tr("Solenoid not linked"));
                    } else if !missing.is_empty() {
                        ui.colored_label(Color32::from_rgb(230, 150, 30), tr("Not in the schematic"));
                    } else if !els.is_empty() {
                        ui.colored_label(Color32::from_rgb(50, 170, 80), tr("Linked"));
                    } else {
                        ui.label("");
                    }
                    ui.end_row();
                }
            });
        });
    }

    fn loops_tab(&mut self, ui: &mut egui::Ui, jump: &mut Option<(usize, u64)>, report: &mut Option<ReportId>) {
        let idx = openecraft_core::index::Index::build(&self.session.prj);
        let loops = process::loops(&self.session.prj, &idx);
        let (mut add, mut remove, mut define) = (false, None, None);
        ui.horizontal_wrapped(|ui| {
            add = ui.button(tr("➕ Loop")).clicked();
            if ui.button(tr("📊 PCT loop overview")).clicked() {
                *report = Some(ReportKind::PctLoops.into());
            }
        });
        ui.weak(tr("Instruments in the P&ID carry function letters and loop number in their label (TIC 101); their device tag links them with the schematic."));
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("pctloops").striped(true).num_columns(8).spacing([10.0, 4.0]).show(ui, |ui| {
                for h in [tr("PCT loop"), tr("Description"), tr("Range"), tr("Further devices"), tr("Functions"), tr("Devices"), tr("Status"), ""] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for l in &loops {
                    match self.session.prj.pct_loops.iter_mut().position(|d| d.name == l.name) {
                        Some(i) => {
                            let d = &mut self.session.prj.pct_loops[i];
                            ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(60.0));
                            ui.add(egui::TextEdit::singleline(&mut d.description).desired_width(200.0));
                            ui.add(egui::TextEdit::singleline(&mut d.range).desired_width(100.0));
                            tags_edit(ui, &mut d.devices, "-KF101", 120.0);
                        }
                        None => {
                            ui.label(&l.name);
                            if ui.button(tr("Add to the pre-planning")).clicked() {
                                define = Some(l.name.clone());
                            }
                            ui.label("");
                            ui.label("");
                        }
                    }
                    let funcs = l.functions.join(", ");
                    if l.target.is_some() && ui.link(&funcs).clicked() {
                        *jump = l.target;
                    } else if l.target.is_none() {
                        ui.weak(tr("not in the P&ID"));
                    }
                    let devices = l.devices.iter().map(|d| if l.missing.contains(d) { format!("{d}?") } else { d.clone() }).collect::<Vec<_>>().join(", ");
                    ui.label(devices).on_hover_text(tr("? = not in the schematic yet"));
                    ui.colored_label(status_color(l.status), l.status.title());
                    if l.defined && ui.small_button("🗑").on_hover_text(tr("Delete loop")).clicked() {
                        remove = Some(l.name.clone());
                    }
                    ui.end_row();
                }
            });
        });
        if add {
            let next = (1..).map(|n| (100 + n).to_string()).find(|n| !loops.iter().any(|l| &l.name == n)).unwrap_or_default();
            self.session.prj.pct_loops.push(PctLoop { name: next, ..Default::default() });
        }
        if let Some(name) = define {
            self.session.prj.pct_loops.push(PctLoop { name, ..Default::default() });
        }
        if let Some(name) = remove {
            self.session.prj.pct_loops.retain(|l| l.name != name);
        }
    }
}
