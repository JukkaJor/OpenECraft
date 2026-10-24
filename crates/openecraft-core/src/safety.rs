//! Functional safety (ISO 13849-1): safety functions made of subsystems (input, logic,
//! output), each with its devices, category, diagnostic coverage and common cause
//! measures. The performance level of a subsystem follows from the simplified
//! procedure (figure 5: category, DCavg, MTTFd of a channel); the PL of the function
//! from the combination of its subsystems (table 11). Devices with a certified PL or
//! PFHd (`PL: e`, `PFHd: 2.5E-9`) count with that; others with their `MTTFd` (years)
//! or `B10d` (cycles) and the operations per year of the subsystem.

use openecraft_i18n::{tr, tr_fmt};
use crate::model::*;
use crate::reports::{Issue, Severity};
pub use crate::model::safety::{Category, Pl, SafetyFunction, Subsystem};

impl Pl {
    pub const ALL: [Pl; 5] = [Pl::A, Pl::B, Pl::C, Pl::D, Pl::E];

    pub fn title(self) -> &'static str {
        match self {
            Pl::A => "a",
            Pl::B => "b",
            Pl::C => "c",
            Pl::D => "d",
            Pl::E => "e",
        }
    }

    pub fn parse(text: &str) -> Option<Pl> {
        let t = text.trim().trim_start_matches("PL").trim().to_ascii_lowercase();
        Pl::ALL.into_iter().find(|p| t.starts_with(p.title()))
    }

    /// Safety integrity level of IEC 62061 with the same PFHd range.
    pub fn sil(self) -> Option<u8> {
        match self {
            Pl::A => None,
            Pl::B | Pl::C => Some(1),
            Pl::D => Some(2),
            Pl::E => Some(3),
        }
    }

    /// PL of a probability of a dangerous failure per hour (table 2).
    pub fn of_pfh(pfh: f32) -> Option<Pl> {
        Some(match pfh {
            p if p < 1e-7 => Pl::E,
            p if p < 1e-6 => Pl::D,
            p if p < 3e-6 => Pl::C,
            p if p < 1e-5 => Pl::B,
            p if p < 1e-4 => Pl::A,
            _ => return None,
        })
    }

    fn lower(self) -> Option<Pl> {
        Pl::ALL.into_iter().rev().find(|p| *p < self)
    }
}

impl Category {
    pub const ALL: [Category; 5] = [Category::B, Category::One, Category::Two, Category::Three, Category::Four];

    pub fn title(self) -> &'static str {
        match self {
            Category::B => "B",
            Category::One => "1",
            Category::Two => "2",
            Category::Three => "3",
            Category::Four => "4",
        }
    }
}

/// Band of a value: 0 none / too low, 1 low, 2 medium, 3 high.
fn band(v: f32, limits: [f32; 3]) -> usize {
    limits.iter().filter(|l| v >= **l - 1e-6).count()
}

/// Number of a reliability value ("150 a", "2.5E-9", "1 000 000").
fn parse_number(text: &str) -> Option<f32> {
    // Blanks between groups of thousands go.
    let chars: Vec<char> = text.chars().collect();
    let group = |i: usize| chars.len() >= i + 3 && chars[i..i + 3].iter().all(char::is_ascii_digit) && chars.get(i + 3).is_none_or(|c| !c.is_ascii_digit());
    let t: String = chars
        .iter()
        .enumerate()
        .filter(|&(i, c)| !(matches!(c, ' ' | '_') && i > 0 && chars[i - 1].is_ascii_digit() && group(i + 1)))
        .map(|(_, c)| *c)
        .collect();
    let start = t.find(|c: char| c.is_ascii_digit())?;
    let num: String = t[start..].chars().take_while(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '-' | '+')).collect();
    num.trim_end_matches(['e', 'E', '-', '+']).parse().ok()
}

/// Reliability data of a device.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct DeviceData {
    pl: Option<Pl>,
    pfh: Option<f32>,
    mttfd: Option<f32>,
    b10d: Option<f32>,
}

