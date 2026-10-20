//! Main window of the parts manager: connection, part tree, part editor.

use crate::session::{FollowUp, Session};
use egui::{Color32, Key, KeyboardShortcut, Modifiers, RichText};
use openecraft_parts::to_csv;
use openecraft_ui::client::ServerClient;
use openecraft_ui::io::{short_name, FileIo, FileKind, Loaded};
use openecraft_ui::part_editor::{self as pe, EditorState, ModelKind};

/// What a file is opened for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// CSV / JSON parts file, uploaded to the server.
    Parts,
    /// 2D / 3D model of the part in the editor.
    Model(ModelKind),
}

impl FileKind for Purpose {
    fn filter(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Purpose::Parts => ("Parts (CSV, JSON)", &["csv", "json", "txt"]),
            Purpose::Model(kind) => kind.filter(),
        }
    }
}

/// Action waiting for confirmation.
enum Confirm {
    /// Discard unsaved changes, then open this part (`None`: start a new part).
    Discard(Option<String>),
    /// Delete this part on the server.
    Delete(String),
}

pub struct ManagerApp {
    client: ServerClient,
    session: Session,
    editor: EditorState,
    io: FileIo<Purpose>,
    filter: String,
    confirm: Option<Confirm>,
    /// Part number a model import was requested for.
    model_target: Option<String>,
    started: bool,
    title: String,
}

const URL_KEY: &str = "server_url";

