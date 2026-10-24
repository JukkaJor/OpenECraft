//! Built-in plugins, compiled into every front end. New features that add commands,
//! checks, reports or exports register them here (see AGENTS.md, extensibility).

use crate::registry::{Registry, Table};
use crate::{Info, Plugin, Text};
use openecraft_core::model::Project;
use openecraft_core::reports::{Analysis, Severity};
use openecraft_core::structure;
use openecraft_i18n::tr_const;
use serde_json::Value;

mod manufacturing;
mod panel;
pub use manufacturing::PanelManufacturing;
pub use panel::PanelChecks;

pub fn plugins() -> Vec<Box<dyn Plugin>> {
    vec![Box::new(Statistics), Box::new(PanelChecks), Box::new(PanelManufacturing)]
}

/// Key figures of the project: as command, report and text export of the device tags.
pub struct Statistics;

impl Plugin for Statistics {
    fn info(&self) -> Info {
        Info {
            id: "statistics".into(),
            name: Text::Borrowed(tr_const!("Project statistics")),
            version: env!("CARGO_PKG_VERSION").into(),
            description: Text::Borrowed(tr_const!("Key figures of the project and a list of its device tags")),
        }
    }

    fn register(&self, reg: &mut Registry) -> Result<(), String> {
        reg.command("project_statistics", tr_const!("Key figures of the project: pages, symbols, wires, devices, parts, cables and check results"), vec![], |prj, _| {
            Ok(figures(prj, &Analysis::build(prj)).into_iter().map(|(key, _, n)| (key.to_string(), Value::from(n))).collect::<serde_json::Map<_, _>>().into())
        })
        .report("project_statistics", tr_const!("Project statistics"), |prj, an| {
            let mut table = Table::new([tr_const!("Figure"), tr_const!("Count")]);
            for (_, title, n) in figures(prj, an) {
                table.push(vec![openecraft_i18n::tr(title).to_string(), n.to_string()], None);
            }
            Ok(table)
        })
        .exporter("device_tags", tr_const!("Device tags (text)"), "txt", |prj| Ok(structure::device_tags(prj).into_iter().map(|t| t + "\n").collect()));
        Ok(())
    }
}

/// (JSON key, title, value) of each figure.
fn figures(prj: &Project, an: &Analysis) -> Vec<(&'static str, &'static str, usize)> {
    let count = |s: Severity| an.issues.iter().filter(|i| i.severity == s).count();
    vec![
        ("pages", tr_const!("Pages"), prj.pages.len()),
        ("symbols", tr_const!("Symbols"), prj.pages.iter().map(|p| p.symbols.len()).sum()),
        ("wires", tr_const!("Wires"), prj.pages.iter().map(|p| p.wires.len()).sum()),
        ("devices", tr_const!("Devices"), structure::device_tags(prj).len()),
        ("parts", tr_const!("Parts"), prj.parts.len()),
        ("cables", tr_const!("Cables"), prj.cables.len()),
        ("errors", tr_const!("Errors"), count(Severity::Error)),
        ("warnings", tr_const!("Warnings"), count(Severity::Warning)),
    ]
}

#[cfg(test)]
mod tests;
