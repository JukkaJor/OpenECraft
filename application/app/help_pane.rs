//! The Help pane (F1): the quick guide, the tutorial (`docs/tutorial.md`, rendered),
//! the keyboard shortcuts and the version. F1 or the pane menu › Help opens the
//! tutorial chapter of the pane in use.

use super::*;
use crate::shortcuts;

const TUTORIAL: &str = include_str!("../../docs/tutorial.md");
const PRODUCT: &str = "OpenECraft";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Section {
    Guide,
    /// Chapter of the tutorial (index of its `##` heading; 0 is the introduction).
    Tutorial(usize),
    Shortcuts,
    About,
}

pub(super) struct HelpUi {
    section: Section,
    scroll_top: bool,
}

impl Default for HelpUi {
    fn default() -> Self {
        HelpUi { section: Section::Guide, scroll_top: false }
    }
}

/// The chapters of the tutorial: heading and lines.
fn chapters() -> Vec<(String, Vec<&'static str>)> {
    let mut out: Vec<(String, Vec<&'static str>)> = vec![(String::new(), Vec::new())];
    for line in TUTORIAL.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            out.push((h.trim().to_string(), Vec::new()));
        } else if let Some(h) = line.strip_prefix("# ") {
            out[0].0 = h.trim().to_string();
        } else if let Some(last) = out.last_mut() {
            last.1.push(line);
        }
    }
    out
}

/// The tutorial chapter of a pane (by the number in its heading).
fn chapter_of(pane: Pane) -> usize {
    use ToolPane as T;
    match pane {
        Pane::Navigator(LeftTab::Pages) => 2,
        Pane::Page | Pane::Library | Pane::Navigator(LeftTab::Devices | LeftTab::Unplaced) | Pane::Properties => 3,
        Pane::Navigator(LeftTab::Connections | LeftTab::Cables) | Pane::Tool(T::Potentials | T::Cables) => 4,
        Pane::Tool(T::Parts | T::DataPortal) => 5,
        Pane::Navigator(LeftTab::Strips) | Pane::Tool(T::TerminalStrips | T::Plugs) => 6,
        Pane::Navigator(LeftTab::Plc) | Pane::Tool(T::PlcOverview | T::PlcHardware) => 7,
        Pane::Navigator(LeftTab::Check) => 8,
        Pane::Tool(T::Reports | T::ReportBlocks | T::Tables | T::Forms) => 9,
        Pane::LayoutSpaces | Pane::LayoutView | Pane::LayoutProperties | Pane::WiringChecklist | Pane::WiringWorkbench => 10,
        Pane::Navigator(LeftTab::Review) | Pane::Tool(T::Revisions | T::Compare | T::ChangeRequests) => 12,
        Pane::Tool(T::Plugins) => 13,
        _ => 1,
    }
}

impl App {
    /// Opens the help at the chapter of a pane.
    pub(super) fn help_for(&mut self, pane: Pane) {
        self.help_chapter(chapter_of(pane));
    }

    /// Opens a chapter of the tutorial in the Help pane.
    pub(super) fn help_chapter(&mut self, chapter: usize) {
        self.help.section = Section::Tutorial(chapter);
        self.help.scroll_top = true;
        self.show_pane(Pane::Tool(ToolPane::Help));
    }

