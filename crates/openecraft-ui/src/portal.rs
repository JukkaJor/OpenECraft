//! Data portal browser: faceted search on the parts server, part details with files
//! and history, favorites and the update check of local parts. [`Browser`] holds the
//! state (no UI code, unit tested); [`browser`] draws it and sends the requests.

use crate::client::{FileUse, Reply, ServerClient};
use openecraft_client::PortalKind;
use crate::part_editor::{part_summary, size_text};
use egui::{Color32, RichText};
use openecraft_i18n::{tr, tr_fmt};
use openecraft_parts::api::PartQuery;
use openecraft_parts::portal::{check_updates, lifecycle_from_key, top_group_from_key, Facet, HistoryEntry, Lifecycle, PartUpdate, SearchResult, Sort};
use openecraft_parts::{date_from_unix, productgroups, Part, PRODUCT_GROUPS};
use std::collections::BTreeSet;

/// Parts per result page.
pub const PAGE: usize = 50;

/// What the application has to do for the browser.
#[derive(Clone, Debug, PartialEq)]
pub enum BrowserEvent {
    /// Replace the local parts listed in [`Browser::updates`] by their server versions.
    ApplyUpdates,
}

#[derive(Default)]
pub struct Browser {
    /// Search text (applied with the next search).
    pub text: String,
    pub query: PartQuery,
    pub result: Option<SearchResult>,
    /// Part shown in the details.
    pub selected: Option<String>,
    /// Parts marked for adding.
    pub marked: BTreeSet<String>,
    pub history: Option<(String, Vec<HistoryEntry>)>,
    pub favorites: Vec<String>,
    pub only_favorites: bool,
    /// Result of the last update check and the server versions it compared with.
    pub updates: Option<Vec<PartUpdate>>,
    pub remote: Vec<Part>,
    pub user_name: String,
    pub password: String,
    /// A new search is due.
    dirty: bool,
}

impl Browser {
    pub fn new() -> Browser {
        Browser { dirty: true, ..Default::default() }
    }

    /// The query of the next search: filters, text, page and favorites.
    pub fn search_query(&self) -> PartQuery {
        let text = self.text.trim();
        let mut q = PartQuery { q: (!text.is_empty()).then(|| text.to_string()), brief: true, limit: Some(PAGE), ..self.query.clone() };
        if self.only_favorites {
            // An empty list would mean "all parts".
            q.numbers = if self.favorites.is_empty() { vec![String::new()] } else { self.favorites.clone() };
        }
        q
    }

    /// Takes the search request if one is due.
    pub fn take_search(&mut self) -> Option<PartQuery> {
        std::mem::take(&mut self.dirty).then(|| self.search_query())
    }

    pub fn refresh(&mut self) {
        self.dirty = true;
    }

    /// New search from the first page.
    pub fn restart(&mut self) {
        self.query.offset = None;
        self.dirty = true;
    }

    /// Value of a fixed facet ([`openecraft_parts::portal::FACETS`]) in the query.
    pub fn facet_filter(&self, name: &str) -> Option<String> {
        match name {
            "top_group" => self.query.top_group.map(|t| format!("{t:?}")),
            "group" => self.query.group.clone(),
            "subgroup" => self.query.subgroup.clone(),
            "manufacturer" => self.query.manufacturer.clone(),
            "catalog" => self.query.catalog.clone(),
            "lifecycle" => self.query.lifecycle.map(|l| format!("{l:?}")),
            _ => None,
        }
    }

    /// Selects a facet value, or clears it if it is selected.
    pub fn toggle_facet(&mut self, name: &str, value: &str) {
        let value = (self.facet_filter(name).as_deref() != Some(value)).then(|| value.to_string());
        match name {
            "top_group" => self.query.top_group = value.as_deref().and_then(top_group_from_key),
            "group" => self.query.group = value,
            "subgroup" => self.query.subgroup = value,
            "manufacturer" => self.query.manufacturer = value,
            "catalog" => self.query.catalog = value,
            "lifecycle" => self.query.lifecycle = value.as_deref().and_then(lifecycle_from_key),
            _ => return,
        }
        self.restart();
    }

    pub fn attribute_filter(&self, name: &str) -> Option<&str> {
        self.query.attributes.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }

    /// Selects an attribute value (replacing another value of it), or clears it.
    pub fn toggle_attribute(&mut self, name: &str, value: &str) {
        let selected = self.attribute_filter(name) == Some(value);
        self.query.attributes.retain(|(k, _)| !k.eq_ignore_ascii_case(name));
        if !selected {
            self.query.attributes.push((name.to_string(), value.to_string()));
        }
        self.restart();
    }

