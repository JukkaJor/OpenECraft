//! PLC hardware window: stations with racks and modules, bus systems with the port
//! connections and a topology view, device descriptions, automatic configuration and
//! the exchange with PLC programming systems (AutomationML, IEC 61131-3).

use openecraft_i18n::{tr, tr_fmt};
use super::App;
use crate::io::Purpose;
use crate::model::PlcCard;
use crate::plc_hw::{self, BusKind, BusSystem, PortRef, Scheme, Station, StationKind};
use crate::{aml, plc};
use egui::{pos2, vec2, Align2, Color32, RichText, Stroke};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    Stations,
    Bus,
    Descriptions,
}

#[derive(Default)]
pub struct PlcUi {
    pub open: bool,
    tab: Tab,
    station: String,
    rename: String,
    new_a: PortRef,
    new_b: PortRef,
    new_cable: String,
    /// The automatic configuration creates no bus cables.
    no_cables: bool,
}

enum Action {
    Auto,
    ExportAml,
    ImportAml,
    ExportIec,
    ImportDescriptions,
    AddStation,
    RemoveStation(String),
    Rename(String, String),
    Scheme(usize, Scheme),
    AddCard(String),
    Card(String, PlcCard),
    AddNetwork,
    RemoveNetwork(String),
    Connect,
    Disconnect(usize),
    CreateCables,
}

fn combo(ui: &mut egui::Ui, id: impl std::hash::Hash + std::fmt::Debug, value: &mut String, options: &[String], width: f32) {
    egui::ComboBox::from_id_salt(id).width(width).selected_text(value.as_str()).show_ui(ui, |ui| {
        ui.selectable_value(value, String::new(), "–");
        for o in options {
            ui.selectable_value(value, o.clone(), o);
        }
    });
}

