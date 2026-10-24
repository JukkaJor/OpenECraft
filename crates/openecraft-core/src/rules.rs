//! Rule libraries of standards and customer rule sets. A rule set tightens the limits
//! of the electrical calculations (voltage drop, disconnection times, minimum
//! cross-sections, load reserve, selectivity, shielding) and adds checks of the form
//! "every item of a scope that meets these conditions must meet this one", e.g.
//! "connections of potential kind PE must be GNYE" or "parts must have a UL listing".
//! The built-in libraries are selected by id; customer rule sets are stored in the
//! project and exchanged as JSON files.

use openecraft_i18n::{tr, tr_fmt};
use crate::electrical::net_kind;
use crate::model::*;
use crate::netlist::{pin_designation, Connection, Netlist};
use crate::potentials::PotentialKind;
use crate::issue::{Issue, Severity};
use std::collections::{BTreeMap, BTreeSet};
pub use crate::model::rules::{Check, Condition, Level, LimitOverrides, Limits, Op, RuleSet, Scope};

impl Default for Limits {
    fn default() -> Self {
        Limits {
            voltage_drop: crate::sizing::VOLTAGE_DROP_LIMIT,
            disconnect_final: 0.4,
            disconnect_distribution: 5.0,
            min_cross_section_power: 0.0,
            min_cross_section_control: 0.0,
            load_reserve: 0.0,
            max_unbalance: 0.0,
            total_selectivity: false,
            shield_drive_cables: true,
            shield_analog_cables: false,
            surge_protection: false,
        }
    }
}

impl Limits {
    /// Takes over the stricter values of a rule set.
    fn tighten(&mut self, o: &LimitOverrides) {
        let lower = |v: &mut f32, o: Option<f32>| {
            if let Some(x) = o.filter(|x| *x > 0.0) {
                *v = if *v > 0.0 { v.min(x) } else { x };
            }
        };
        lower(&mut self.voltage_drop, o.voltage_drop);
        lower(&mut self.disconnect_final, o.disconnect_final);
        lower(&mut self.disconnect_distribution, o.disconnect_distribution);
        lower(&mut self.max_unbalance, o.max_unbalance);
        for (v, x) in [
            (&mut self.min_cross_section_power, o.min_cross_section_power),
            (&mut self.min_cross_section_control, o.min_cross_section_control),
            (&mut self.load_reserve, o.load_reserve),
        ] {
            *v = v.max(x.unwrap_or(0.0));
        }
        for (v, x) in [
            (&mut self.total_selectivity, o.total_selectivity),
            (&mut self.shield_drive_cables, o.shield_drive_cables),
            (&mut self.shield_analog_cables, o.shield_analog_cables),
            (&mut self.surge_protection, o.surge_protection),
        ] {
            *v |= x.unwrap_or(false);
        }
    }

    /// Share of a rating that may be used, 0 … 1.
    pub fn usable(&self) -> f32 {
        (1.0 - self.load_reserve / 100.0).clamp(0.0, 1.0)
    }
}

impl Scope {
    pub const ALL: [Scope; 7] = [Scope::Project, Scope::Page, Scope::Device, Scope::Part, Scope::Cable, Scope::Connection, Scope::Potential];

    pub fn title(self) -> &'static str {
        match self {
            Scope::Project => tr("Project"),
            Scope::Page => tr("Pages"),
            Scope::Device => tr("Devices"),
            Scope::Part => tr("Parts"),
            Scope::Cable => tr("Cables"),
            Scope::Connection => tr("Connections"),
            Scope::Potential => tr("Potentials"),
        }
    }

    /// Fields of the items (besides `tech:<name>` of parts and `prop:<name>` of the project).
    pub fn fields(self) -> &'static [&'static str] {
        match self {
            Scope::Project => &["name", "description", "company", "creator", "date", "revision"],
            Scope::Page => &["name", "description", "function", "location", "kind"],
            Scope::Device => &["tag", "description", "part", "parts", "manufacturer"],
            Scope::Part => &["number", "type_number", "order_number", "manufacturer", "supplier", "description", "group", "certificates", "voltage", "current", "power", "datasheet"],
            Scope::Cable => &["tag", "type", "cores", "cross_section", "length", "part", "shielded", "bus"],
            Scope::Connection => &["potential", "kind", "color", "cross_section", "cable", "number", "from", "to"],
            Scope::Potential => &["name", "kind", "voltage", "color", "cross_section", "description"],
        }
    }
}

impl Op {
    pub const ALL: [Op; 11] = [Op::NotEmpty, Op::Empty, Op::Equals, Op::NotEquals, Op::Contains, Op::NotContains, Op::StartsWith, Op::OneOf, Op::NoneOf, Op::AtLeast, Op::AtMost];