fn device_data(prj: &Project, tag: &str) -> DeviceData {
    let mut d = DeviceData::default();
    for p in crate::sizing::device_parts(prj, tag) {
        d.pl = d.pl.or_else(|| p.tech_value("PL").and_then(Pl::parse));
        d.pfh = d.pfh.or_else(|| p.tech_value("PFHd").and_then(parse_number));
        d.mttfd = d.mttfd.or_else(|| p.tech_value("MTTFd").and_then(parse_number));
        d.b10d = d.b10d.or_else(|| p.tech_value("B10d").and_then(parse_number));
    }
    d
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubsystemResult {
    pub name: String,
    /// MTTFd of a channel (capped), years.
    pub mttfd: Option<f32>,
    pub pl: Option<Pl>,
    pub problems: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    pub function: String,
    pub required: Pl,
    pub achieved: Option<Pl>,
    pub subsystems: Vec<SubsystemResult>,
}

impl Evaluation {
    pub fn is_met(&self) -> bool {
        self.achieved.is_some_and(|a| a >= self.required)
    }
}

/// PL of the simplified procedure (figure 5) by category, DCavg and MTTFd band.
fn figure5(cat: Category, dc: usize, mttfd: usize) -> Option<Pl> {
    use Pl::*;
    let row: [Option<Pl>; 3] = match (cat, dc) {
        (Category::B, _) => [Some(A), Some(B), Some(B)],
        (Category::One, _) => [None, None, Some(C)],
        (Category::Two, 1) => [Some(A), Some(B), Some(C)],
        (Category::Two, _) => [Some(B), Some(C), Some(D)],
        (Category::Three, 1) => [Some(B), Some(C), Some(D)],
        (Category::Three, _) => [Some(C), Some(D), Some(E)],
        (Category::Four, _) => [None, None, Some(E)],
    };
    row[mttfd.checked_sub(1)?]
}

fn evaluate_subsystem(prj: &Project, s: &Subsystem) -> SubsystemResult {
    let mut r = SubsystemResult { name: s.name.clone(), mttfd: None, pl: None, problems: Vec::new() };
    if s.devices.is_empty() {
        r.problems.push(tr_fmt!("Subsystem {} has no devices", s.name));
        return r;
    }
    let mut certified: Option<Pl> = None;
    // Sum of 1 / MTTFd of the devices without certified data.
    let mut inv = 0.0;
    let mut computed = false;
    let mut missing = false;
    for tag in &s.devices {
        if !prj.devices.contains_key(tag) && !prj.pages.iter().any(|p| p.symbols.iter().any(|x| &x.tag == tag)) {
            r.problems.push(tr_fmt!("Device {} does not exist", tag));
            missing = true;
            continue;
        }
        let d = device_data(prj, tag);
        if let Some(pl) = d.pl.or_else(|| d.pfh.and_then(Pl::of_pfh)) {
            certified = Some(certified.map_or(pl, |c| c.min(pl)));
            continue;
        }
        computed = true;
        match (d.mttfd, d.b10d) {
            (Some(m), _) if m > 0.0 => inv += 1.0 / m,
            (_, Some(b)) if s.operations > 0.0 => inv += 0.1 * s.operations / b,
            (_, Some(_)) => {
                r.problems.push(tr_fmt!("{}: operations per year needed for the B10d of {}", s.name, tag));
                missing = true;
            }
            _ if s.mttfd > 0.0 => {}
            _ => {
                r.problems.push(tr_fmt!("{} has no MTTFd, B10d, PL or PFHd", tag));
                missing = true;
            }
        }
    }
    if s.mttfd > 0.0 && computed {
        inv = 1.0 / s.mttfd;
    }
    if missing {
        return r;
    }
    let mut pl = certified;
    if computed {
        let cap = if s.category == Category::Four { 2500.0 } else { 100.0 };
        let mttfd = if inv > 0.0 { (1.0 / inv).min(cap) } else { cap };
        r.mttfd = Some(mttfd);
        let m = band(mttfd, [3.0, 10.0, 30.0]);
        let dc = band(s.dc, [60.0, 90.0, 99.0]);
        let before = r.problems.len();
        if m == 0 {
            r.problems.push(tr_fmt!("{}: MTTFd {} years is below 3 years", s.name, format!("{mttfd:.1}")));
        }
        match s.category {
            Category::One if m < 3 => r.problems.push(tr_fmt!("{}: category 1 needs a high MTTFd", s.name)),
            Category::Two | Category::Three if dc == 0 => r.problems.push(tr_fmt!("{}: category {} needs a DCavg of at least 60 %", s.name, s.category.title())),
            Category::Four if dc < 3 || m < 3 => r.problems.push(tr_fmt!("{}: category 4 needs a high DCavg and MTTFd", s.name)),
            _ => {}
        }
        if s.category >= Category::Two && !s.ccf {
            r.problems.push(tr_fmt!("{}: category {} needs measures against common cause failure", s.name, s.category.title()));
        }
        let figure = if r.problems.len() == before { figure5(s.category, dc, m) } else { None };
        pl = figure.map(|f| pl.map_or(f, |c| c.min(f)));
    }
    r.pl = pl;
    r
}

/// PL of the series of subsystems (table 11).
pub fn combine(pls: &[Pl]) -> Option<Pl> {
    let low = *pls.iter().min()?;
    let n = pls.iter().filter(|p| **p == low).count();
    let max_n = match low {
        Pl::A | Pl::D | Pl::E => 3,
        Pl::B | Pl::C => 2,
    };
    if n > max_n {
        low.lower()
    } else {
        Some(low)
    }
}

pub fn evaluate(prj: &Project, f: &SafetyFunction) -> Evaluation {
    let subsystems: Vec<SubsystemResult> = f.subsystems.iter().map(|s| evaluate_subsystem(prj, s)).collect();
    let pls: Option<Vec<Pl>> = subsystems.iter().map(|s| s.pl).collect();
    let achieved = pls.filter(|p| !p.is_empty()).and_then(|p| combine(&p));
    Evaluation { function: f.name.clone(), required: f.required, achieved, subsystems }
}

/// Check messages of the safety functions.
pub fn problems(prj: &Project) -> Vec<Issue> {
    let mut out = Vec::new();
    for f in &prj.electrical.safety {
        let e = evaluate(prj, f);
        for s in &e.subsystems {
            for p in &s.problems {
                out.push(Issue { severity: Severity::Warning, message: tr_fmt!("Safety function {}: {}", f.name, p), target: None });
            }
        }
        match e.achieved {
            Some(a) if a < f.required => out.push(Issue {
                severity: Severity::Error,
                message: tr_fmt!("Safety function {} reaches PL {}, PL {} is required", f.name, a.title(), f.required.title()),
                target: None,
            }),
            None if f.subsystems.is_empty() => {
                out.push(Issue { severity: Severity::Warning, message: tr_fmt!("Safety function {} has no subsystems", f.name), target: None })
            }
            _ => {}
        }
    }
    out
}

/// Text of a PL with its SIL, e.g. "d (SIL 2)".
pub fn pl_text(pl: Option<Pl>) -> String {
    match pl {
        Some(p) => match p.sil() {
            Some(s) => format!("{} (SIL {s})", p.title()),
            None => p.title().to_string(),
        },
        None => tr("not reached").to_string(),
    }
}

#[cfg(test)]
mod tests;