    pub fn has_filters(&self) -> bool {
        let q = &self.query;
        q.top_group.is_some()
            || q.group.is_some()
            || q.subgroup.is_some()
            || q.manufacturer.is_some()
            || q.catalog.is_some()
            || q.lifecycle.is_some()
            || !q.attributes.is_empty()
    }

    pub fn clear_filters(&mut self) {
        self.query = PartQuery { sort: self.query.sort, descending: self.query.descending, ..Default::default() };
        self.restart();
    }

    /// Current page and number of pages (1 based).
    pub fn page(&self) -> (usize, usize) {
        let total = self.result.as_ref().map_or(0, |r| r.total);
        (self.query.offset.unwrap_or(0) / PAGE + 1, total.div_ceil(PAGE).max(1))
    }

    pub fn go_to_page(&mut self, page: usize) {
        let (_, pages) = self.page();
        self.query.offset = Some((page.clamp(1, pages) - 1) * PAGE);
        self.dirty = true;
    }

    pub fn is_favorite(&self, number: &str) -> bool {
        self.favorites.iter().any(|f| f == number)
    }

    /// The favorites with `number` added or removed.
    pub fn toggled_favorite(&self, number: &str) -> Vec<String> {
        let mut list: Vec<String> = self.favorites.iter().filter(|f| *f != number).cloned().collect();
        if !self.is_favorite(number) {
            list.push(number.to_string());
        }
        list
    }

    pub fn selected_part(&self) -> Option<&Part> {
        let n = self.selected.as_ref()?;
        self.result.as_ref()?.parts.iter().find(|p| &p.number == n)
    }

    /// Takes a server reply. `local`: the parts an update check compares with.
    pub fn apply(&mut self, reply: &Reply, local: &[Part]) {
        match reply {
            Reply::Search(Ok(r)) => {
                let numbers: BTreeSet<&str> = r.parts.iter().map(|p| p.number.as_str()).collect();
                if self.selected.as_deref().is_some_and(|n| !numbers.contains(n)) {
                    self.selected = None;
                }
                self.result = Some(r.clone());
            }
            Reply::History(number, Ok(h)) => self.history = Some((number.clone(), h.clone())),
            Reply::Favorites(Ok(f)) => {
                self.favorites = f.clone();
                if self.only_favorites {
                    self.dirty = true;
                }
            }
            Reply::Versions(Ok(r)) => {
                self.updates = Some(check_updates(local, &r.parts));
                self.remote = r.parts.clone();
            }
            Reply::SignedIn(Ok(_)) => self.password.clear(),
            Reply::SignedOut => {
                self.favorites.clear();
                self.only_favorites = false;
                self.dirty = true;
            }
            _ => {}
        }
    }

    /// Forgets the update check (after applying it).
    pub fn updates_applied(&mut self) {
        self.updates = None;
        self.remote.clear();
    }
}

/// Translated title of a fixed facet.
fn facet_title(name: &str) -> String {
    match name {
        "top_group" => tr("Product top group").into(),
        "group" => tr("Product group").into(),
        "subgroup" => tr("Product subgroup").into(),
        "manufacturer" => tr("Manufacturer").into(),
        "catalog" => tr("Catalog").into(),
        "lifecycle" => tr("Lifecycle").into(),
        "Voltage" => tr("Voltage").into(),
        "Current" => tr("Current").into(),
        "Power" => tr("Power").into(),
        other => other.to_string(),
    }
}

/// Group names are stored in English; known ones are shown translated.
pub fn group_text(value: &str) -> String {
    let own = PRODUCT_GROUPS.iter().flat_map(|(g, subs)| std::iter::once(*g).chain(subs.iter().copied()));
    let eplan = productgroups::GROUPS.iter().chain(productgroups::SUBGROUPS).map(|(_, n)| *n);
    match own.chain(eplan).find(|n| n.eq_ignore_ascii_case(value)) {
        Some(n) => tr(n).to_string(),
        None => value.to_string(),
    }
}

/// Translated text of a facet value.
fn value_text(facet: &str, value: &str) -> String {
    match facet {
        "top_group" => top_group_from_key(value).map_or(value.to_string(), |t| tr(t.title()).to_string()),
        "lifecycle" => lifecycle_from_key(value).map_or(value.to_string(), |l| tr(l.title()).to_string()),
        "group" | "subgroup" => group_text(value),
        _ => value.to_string(),
    }
}

