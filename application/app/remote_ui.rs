//! Project server window and shared editing: opening and saving projects on the
//! project server, locks on pages and on the project data. Changes without a lock
//! request it; changes of what another user has locked are taken back.

use super::App;
use openecraft_core::history::History;
use crate::remote::{Remote, Reply};
use egui::{Color32, RichText};
use openecraft_core::collab::{valid_name, Grant, ProjectRight, Right, Shared, Snapshot, Subject, Target};
use openecraft_i18n::{tr, tr_fmt};

impl App {
    /// Handles the answers of the project server.
    pub(super) fn poll_remote(&mut self, ctx: &egui::Context) {
        for reply in self.remote.poll(ctx) {
            match reply {
                Reply::Opened(name, Ok(opened)) => {
                    let (shared, prj) = Shared::open(&name, *opened);
                    let next_id = prj.next_id;
                    self.replace_project(prj, None);
                    self.session.prj.next_id = next_id;
                    // Undo must not go back to the project opened before.
                    let rev = self.session.history.rev + 1;
                    self.session.history = History::new(&self.session.prj);
                    self.session.history.rev = rev;
                    self.session.saved_rev = rev;
                    self.remote.checked_rev = rev;
                    self.remote.shared = Some(shared);
                    self.remote.window = false;
                    self.notify(ctx, self.remote.status.clone());
                }
                Reply::Saved(_, Ok(snapshot)) => {
                    self.take_snapshot(ctx, *snapshot, true);
                    self.notify(ctx, self.remote.status.clone());
                }
                Reply::Snapshot(_, Ok(snapshot)) => self.take_snapshot(ctx, *snapshot, false),
                Reply::Status(_, Ok(status)) => self.remote.take_status(status, &[]),
                Reply::Locked(_, targets, Ok(status)) => self.remote.take_status(status, &targets),
                Reply::Locked(_, targets, Err(_)) => {
                    if let Some(sh) = &mut self.remote.shared {
                        sh.denied(&targets);
                        self.session.prj = sh.revert(&self.session.prj, &targets);
                    }
                    self.notify(ctx, self.remote.status.clone());
                }
                Reply::Opened(_, Err(_)) | Reply::Saved(_, Err(_)) | Reply::Snapshot(_, Err(_)) | Reply::Status(_, Err(_)) | Reply::Access(_, Err(_)) => {
                    self.notify(ctx, self.remote.status.clone());
                }
                Reply::List(_) | Reply::Login(_) | Reply::Access(_, Ok(_)) | Reply::Closed => {}
            }
        }
    }

    /// Takes a newer state of the server; undo steps keep the changes of others.
    fn take_snapshot(&mut self, ctx: &egui::Context, snapshot: Snapshot, saved: bool) {
        let clean = !self.session.dirty() || (saved && self.session.history.rev == self.remote.saving_rev);
        let Some(sh) = &mut self.remote.shared else { return };
        let (lost, apply) = sh.update(snapshot);
        let lost: Vec<String> = lost.iter().map(|t| Remote::describe(&self.session.prj, *t)).collect();
        self.session.history.rebase(&mut self.session.prj, apply);
        self.remote.checked_rev = self.session.history.rev;
        if clean {
            self.session.mark_saved();
        }
        self.session.page = self.session.page.min(self.session.prj.pages.len().saturating_sub(1));
        if !lost.is_empty() {
            self.notify(ctx, tr_fmt!("Changed by another user in the meantime, your change was replaced: {}", lost.join(", ")));
        }
    }

    /// Requests the locks for new changes; changes of what others have locked are
    /// taken back. Call after the history has committed the frame's changes.
    pub(super) fn check_locks(&mut self, ctx: &egui::Context) {
        if self.remote.checked_rev == self.session.history.rev {
            return;
        }
        self.remote.checked_rev = self.session.history.rev;
        let Some(sh) = &self.remote.shared else { return };
        let (held, free): (Vec<Target>, Vec<Target>) = sh.unlocked_changes(&self.session.prj).into_iter().partition(|t| sh.holder(*t).is_some());
        if !held.is_empty() {
            let who: Vec<String> = held
                .iter()
                .map(|t| tr_fmt!("{} (locked by {})", Remote::describe(&self.session.prj, *t), sh.holder(*t).map(|l| l.user.as_str()).unwrap_or_default()))
                .collect();
            self.session.prj = sh.revert(&self.session.prj, &held);
            self.notify(ctx, tr_fmt!("Not changed: {}", who.join(", ")));
        }
        if !free.is_empty() {
            self.remote.lock(free);
        }
    }