    pub fn title(self) -> &'static str {
        match self {
            Op::NotEmpty => tr("is not empty"),
            Op::Empty => tr("is empty"),
            Op::Equals => tr("equals"),
            Op::NotEquals => tr("does not equal"),
            Op::Contains => tr("contains"),
            Op::NotContains => tr("does not contain"),
            Op::StartsWith => tr("starts with"),
            Op::OneOf => tr("is one of"),
            Op::NoneOf => tr("is none of"),
            Op::AtLeast => tr("is at least"),
            Op::AtMost => tr("is at most"),
        }
    }

    pub fn needs_value(self) -> bool {
        !matches!(self, Op::NotEmpty | Op::Empty)
    }
}

/// First number of a text ("1.5 mm²" → 1.5).
fn number(text: &str) -> Option<f32> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let num: String = text[start..].chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
    num.replace(',', ".").parse().ok()
}

impl Condition {
    pub fn new(field: &str, op: Op, value: &str) -> Condition {
        Condition { field: field.into(), op, value: value.into() }
    }

    pub fn holds(&self, item: &Item) -> bool {
        let v = item.get(&self.field);
        let (lv, want) = (v.to_lowercase(), self.value.trim().to_lowercase());
        let list = || want.split(',').map(str::trim).filter(|s| !s.is_empty()).any(|w| w == lv.trim());
        match self.op {
            Op::NotEmpty => !v.trim().is_empty(),
            Op::Empty => v.trim().is_empty(),
            Op::Equals => lv.trim() == want,
            Op::NotEquals => lv.trim() != want,
            Op::Contains => lv.contains(&want),
            Op::NotContains => !lv.contains(&want),
            Op::StartsWith => lv.trim().starts_with(&want),
            Op::OneOf => list(),
            Op::NoneOf => !list(),
            Op::AtLeast => matches!((number(&v), number(&want)), (Some(a), Some(b)) if a >= b - 1e-6),
            Op::AtMost => matches!((number(&v), number(&want)), (Some(a), Some(b)) if a <= b + 1e-6),
        }
    }

    pub fn text(&self) -> String {
        if self.op.needs_value() {
            format!("{} {} {}", self.field, self.op.title(), self.value)
        } else {
            format!("{} {}", self.field, self.op.title())
        }
    }
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Error, Level::Warning, Level::Info];

    pub fn title(self) -> &'static str {
        match self {
            Level::Error => tr("Error"),
            Level::Warning => tr("Warning"),
            Level::Info => tr("Info"),
        }
    }

    pub fn severity(self) -> Severity {
        match self {
            Level::Error => Severity::Error,
            Level::Warning => Severity::Warning,
            Level::Info => Severity::Info,
        }
    }
}

impl Check {
    fn new(scope: Scope, when: &[(&str, Op, &str)], require: (&str, Op, &str), level: Level, message: &str) -> Check {
        Check {
            scope,
            when: when.iter().map(|w| Condition::new(w.0, w.1, w.2)).collect(),
            require: Condition::new(require.0, require.1, require.2),
            level,
            message: message.into(),
        }
    }
}

