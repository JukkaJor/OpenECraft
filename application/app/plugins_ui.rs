//! Plugins menu and windows: commands of the plugins with a form for their parameters,
//! plugin reports and exports, and the plugin manager (enable / disable, load errors).

use super::App;
use egui::RichText;
use openecraft_api::params::{text, Kind, Param};
use openecraft_api::{Command, Origin, Table, PLUGIN_PATH_VAR};
use openecraft_i18n::{tr, tr_fmt};
use serde_json::{Map, Value};

/// Storage key of the disabled plugin ids.
pub const STORAGE_KEY: &str = "disabled_plugins";

#[derive(Default)]
pub struct PluginsUi {
    /// Plugin manager window.
    pub open: bool,
    /// Command whose parameters are entered, with the text typed per parameter.
    command: Option<(Command, Vec<String>)>,
    /// Answer of the last command run.
    result: Option<Result<String, String>>,
    /// Report shown: its title and table.
    report: Option<(String, String, Result<Table, String>)>,
}

/// Arguments from the texts of the form; empty fields are left out.
fn form_args(params: &[Param], texts: &[String]) -> Result<Map<String, Value>, String> {
    let mut out = Map::new();
    for (p, t) in params.iter().zip(texts) {
        if t.trim().is_empty() {
            continue;
        }
        let v = p.kind.parse(t).ok_or_else(|| tr_fmt!("Parameter {} has the wrong type", p.name))?;
        out.insert(p.name.to_string(), v);
    }
    Ok(out)
}

impl App {
    pub(super) fn plugins_menu(&mut self, ui: &mut egui::Ui) {
        let commands: Vec<Command> = self.plugins.commands().cloned().collect();
        for c in commands {
            if ui.button(&c.name).on_hover_text(text(&c.help)).clicked() {
                let texts = vec![String::new(); c.params.len()];
                self.plugins_ui.result = None;
                if c.params.is_empty() {
                    self.run_plugin_command(&c, &[]);
                }
                self.plugins_ui.command = Some((c, texts));
            }
        }
        let reports: Vec<(String, String)> = self.plugins.reports().map(|r| (r.name.clone(), text(&r.title).to_string())).collect();
        if !reports.is_empty() {
            ui.separator();
        }
        for (name, title) in reports {
            if ui.button(tr_fmt!("📊 {}", title)).clicked() {
                self.show_plugin_report(name, title);
            }
        }
        let exporters: Vec<(String, String, String)> = self.plugins.exporters().map(|e| (e.name.clone(), text(&e.title).to_string(), e.extension.clone())).collect();
        if !exporters.is_empty() {
            ui.separator();
        }
        for (name, title, ext) in exporters {
            if ui.button(tr_fmt!("⬇ {}", title)).clicked() {
                let ctx = ui.ctx().clone();
                let result = self.plugins.exporter(&name).map(|e| (e.run)(&self.prj));
                match result {
                    Some(Ok(content)) => self.export(&ctx, format!("{}.{ext}", name), (&title, &ext), content),
                    Some(Err(e)) => self.notify(&ctx, tr_fmt!("Export failed: {}", e)),
                    None => {}
                }
            }
        }
        ui.separator();
        if ui.button(tr("Manage plugins…")).clicked() {
            self.plugins_ui.open = true;
        }
    }

    fn run_plugin_command(&mut self, c: &Command, texts: &[String]) {
        let result = form_args(&c.params, texts).and_then(|args| self.plugins.run(c, &mut self.prj, &args));
        self.plugins_ui.result = Some(result.map(|v| match v {
            Value::String(s) => s,
            Value::Null => tr("Done.").to_string(),
            v => serde_json::to_string_pretty(&v).unwrap_or_default(),
        }));
    }

    fn show_plugin_report(&mut self, name: String, title: String) {
        let table = match self.plugins.report(&name) {
            Some(r) => (r.run)(&self.prj, &self.analysis),
            None => return,
        };
        self.plugins_ui.report = Some((name, title, table));
    }

    /// Plugins whose checks, reports or menu entries changed: the analysis is rebuilt.
    fn plugins_changed(&mut self) {
        self.analysis_rev = self.history.rev.wrapping_add(1);
    }