impl ManagerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> ManagerApp {
        let mut client = ServerClient::default();
        if let Some(url) = cc.storage.and_then(|s| eframe::get_value::<String>(s, URL_KEY)) {
            client.url = url;
        }
        ManagerApp {
            client,
            session: Session::default(),
            editor: EditorState::default(),
            io: FileIo::default(),
            filter: String::new(),
            confirm: None,
            model_target: None,
            started: false,
            title: String::new(),
        }
    }

    fn writable(&self) -> bool {
        self.client.writable() && !self.client.busy()
    }

    fn poll(&mut self, ctx: &egui::Context) {
        for reply in self.client.poll() {
            match self.session.apply(&reply) {
                Some(FollowUp::Reload) => self.client.download(ctx),
                Some(FollowUp::Delete(number)) => self.client.delete(ctx, &number),
                None => {}
            }
        }
        if let Some(loaded) = self.io.poll() {
            self.loaded(ctx, loaded);
        }
    }

    fn loaded(&mut self, ctx: &egui::Context, loaded: Loaded<Purpose>) {
        match loaded.purpose {
            Purpose::Parts => self.client.upload_file(ctx, loaded.bytes),
            Purpose::Model(kind) => {
                let target = self.model_target.take();
                let draft = self.session.draft.as_mut().filter(|d| Some(&d.part.number) == target.as_ref());
                self.client.status = match draft {
                    Some(d) => pe::apply_model(&mut d.part, kind, &loaded.name, &loaded.bytes)
                        .map(|m| format!("{m} – save to upload it"))
                        .unwrap_or_else(|e| format!("Model import failed: {e}")),
                    None => format!("{}: the part is no longer open", short_name(&loaded.name)),
                };
            }
        }
    }

    /// Opens a part or starts a new one, asking first if changes would be lost.
    fn open(&mut self, number: Option<String>) {
        if self.session.dirty() {
            self.confirm = Some(Confirm::Discard(number));
            return;
        }
        match number {
            Some(n) => self.session.select(&n),
            None => self.session.new_part(),
        }
    }

    fn save(&mut self, ctx: &egui::Context) {
        if let (Some(d), None) = (&self.session.draft, self.session.save_problem()) {
            let part = d.part.clone();
            self.client.save(ctx, &part);
        }
    }

    fn export(&mut self, csv: bool) {
        let (name, filter, content) = if csv {
            ("parts.csv", ("CSV", "csv"), to_csv(&self.session.parts))
        } else {
            ("parts.json", ("JSON", "json"), serde_json::to_string_pretty(&self.session.database()).unwrap_or_default())
        };
        self.client.status = match self.io.save_as(name, filter, &content) {
            Ok(Some(path)) => format!("Exported {} parts to {}", self.session.parts.len(), short_name(&path)),
            Ok(None) => return,
            Err(e) => format!("Export failed: {e}"),
        };
    }

    fn connection_bar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("OpenECraft parts manager").strong());
            ui.separator();
            ui.label("Server");
            let url = ui.add(egui::TextEdit::singleline(&mut self.client.url).desired_width(240.0));
            ui.label("Access token");
            let token = ui.add(
                egui::TextEdit::singleline(&mut self.client.token).password(true).desired_width(160.0).hint_text("needed for changes"),
            );
            let enter = (url.lost_focus() || token.lost_focus()) && ui.input(|i| i.key_pressed(Key::Enter));
            if ui.add_enabled(!self.client.busy(), egui::Button::new("Connect")).clicked() || enter {
                self.client.check(&ctx);
            }
            ui.separator();
            match &self.client.info {
                Some(info) => {
                    ui.label(format!("{} {}", info.name, info.version));
                    if info.writable {
                        ui.label(RichText::new("writable with token").color(Color32::from_rgb(60, 160, 90)));
                    } else {
                        ui.label(RichText::new("read-only server").color(Color32::from_rgb(220, 140, 0)))
                            .on_hover_text("Start openecraft-server with --token to allow changes");
                    }
                }
                None => {
                    ui.weak("not connected");
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut pref = ui.ctx().options(|o| o.theme_preference);
                egui::ComboBox::from_id_salt("theme").selected_text(format!("{pref:?}")).width(70.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut pref, egui::ThemePreference::System, "System");
                    ui.selectable_value(&mut pref, egui::ThemePreference::Light, "Light");
                    ui.selectable_value(&mut pref, egui::ThemePreference::Dark, "Dark");
                });
                if pref != ui.ctx().options(|o| o.theme_preference) {
                    ui.ctx().set_theme(pref);
                }
            });
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let connected = self.session.loaded;
        let writable = self.writable();
        let has_draft = self.session.draft.is_some();
        let dirty = self.session.dirty();
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(connected, egui::Button::new("➕ New")).clicked() {
                self.open(None);
            }
            if ui.add_enabled(has_draft, egui::Button::new("⎘ Copy")).on_hover_text("New part based on the open one").clicked() {
                self.session.copy_part();
            }
            let save = ui
                .add_enabled(writable && dirty && self.session.save_problem().is_none(), egui::Button::new("✔ Save"))
                .on_hover_text("Create / update the part on the server (Ctrl+S)");
            if save.clicked() {
                self.save(&ctx);
            }
            if ui.add_enabled(dirty, egui::Button::new("⟲ Revert")).on_hover_text("Discard the changes").clicked() {
                self.session.revert();
            }
            let deletable = self.session.draft.as_ref().and_then(|d| d.original.clone());
            if ui.add_enabled(writable && deletable.is_some(), egui::Button::new("🗑 Delete")).clicked() {
                self.confirm = deletable.map(Confirm::Delete);
            }
            ui.separator();
            if ui.add_enabled(!self.client.busy() && connected, egui::Button::new("⟳ Reload")).on_hover_text("Load all parts from the server again").clicked() {
                self.client.download(&ctx);
            }
            ui.menu_button("Import / export", |ui| {
                if ui
                    .add_enabled(writable, egui::Button::new("⬆ Upload parts file (CSV / JSON)…"))
                    .on_hover_text("Adds new parts and replaces existing ones on the server")
                    .clicked()
                {
                    self.io.open(&ctx, Purpose::Parts);
                }
                ui.separator();
                if ui.add_enabled(connected, egui::Button::new("Export as CSV…")).clicked() {
                    self.export(true);
                }
                if ui.add_enabled(connected, egui::Button::new("Export as JSON…")).clicked() {
                    self.export(false);
                }
            });
            if self.client.busy() {
                ui.spinner();
            }
        });
    }

    fn part_list(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filter (number, type, manufacturer, text …)"));
        ui.label(RichText::new(format!("{} parts", self.session.parts.len())).weak());
        if let Some(d) = self.session.draft.as_ref().filter(|d| d.original.is_none()) {
            ui.label(RichText::new(format!("➕ {} (not saved)", d.part.number)).italics());
        }
        egui::ScrollArea::vertical().id_salt("tree").auto_shrink([false, false]).show(ui, |ui| {
            let refs: Vec<_> = self.session.parts.iter().collect();
            let before = self.session.selected().map(String::from);
            let mut selected = before.clone();
            pe::part_tree(ui, "parts", &refs, &self.filter, &mut selected);
            if selected != before {
                self.open(selected);
            }
        });
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        if !self.session.loaded {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.heading("Connect to a parts server");
                ui.label("Enter the address of an openecraft-server and click Connect.");
                ui.label("The access token is only needed for changes; it is not stored.");
                ui.add_space(8.0);
                ui.weak("Start a server with:  openecraft-server --token <secret>");
            });
            return;
        }
        let numbers: Vec<String> = self.session.parts.iter().map(|p| p.number.clone()).collect();
        let problem = self.session.save_problem();
        let dirty = self.session.dirty();
        let Some(d) = self.session.draft.as_mut() else {
            ui.weak("Select a part in the tree or create a new one.");
            return;
        };
        ui.horizontal(|ui| {
            ui.heading(&d.part.number);
            match (&d.original, dirty) {
                (None, _) => ui.label(RichText::new("new part, not saved").color(Color32::from_rgb(220, 140, 0))),
                (Some(_), true) => ui.label(RichText::new("changed, not saved").color(Color32::from_rgb(220, 140, 0))),
                (Some(_), false) => ui.weak("saved"),
            };
            if let Some(original) = d.original.as_ref().filter(|o| **o != d.part.number) {
                ui.weak(format!("(renamed from {original})"));
            }
        });
        ui.separator();
        let duplicate = problem == Some("Part number already exists");
        if let Some(kind) = pe::part_editor(ui, &mut d.part, &mut self.editor, &numbers, duplicate) {
            self.model_target = Some(d.part.number.clone());
            let ctx = ui.ctx().clone();
            self.io.open(&ctx, Purpose::Model(kind));
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(confirm) = &self.confirm else { return };
        let (text, button) = match confirm {
            Confirm::Discard(_) => {
                let n = self.session.draft.as_ref().map(|d| d.part.number.clone()).unwrap_or_default();
                (format!("Discard the unsaved changes of {n}?"), "Discard")
            }
            Confirm::Delete(n) => (format!("Delete part {n} from the server? This cannot be undone."), "🗑 Delete"),
        };
        let mut answer = None;
        egui::Modal::new(egui::Id::unique("confirm")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(text);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(button).clicked() {
                    answer = Some(true);
                }
                if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                    answer = Some(false);
                }
            });
        });
        match answer {
            Some(true) => match self.confirm.take() {
                Some(Confirm::Discard(number)) => {
                    self.session.revert();
                    self.open(number);
                }
                Some(Confirm::Delete(number)) => self.client.delete(ctx, &number),
                None => {}
            },
            Some(false) => self.confirm = None,
            None => {}
        }
    }
}

impl eframe::App for ManagerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.started {
            self.started = true;
            self.client.check(&ctx);
        }
        self.poll(&ctx);
        if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::S))) && self.writable() {
            self.save(&ctx);
        }

        egui::Panel::top("connection").show(ui, |ui| self.connection_bar(ui));
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status").show(ui, |ui| {
            let color = if self.client.failed() { Color32::from_rgb(220, 60, 60) } else { ui.visuals().text_color() };
            ui.label(RichText::new(&self.client.status).color(color));
        });
        egui::Panel::left("parts").resizable(true).default_size(300.0).show(ui, |ui| self.part_list(ui));
        egui::CentralPanel::default().show(ui, |ui| self.editor(ui));
        self.dialogs(&ctx);

        let title = format!("OpenECraft parts manager – {}{}", self.client.url, if self.session.dirty() { " *" } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, URL_KEY, &self.client.url);
    }
}