impl App {
    pub(super) fn plc_hw_window(&mut self, ctx: &egui::Context) {
        if !self.plc_ui.open {
            return;
        }
        let mut open = true;
        let mut actions = Vec::new();
        egui::Window::new(tr("PLC hardware")).open(&mut open).default_size([980.0, 560.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr("⚙ Automatic configuration")).on_hover_text(tr("Stations for CPUs and bus couplers, bus systems, slots, start addresses, point addresses, IP addresses and a line topology")).clicked() {
                    actions.push(Action::Auto);
                }
                let mut cables = !self.plc_ui.no_cables;
                ui.checkbox(&mut cables, tr("with bus cables"));
                self.plc_ui.no_cables = !cables;
                ui.separator();
                if ui.button(tr("⬇ Export AutomationML…")).on_hover_text(tr("Hardware configuration and PLC tags for TIA Portal and other PLC tools")).clicked() {
                    actions.push(Action::ExportAml);
                }
                if ui.button(tr("⬆ Import AutomationML…")).on_hover_text(tr("Stations, modules, addresses, subnets and tag comments from a PLC tool")).clicked() {
                    actions.push(Action::ImportAml);
                }
                if ui.button(tr("⬇ Export IEC 61131-3 variables…")).on_hover_text(tr("Global variable list with direct addresses for CODESYS, TwinCAT and others")).clicked() {
                    actions.push(Action::ExportIec);
                }
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.plc_ui.tab, Tab::Stations, tr("Stations and racks"));
                ui.selectable_value(&mut self.plc_ui.tab, Tab::Bus, tr("Bus topology"));
                ui.selectable_value(&mut self.plc_ui.tab, Tab::Descriptions, tr("Device descriptions"));
            });
            ui.separator();
            let problems = plc_hw::problems(&self.prj);
            let footer = if problems.is_empty() { 0.0 } else { 24.0 + 18.0 * problems.len().min(4) as f32 };
            egui::ScrollArea::both().auto_shrink([false, false]).max_height(ui.available_height() - footer).show(ui, |ui| match self.plc_ui.tab {
                Tab::Stations => self.plc_stations(ui, &mut actions),
                Tab::Bus => self.plc_bus(ui, &mut actions),
                Tab::Descriptions => self.plc_descriptions(ui, &mut actions),
            });
            if !problems.is_empty() {
                ui.separator();
                egui::ScrollArea::vertical().id_salt("plcproblems").max_height(footer).show(ui, |ui| {
                    for p in problems {
                        ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {}", p.message));
                    }
                });
            }
        });
        self.plc_ui.open = open;
        for a in actions {
            self.plc_action(ctx, a);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn plc_stations(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let names: Vec<String> = self.prj.plc.stations.iter().map(|s| s.name.clone()).collect();
        if !names.contains(&self.plc_ui.station) {
            self.plc_ui.station = names.first().cloned().unwrap_or_default();
            self.plc_ui.rename = self.plc_ui.station.clone();
        }
        let unassigned: Vec<String> = plc::cards(&self.prj).into_iter().filter(|c| plc_hw::station_of(&self.prj, c).is_none()).collect();
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Station"));
            for n in &names {
                if ui.selectable_label(*n == self.plc_ui.station, n).clicked() {
                    self.plc_ui.station = n.clone();
                    self.plc_ui.rename = n.clone();
                }
            }
            if ui.button(tr("➕ Station")).clicked() {
                actions.push(Action::AddStation);
            }
        });
        if !unassigned.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.weak(tr_fmt!("Cards without station: {}", unassigned.join(", ")));
            });
        }
        let Some(si) = self.prj.plc.stations.iter().position(|s| s.name == self.plc_ui.station) else {
            ui.weak(tr("No stations: add one or use the automatic configuration."));
            return;
        };
        let descs = plc_hw::descriptions(&self.prj);
        let heads: Vec<String> = descs.iter().filter(|d| d.kind.head()).map(|d| d.id.clone()).collect();
        let modules_desc: Vec<String> = descs.iter().filter(|d| !d.kind.head()).map(|d| d.id.clone()).collect();
        let networks: Vec<String> = self.prj.plc.networks.iter().map(|n| n.name.clone()).collect();
        let controllers: Vec<String> = self.prj.plc.stations.iter().filter(|s| s.kind == StationKind::Controller).map(|s| s.name.clone()).collect();
        let mut st = self.prj.plc.stations[si].clone();
        let old_scheme = st.scheme;
        ui.separator();
        egui::Grid::new("plcstation").num_columns(4).spacing([10.0, 4.0]).show(ui, |ui| {
            ui.label(tr("Name"));
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.plc_ui.rename).desired_width(120.0));
                let new = self.plc_ui.rename.trim().to_string();
                if ui.add_enabled(!new.is_empty() && new != st.name && !names.contains(&new), egui::Button::new(tr("Rename"))).clicked() {
                    actions.push(Action::Rename(st.name.clone(), new));
                }
            });
            ui.label(tr("Type"));
            egui::ComboBox::from_id_salt("plckind").selected_text(st.kind.title()).show_ui(ui, |ui| {
                for k in StationKind::ALL {
                    ui.selectable_value(&mut st.kind, k, k.title());
                }
            });
            ui.end_row();
            ui.label(tr("Device tag"));
            ui.add(egui::TextEdit::singleline(&mut st.tag).desired_width(120.0));
            ui.label(tr("Head (CPU / coupler)"));
            combo(ui, "plchead", &mut st.device, &heads, 200.0);
            ui.end_row();
            ui.label(tr("Address scheme"));
            egui::ComboBox::from_id_salt("plcscheme").selected_text(st.scheme.title()).show_ui(ui, |ui| {
                for k in Scheme::ALL {
                    ui.selectable_value(&mut st.scheme, k, k.title());
                }
            });
            ui.label(tr("Bus system"));
            combo(ui, "plcnet", &mut st.network, &networks, 140.0);
            ui.end_row();
            ui.label(tr("IP / bus address"));
            ui.add(egui::TextEdit::singleline(&mut st.address).desired_width(120.0));
            ui.label(tr("Device number"));
            ui.add(egui::DragValue::new(&mut st.device_number).range(0..=9999));
            ui.end_row();
            if st.kind != StationKind::Controller {
                ui.label(tr("Controller"));
                combo(ui, "plcctrl", &mut st.controller, &controllers, 120.0);
            } else {
                ui.label("");
                ui.label("");
            }
            ui.label(tr("Comment"));
            ui.add(egui::TextEdit::singleline(&mut st.comment).desired_width(200.0));
            ui.end_row();
        });
        if let Some(d) = plc_hw::station_description(&self.prj, &st) {
            ui.weak(tr_fmt!("{}: {} slots, ports {}", d.title(), d.slots, d.ports.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")));
        }
        if st != self.prj.plc.stations[si] {
            if st.scheme != old_scheme {
                actions.push(Action::Scheme(si, st.scheme));
                st.scheme = old_scheme;
            }
            self.prj.plc.stations[si] = st.clone();
        }
        ui.horizontal(|ui| {
            if ui.button(tr("🗑 Remove station")).clicked() {
                actions.push(Action::RemoveStation(st.name.clone()));
            }
            if !unassigned.is_empty() {
                let mut add = String::new();
                ui.label(tr("Plug in card"));
                combo(ui, "plcaddcard", &mut add, &unassigned, 100.0);
                if !add.is_empty() {
                    actions.push(Action::AddCard(add));
                }
            }
        });
        ui.separator();
        let modules = plc_hw::modules(&self.prj, &st.name);
        if modules.is_empty() {
            ui.weak(tr("No modules: plug in PLC cards of the schematic."));
            return;
        }
        egui::Grid::new("plcmodules").striped(true).num_columns(9).spacing([10.0, 4.0]).show(ui, |ui| {
            for h in [tr("Rack"), tr("Slot"), tr("Card"), tr("Device description"), tr("Inputs"), tr("Outputs"), tr("Input bytes"), tr("Output bytes"), ""] {
                ui.label(RichText::new(h).strong());
            }
            ui.end_row();
            for m in &modules {
                let mut data = self.prj.plc_cards.get(&m.card).cloned().unwrap_or_default();
                let before = data.clone();
                let (mut rack, mut slot) = (m.rack, m.slot);
                ui.add(egui::DragValue::new(&mut rack).range(0..=99));
                ui.add(egui::DragValue::new(&mut slot).range(0..=999));
                data.rack = rack.to_string();
                data.slot = slot.to_string();
                ui.label(RichText::new(&m.card).monospace());
                let current = m.description.as_ref().map(|d| d.title()).unwrap_or_else(|| "–".into());
                egui::ComboBox::from_id_salt(("plcmoddesc", &m.card)).width(220.0).selected_text(current).show_ui(ui, |ui| {
                    ui.selectable_value(&mut data.device, String::new(), tr("from the parts"));
                    for id in &modules_desc {
                        let title = descs.iter().find(|d| &d.id == id).map(|d| d.title()).unwrap_or_default();
                        ui.selectable_value(&mut data.device, id.clone(), title);
                    }
                });
                let cap = |used: u32, n: Option<u32>| match n {
                    Some(n) => format!("{used} / {n}"),
                    None => used.to_string(),
                };
                ui.label(cap(m.inputs_used, m.description.as_ref().map(|d| d.inputs)));
                ui.label(cap(m.outputs_used, m.description.as_ref().map(|d| d.outputs)));
                let range = |start: u32, bytes: u32| if bytes == 0 { "–".to_string() } else { format!("{start} … {}", start + bytes - 1) };
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut data.start_byte).range(0..=65535));
                    ui.weak(range(m.input_start, m.input_bytes));
                });
                ui.horizontal(|ui| {
                    let mut out = data.output_byte.unwrap_or(data.start_byte);
                    ui.add(egui::DragValue::new(&mut out).range(0..=65535));
                    data.output_byte = (out != data.start_byte).then_some(out);
                    ui.weak(range(m.output_start, m.output_bytes));
                });
                if ui.small_button("✖").on_hover_text(tr("Remove from the station")).clicked() {
                    data.station.clear();
                }
                if data != before {
                    actions.push(Action::Card(m.card.clone(), data));
                }
                ui.end_row();
            }
        });
    }

    fn plc_bus(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Bus systems")).strong());
            if ui.button(tr("➕ Bus system")).clicked() {
                actions.push(Action::AddNetwork);
            }
        });
        let mut nets = self.prj.plc.networks.clone();
        egui::Grid::new("plcnets").striped(true).num_columns(4).spacing([10.0, 4.0]).show(ui, |ui| {
            for (i, n) in nets.iter_mut().enumerate() {
                ui.label(&n.name);
                egui::ComboBox::from_id_salt(("plcnetkind", i)).selected_text(n.kind.title()).show_ui(ui, |ui| {
                    for k in BusKind::ALL {
                        ui.selectable_value(&mut n.kind, k, k.title());
                    }
                });
                ui.add(egui::TextEdit::singleline(&mut n.subnet).desired_width(120.0).hint_text(tr("Subnet")));
                if ui.small_button("🗑").clicked() {
                    actions.push(Action::RemoveNetwork(n.name.clone()));
                }
                ui.end_row();
            }
        });
        if nets != self.prj.plc.networks {
            self.prj.plc.networks = nets;
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Connections")).strong());
            let open = self.prj.plc.connections.iter().filter(|c| c.cable.is_empty()).count();
            if ui.add_enabled(open > 0, egui::Button::new(tr("➕ Create bus cables"))).on_hover_text(tr("A cable of the bus type for every connection without cable")).clicked() {
                actions.push(Action::CreateCables);
            }
        });
        let stations: Vec<String> = self.prj.plc.stations.iter().map(|s| s.name.clone()).collect();
        let cables: Vec<String> = self.prj.cables.iter().map(|c| c.tag.clone()).collect();
        let ports_of = |name: &str| -> Vec<String> {
            self.prj.plc.station(name).map(|st| plc_hw::ports(&self.prj, st).into_iter().map(|p| p.name).collect()).unwrap_or_default()
        };
        egui::Grid::new("plcconns").striped(true).num_columns(6).spacing([10.0, 4.0]).show(ui, |ui| {
            for h in [tr("Bus system"), tr("Port A"), tr("Port B"), tr("Cable"), tr("Type"), ""] {
                ui.label(RichText::new(h).strong());
            }
            ui.end_row();
            for (i, c) in self.prj.plc.connections.iter().enumerate() {
                ui.label(&c.network);
                ui.label(c.a.to_string());
                ui.label(c.b.to_string());
                ui.label(&c.cable);
                match self.prj.cable(&c.cable) {
                    Some(cable) => ui.label(&cable.cable_type),
                    None if c.cable.is_empty() => ui.label(""),
                    None => ui.colored_label(ui.visuals().warn_fg_color, tr("missing")),
                };
                if ui.small_button("✖").clicked() {
                    actions.push(Action::Disconnect(i));
                }
                ui.end_row();
            }
            ui.label("");
            let (pa, pb) = (ports_of(&self.plc_ui.new_a.station), ports_of(&self.plc_ui.new_b.station));
            for (id, p, ports) in [("plcna", &mut self.plc_ui.new_a, &pa), ("plcnb", &mut self.plc_ui.new_b, &pb)] {
                ui.horizontal(|ui| {
                    combo(ui, (id, 0), &mut p.station, &stations, 90.0);
                    combo(ui, (id, 1), &mut p.port, ports, 70.0);
                });
            }
            combo(ui, "plcncable", &mut self.plc_ui.new_cable, &cables, 70.0);
            ui.label("");
            let ok = [&self.plc_ui.new_a, &self.plc_ui.new_b].iter().all(|p| !p.station.is_empty() && !p.port.is_empty());
            if ui.add_enabled(ok, egui::Button::new(tr("Connect"))).clicked() {
                actions.push(Action::Connect);
            }
            ui.end_row();
        });
        ui.separator();
        self.plc_topology(ui);
    }

    /// Stations of each bus system side by side with their ports and cables.
    fn plc_topology(&self, ui: &mut egui::Ui) {
        let hw = &self.prj.plc;
        let (box_w, box_h, gap, row_h) = (130.0, 54.0, 50.0, 110.0);
        let mut rows: Vec<(String, Vec<&Station>)> = hw.networks.iter().map(|n| (format!("{} ({})", n.name, n.kind.title()), hw.stations.iter().filter(|s| s.network == n.name).collect())).collect();
        let loose: Vec<&Station> = hw.stations.iter().filter(|s| hw.network(&s.network).is_none()).collect();
        if !loose.is_empty() {
            rows.push((tr("Without bus system").to_string(), loose));
        }
        if rows.is_empty() {
            ui.weak(tr("No bus systems."));
            return;
        }
        let width = rows.iter().map(|r| r.1.len()).max().unwrap_or(1).max(1) as f32 * (box_w + gap) + 20.0;
        let (resp, painter) = ui.allocate_painter(vec2(width.max(ui.available_width()), rows.len() as f32 * row_h + 10.0), egui::Sense::hover());
        let v = ui.visuals();
        let (fg, weak, accent) = (v.text_color(), v.weak_text_color(), v.selection.bg_fill);
        let font = egui::FontId::proportional(12.0);
        let small = egui::FontId::proportional(10.0);
        let mut port_pos: std::collections::HashMap<PortRef, egui::Pos2> = Default::default();
        for (r, (title, stations)) in rows.iter().enumerate() {
            let y0 = resp.rect.top() + r as f32 * row_h + 4.0;
            painter.text(pos2(resp.rect.left() + 4.0, y0), Align2::LEFT_TOP, title, font.clone(), weak);
            for (i, st) in stations.iter().enumerate() {
                let rect = egui::Rect::from_min_size(pos2(resp.rect.left() + 10.0 + i as f32 * (box_w + gap), y0 + 22.0), vec2(box_w, box_h));
                let fill = if st.kind == StationKind::Controller { accent.gamma_multiply(0.5) } else { Color32::TRANSPARENT };
                painter.rect(rect, 4.0, fill, Stroke::new(1.0, fg), egui::StrokeKind::Inside);
                painter.text(rect.center_top() + vec2(0.0, 4.0), Align2::CENTER_TOP, &st.name, font.clone(), fg);
                let sub = [st.tag.as_str(), st.address.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("  ");
                painter.text(rect.center_top() + vec2(0.0, 20.0), Align2::CENTER_TOP, sub, small.clone(), weak);
                let ports = plc_hw::ports(&self.prj, st);
                for (k, p) in ports.iter().enumerate() {
                    let x = rect.left() + (k as f32 + 0.5) * box_w / ports.len() as f32;
                    let pt = pos2(x, rect.bottom());
                    painter.circle_filled(pt, 3.0, fg);
                    painter.text(pt + vec2(0.0, -3.0), Align2::CENTER_BOTTOM, &p.name, small.clone(), weak);
                    port_pos.insert(PortRef::new(&st.name, &p.name), pt);
                }
            }
        }
        for c in &hw.connections {
            let (Some(a), Some(b)) = (port_pos.get(&c.a), port_pos.get(&c.b)) else { continue };
            let dip = a.y.max(b.y) + 14.0;
            let pts = vec![*a, pos2(a.x, dip), pos2(b.x, dip), *b];
            painter.add(egui::Shape::line(pts, Stroke::new(2.0, accent)));
            if !c.cable.is_empty() {
                painter.text(pos2((a.x + b.x) / 2.0, dip + 2.0), Align2::CENTER_TOP, &c.cable, small.clone(), fg);
            }
        }
    }

    fn plc_descriptions(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.horizontal(|ui| {
            if ui.button(tr("⬆ Import GSDML / ESI…")).on_hover_text(tr("Device descriptions of PROFINET (GSDML) and EtherCAT (ESI) devices")).clicked() {
                actions.push(Action::ImportDescriptions);
            }
            ui.weak(tr("Parts of the product group PLC give descriptions as well (technical data: Inputs, Outputs, Slots, Bus, Ports)."));
        });
        egui::Grid::new("plcdescs").striped(true).num_columns(8).spacing([10.0, 4.0]).show(ui, |ui| {
            for h in [tr("Name"), tr("Type"), tr("Order number"), tr("Inputs"), tr("Outputs"), tr("Slots"), tr("Ports"), tr("Source")] {
                ui.label(RichText::new(h).strong());
            }
            ui.end_row();
            for d in plc_hw::descriptions(&self.prj) {
                ui.label(d.title());
                ui.label(d.kind.title());
                ui.label(&d.order_number);
                ui.label(format!("{} ({} B)", d.inputs, d.input_bytes));
                ui.label(format!("{} ({} B)", d.outputs, d.output_bytes));
                ui.label(if d.slots > 0 { d.slots.to_string() } else { String::new() });
                ui.label(d.ports.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "));
                ui.label(if d.source.is_empty() { tr("built-in") } else { d.source.as_str() });
                ui.end_row();
            }
        });
    }

    fn plc_action(&mut self, ctx: &egui::Context, a: Action) {
        match a {
            Action::Auto => {
                let r = plc_hw::auto_configure(&mut self.prj, !self.plc_ui.no_cables);
                self.notify(ctx, tr_fmt!("{} stations, {} bus systems, {} cards assigned, {} addresses changed, {} connections, {} cables", r.stations, r.networks, r.assigned, r.addresses, r.connections, r.cables));
            }
            Action::ExportAml => {
                let text = aml::export(&self.prj, &self.analysis.netlist);
                self.export(ctx, format!("{}.aml", self.prj.info.name), ("AutomationML", "aml"), text);
            }
            Action::ImportAml => self.io.open(ctx, Purpose::Aml),
            Action::ExportIec => {
                let text = plc::global_variables(&self.prj, &self.analysis.netlist);
                self.export(ctx, "plc_variables.st".into(), ("IEC 61131-3", "st"), text);
            }
            Action::ImportDescriptions => self.io.open(ctx, Purpose::DeviceDescription),
            Action::AddStation => {
                let name = (1..).map(|i| format!("PLC{i}")).find(|n| self.prj.plc.station(n).is_none()).unwrap();
                self.prj.plc.stations.push(Station { name: name.clone(), device: "GEN.CPU-PN".into(), ..Default::default() });
                self.plc_ui.station = name.clone();
                self.plc_ui.rename = name;
            }
            Action::RemoveStation(name) => plc_hw::remove_station(&mut self.prj, &name),
            Action::Rename(old, new) => {
                plc_hw::rename_station(&mut self.prj, &old, &new);
                self.plc_ui.station = new;
            }
            Action::Scheme(i, scheme) => {
                self.prj.plc.stations[i].scheme = scheme;
                let n = plc_hw::convert_addresses(&mut self.prj);
                self.notify(ctx, tr_fmt!("{} addresses changed", n));
            }
            Action::AddCard(card) => {
                let station = self.plc_ui.station.clone();
                let slot = plc_hw::modules(&self.prj, &station).iter().map(|m| m.slot + 1).max();
                let first = self.prj.plc.station(&station).map(|s| plc_hw::head_slot(s) + 1).unwrap_or(1);
                let c = self.prj.plc_cards.entry(card.clone()).or_default();
                c.station = station;
                c.rack = "0".into();
                c.slot = slot.unwrap_or(first).to_string();
                plc::readdress(&mut self.prj, &card);
            }
            Action::Card(card, data) => {
                let readdress = self.prj.plc_cards.get(&card).is_none_or(|c| (c.start_byte, c.output_byte, &c.slot) != (data.start_byte, data.output_byte, &data.slot));
                self.prj.plc_cards.insert(card.clone(), data);
                if readdress {
                    plc::readdress(&mut self.prj, &card);
                }
            }
            Action::AddNetwork => {
                let name = (1..).map(|i| format!("BUS_{i}")).find(|n| self.prj.plc.network(n).is_none()).unwrap();
                self.prj.plc.networks.push(BusSystem { name, kind: BusKind::Profinet, subnet: "192.168.0.0".into() });
            }
            Action::RemoveNetwork(name) => plc_hw::remove_network(&mut self.prj, &name),
            Action::Connect => {
                let (a, b, cable) = (self.plc_ui.new_a.clone(), self.plc_ui.new_b.clone(), self.plc_ui.new_cable.clone());
                if let Err(e) = plc_hw::connect(&mut self.prj, a, b, &cable) {
                    self.notify(ctx, e);
                }
            }
            Action::CreateCables => {
                let n = plc_hw::create_cables(&mut self.prj);
                self.notify(ctx, tr_fmt!("{} bus cables created", n));
            }
            Action::Disconnect(i) => {
                if i < self.prj.plc.connections.len() {
                    self.prj.plc.connections.remove(i);
                }
            }
        }
    }

    /// Loaded AutomationML or device description file.
    pub(super) fn plc_file_loaded(&mut self, ctx: &egui::Context, purpose: Purpose, name: &str, text: &str) {
        match purpose {
            Purpose::Aml => match aml::import(&mut self.prj, text) {
                Ok(r) => self.notify(ctx, tr_fmt!("Imported {} stations, {} modules, {} bus systems, {} connections; {} addresses and {} texts changed", r.stations, r.modules, r.networks, r.connections, r.addresses, r.texts)),
                Err(e) => self.notify(ctx, tr_fmt!("Import failed: {}", e)),
            },
            _ => match crate::devdesc::import(text, crate::io::short_name(name)) {
                Ok(list) => {
                    let n = list.len();
                    self.prj.plc.add_descriptions(list);
                    self.notify(ctx, tr_fmt!("{} device descriptions imported", n));
                }
                Err(e) => self.notify(ctx, tr_fmt!("Import failed: {}", e)),
            },
        }
    }
}