fn lifecycle_color(l: Lifecycle) -> Color32 {
    match l {
        Lifecycle::Active => Color32::from_rgb(70, 160, 80),
        Lifecycle::PhaseOut => Color32::from_rgb(220, 140, 40),
        Lifecycle::Discontinued => Color32::from_rgb(220, 60, 60),
    }
}

fn facet_list(ui: &mut egui::Ui, f: &Facet, title: String, selected: Option<&str>, value: impl Fn(&str) -> String) -> Option<String> {
    let mut clicked = None;
    let id = ui.make_persistent_id(("facet", &f.name));
    egui::CollapsingHeader::new(RichText::new(title).strong()).id_salt(id).default_open(selected.is_some() || f.values.len() <= 8).show(ui, |ui| {
        for v in &f.values {
            let on = selected == Some(v.value.as_str());
            if ui.selectable_label(on, format!("{}  ({})", value(&v.value), v.count)).clicked() {
                clicked = Some(v.value.clone());
            }
        }
    });
    clicked
}

/// Choice of the kind of server (OpenECraft parts server or EPLAN Data Portal).
/// Returns true if it changed.
pub fn kind_choice(ui: &mut egui::Ui, client: &mut ServerClient) -> bool {
    let mut kind = client.kind;
    egui::ComboBox::from_id_salt("parts_server_kind").selected_text(tr(kind.title())).show_ui(ui, |ui| {
        for k in PortalKind::ALL {
            ui.selectable_value(&mut kind, k, tr(k.title()));
        }
    });
    let changed = kind != client.kind;
    client.set_kind(kind);
    changed
}

/// Label and hint of the token field for the kind of server.
pub fn token_texts(client: &ServerClient) -> (&'static str, &'static str) {
    match client.kind {
        PortalKind::OpenECraft => (tr("Access token"), tr("only needed for uploads")),
        PortalKind::Eplan => (tr("Personal access token"), tr("created in your EPLAN account (ePulse)")),
    }
}