    pub(super) fn help_pane(&mut self, ui: &mut egui::Ui) {
        let chapters = chapters();
        egui::Panel::left("help_sections").resizable(true).default_size(220.0).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("help_sections").auto_shrink([false, false]).show(ui, |ui| {
                for (s, title) in [(Section::Guide, tr("Quick guide")), (Section::Shortcuts, tr("Keyboard shortcuts")), (Section::About, tr("About OpenECraft"))] {
                    if ui.selectable_label(self.help.section == s, title).clicked() {
                        self.help.section = s;
                        self.help.scroll_top = true;
                    }
                }
                ui.separator();
                ui.label(RichText::new(tr("Tutorial")).strong());
                for (i, (title, _)) in chapters.iter().enumerate() {
                    let title = if i == 0 { tr("Introduction").to_string() } else { title.clone() };
                    if ui.selectable_label(self.help.section == Section::Tutorial(i), title).clicked() {
                        self.help.section = Section::Tutorial(i);
                        self.help.scroll_top = true;
                    }
                }
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let mut area = egui::ScrollArea::vertical().id_salt("help_text").auto_shrink([false, false]);
            if std::mem::take(&mut self.help.scroll_top) {
                area = area.vertical_scroll_offset(0.0);
            }
            area.show(ui, |ui| {
                ui.set_max_width(760.0);
                match self.help.section {
                    Section::Guide => self.quick_guide(ui),
                    Section::Tutorial(i) => {
                        if let Some((title, lines)) = chapters.get(i) {
                            ui.heading(title);
                            markdown(ui, lines);
                        }
                    }
                    Section::Shortcuts => self.shortcut_list(ui),
                    Section::About => {
                        ui.heading(PRODUCT);
                        ui.label(tr_fmt!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.label(tr("An open electrical CAE editor in the spirit of EPLAN Electric P8."));
                        ui.hyperlink("https://github.com/nilres/OpenECraft");
                    }
                }
            });
        });
    }

    fn quick_guide(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(tr("Workflow")).strong());
        for p in [
            tr("The ribbon: File opens, saves, imports, exports and prints; the tabs hold the commands by topic. Alt shows the key tips, Ctrl+Shift+P finds any command, F6 moves between the panes, F1 opens this help."),
            tr("1. Insert › Insert center (Ins): search a symbol, a macro or a device and click into the page to place it. Device tags (-K1, -S2 …) are assigned automatically."),
            tr("2. Select a coil, then place contacts: they are assigned to that coil and numbered (13/14, 21/22 …). Cross references and the contact image update live."),
            tr("3. Draw wires with the wire tool (W). Wires end automatically on connection points. T-junctions get a connection dot."),
            tr("4. Potentials (L1, +24V …) and interruption points name the nets across pages. Select a wire to highlight its potential."),
            tr("5. Assign parts in the properties (Parts tab); the reports (Home › Reports) list devices, bill of materials, terminals, connections and PLC and are inserted as report pages."),
            tr("6. Structure: give pages a function (=A1) and location (+CAB1) in the page properties. Devices placed there get it (=A1+CAB1-K1); on the page only the differing part is shown."),
            tr("7. Cables: Connections › Cables, select wires and use Assign to cable. Wire numbers: Connections › Wire numbering, or type a number in the wire properties."),
            tr("8. Macros: select items, right click › Save selection as macro; insert them from the Insert center or Insert › Macro. Placeholders {NAME} are filled from value sets."),
            tr("9. Devices › Terminal strips, Devices › PLC overview and Project data › Revisions: strip order, jumpers and spares; PLC addresses and symbol tables; revisions with change marks and released (locked) pages."),
            tr("10. Master data › Symbol editor: draw your own symbols (or copy built-in ones) with connection points and function; they are saved in the project and can be exchanged as library files."),
            tr("11. Master data › Forms: own title blocks with placeholders ({project}, {page}, {prop:Name} …) and the columns of report pages; Project data › Report blocks generate report pages per strip, plug, cable or device."),
            tr("12. Panel layout (workspace Panel layout, tab Layout space): enclosures, rails, ducts, busbars and cutouts; Layout space › NC data exports the drilling pattern (DXF / DWG / CSV). The production wire list contains stripping lengths and ferrules."),
            tr("13. Harnesses: Connections › Harnesses; select the wires between two devices › Assign to harness, or per connection in the wire properties. Harness data report with lengths, connectors and bundle diameter, KBL export."),
            tr("14. Door: switch the panel front view to Door and place command / signal devices there; their door cutouts are generated. Collisions are checked with the parts' 3D models."),
            tr("15. Connection definition points on a wire set number, colour, cross-section and core of the whole connection; a cable definition line across wires makes them cores of a cable. Auto-connect joins facing connection points, Route (A) leads wires around symbols."),
            tr("16. Graphics: circles, ellipses, arcs, polygons, dimensions and images on layers (navigator › Layers: visible, locked, printed). Drag the tag of a selected symbol to place it; further properties are placed in its properties."),
        ] {
            ui.label(p);
            ui.add_space(4.0);
        }
    }

