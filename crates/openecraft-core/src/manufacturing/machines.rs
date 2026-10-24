//! CNC machines of the panel production: drilling and milling centres, punching
//! machines and busbar machines, each with its working area, tools, controller dialect
//! and the figures of the time estimate.

use crate::model::Project;
use openecraft_i18n::{tr, tr_fmt};
pub use crate::model::manufacturing::{Dialect, Machine, MachineKind, Tool, ToolKind};

pub const DEFAULT_SHEET: &str = "drill_mill_iso";
pub const DEFAULT_BAR: &str = "busbar_cnc";

impl MachineKind {
    pub const ALL: [MachineKind; 3] = [MachineKind::Machining, MachineKind::Punching, MachineKind::Busbar];

    pub fn title(self) -> &'static str {
        match self {
            MachineKind::Machining => tr("Drilling and milling centre"),
            MachineKind::Punching => tr("Punching machine"),
            MachineKind::Busbar => tr("Busbar machine"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            MachineKind::Machining => "machining",
            MachineKind::Punching => "punching",
            MachineKind::Busbar => "busbar",
        }
    }

    pub fn from_key(s: &str) -> Option<MachineKind> {
        MachineKind::ALL.into_iter().find(|k| k.key() == s.trim())
    }
}

impl Dialect {
    pub const ALL: [Dialect; 4] = [Dialect::Iso, Dialect::Heidenhain, Dialect::Punch, Dialect::Csv];

    pub fn key(self) -> &'static str {
        match self {
            Dialect::Iso => "iso",
            Dialect::Heidenhain => "heidenhain",
            Dialect::Punch => "punch",
            Dialect::Csv => "csv",
        }
    }

    pub fn from_key(s: &str) -> Option<Dialect> {
        Dialect::ALL.into_iter().find(|d| d.key() == s.trim())
    }

    /// File extension of the programs.
    pub fn extension(self) -> &'static str {
        match self {
            Dialect::Iso => "nc",
            Dialect::Heidenhain => "h",
            Dialect::Punch => "pun",
            Dialect::Csv => "csv",
        }
    }
}

impl ToolKind {
    pub fn title(self) -> &'static str {
        match self {
            ToolKind::Drill => tr("Drill"),
            ToolKind::Tap => tr("Tap"),
            ToolKind::Mill => tr("End mill"),
            ToolKind::PunchRound => tr("Round punch"),
            ToolKind::PunchRect => tr("Rectangular punch"),
        }
    }

    /// Letter of the tool lists (`D4.2`, `M5`, `F10`, `R22.5`, `Q92x92`).
    fn letter(self) -> char {
        match self {
            ToolKind::Drill => 'D',
            ToolKind::Tap => 'M',
            ToolKind::Mill => 'F',
            ToolKind::PunchRound => 'R',
            ToolKind::PunchRect => 'Q',
        }
    }
}

impl Tool {
    /// Short name: `D4.2`, `M5`, `F10`, `R22.5`, `Q92x92`.
    pub fn name(&self) -> String {
        let n = super::num;
        match self.kind {
            ToolKind::PunchRect => format!("Q{}x{}", n(self.diameter), n(self.height)),
            k => format!("{}{}", k.letter(), n(self.diameter)),
        }
    }

    /// A rectangular punch fits a w × h opening (either way round).
    pub fn fits_rect(&self, w: f32, h: f32) -> bool {
        let close = |a: f32, b: f32| (a - b).abs() < 0.05;
        self.kind == ToolKind::PunchRect && ((close(self.diameter, w) && close(self.height, h)) || (close(self.diameter, h) && close(self.height, w)))
    }
}

/// Tool list of the form `D4.2 T1; M5 T21; F10 T30; R22.5 T101; Q92x92 T120` (tool
/// numbers optional, counted from 1).
pub fn parse_tools(text: &str) -> Result<Vec<Tool>, String> {
    let mut v: Vec<Tool> = Vec::new();
    for entry in text.split([';', '\n']).map(str::trim).filter(|s| !s.is_empty()) {
        let mut words = entry.split_whitespace();
        let spec = words.next().unwrap_or_default();
        let number = match words.next() {
            Some(t) => t.trim_start_matches(['T', 't']).parse().map_err(|_| tr_fmt!("Unknown tool {}", entry))?,
            None => v.iter().map(|t| t.number).max().unwrap_or(0) + 1,
        };
        let (letter, size) = spec.split_at(spec.chars().next().map(char::len_utf8).unwrap_or(0));
        let parse = |s: &str| s.replace(',', ".").parse::<f32>().ok().filter(|v| *v > 0.0);
        let kind = match letter.to_ascii_uppercase().as_str() {
            "D" => ToolKind::Drill,
            "M" => ToolKind::Tap,
            "F" => ToolKind::Mill,
            "R" => ToolKind::PunchRound,
            "Q" => ToolKind::PunchRect,
            _ => return Err(tr_fmt!("Unknown tool {}", entry)),
        };
        let (diameter, height) = match kind {
            ToolKind::PunchRect => {
                let (w, h) = size.split_once(['x', 'X']).unwrap_or((size, size));
                (parse(w), parse(h))
            }
            _ => (parse(size), Some(0.0)),
        };
        let (Some(diameter), Some(height)) = (diameter, height) else { return Err(tr_fmt!("Unknown tool {}", entry)) };
        v.push(Tool { number, kind, diameter, height });
    }
    Ok(v)
}

