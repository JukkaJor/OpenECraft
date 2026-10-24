//! Protective devices: characteristic, rated current, breaking capacity and the
//! time-current curves (IEC 60898-1 circuit breakers, IEC 60947-4-1 motor protection,
//! IEC 60269 fuses) behind the disconnection times, short-circuit withstand and
//! selectivity. The curves are bands (shortest and longest tripping time) fitted to
//! the conventional currents and gates of the standards; device data from the parts
//! (`Characteristic: C`, `Breaking capacity: 10 kA`, `Instantaneous trip: 13 x In`,
//! `Let-through energy: 25000 A²s`) replaces the defaults.

use openecraft_i18n::tr;
use crate::electrical::value_with_unit;
use crate::model::Project;
use crate::parts::{FnKind, Part};
use serde::{Deserialize, Serialize};

/// The device does not trip.
pub const NO_TRIP: f32 = f32::INFINITY;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Characteristic {
    B,
    C,
    D,
    K,
    Z,
    /// Full-range fuse.
    Gg,
    /// Partial-range (motor) fuse.
    Am,
    /// Motor protection switch, trip class 10.
    Motor,
}

impl Characteristic {
    pub fn title(self) -> &'static str {
        match self {
            Characteristic::B => "B",
            Characteristic::C => "C",
            Characteristic::D => "D",
            Characteristic::K => "K",
            Characteristic::Z => "Z",
            Characteristic::Gg => "gG",
            Characteristic::Am => "aM",
            Characteristic::Motor => tr("Motor protection (class 10)"),
        }
    }

    pub fn is_fuse(self) -> bool {
        matches!(self, Characteristic::Gg | Characteristic::Am)
    }

    fn parse(text: &str) -> Option<Characteristic> {
        Some(match text.trim() {
            "B" => Characteristic::B,
            "C" => Characteristic::C,
            "D" => Characteristic::D,
            "K" => Characteristic::K,
            "Z" => Characteristic::Z,
            t if t.eq_ignore_ascii_case("gG") || t.eq_ignore_ascii_case("gL") => Characteristic::Gg,
            t if t.eq_ignore_ascii_case("aM") => Characteristic::Am,
            _ => return None,
        })
    }

    /// Range of the instantaneous trip in multiples of the rated current.
    fn instantaneous(self) -> (f32, f32) {
        match self {
            Characteristic::B => (3.0, 5.0),
            Characteristic::C => (5.0, 10.0),
            Characteristic::D => (10.0, 20.0),
            Characteristic::K => (8.0, 14.0),
            Characteristic::Z => (2.0, 3.0),
            Characteristic::Motor => (11.0, 15.0),
            Characteristic::Gg | Characteristic::Am => (f32::INFINITY, f32::INFINITY),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Protection {
    pub characteristic: Characteristic,
    /// Rated current (setting) in A.
    pub rated: f32,
    /// Rated (ultimate) short-circuit breaking capacity in kA.
    pub breaking: Option<f32>,
    /// Instantaneous trip range in multiples of the rated current.
    pub instantaneous: (f32, f32),
    /// Let-through energy at the breaking capacity in A²s.
    pub let_through: Option<f32>,
}

/// First number of a text ("25000 A²s" → 25000, "2.5e4" → 25000).
fn first_number(text: &str) -> Option<f32> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let num: String = text[start..].chars().take_while(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | 'e' | 'E' | '-' | '+')).collect();
    num.replace(',', ".").trim_end_matches(['e', 'E', '-', '+']).parse().ok()
}

/// Characteristic letter in a type designation: `C16`, `B10`, `MCB-1P-C6`.
fn letter_in(text: &str) -> Option<Characteristic> {
    text.split(|c: char| !c.is_alphanumeric()).find_map(|tok| {
        let mut chars = tok.chars();
        let first = chars.next()?;
        let rest = chars.as_str();
        (!rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())).then(|| Characteristic::parse(&first.to_string())).flatten()
    })
}

