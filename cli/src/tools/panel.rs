//! Panel layout tools: enclosure systems and their components, accessories on the
//! plate, the door and the walls, fastening, parametric placement (anchors, patterns,
//! aligning, rails), assemblies and 3D macros.

use super::{Args, Tool};
use crate::session::Session;
use openecraft_core::panel::{self};
use openecraft_i18n::tr_const;
use serde_json::{json, Value};

pub static TOOLS: &[Tool] = &[
    Tool {
        name: "enclosure_systems",
        help: tr_const!("Lists the enclosure systems with their standard sizes, wall thickness, plinth and whether they can be bayed"),
        params: &[],
        needs_project: false,
        run: enclosure_systems,
    },
];

fn enclosure_systems(_: &mut Session, _: &Args) -> Result<Value, String> {
    Ok(Value::Array(
        panel::SYSTEMS
            .iter()
            .map(|s| {
                json!({
                    "system": s.id(),
                    "title": s.title(),
                    "sizes": s.data.sizes.iter().map(|x| [x.0, x.1, x.2]).collect::<Vec<_>>(),
                    "wall_mm": s.data.wall,
                    "flange_mm": s.data.flange,
                    "plinth_mm": s.data.plinth,
                    "bayable": s.data.bayable,
                    "protection": s.data.ip,
                })
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests;