/// Tool list as text (the inverse of [`parse_tools`]).
pub fn tools_text(tools: &[Tool]) -> String {
    tools.iter().map(|t| format!("{} T{}", t.name(), t.number)).collect::<Vec<_>>().join("; ")
}

impl Default for Machine {
    fn default() -> Self {
        Machine {
            id: String::new(),
            name: String::new(),
            kind: MachineKind::Machining,
            dialect: Dialect::Iso,
            area: (2000.0, 1500.0),
            max_thickness: 4.0,
            tools: Vec::new(),
            rapid: 20000.0,
            feed: 600.0,
            hit_time: 2.0,
            tool_change: 8.0,
            time_factor: 1.0,
        }
    }
}

impl Machine {
    /// The tool of a kind whose size is closest to `size` within `tolerance`.
    pub fn tool(&self, kind: ToolKind, size: f32, tolerance: f32) -> Option<&Tool> {
        self.tools
            .iter()
            .filter(|t| t.kind == kind && (t.diameter - size).abs() <= tolerance + 1e-4)
            .min_by(|a, b| (a.diameter - size).abs().total_cmp(&(b.diameter - size).abs()))
    }

    /// The largest tool of a kind not larger than `max`.
    pub fn largest(&self, kind: ToolKind, max: f32) -> Option<&Tool> {
        self.tools.iter().filter(|t| t.kind == kind && t.diameter <= max + 1e-4 && (kind != ToolKind::PunchRect || t.height <= max + 1e-4)).max_by(|a, b| a.diameter.total_cmp(&b.diameter))
    }

    pub fn tool_by_number(&self, number: u32) -> Option<&Tool> {
        self.tools.iter().find(|t| t.number == number)
    }
}

fn tools(list: &str) -> Vec<Tool> {
    parse_tools(list).unwrap_or_default()
}

const DRILLS: &str = "D2.5 T1; D3.3 T2; D3.4 T3; D4.2 T4; D4.5 T5; D5 T6; D5.5 T7; D6 T8; D6.6 T9; D6.8 T10; D7 T11; D8.5 T12; D9 T13; D11 T14; M3 T21; M4 T22; M5 T23; M6 T24; M8 T25; F6 T31; F10 T32";

/// The machines every project knows.
pub fn builtin() -> Vec<Machine> {
    vec![
        Machine { id: DEFAULT_SHEET.into(), name: "Drilling / milling centre (ISO)".into(), tools: tools(DRILLS), ..Default::default() },
        Machine { id: "drill_mill_heidenhain".into(), name: "Drilling / milling centre (Heidenhain)".into(), dialect: Dialect::Heidenhain, tools: tools(DRILLS), ..Default::default() },
        Machine {
            id: "punch_turret".into(),
            name: "Turret punch press".into(),
            kind: MachineKind::Punching,
            dialect: Dialect::Punch,
            area: (2500.0, 1250.0),
            max_thickness: 3.0,
            tools: tools("R3.3 T101; R3.4 T102; R4.2 T103; R4.5 T104; R5 T105; R5.5 T106; R6 T107; R6.6 T108; R6.8 T109; R7 T110; R9 T111; R11 T112; R12.5 T113; R16.2 T114; R20.5 T115; R22.5 T116; R25.5 T117; R32.5 T118; Q5x20 T130; Q10x10 T131; Q30x30 T132; Q92x92 T133; Q45x45 T134; M4 T141; M5 T142; M6 T143"),
            rapid: 60000.0,
            hit_time: 0.6,
            tool_change: 4.0,
            ..Default::default()
        },
        Machine {
            id: DEFAULT_BAR.into(),
            name: "Busbar machine (cut, punch, bend)".into(),
            kind: MachineKind::Busbar,
            dialect: Dialect::Csv,
            area: (6000.0, 120.0),
            max_thickness: 15.0,
            tools: tools("R6.6 T1; R9 T2; R11 T3; R13.5 T4; Q8.5x14 T5"),
            hit_time: 6.0,
            tool_change: 10.0,
            ..Default::default()
        },
    ]
}

/// Built-in and project machines; project machines replace built-in ones of the same id.
pub fn all(prj: &Project) -> Vec<Machine> {
    let own = &prj.layout.manufacturing.machines;
    let mut v: Vec<Machine> = builtin().into_iter().filter(|b| !own.iter().any(|m| m.id == b.id)).collect();
    v.extend(own.iter().cloned());
    v
}

pub fn find(prj: &Project, id: &str) -> Option<Machine> {
    all(prj).into_iter().find(|m| m.id == id)
}

/// The machine of an id, or an error naming it.
pub fn get(prj: &Project, id: &str) -> Result<Machine, String> {
    find(prj, id).ok_or_else(|| tr_fmt!("No machine {}", id))
}

#[cfg(test)]
mod tests;