    /// Saves to the project server; `release` locks are released afterwards.
    pub(super) fn save_remote(&mut self, release: Vec<Target>) {
        self.remote.saving_rev = self.session.history.rev;
        self.remote.save(&self.session.prj, release);
    }

    fn my_locks(&self) -> Vec<Target> {
        let Some(sh) = &self.remote.shared else { return Vec::new() };
        sh.locks.iter().filter(|l| l.session == sh.session.session).map(|l| l.target).collect()
    }

    /// Text and colour of a page's lock in the page list.
    pub(super) fn page_lock_mark(&self, id: u64) -> Option<(String, Color32)> {
        let sh = self.remote.shared.as_ref()?;
        if let Some(l) = sh.holder(Target::Page(id)) {
            return Some((format!("🔒 {}", l.user), Color32::from_rgb(220, 140, 0)));
        }
        sh.owns(Target::Page(id)).then(|| ("✏".to_string(), Color32::from_rgb(60, 160, 90)))
    }

    /// The user holding the lock of the current page (another session).
    pub(super) fn page_locked_by(&self) -> Option<String> {
        let id = self.session.prj.pages.get(self.session.page)?.id;
        Some(self.remote.shared.as_ref()?.holder(Target::Page(id))?.user.clone())
    }

    /// Context menu entries of a page in the page list: the target to lock, or to
    /// save and release (true).
    pub(super) fn page_lock_menu(&self, ui: &mut egui::Ui, id: u64) -> Option<(Target, bool)> {
        let sh = self.remote.shared.as_ref()?;
        let target = Target::Page(id);
        if sh.owns(target) {
            ui.button(tr("Save and release page")).clicked().then_some((target, true))
        } else if sh.holder(target).is_none() {
            ui.button(tr("Lock page")).clicked().then_some((target, false))
        } else {
            None
        }
    }