    fn shortcut_list(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("help_keys").striped(true).show(ui, |ui| {
            let sc = &self.editor.shortcuts;
            let configured: Vec<(String, &str)> = shortcuts::binding_ids().into_iter().map(|id| (sc.text(id), tr(shortcuts::title(id)))).filter(|(k, _)| !k.is_empty()).collect();
            let fixed: Vec<(String, &str)> = [
                (tr("Esc"), tr("Cancel / select tool / clear selection")),
                (tr("Del"), tr("Delete selection")),
                (tr("Arrows"), tr("Move selection by one grid step")),
                (tr("Ctrl+C / X / V"), tr("Copy / cut / paste (devices get new tags)")),
                (tr("Wheel / right or middle drag"), tr("Zoom / pan")),
                (tr("Shift + click / drag"), tr("Add to selection")),
                ("Alt / F10", tr("Key tips of the ribbon")),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
            for (k, v) in configured.into_iter().chain(fixed) {
                ui.label(RichText::new(k).monospace());
                ui.label(v);
                ui.end_row();
            }
        });
        ui.add_space(8.0);
        if ui.button(tr("Keyboard shortcuts…")).clicked() {
            self.settings.open_at(settings_ui::Node::Shortcuts);
        }
    }
}

/// Renders the Markdown of the tutorial: headings, lists, paragraphs, code blocks,
/// `code` and *emphasis*.
fn markdown(ui: &mut egui::Ui, lines: &[&str]) {
    let mut para = String::new();
    let mut code = false;
    let flush = |ui: &mut egui::Ui, para: &mut String| {
        if !para.trim().is_empty() {
            ui.label(inline(ui, para.trim()));
            ui.add_space(4.0);
        }
        para.clear();
    };
    for line in lines {
        let t = line.trim_end();
        if t.trim_start().starts_with("```") {
            flush(ui, &mut para);
            code = !code;
            continue;
        }
        if code {
            ui.monospace(t);
            continue;
        }
        if let Some(h) = t.strip_prefix("### ") {
            flush(ui, &mut para);
            ui.label(RichText::new(h).strong());
        } else if t.trim_start().starts_with("- ") || starts_numbered(t) {
            flush(ui, &mut para);
            para.push_str(t.trim_start());
        } else if t.is_empty() {
            flush(ui, &mut para);
        } else {
            if !para.is_empty() {
                para.push(' ');
            }
            para.push_str(t.trim());
        }
    }
    flush(ui, &mut para);
}

fn starts_numbered(t: &str) -> bool {
    let t = t.trim_start();
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    digits > 0 && t[digits..].starts_with(". ")
}

/// A line with `code` (monospace) and *emphasis* / **strong** (italics, strong).
fn inline(ui: &egui::Ui, s: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let body = egui::TextStyle::Body.resolve(ui.style());
    let mono = egui::TextStyle::Monospace.resolve(ui.style());
    let color = ui.visuals().text_color();
    let strong = ui.visuals().strong_text_color();
    let text = s.strip_prefix("- ").map(|r| format!("•  {r}")).unwrap_or_else(|| s.to_string());
    let mut code = false;
    for (i, part) in text.split('`').enumerate() {
        if i > 0 {
            code = !code;
        }
        if code {
            job.append(part, 0.0, egui::TextFormat::simple(mono.clone(), color));
            continue;
        }
        for (j, seg) in part.split('*').enumerate() {
            let fmt = if j % 2 == 1 { egui::TextFormat { italics: true, color: strong, ..egui::TextFormat::simple(body.clone(), strong) } } else { egui::TextFormat::simple(body.clone(), color) };
            job.append(seg, 0.0, fmt);
        }
    }
    job.wrap.max_width = ui.available_width();
    job
}