impl Protection {
    /// Protection data of a part with a protective function of this kind.
    pub fn of_part(part: &Part, kind: FnKind) -> Option<Protection> {
        let rated = crate::sizing::parse_current(&part.current)?;
        let characteristic = part
            .tech_value("Characteristic")
            .or_else(|| part.tech_value("Utilisation category"))
            .and_then(Characteristic::parse)
            .unwrap_or_else(|| match kind {
                FnKind::MotorProtection => Characteristic::Motor,
                FnKind::Fuse | FnKind::Fuse3 if part.description.contains(" aM") => Characteristic::Am,
                FnKind::Fuse | FnKind::Fuse3 => Characteristic::Gg,
                _ => letter_in(&part.type_number).or_else(|| letter_in(&part.number)).or_else(|| letter_in(&part.description)).unwrap_or(Characteristic::C),
            });
        let breaking = ["Breaking capacity", "Icu", "Icn"].iter().find_map(|k| part.tech_value(k)).and_then(|v| value_with_unit(v, "kA").or_else(|| first_number(v)));
        let instantaneous = match part.tech_value("Instantaneous trip").and_then(first_number) {
            Some(m) if characteristic == Characteristic::Motor => (m * 0.8, m * 1.2),
            Some(m) => (m, m),
            None => characteristic.instantaneous(),
        };
        let let_through = part.tech_value("Let-through energy").and_then(first_number);
        Some(Protection { characteristic, rated, breaking, instantaneous, let_through })
    }

    /// Shortest and longest tripping time at the current `i` in s ([`NO_TRIP`]: the
    /// device does not trip).
    pub fn trip_time(&self, i: f32) -> (f32, f32) {
        let m = i / self.rated.max(0.01);
        let curve = |t: f32, m0: f32, m: f32, cap: f32| if m > m0 { (t * ((2.55 - m0) / (m - m0)).powi(2)).min(cap) } else { NO_TRIP };
        let (lo, hi) = self.instantaneous;
        match self.characteristic {
            Characteristic::Gg => (
                if m > 1.25 { (10.0 * (2.06 / m).powf(4.87)).min(3600.0) } else { NO_TRIP },
                if m > 1.6 { (5.0 * (4.06 / m).powf(4.66)).min(3600.0) } else { NO_TRIP },
            ),
            Characteristic::Am => (
                if m > 4.0 { (10.0 * (4.0 / m).powf(4.87)).min(3600.0) } else { NO_TRIP },
                if m > 6.3 { (5.0 * (8.0 / m).powf(4.66)).min(3600.0) } else { NO_TRIP },
            ),
            Characteristic::Motor => {
                let thermal = |t: f32, m0: f32| if m > m0 { (t * ((7.2 - m0) / (m - m0)).powi(2)).min(7200.0) } else { NO_TRIP };
                (if m >= lo { 0.0 } else { thermal(4.0, 1.05) }, if m >= hi { 0.1 } else { thermal(10.0, 1.2) })
            }
            // 2.55 In: 1 … 60 s by IEC 60898-1; the lower band of real devices is about 5 s.
            _ => (if m >= lo { 0.0 } else { curve(5.0, 1.13, m, 3600.0) }, if m >= hi { 0.1 } else { curve(60.0, 1.45, m, 3600.0) }),
        }
    }

    /// Smallest current that trips the device within `t` seconds at the latest.
    pub fn current_for(&self, t: f32) -> f32 {
        let (mut a, mut b) = (self.rated, self.rated * 1000.0);
        if self.trip_time(b).1 > t {
            return f32::INFINITY;
        }
        for _ in 0..60 {
            let m = (a * b).sqrt();
            if self.trip_time(m).1 <= t {
                b = m;
            } else {
                a = m;
            }
        }
        b
    }

    /// Energy let through at the current `i` in A²s; `None` in the instantaneous range
    /// of a circuit breaker without data from the manufacturer.
    pub fn energy(&self, i: f32) -> Option<f32> {
        let t = self.trip_time(i).1;
        if t == NO_TRIP {
            return None;
        }
        if self.characteristic.is_fuse() {
            return Some(i * i * t.max(0.004));
        }
        if t >= 0.1 && i < self.instantaneous.1 * self.rated {
            Some(i * i * t)
        } else {
            self.let_through
        }
    }
}

/// Protection data of a device: its part with a protective function.
pub fn of_device(prj: &Project, tag: &str) -> Option<Protection> {
    crate::sizing::device_parts(prj, tag).into_iter().find_map(|p| {
        let f = p.functions.iter().find(|f| crate::sizing::PROTECTIVE.contains(&f.kind))?;
        Protection::of_part(p, f.kind)
    })
}

#[cfg(test)]
mod tests;