/// Built-in rule libraries.
pub fn library() -> Vec<RuleSet> {
    use Op::*;
    let pe = |colors: &str, message: &str| Check::new(Scope::Connection, &[("kind", Equals, "PE"), ("color", NotEmpty, "")], ("color", OneOf, colors), Level::Warning, message);
    let n = |colors: &str, message: &str| Check::new(Scope::Connection, &[("kind", Equals, "N"), ("color", NotEmpty, "")], ("color", OneOf, colors), Level::Warning, message);
    let dc = |colors: &str, message: &str| Check::new(Scope::Connection, &[("kind", OneOf, "DC+, DC-"), ("color", NotEmpty, "")], ("color", OneOf, colors), Level::Info, message);
    vec![
        RuleSet {
            id: "iec60204-1".into(),
            name: "IEC 60204-1".into(),
            description: tr("Electrical equipment of machines: voltage drop, disconnection, cross-sections, conductor colours").into(),
            source: "IEC 60204-1:2016".into(),
            active: true,
            limits: LimitOverrides {
                voltage_drop: Some(5.0),
                disconnect_final: Some(0.4),
                disconnect_distribution: Some(5.0),
                min_cross_section_power: Some(0.75),
                min_cross_section_control: Some(0.2),
                ..Default::default()
            },
            checks: vec![
                pe("GNYE, GN-YE, GN/YE", tr("Protective conductors must be green-yellow (IEC 60204-1, 13.2.2)")),
                n("BU, LBU", tr("Neutral conductors must be light blue (IEC 60204-1, 13.2.3)")),
                dc("BU, DBU", tr("DC control circuits should be blue (IEC 60204-1, 13.2.4)")),
                Check::new(Scope::Device, &[("tag", NotEmpty, "")], ("part", NotEmpty, ""), Level::Info, tr("Devices should have a part (IEC 60204-1, 17.2: parts list)")),
            ],
        },
        RuleSet {
            id: "iec61439-1".into(),
            name: "IEC 61439-1".into(),
            description: tr("Low-voltage switchgear assemblies: rated diversity, temperature rise, documentation").into(),
            source: "IEC 61439-1:2020".into(),
            active: true,
            limits: LimitOverrides { load_reserve: Some(0.0), ..Default::default() },
            checks: vec![
                Check::new(Scope::Part, &[("current", NotEmpty, "")], ("tech:Power loss", NotEmpty, ""), Level::Info, tr("Parts need power-loss data for the temperature-rise verification (IEC 61439-1, 10.10)")),
                Check::new(Scope::Project, &[], ("company", NotEmpty, ""), Level::Warning, tr("The assembly manufacturer must be named (IEC 61439-1, 6.1)")),
            ],
        },
        RuleSet {
            id: "ul508a".into(),
            name: "UL 508A / NFPA 79".into(),
            description: tr("Industrial control panels for North America: listed components, AWG cross-sections, conductor colours").into(),
            source: "UL 508A, NFPA 79:2024".into(),
            active: true,
            limits: LimitOverrides { min_cross_section_power: Some(2.08), min_cross_section_control: Some(0.82), ..Default::default() },
            checks: vec![
                Check::new(Scope::Part, &[("current", NotEmpty, "")], ("certificates", Contains, "UL"), Level::Warning, tr("Components must be UL listed or recognized (UL 508A, 29)")),
                pe("GN, GNYE", tr("Equipment grounding conductors must be green (NFPA 79, 13.2.2)")),
                n("WH, GY", tr("Grounded conductors must be white or grey (NFPA 79, 13.2.3)")),
                dc("BU", tr("DC control circuits should be blue (NFPA 79, 13.2.4)")),
            ],
        },
        RuleSet {
            id: "iso13849-1".into(),
            name: "ISO 13849-1".into(),
            description: tr("Safety-related parts of control systems: reliability data of the safety devices").into(),
            source: "ISO 13849-1:2023".into(),
            active: true,
            limits: LimitOverrides::default(),
            checks: vec![
                Check::new(Scope::Part, &[("description", Contains, "emergency stop")], ("tech:B10d", NotEmpty, ""), Level::Warning, tr("Emergency stop devices need a B10d value (ISO 13849-1, annex C)")),
                Check::new(Scope::Part, &[("description", Contains, "safety relay")], ("tech:PL", NotEmpty, ""), Level::Warning, tr("Safety relays need their performance level (ISO 13849-1, 6.3)")),
            ],
        },
        RuleSet {
            id: "emc".into(),
            name: "EMC".into(),
            description: tr("Electromagnetic compatibility: shielded drive and analogue cables, surge protection").into(),
            source: "IEC 60204-1 4.4.2, IEC 61800-3, IEC 61000-6-2/-4".into(),
            active: true,
            limits: LimitOverrides { shield_drive_cables: Some(true), shield_analog_cables: Some(true), surge_protection: Some(true), ..Default::default() },
            checks: Vec::new(),
        },
    ]
}

/// The rule sets the project is verified against: the selected libraries and the
/// active customer rule sets.
pub fn active(prj: &Project) -> Vec<RuleSet> {
    let mut out: Vec<RuleSet> = library().into_iter().filter(|r| prj.electrical.standards.contains(&r.id)).collect();
    out.extend(prj.electrical.rule_sets.iter().filter(|r| r.active).cloned());
    out
}

/// The limits in effect for the project.
pub fn limits(prj: &Project) -> Limits {
    let mut l = Limits::default();
    for r in active(prj) {
        l.tighten(&r.limits);
    }
    l
}

/// An item a check looks at: its label, where it is drawn and its fields.
#[derive(Clone, Debug, Default)]
pub struct Item {
    pub label: String,
    pub target: Option<(usize, u64)>,
    pub fields: BTreeMap<String, String>,
}

impl Item {
    pub fn get(&self, field: &str) -> String {
        self.fields.get(field).cloned().unwrap_or_default()
    }

    fn set(&mut self, field: &str, value: impl Into<String>) {
        self.fields.insert(field.into(), value.into());
    }
}

