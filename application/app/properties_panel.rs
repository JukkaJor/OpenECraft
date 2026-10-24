//! The property editor on the right: the selected symbol, wire, text or page.

use super::*;

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn properties(&mut self, ui: &mut egui::Ui, idx: &Index) {
        ui.heading(tr("Properties"));
        let sel: Vec<u64> = self.editor.selection.iter().copied().collect();
        let pi = self.session.page;
        if self.session.prj.pages[pi].locked {
            ui.label(RichText::new(tr("🔒 Released page – read only")).color(Color32::from_rgb(220, 140, 0)));
            ui.disable();
        } else if let Some(user) = self.page_locked_by() {
            ui.label(RichText::new(tr_fmt!("🔒 Locked by {} – read only", user)).color(Color32::from_rgb(220, 140, 0)));
            ui.disable();
        }
        if sel.is_empty() {
            let pg = &mut self.session.prj.pages[pi];
            ui.label(RichText::new(tr_fmt!("Page {} – {}", structure::page_full_name(pg), pg.kind.label())).strong());
            ui.label(tr("Description"));
            ui.text_edit_singleline(&mut pg.description);
            if ui.link(tr("Structure (function / location)…")).on_hover_text(tr("Page properties")).clicked() {
                self.page_props = Some(pi);
            }
            ui.add_space(6.0);
            ui.weak(self.editor.tool.hint());
            return;
        }
        if sel.len() > 1 {
            ui.label(tr_fmt!("{} objects selected", sel.len()));
            ui.horizontal(|ui| {
                if ui.button(tr("⟳ Rotate")).clicked() {
                    ops::rotate(&mut self.session.prj.pages[pi], &self.editor.selection);
                }
                if ui.button(tr("🗑 Delete")).clicked() {
                    ops::delete(&mut self.session.prj.pages[pi], &self.editor.selection);
                    self.editor.selection.clear();
                }
            });
            let wires: Vec<u64> = self.session.prj.pages[pi].wires.iter().filter(|w| self.editor.selection.contains(&w.id)).map(|w| w.id).collect();
            if !wires.is_empty() {
                ui.separator();
                ui.label(tr_fmt!("{} wires", wires.len()));
                let first = self.session.prj.pages[pi].wires.iter().find(|w| w.id == wires[0]).cloned().unwrap_or_default();
                let (mut color, mut cs) = (first.color.clone(), first.cross_section.clone());
                let auto = potentials::props_of_wire(&self.session.prj, &self.session.analysis.netlist, &self.session.analysis.open, first.id);
                egui::Grid::new("wprops").num_columns(2).show(ui, |ui| {
                    potentials_ui::wire_fields(ui, &mut color, &mut cs, &auto);
                });
                if color != first.color || cs != first.cross_section {
                    for w in self.session.prj.pages[pi].wires.iter_mut().filter(|w| wires.contains(&w.id)) {
                        w.color = color.clone();
                        w.cross_section = cs.clone();
                    }
                }
                let cables: Vec<String> = self.session.prj.cables.iter().map(|c| c.tag.clone()).collect();
                let mut assign = None;
                ui.horizontal(|ui| {
                    ui.menu_button(tr("Assign to cable"), |ui| {
                        for c in &cables {
                            if ui.button(c).clicked() {
                                assign = Some(c.clone());
                            }
                        }
                        if cables.is_empty() {
                            ui.weak(tr("No cables yet – Project › Cables…"));
                        }
                    })
                    .response
                    .on_hover_text(tr("The wires get the free cores of the cable in order (left to right)"));
                    if ui.button(tr("Remove from cable")).clicked() {
                        for w in self.session.prj.pages[pi].wires.iter_mut().filter(|w| wires.contains(&w.id)) {
                            w.cable.clear();
                            w.core.clear();
                        }
                    }
                });
                let harnesses: Vec<String> = self.session.prj.harnesses.iter().map(|h| h.tag.clone()).collect();
                let mut to_harness = None;
                ui.horizontal(|ui| {
                    ui.menu_button(tr("Assign to harness"), |ui| {
                        for h in &harnesses {
                            if ui.button(h).clicked() {
                                to_harness = Some(h.clone());
                            }
                        }
                        if harnesses.is_empty() {
                            ui.weak(tr("No harnesses yet – Project › Harnesses…"));
                        }
                    })
                    .response
                    .on_hover_text(tr("Connections whose two ends are both on selected wires"));
                    if ui.button(tr("Remove from harness")).clicked() {
                        to_harness = Some(String::new());
                    }
                });
                if let Some(tag) = to_harness {
                    let sel: std::collections::BTreeSet<u64> = wires.iter().copied().collect();
                    let conns = openecraft_core::harness::connections_at(&self.session.prj, &self.session.analysis, pi, &sel);
                    let ends: Vec<(String, String)> = conns
                        .iter()
                        .map(|&i| {
                            let c = &self.session.analysis.connections[i];
                            (pin_designation(&self.session.prj, c.a), pin_designation(&self.session.prj, c.b))
                        })
                        .collect();
                    for (a, b) in &ends {
                        openecraft_core::harness::assign(&mut self.session.prj, a, b, &tag);
                    }
                    let ctx = ui.ctx().clone();
                    self.notify(&ctx, if tag.is_empty() { tr_fmt!("{} connections removed from their harness", ends.len()) } else { tr_fmt!("{} connections assigned to {}", ends.len(), tag) });
                }
                if let Some(c) = assign {
                    let sel: std::collections::BTreeSet<u64> = wires.iter().copied().collect();
                    let n = ops::assign_cores(&mut self.session.prj, pi, &sel, &c);
                    let ctx = ui.ctx().clone();
                    self.notify(&ctx, tr_fmt!("{} of {} wires assigned to cores of {}", n, wires.len(), c));
                }
            }
            return;
        }
        let id = sel[0];
        let page = &self.session.prj.pages[pi];
        if let Some(si) = page.symbols.iter().position(|s| s.id == id) {
            self.symbol_properties(ui, idx, pi, si);
        } else if self.session.analysis.open.net_of_wire(id).is_some_and(|n| n.fluid && !n.mixed) && page.wires.iter().any(|w| w.id == id) {
            self.hose_properties(ui, id);
        } else if let Some(wi) = page.wires.iter().position(|w| w.id == id) {
            let net = self.session.analysis.netlist.net_of_wire(id);
            if let Some(net) = net {
                ui.label(RichText::new(tr_fmt!("Potential: {}", net.name)).strong());
                ui.label(tr("Connected to:"));
                let mut v: Vec<String> = net.pins.iter().map(|p| pin_designation(&self.session.prj, *p)).collect();
                v.sort_by(|a, b| natural_cmp(a, b));
                v.dedup();
                ui.label(RichText::new(v.join(", ")).monospace());
                ui.weak(tr("All wires of this potential are highlighted."));
            }
            ui.separator();
            let open_net = self.session.analysis.open.wire_net.get(&id).map(|&n| self.session.analysis.open.nets[n].clone());
            let number_now = open_net.as_ref().map(|n| wirenum::number_of(&self.session.prj, n)).unwrap_or_default();
            let mut number = number_now.clone();
            let here = Dt::of_page(&self.session.prj.pages[pi]);
            let cables: Vec<(String, Vec<String>)> = self.session.prj.cables.iter().map(|c| (c.tag.clone(), c.core_list())).collect();
            // Cores of the wire's cable already used by other connections
            let own: Vec<u64> = open_net.as_ref().map(|n| n.wires.iter().map(|w| w.1).collect()).unwrap_or_default();
            let cable_now = self.session.prj.pages[pi].wires[wi].cable.clone();
            let used: Vec<String> = self
                .session.prj
                .pages
                .iter()
                .flat_map(|pg| pg.wires.iter())
                .filter(|x| x.cable == cable_now && !cable_now.is_empty() && !own.contains(&x.id))
                .map(|x| x.core.clone())
                .collect();
            let harnesses: Vec<String> = self.session.prj.harnesses.iter().map(|h| h.tag.clone()).collect();
            // Connections of this wire's potential section, each with its own harness.
            let conns: Vec<(String, String, String)> = openecraft_core::harness::connections_of_wire(&self.session.analysis, id)
                .into_iter()
                .map(|i| {
                    let c = &self.session.analysis.connections[i];
                    (pin_designation(&self.session.prj, c.a), pin_designation(&self.session.prj, c.b), c.harness.clone())
                })
                .collect();
            let mut harness_change = None;
            let auto = potentials::props_of_wire(&self.session.prj, &self.session.analysis.netlist, &self.session.analysis.open, id);
            let w = &mut self.session.prj.pages[pi].wires[wi];
            egui::Grid::new("wprops1").num_columns(2).show(ui, |ui| {
                ui.label(tr("Wire number"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut number).hint_text(tr("not numbered")).desired_width(90.0))
                        .on_hover_text(tr("Applies to all wires of this connection; automatic numbering keeps manual numbers"));
                    if w.number_fixed {
                        ui.weak(tr("manual"));
                    }
                });
                ui.end_row();
                ui.label(tr("Cable"));
                let shown = if w.cable.is_empty() { "(none)".to_string() } else { here.relative(&w.cable) };
                egui::ComboBox::from_id_salt("wcable").selected_text(shown).show_ui(ui, |ui| {
                    if ui.selectable_label(w.cable.is_empty(), "(none)").clicked() {
                        w.cable.clear();
                        w.core.clear();
                    }
                    for (tag, _) in &cables {
                        if ui.selectable_label(w.cable == *tag, tag).clicked() && w.cable != *tag {
                            w.cable = tag.clone();
                            w.core.clear();
                        }
                    }
                });
                ui.end_row();
                if let Some((_, cores)) = cables.iter().find(|(t, _)| *t == w.cable) {
                    ui.label(tr("Core"));
                    egui::ComboBox::from_id_salt("wcore").selected_text(w.core.clone()).show_ui(ui, |ui| {
                        for c in cores {
                            let text = if used.contains(c) { tr_fmt!("{}  (used)", c) } else { c.clone() };
                            ui.selectable_value(&mut w.core, c.clone(), text);
                        }
                    });
                    ui.end_row();
                }
                potentials_ui::wire_fields(ui, &mut w.color, &mut w.cross_section, &auto);
                ui.label(tr("From"));
                ui.label(format!("{:.0} / {:.0}", w.a.x, w.a.y));
                ui.end_row();
                ui.label(tr("To"));
                ui.label(format!("{:.0} / {:.0}", w.b.x, w.b.y));
                ui.end_row();
            });
            if number != number_now {
                if let Some(net) = &open_net {
                    wirenum::set(&mut self.session.prj, net, number.trim(), true);
                }
            }
            if !conns.is_empty() {
                ui.separator();
                ui.label(RichText::new(tr("Connections / harness")).strong()).on_hover_text(tr("Each point-to-point connection can belong to a harness"));
                egui::Grid::new("wharness").num_columns(2).show(ui, |ui| {
                    for (i, (a, b, h)) in conns.iter().enumerate() {
                        ui.label(RichText::new(format!("{a} – {b}")).monospace());
                        let shown = if h.is_empty() { "(none)".to_string() } else { here.relative(h) };
                        egui::ComboBox::from_id_salt(("wharness", i)).selected_text(shown).show_ui(ui, |ui| {
                            if ui.selectable_label(h.is_empty(), "(none)").clicked() {
                                harness_change = Some((a.clone(), b.clone(), String::new()));
                            }
                            for t in &harnesses {
                                if ui.selectable_label(t == h, t).clicked() {
                                    harness_change = Some((a.clone(), b.clone(), t.clone()));
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
            }
            if let Some((a, b, tag)) = harness_change {
                openecraft_core::harness::assign(&mut self.session.prj, &a, &b, &tag);
            }
        } else {
            self.graphic_properties(ui, id);
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn symbol_properties(&mut self, ui: &mut egui::Ui, idx: &Index, pi: usize, si: usize) {
        let s = self.session.prj.pages[pi].symbols[si].clone();
        let Some(def) = symbols::get(&s.sym) else {
            ui.label(tr_fmt!("Unknown symbol {}", s.sym));
            return;
        };
        ui.label(RichText::new(tr(def.name)).strong());
        let mut edited = s.clone();
        let here = Dt::of_page(&self.session.prj.pages[pi]);
        egui::Grid::new("sprops").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
            if def.func.has_tag() {
                ui.label(match def.func {
                    Func::Terminal => tr("Terminal strip"),
                    Func::Plc => tr("PLC card"),
                    Func::Plug(_) => tr("Plug"),
                    _ => tr("Device tag"),
                });
                ui.horizontal(|ui| {
                    // Shown relative to the page structure, stored complete.
                    let mut shown = here.relative(&edited.tag);
                    let before = shown.clone();
                    ui.add(egui::TextEdit::singleline(&mut shown).desired_width(90.0).font(egui::TextStyle::Monospace))
                        .on_hover_text(tr_fmt!("Complete tag: {}\nAspects missing here are taken from the page (=function +location).", edited.tag));
                    if shown != before {
                        edited.tag = if shown.trim().is_empty() { String::new() } else { here.complete(&shown) };
                    }
                    // Assign to an existing device (e.g. contact → coil)
                    egui::ComboBox::from_id_salt("assign").selected_text("…").width(30.0).show_ui(ui, |ui| {
                        for tag in idx.sorted_tags() {
                            let dev = &idx.devices[tag];
                            let fits = match def.func {
                                Func::Aux(_) => !dev.mains.is_empty(),
                                Func::Terminal | Func::Plc | Func::Plug(_) => !dev.others.is_empty(),
                                _ => true,
                            };
                            if fits && ui.selectable_label(*tag == edited.tag, tag.as_str()).clicked() {
                                edited.tag = tag.clone();
                            }
                        }
                    })
                    .response
                    .on_hover_text(tr("Assign to existing device"));
                });
                ui.end_row();
            }
            let label_name = match def.func {
                Func::Terminal => Some(tr("Terminal number")),
                Func::Plug(_) => Some(tr("Pin")),
                Func::Plc => Some(tr("Address")),
                Func::Potential => Some(tr("Potential name")),
                Func::Interruption => Some(tr("Name")),
                _ => None,
            };
            if let Some(name) = label_name {
                ui.label(name);
                ui.add(egui::TextEdit::singleline(&mut edited.label).desired_width(120.0));
                ui.end_row();
            }
            if def.func.has_tag() {
                ui.label(tr("Function text"));
                ui.add(egui::TextEdit::multiline(&mut edited.function_text).desired_rows(2).desired_width(160.0));
                ui.end_row();
            }
            if def.show_pin_labels {
                ui.label(tr("Connection points"));
                ui.horizontal_wrapped(|ui| {
                    for pin in edited.pins.iter_mut() {
                        ui.add(egui::TextEdit::singleline(pin).desired_width(32.0));
                    }
                });
                ui.end_row();
            }
            ui.label(tr("Orientation"));
            ui.horizontal(|ui| {
                if ui.button("⟲").clicked() {
                    edited.rot = (edited.rot + 3) % 4;
                }
                ui.label(format!("{}°", edited.rot as u32 * 90));
                if ui.button("⟳").clicked() {
                    edited.rot = (edited.rot + 1) % 4;
                }
                ui.checkbox(&mut edited.mirror, tr("Mirror"));
            });
            ui.end_row();
            ui.label(tr("Position"));
            ui.label(tr_fmt!("{} / {} mm  (column {})", format!("{:.0}", s.pos.x), format!("{:.0}", s.pos.y), openecraft_core::frame::of_page(&self.session.prj, pi).column_of(s.pos)));
            ui.end_row();
        });
        if let Func::Aux(kind) = def.func {
            if def.auto_number && ui.button(tr("Renumber contact")).on_hover_text(tr("Next free contact numbers on this device")).clicked() {
                edited.pins = ops::next_contact_pins(&self.session.prj, &edited.tag, kind, Some(s.id));
            }
        }
        if edited != s {
            self.session.prj.pages[pi].symbols[si] = edited.clone();
        }
        ui.horizontal(|ui| {
            ui.label(tr("Function definition"));
            properties_ui::function_def_combo(ui, &mut self.session.prj, s.id);
        });

        // Device data
        if def.func.has_tag() && !edited.tag.is_empty() {
            ui.separator();
            ui.label(RichText::new(tr_fmt!("Device {}", edited.tag)).strong());
            let mut data = self.session.prj.devices.get(&edited.tag).cloned().unwrap_or_default();
            let before = data.clone();
            ui.horizontal(|ui| {
                ui.label(tr("Description"));
                ui.text_edit_singleline(&mut data.description);
            });
            // Part assignment (several parts per device, e.g. contactor + auxiliary block)
            ui.label(RichText::new(tr("Parts")).strong());
            let mut remove = None;
            let mut show_part = None;
            egui::Grid::new("devparts").num_columns(2).striped(true).show(ui, |ui| {
                for (i, r) in data.parts.iter_mut().enumerate() {
                    let known = self.session.prj.parts.iter().find(|p| p.number == r.number);
                    let link = ui.link(RichText::new(&r.number).monospace());
                    let link = match known {
                        Some(p) => link.on_hover_text(format!("{}\n{}", p.description, p.template_summary())),
                        None => link.on_hover_text(tr("Not in the project parts – synchronize in parts management")),
                    };
                    if link.clicked() {
                        show_part = Some(r.number.clone());
                    }
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut r.count).range(1..=999).prefix("× "));
                        if ui.small_button("🗑").on_hover_text(tr("Remove part")).clicked() {
                            remove = Some(i);
                        }
                    });
                    ui.end_row();
                }
            });
            if let Some(i) = remove {
                data.parts.remove(i);
            }
            if let Some(n) = show_part {
                self.parts_ui.show_part(&n, true);
            }
            ui.horizontal(|ui| {
                if ui.button(tr("➕ Add part…")).on_hover_text(tr("Part selection from the parts database")).clicked() {
                    self.parts_ui.select_for(&edited.tag);
                }
                if !data.parts.is_empty()
                    && ui
                        .button(tr("Apply designations"))
                        .on_hover_text(tr("Takes the connection point designations of all functions of this device from the function templates of its parts"))
                        .clicked()
                {
                    parts::apply_designations(&mut self.session.prj, Some(&self.parts_db), &edited.tag);
                }
            });
            if data != before {
                self.session.prj.devices.insert(edited.tag.clone(), data.clone());
            }
            egui::CollapsingHeader::new(tr("Device properties")).id_salt("device_props").show(ui, |ui| {
                properties_ui::property_grid(ui, &mut self.session.prj, &properties::Object::Device(edited.tag.clone()), &mut self.properties_ui.device_grid, "devgrid");
            });
            if let Some((ei, _)) = openecraft_core::panel::locate(&self.session.prj, &edited.tag) {
                let enc = self.session.prj.layout.enclosures[ei].name.clone();
                if ui.button(tr_fmt!("🗄 Show in panel layout ({})", enc)).clicked() && self.panel_ui.show_device(&self.session.prj, &edited.tag) {
                    self.workspace = Workspace::Panel;
                }
            }
            // Functions placed vs. provided by the parts' function templates
            let usage = parts::usage(&self.session.prj, None, &edited.tag);
            if usage.values().any(|(_, a)| *a > 0) {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for (kind, (placed, avail)) in &usage {
                        let txt = format!("{} {placed}/{avail}", tr(kind.title()));
                        let color = if placed > avail && !matches!(kind, parts::FnKind::Terminal | parts::FnKind::PlugPin) {
                            Color32::from_rgb(220, 60, 60)
                        } else if placed < avail {
                            ui.visuals().weak_text_color()
                        } else {
                            ui.visuals().text_color()
                        };
                        ui.label(RichText::new(txt).small().color(color)).on_hover_text(tr("placed / provided by the parts"));
                    }
                });
            }
            // Rename the complete device
            let rename_id = ui.scope_id().with("rename");
            let mut new_tag: String = ui.data_mut(|d| d.get_temp(rename_id)).unwrap_or_else(|| edited.tag.clone());
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut new_tag).desired_width(90.0));
                if ui.button(tr("Rename device")).on_hover_text(tr("Changes the tag of all functions of this device")).clicked() {
                    ops::rename_device(&mut self.session.prj, &edited.tag, &new_tag);
                }
            });
            ui.data_mut(|d| d.insert_temp(rename_id, new_tag));

            // Cross references
            if let Some(dev) = idx.devices.get(&edited.tag) {
                ui.separator();
                ui.label(RichText::new(tr("Cross references")).strong());
                let mut jump = None;
                for (kind, locs) in [(tr("Main"), &dev.mains), (tr("Aux"), &dev.auxs), ("", &dev.others)] {
                    for loc in locs {
                        let inst = self.session.prj.pages[loc.page].symbols.iter().find(|x| x.id == loc.sym_id);
                        let Some(inst) = inst else { continue };
                        let dname = symbols::get(&inst.sym).map(|d| tr(d.name)).unwrap_or("?");
                        let extra = if inst.label.is_empty() { inst.pins.join("-") } else { inst.label.clone() };
                        let me = if loc.sym_id == s.id { " ◀" } else { "" };
                        if ui.link(format!("{} {kind} {dname} {extra}{me}", loc.reference(&self.session.prj))).clicked() {
                            jump = Some((loc.page, loc.sym_id));
                        }
                    }
                }
                if let Some((pg, id)) = jump {
                    self.goto(pg, id);
                }
            }
        }

        egui::CollapsingHeader::new(tr("Function properties")).id_salt("function_props").show(ui, |ui| {
            properties_ui::property_grid(ui, &mut self.session.prj, &properties::Object::Function(s.id), &mut self.properties_ui.function_grid, "fngrid");
        });

        // Potentials at the connection points
        if !def.pins.is_empty() {
            ui.separator();
            ui.label(RichText::new(tr("Connections")).strong());
            egui::Grid::new("pinnets").num_columns(2).striped(true).show(ui, |ui| {
                for i in 0..def.pins.len() {
                    let pin = PinRef { page: pi, sym: si, pin: i };
                    ui.label(RichText::new(pin_designation(&self.session.prj, pin)).monospace());
                    let net = self.session.analysis.netlist.pin_net.get(&(s.id, i)).map(|&n| self.session.analysis.netlist.nets[n].name.clone());
                    let unconnected = self.session.analysis.netlist.unconnected.iter().any(|&(_, id, p)| id == s.id && p == i);
                    if unconnected {
                        ui.label(RichText::new(tr("open")).color(Color32::from_rgb(220, 120, 0)));
                    } else {
                        ui.label(net.unwrap_or_default());
                    }
                    ui.end_row();
                }
            });
        }
        if self.session.prj.pages[pi].symbols.get(si).is_some_and(|x| x.id == s.id) && def.show_tag {
            ui.separator();
            self.placement_properties(ui, pi, si);
        }
    }
}