    pub(super) fn plugins_windows(&mut self, ctx: &egui::Context) {
        self.plugin_command_window(ctx);
        self.plugin_report_window(ctx);
        if !self.plugins_ui.open {
            return;
        }
        let mut open = true;
        let mut toggled = Vec::new();
        egui::Window::new(tr("Plugins")).open(&mut open).default_size([620.0, 420.0]).show(ctx, |ui| {
            ui.weak(tr_fmt!("Plugins add commands, checks, reports and exports. External plugins are loaded from the folders in the environment variable {} (one subfolder with a plugin.json per plugin).", PLUGIN_PATH_VAR));
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for l in self.plugins.list() {
                    ui.horizontal(|ui| {
                        let mut on = l.enabled;
                        if ui.checkbox(&mut on, RichText::new(text(&l.info.name)).strong()).changed() {
                            toggled.push((l.info.id.clone(), on));
                        }
                        ui.weak(&l.info.version);
                        match &l.origin {
                            Origin::BuiltIn => ui.weak(tr("built-in")),
                            Origin::External(dir) => ui.weak(dir.display().to_string()),
                        };
                    });
                    ui.indent(&l.info.id, |ui| {
                        if !l.info.description.is_empty() {
                            ui.label(text(&l.info.description));
                        }
                        let r = &l.registry;
                        ui.weak(tr_fmt!("{} commands, {} checks, {} reports, {} exports", r.commands.len(), r.checks.len(), r.reports.len(), r.exporters.len()));
                    });
                }
                for e in &self.plugins.errors {
                    ui.label(RichText::new(e).color(egui::Color32::from_rgb(220, 60, 60)));
                }
            });
        });
        self.plugins_ui.open = open;
        if !toggled.is_empty() {
            for (id, on) in toggled {
                self.plugins.set_enabled(&id, on);
            }
            self.plugins_changed();
        }
    }

    fn plugin_command_window(&mut self, ctx: &egui::Context) {
        let Some((cmd, mut texts)) = self.plugins_ui.command.take() else { return };
        let mut open = true;
        let mut run = false;
        egui::Window::new(tr_fmt!("Plugin command {}", cmd.name)).open(&mut open).resizable(false).show(ctx, |ui| {
            ui.label(text(&cmd.help));
            if !cmd.params.is_empty() {
                egui::Grid::new("plugin_params").num_columns(2).show(ui, |ui| {
                    for (p, t) in cmd.params.iter().zip(texts.iter_mut()) {
                        let name = if p.required { format!("{} *", p.name) } else { p.name.to_string() };
                        ui.label(name).on_hover_text(text(&p.help));
                        if p.kind == Kind::Bool {
                            let mut on = Kind::Bool.parse(t) == Some(Value::Bool(true));
                            if ui.checkbox(&mut on, "").changed() {
                                *t = on.to_string();
                            }
                        } else {
                            ui.add(egui::TextEdit::singleline(t).hint_text(p.kind.hint()).desired_width(260.0));
                        }
                        ui.end_row();
                    }
                });
                run = ui.button(tr("▶ Run")).clicked();
            }
            match &self.plugins_ui.result {
                Some(Ok(out)) => {
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| ui.monospace(out));
                }
                Some(Err(e)) => {
                    ui.separator();
                    ui.label(RichText::new(e).color(egui::Color32::from_rgb(220, 60, 60)));
                }
                None => {}
            }
        });
        if run {
            self.run_plugin_command(&cmd, &texts);
        }
        if open {
            self.plugins_ui.command = Some((cmd, texts));
        }
    }

    fn plugin_report_window(&mut self, ctx: &egui::Context) {
        let Some((name, title, table)) = self.plugins_ui.report.take() else { return };
        let mut open = true;
        let (mut jump, mut export, mut refresh) = (None, false, false);
        egui::Window::new(title.as_str()).open(&mut open).default_size([700.0, 420.0]).show(ctx, |ui| {
            ui.horizontal(|ui| {
                export = ui.add_enabled(table.is_ok(), egui::Button::new(tr("⬇ Export CSV"))).clicked();
                refresh = ui.button(tr("⟳ Update")).clicked();
            });
            ui.separator();
            match &table {
                Ok(t) => {
                    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                        egui::Grid::new("plugin_report_grid").striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                            for h in t.header_texts() {
                                ui.label(RichText::new(h).strong());
                            }
                            ui.end_row();
                            for (row, target) in t.rows.iter().zip(t.targets.iter().chain(std::iter::repeat(&None))) {
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
                        if t.rows.is_empty() {
                            ui.weak(tr("(no entries)"));
                        }
                    });
                }
                Err(e) => {
                    ui.label(RichText::new(e).color(egui::Color32::from_rgb(220, 60, 60)));
                }
            }
        });
        if let (true, Ok(t)) = (export, &table) {
            self.export(ctx, format!("{name}.csv"), ("CSV", "csv"), t.to_csv());
        }
        if let Some((pg, id)) = jump {
            self.goto(pg, id);
        }
        if refresh {
            self.show_plugin_report(name, title);
        } else if open {
            self.plugins_ui.report = Some((name, title, table));
        }
    }
}

#[cfg(test)]
mod tests;