/// Code of a potential kind in checks.
pub fn kind_code(k: PotentialKind) -> &'static str {
    match k {
        PotentialKind::L => "L",
        PotentialKind::N => "N",
        PotentialKind::Pe => "PE",
        PotentialKind::DcPlus => "DC+",
        PotentialKind::DcMinus => "DC-",
        PotentialKind::Signal => "Signal",
        PotentialKind::Other => "Other",
    }
}

/// The items of a scope.
pub fn items(prj: &Project, conns: &[Connection], scope: Scope) -> Vec<Item> {
    let mut out = Vec::new();
    match scope {
        Scope::Project => {
            let i = &prj.info;
            let mut it = Item { label: i.name.clone(), ..Default::default() };
            for (k, v) in [("name", &i.name), ("description", &i.description), ("company", &i.company), ("creator", &i.creator), ("date", &i.date), ("revision", &i.revision)] {
                it.set(k, v.clone());
            }
            for (k, v) in &i.properties {
                it.set(&format!("prop:{k}"), v.clone());
            }
            out.push(it);
        }
        Scope::Page => {
            for (pi, pg) in prj.pages.iter().enumerate() {
                let mut it = Item { label: pg.name.clone(), target: Some((pi, 0)), ..Default::default() };
                for (k, v) in [("name", pg.name.clone()), ("description", pg.description.clone()), ("function", pg.function.clone()), ("location", pg.location.clone()), ("kind", pg.kind.label())] {
                    it.set(k, v);
                }
                out.push(it);
            }
        }
        Scope::Device => {
            let mut seen = BTreeSet::new();
            for (pi, pg) in prj.pages.iter().enumerate().filter(|(_, p)| p.kind == PageKind::Schematic) {
                for s in &pg.symbols {
                    if s.tag.is_empty() || !prj.symbol(&s.sym).is_some_and(|d| d.func.has_tag()) || !seen.insert(s.tag.clone()) {
                        continue;
                    }
                    let d = prj.devices.get(&s.tag);
                    let parts: Vec<&str> = d.map(|d| d.parts.iter().map(|p| p.number.as_str()).collect()).unwrap_or_default();
                    let mut it = Item { label: s.tag.clone(), target: Some((pi, s.id)), ..Default::default() };
                    it.set("tag", s.tag.clone());
                    it.set("description", d.map(|d| d.description.clone()).unwrap_or_default());
                    it.set("part", parts.first().copied().unwrap_or(""));
                    it.set("parts", parts.len().to_string());
                    it.set("manufacturer", parts.first().and_then(|n| crate::sizing::part(prj, n)).map(|p| p.manufacturer.clone()).unwrap_or_default());
                    out.push(it);
                }
            }
        }
        Scope::Part => {
            let mut used: BTreeSet<&str> = prj.devices.values().flat_map(|d| d.parts.iter().map(|p| p.number.as_str())).collect();
            used.extend(prj.cables.iter().map(|c| c.part.as_str()).filter(|p| !p.is_empty()));
            for p in used.into_iter().filter_map(|n| crate::sizing::part(prj, n)) {
                let mut it = Item { label: p.number.clone(), ..Default::default() };
                for (k, v) in [
                    ("number", &p.number),
                    ("type_number", &p.type_number),
                    ("order_number", &p.order_number),
                    ("manufacturer", &p.manufacturer),
                    ("supplier", &p.supplier),
                    ("description", &p.description),
                    ("group", &p.group),
                    ("certificates", &p.certificates),
                    ("voltage", &p.voltage),
                    ("current", &p.current),
                    ("power", &p.power),
                    ("datasheet", &p.datasheet),
                ] {
                    it.set(k, v.clone());
                }
                for l in p.tech_data.lines() {
                    if let Some((k, v)) = l.split_once(':') {
                        it.set(&format!("tech:{}", k.trim()), v.trim());
                    }
                }
                out.push(it);
            }
        }
        Scope::Cable => {
            for c in &prj.cables {
                let mut it = Item { label: c.tag.clone(), ..Default::default() };
                it.set("tag", c.tag.clone());
                it.set("type", c.cable_type.clone());
                it.set("cores", c.core_list().len().to_string());
                it.set("cross_section", c.cross_section.clone());
                it.set("length", format!("{}", c.length));
                it.set("part", c.part.clone());
                it.set("shielded", if crate::routing3d::is_shielded(c) { "yes" } else { "no" });
                it.set("bus", c.bus.map(|b| format!("{b:?}")).unwrap_or_default());
                out.push(it);
            }
        }
        Scope::Connection => {
            for c in conns {
                let (from, to) = (pin_designation(prj, c.a), pin_designation(prj, c.b));
                let s = &prj.pages[c.a.page].symbols[c.a.sym];
                let mut it = Item { label: format!("{from} – {to}"), target: Some((c.a.page, s.id)), ..Default::default() };
                it.set("potential", c.net.clone());
                it.set("kind", kind_code(net_kind(prj, &c.net)));
                it.set("color", c.color.clone());
                it.set("cross_section", c.cross_section.clone());
                it.set("cable", c.cable.clone());
                it.set("number", c.number.clone());
                it.set("from", from);
                it.set("to", to);
                out.push(it);
            }
        }
        Scope::Potential => {
            for p in &prj.potentials {
                let mut it = Item { label: p.name.clone(), ..Default::default() };
                for (k, v) in [("name", p.name.clone()), ("kind", kind_code(p.kind).to_string()), ("voltage", p.voltage.clone()), ("color", p.color.clone()), ("cross_section", p.cross_section.clone()), ("description", p.description.clone())] {
                    it.set(k, v);
                }
                out.push(it);
            }
        }
    }
    out
}