    pub(super) fn remote_window(&mut self, ctx: &egui::Context) {
        if !self.remote.window {
            return;
        }
        let mut open = true;
        egui::Window::new(tr("Project server")).open(&mut open).default_width(520.0).show(ctx, |ui| {
            let connected = self.remote.shared.is_some();
            egui::Grid::new("remote_login").num_columns(2).show(ui, |ui| {
                ui.label(tr("Server"));
                ui.add_enabled(!connected, egui::TextEdit::singleline(&mut self.remote.url).desired_width(300.0));
                ui.end_row();
                ui.label(tr("User name"));
                ui.add_enabled(!connected, egui::TextEdit::singleline(&mut self.remote.user).desired_width(300.0));
                ui.end_row();
                ui.label(tr("Password"));
                ui.horizontal(|ui| {
                    let r = ui.add_enabled(!connected, egui::TextEdit::singleline(&mut self.remote.password).password(true).desired_width(200.0));
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let can = !connected && !self.remote.password.is_empty() && !self.remote.user.trim().is_empty() && !self.remote.busy();
                    if (ui.add_enabled(can, egui::Button::new(tr("Sign in"))).on_hover_text(tr("With a user account of the server; replaces the access token")).clicked() || enter) && can {
                        self.remote.sign_in();
                    }
                });
                ui.end_row();
                ui.label(tr("Access token"));
                ui.add_enabled(!connected, egui::TextEdit::singleline(&mut self.remote.token).password(true).desired_width(300.0));
                ui.end_row();
                if let Some(role) = self.remote.role {
                    ui.label(tr("Role"));
                    ui.label(role.title());
                    ui.end_row();
                }
                if connected && !self.remote.rights.is_empty() {
                    ui.label(tr("Rights"));
                    let rights: Vec<&str> = self.remote.rights.iter().map(|r| tr(r.title())).collect();
                    ui.add(egui::Label::new(rights.join(", ")).wrap());
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                if ui.add_enabled(!self.remote.busy(), egui::Button::new(tr("⟳ List projects"))).clicked() {
                    self.remote.list();
                }
                if self.remote.busy() {
                    ui.spinner();
                }
                ui.label(&self.remote.status);
            });
            ui.separator();
            if connected {
                self.shared_section(ui);
            } else {
                self.projects_section(ui);
            }
        });
        self.remote.window &= open;
    }

    fn projects_section(&mut self, ui: &mut egui::Ui) {
        let has_user = !self.remote.user.trim().is_empty();
        if !has_user {
            ui.colored_label(Color32::from_rgb(220, 140, 0), tr("Enter a user name: other users see it on your locks."));
        }
        let mut open = None;
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            egui::Grid::new("remote_projects").striped(true).num_columns(5).show(ui, |ui| {
                for h in [tr("Name"), tr("Project"), tr("Pages"), tr("Users"), ""] {
                    ui.strong(h);
                }
                ui.end_row();
                for p in &self.remote.projects {
                    ui.label(&p.name);
                    ui.label(&p.title);
                    ui.label(p.pages.to_string());
                    ui.label(p.users.join(", "));
                    if ui.add_enabled(has_user, egui::Button::new(tr("Open"))).clicked() {
                        open = Some(p.name.clone());
                    }
                    ui.end_row();
                }
            });
        });
        if self.session.dirty() {
            ui.colored_label(Color32::from_rgb(220, 140, 0), tr("Opening a project replaces the current one, its unsaved changes are lost."));
        }
        if let Some(name) = open {
            self.remote.open(&name);
        }
        ui.separator();
        ui.label(tr("Upload the current project as a new shared project:"));
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.remote.new_name).hint_text(tr("Name on the server")));
            let ok = has_user && valid_name(&self.remote.new_name);
            if ui
                .add_enabled(ok, egui::Button::new(tr("⬆ Upload")))
                .on_hover_text(tr("Letters, digits, space, - _ . (up to 64 characters)"))
                .clicked()
            {
                let name = self.remote.new_name.clone();
                self.remote.create(&name, &self.session.prj);
            }
        });
    }

    fn shared_section(&mut self, ui: &mut egui::Ui) {
        let Some(sh) = &self.remote.shared else { return };
        ui.label(RichText::new(tr_fmt!("Shared project {} – revision {}, user {}", sh.name, sh.revision, sh.session.user)).strong());
        if self.remote.viewer() && self.remote.may(ProjectRight::Review) {
            ui.colored_label(Color32::from_rgb(220, 140, 0), tr("You may review this project: comments, markup and approvals, no other changes."));
        } else if self.remote.viewer() {
            ui.colored_label(Color32::from_rgb(220, 140, 0), tr("You may only view this project."));
        }
        let mine = self.my_locks();
        let others: Vec<(Target, String)> = sh.locks.iter().filter(|l| l.session != sh.session.session).map(|l| (l.target, l.user.clone())).collect();
        let page_id = self.session.prj.pages.get(self.session.page).map(|p| p.id);
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("💾 Save")).clicked() {
                self.save_remote(Vec::new());
            }
            if ui.add_enabled(!mine.is_empty(), egui::Button::new(tr("Save and release all locks"))).clicked() {
                self.save_remote(mine.clone());
            }
            if let Some(id) = page_id {
                if !mine.contains(&Target::Page(id)) && ui.button(tr("Lock current page")).clicked() {
                    self.remote.lock(vec![Target::Page(id)]);
                }
            }
            if !mine.contains(&Target::Project)
                && ui.button(tr("Lock project data")).on_hover_text(tr("Page list, devices, parts, panel layout and settings")).clicked()
            {
                self.remote.lock(vec![Target::Project]);
            }
        });
        ui.add_space(4.0);
        ui.strong(tr("My locks"));
        if mine.is_empty() {
            ui.weak(tr("None – editing a page or the project data locks it."));
        }
        for t in &mine {
            ui.horizontal(|ui| {
                ui.label(format!("✏ {}", Remote::describe(&self.session.prj, *t)));
                if ui.small_button(tr("Save and release")).clicked() {
                    self.save_remote(vec![*t]);
                }
                if ui.small_button(tr("Release without saving")).on_hover_text(tr("Your unsaved changes of it are taken back")).clicked() {
                    if let Some(sh) = &self.remote.shared {
                        self.session.prj = sh.revert(&self.session.prj, &[*t]);
                    }
                    self.remote.release(vec![*t]);
                }
            });
        }
        ui.add_space(4.0);
        ui.strong(tr("Locked by other users"));
        if others.is_empty() {
            ui.weak(tr("None"));
        }
        for (t, user) in &others {
            ui.label(format!("🔒 {} – {}", Remote::describe(&self.session.prj, *t), user));
        }
        if self.remote.may(ProjectRight::ManageAccess) && !self.remote.rights.is_empty() {
            ui.separator();
            self.access_section(ui);
        }
        ui.separator();
        if ui.button(tr("Close shared project")).on_hover_text(tr("Releases all locks; the project stays open as a local copy")).clicked() {
            self.remote.close();
        }
    }

    /// Who may use the open project: grants of roles to users and groups (with
    /// wildcards; directory groups as `directory:group`), for project managers.
    fn access_section(&mut self, ui: &mut egui::Ui) {
        ui.strong(tr("Project access"));
        let busy = self.remote.busy();
        let Some(access) = &mut self.remote.access else {
            if ui.add_enabled(!busy, egui::Button::new(tr("Show who may use the project"))).clicked() {
                self.remote.load_access();
            }
            return;
        };
        let mut remove = None;
        egui::Grid::new("project_access").striped(true).num_columns(5).show(ui, |ui| {
            for h in [tr("User / group"), tr("Name"), tr("Role"), tr("Deny"), ""] {
                ui.strong(h);
            }
            ui.end_row();
            for (i, g) in access.grants.iter_mut().enumerate() {
                let group = matches!(g.subject, Subject::Group(_));
                let mut kind = group;
                egui::ComboBox::from_id_salt(("grant_kind", i)).selected_text(if group { tr("Group") } else { tr("User") }).show_ui(ui, |ui| {
                    ui.selectable_value(&mut kind, false, tr("User"));
                    ui.selectable_value(&mut kind, true, tr("Group"));
                });
                let mut name = g.subject.name().to_string();
                ui.add(egui::TextEdit::singleline(&mut name).desired_width(160.0).hint_text(tr("e.g. corp:Engineering")));
                g.subject = if kind { Subject::Group(name) } else { Subject::User(name) };
                egui::ComboBox::from_id_salt(("grant_role", i)).selected_text(g.role.as_str()).show_ui(ui, |ui| {
                    for r in &access.roles {
                        ui.selectable_value(&mut g.role, r.clone(), r.as_str());
                    }
                });
                ui.checkbox(&mut g.deny, "").on_hover_text(tr("Takes the rights of the role away, whatever other grants give"));
                if ui.small_button("🗑").on_hover_text(tr("Remove")).clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
        if let Some(i) = remove {
            access.grants.remove(i);
        }
        let role = access.roles.first().cloned().unwrap_or_default();
        let mut save = None;
        let mut reload = false;
        ui.horizontal(|ui| {
            if ui.button(tr("Add user")).clicked() {
                access.grants.push(Grant::new(Subject::User(String::new()), &role, ""));
            }
            if ui.button(tr("Add group")).clicked() {
                access.grants.push(Grant::new(Subject::Group(String::new()), &role, ""));
            }
            let complete = access.grants.iter().all(|g| !g.subject.name().trim().is_empty() && !g.role.is_empty());
            if ui.add_enabled(complete && !busy, egui::Button::new(tr("Save access"))).clicked() {
                save = Some(access.grants.clone());
            }
            reload = ui.add_enabled(!busy, egui::Button::new(tr("⟳ Reload"))).clicked();
        });
        if let Some(grants) = save {
            self.remote.save_access(&grants);
        }
        if reload {
            self.remote.access = None;
            self.remote.load_access();
        }
    }
}