/// The data portal browser. `local`: the parts the update check compares (the
/// local parts database). Requests are sent through `client`.
pub fn browser(ui: &mut egui::Ui, b: &mut Browser, client: &mut ServerClient, local: &[Part]) -> Option<BrowserEvent> {
    let ctx = ui.ctx().clone();
    let mut event = None;
    let busy = client.busy();
    let signed_in = client.user().cloned();

    // Search line and account
    ui.horizontal_wrapped(|ui| {
        let edit = ui.add(egui::TextEdit::singleline(&mut b.text).desired_width(260.0).hint_text(tr("Search parts")));
        if ui.button(tr("🔎 Search")).clicked() || edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            b.restart();
        }
        if signed_in.is_some() && ui.toggle_value(&mut b.only_favorites, tr("★ Favorites")).changed() {
            b.restart();
        }
        ui.label(tr("Sort by"));
        let before = (b.query.sort, b.query.descending);
        egui::ComboBox::from_id_salt("portal_sort").selected_text(tr(b.query.sort.title())).show_ui(ui, |ui| {
            for s in Sort::ALL {
                ui.selectable_value(&mut b.query.sort, s, tr(s.title()));
            }
        });
        ui.checkbox(&mut b.query.descending, tr("Descending"));
        if before != (b.query.sort, b.query.descending) {
            b.restart();
        }
        if !client.read_only() {
            ui.separator();
        }
        match &signed_in {
            _ if client.read_only() => {}
            Some(u) => {
                ui.label(tr_fmt!("Signed in as {}", u.name));
                if ui.add_enabled(!busy, egui::Button::new(tr("Sign out"))).clicked() {
                    client.sign_out(&ctx);
                }
            }
            None => {
                ui.add(egui::TextEdit::singleline(&mut b.user_name).desired_width(90.0).hint_text(tr("User name")));
                let pw = ui.add(egui::TextEdit::singleline(&mut b.password).password(true).desired_width(90.0).hint_text(tr("Password")));
                let enter = pw.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.add_enabled(!busy && !b.user_name.trim().is_empty(), egui::Button::new(tr("Sign in"))).clicked() || enter {
                    client.sign_in(&ctx, &b.user_name, &b.password);
                }
            }
        }
        if busy {
            ui.spinner();
        }
    });
    if !client.status.is_empty() {
        let color = if client.failed() { Color32::from_rgb(220, 60, 60) } else { ui.visuals().weak_text_color() };
        ui.label(RichText::new(&client.status).color(color).small());
    }
    ui.separator();

    let h = ui.available_height() - 64.0;
    ui.horizontal_top(|ui| {
        // Facets
        ui.allocate_ui_with_layout(egui::vec2(230.0, h), egui::Layout::top_down(egui::Align::Min), |ui| {
            egui::ScrollArea::vertical().id_salt("portal_facets").auto_shrink([false, false]).show(ui, |ui| {
                if b.has_filters() && ui.button(tr("Clear filters")).clicked() {
                    b.clear_filters();
                }
                let Some(r) = b.result.clone() else { return };
                for f in &r.facets {
                    let selected = b.facet_filter(&f.name);
                    if let Some(v) = facet_list(ui, f, facet_title(&f.name), selected.as_deref(), |v| value_text(&f.name, v)) {
                        b.toggle_facet(&f.name, &v);
                    }
                }
                if !r.attributes.is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new(tr("Technical data")).weak());
                }
                for f in &r.attributes {
                    let selected = b.attribute_filter(&f.name).map(str::to_string);
                    if let Some(v) = facet_list(ui, f, facet_title(&f.name), selected.as_deref(), str::to_string) {
                        b.toggle_attribute(&f.name, &v);
                    }
                }
            });
        });
        ui.separator();
        // Results
        let width = (ui.available_width() * 0.55).max(320.0);
        ui.allocate_ui_with_layout(egui::vec2(width, h), egui::Layout::top_down(egui::Align::Min), |ui| {
            let Some(r) = b.result.clone() else {
                ui.weak(tr("Connect to a parts server and search."));
                return;
            };
            ui.horizontal(|ui| {
                ui.strong(tr_fmt!("{} parts", r.total));
                let (page, pages) = b.page();
                if ui.add_enabled(page > 1, egui::Button::new("◀").small()).clicked() {
                    b.go_to_page(page - 1);
                }
                ui.label(tr_fmt!("Page {} of {}", page, pages));
                if ui.add_enabled(page < pages, egui::Button::new("▶").small()).clicked() {
                    b.go_to_page(page + 1);
                }
            });
            egui::ScrollArea::vertical().id_salt("portal_results").auto_shrink([false, false]).show(ui, |ui| {
                if r.parts.is_empty() {
                    ui.weak(tr("No parts found"));
                }
                for p in &r.parts {
                    ui.horizontal(|ui| {
                        let mut marked = b.marked.contains(&p.number);
                        if ui.checkbox(&mut marked, "").on_hover_text(tr("Mark for adding to the parts database")).changed() {
                            if marked {
                                b.marked.insert(p.number.clone());
                            } else {
                                b.marked.remove(&p.number);
                            }
                        }
                        if b.is_favorite(&p.number) {
                            ui.label("★");
                        }
                        let in_db = local.iter().any(|l| l.number == p.number);
                        let text = RichText::new(format!("{}  {}", p.number, p.description));
                        let text = if in_db { text.strong() } else { text };
                        let resp = ui.selectable_label(b.selected.as_deref() == Some(p.number.as_str()), text);
                        let resp = if in_db { resp.on_hover_text(tr("Already in the parts database")) } else { resp };
                        if resp.clicked() {
                            b.selected = Some(p.number.clone());
                        }
                        if !p.lifecycle.is_active() {
                            ui.colored_label(lifecycle_color(p.lifecycle), tr(p.lifecycle.title()));
                        }
                    });
                }
            });
        });
        ui.separator();
        // Details
        ui.vertical(|ui| {
            egui::ScrollArea::vertical().id_salt("portal_detail").auto_shrink([false, false]).max_height(h).show(ui, |ui| {
                let Some(p) = b.selected_part().cloned() else {
                    ui.weak(tr("Select a part to see its details."));
                    return;
                };
                part_summary(ui, &p);
                ui.horizontal_wrapped(|ui| {
                    if !p.catalog.is_empty() {
                        ui.label(RichText::new(tr("Catalog")).weak());
                        ui.label(&p.catalog);
                    }
                    if p.revision > 0 {
                        ui.label(RichText::new(tr("Last change")).weak());
                        ui.label(date_from_unix(p.modified));
                    }
                });
                if !p.lifecycle.is_active() {
                    ui.colored_label(lifecycle_color(p.lifecycle), tr(p.lifecycle.title()));
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(!busy, egui::Button::new(tr("⬇ Add to the parts database"))).clicked() {
                        client.fetch(&ctx, std::slice::from_ref(&p.number));
                    }
                    if !p.successor.is_empty() && ui.button(tr_fmt!("➡ Successor {}", p.successor)).clicked() {
                        b.text = p.successor.clone();
                        b.selected = Some(p.successor.clone());
                        b.clear_filters();
                    }
                    if signed_in.is_some() {
                        let fav = b.is_favorite(&p.number);
                        if ui.add_enabled(!busy, egui::Button::new(if fav { tr("★ Remove from favorites") } else { tr("☆ Add to favorites") })).clicked() {
                            client.set_favorites(&ctx, &b.toggled_favorite(&p.number));
                        }
                    }
                });
                if !p.attachments.is_empty() {
                    ui.add_space(6.0);
                    ui.strong(tr("Files"));
                    for a in &p.attachments {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(tr(a.kind.title())).weak());
                            ui.label(&a.name);
                            if a.size > 0 {
                                ui.label(RichText::new(size_text(a.size)).weak());
                            }
                            if ui.add_enabled(!busy, egui::Button::new("💾").small()).on_hover_text(tr("Save the file")).clicked() {
                                client.download_file(&ctx, &a.id, &a.name, FileUse::Save);
                            }
                            if a.kind == openecraft_parts::portal::AttachmentKind::Macro
                                && ui.add_enabled(!busy, egui::Button::new(tr("Use macros")).small()).on_hover_text(tr("Adds the macros of the file to the macro library")).clicked()
                            {
                                client.download_file(&ctx, &a.id, &a.name, FileUse::Macros);
                            }
                            if let Some(link) = client.file_link(&a.id) {
                                ui.hyperlink_to("🔗", link).on_hover_text(tr("Open in the browser"));
                            }
                        });
                    }
                }
                ui.add_space(6.0);
                match &b.history {
                    Some((n, h)) if *n == p.number => {
                        ui.strong(tr("History"));
                        for e in h {
                            ui.label(format!("{} · {} · {} · {}", tr_fmt!("Revision {}", e.revision), date_from_unix(e.time), tr(e.change.title()), e.user));
                        }
                    }
                    _ if client.read_only() => {}
                    _ => {
                        if ui.add_enabled(!busy, egui::Button::new(tr("Show history"))).clicked() {
                            client.history(&ctx, &p.number);
                        }
                    }
                }
            });
        });
    });

    // Adding and the update check
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        let marked: Vec<String> = b.marked.iter().cloned().collect();
        if ui.add_enabled(!busy && !marked.is_empty(), egui::Button::new(tr_fmt!("⬇ Add marked parts to the parts database ({})", marked.len()))).clicked() {
            client.fetch(&ctx, &marked);
            b.marked.clear();
        }
        let from_server: Vec<String> = local.iter().map(|p| p.number.clone()).collect();
        if ui
            .add_enabled(!busy && !from_server.is_empty(), egui::Button::new(tr("🔄 Check the parts database for updates")))
            .on_hover_text(tr("Compares the local parts with the parts server: newer versions, phase-out, discontinued and removed parts"))
            .clicked()
        {
            client.versions(&ctx, &from_server);
        }
        if let Some(updates) = &b.updates {
            let applicable = updates.iter().filter(|u| u.kind != openecraft_parts::portal::UpdateKind::Removed).count();
            if updates.is_empty() {
                ui.label(tr("All parts are up to date"));
            } else if ui.add_enabled(applicable > 0, egui::Button::new(tr_fmt!("✔ Apply {} updates", applicable))).clicked() {
                event = Some(BrowserEvent::ApplyUpdates);
            }
        }
    });
    if let Some(updates) = b.updates.as_ref().filter(|u| !u.is_empty()) {
        egui::ScrollArea::vertical().id_salt("portal_updates").max_height(110.0).show(ui, |ui| {
            egui::Grid::new("portal_updates_grid").num_columns(3).striped(true).show(ui, |ui| {
                for u in updates {
                    ui.monospace(&u.number);
                    ui.label(tr(u.kind.title()));
                    ui.label(if u.successor.is_empty() { String::new() } else { tr_fmt!("Successor {}", u.successor) });
                    ui.end_row();
                }
            });
        });
    }

    if let Some(q) = b.take_search() {
        client.search(&ctx, &q);
    }
    event
}

#[cfg(test)]
mod tests;