/// Result of one check.
#[derive(Clone, Debug)]
pub struct CheckResult {
    pub set: String,
    pub check: Check,
    /// Items the check applies to.
    pub checked: usize,
    /// Items that violate it.
    pub failed: Vec<Item>,
}

/// Verifies the project against the active rule sets.
pub fn verify(prj: &Project, conns: &[Connection]) -> Vec<CheckResult> {
    let mut cache: BTreeMap<u8, Vec<Item>> = BTreeMap::new();
    let mut out = Vec::new();
    for set in active(prj) {
        for check in &set.checks {
            let items = cache.entry(check.scope as u8).or_insert_with(|| items(prj, conns, check.scope));
            let applies: Vec<&Item> = items.iter().filter(|it| check.when.iter().all(|c| c.holds(it))).collect();
            let failed = applies.iter().filter(|it| !check.require.holds(it)).map(|it| (*it).clone()).collect();
            out.push(CheckResult { set: set.name.clone(), check: check.clone(), checked: applies.len(), failed });
        }
    }
    out
}

/// Connections below the smallest cross-section of their circuit (power: mains).
fn cross_section_problems(prj: &Project, nl: &Netlist, conns: &[Connection], l: &Limits) -> Vec<Issue> {
    if l.min_cross_section_power <= 0.0 && l.min_cross_section_control <= 0.0 {
        return Vec::new();
    }
    let mains = crate::electrical::mains(prj, nl);
    let mut out = Vec::new();
    for c in conns {
        let Some(mm2) = crate::production::cross_section_mm2(&c.cross_section) else { continue };
        let s = &prj.pages[c.a.page].symbols[c.a.sym];
        let power = nl.pin_net.get(&(s.id, c.a.pin)).is_some_and(|n| mains.contains_key(n));
        let min = if power { l.min_cross_section_power } else { l.min_cross_section_control };
        if net_kind(prj, &c.net) != PotentialKind::Pe && mm2 + 1e-4 < min {
            let label = format!("{} – {}", pin_designation(prj, c.a), pin_designation(prj, c.b));
            let message = if power {
                tr_fmt!("{}: cross-section {} below the {} mm² of power circuits", label, c.cross_section, min)
            } else {
                tr_fmt!("{}: cross-section {} below the {} mm² of control circuits", label, c.cross_section, min)
            };
            out.push(Issue::plain(Severity::Warning, message, Some((c.a.page, s.id))));
        }
    }
    out
}

/// Check messages of the rule sets.
pub fn problems(prj: &Project, nl: &Netlist, conns: &[Connection]) -> Vec<Issue> {
    let mut out = cross_section_problems(prj, nl, conns, &limits(prj));
    for r in verify(prj, conns) {
        for it in &r.failed {
            out.push(crate::issue!(r.check.level.severity(), it.target.filter(|t| t.1 != 0), "{}: {} ({})", it.label, r.check.message, r.set));
        }
    }
    out
}

/// Customer rule sets from a JSON file: one rule set or a list of them.
pub fn import(text: &str) -> Result<Vec<RuleSet>, String> {
    serde_json::from_str::<Vec<RuleSet>>(text)
        .or_else(|_| serde_json::from_str::<RuleSet>(text).map(|r| vec![r]))
        .map_err(|e| tr_fmt!("Not a valid rule set file: {}", e))
}

pub fn export(sets: &[RuleSet]) -> String {
    serde_json::to_string_pretty(sets).unwrap_or_default()
}

#[cfg(test)]
mod tests;
