//! Connector faces: where the pins of a plug connector sit, after the `Pin layout` of
//! its technical data (circular, one row, two rows, grid, D-sub or free positions),
//! seen on the mating face of the male side (the female side is its mirror image).

use crate::Part;

/// Arrangement of the pins on the mating face.
#[derive(Clone, Debug, PartialEq)]
pub enum Layout {
    /// `circle 4`, `circle 7+1`, `circle 9+3`, `circle 11+5+1`: pins on a circle, pin 1
    /// right of the coding key at the top and counting counter-clockwise on the male face
    /// (IEC 61076-2-101: `2 1 / 3 4`), then an inner ring (more than two pins) and the
    /// centre pins (one or two) (M8, M12, M16, M23, GX aviation connectors).
    Circle { ring: usize, inner: usize, centre: usize },
    /// `row 8`: one row, pin 1 on the left (pluggable terminal blocks, RJ45).
    Row(usize),
    /// `zigzag 10`: two rows, odd pins in the upper row and even pins below, so that
    /// core n of a ribbon cable lands on pin n (IDC sockets, box headers).
    Zigzag(usize),
    /// `grid 2x6`: rows × columns, numbered row by row (Micro-Fit, heavy duty inserts).
    Grid { rows: usize, cols: usize },
    /// `cols 20x2`: rows × columns, numbered column by column (PLC front connectors).
    Cols { rows: usize, cols: usize },
    /// `dsub 9`: D-sub, the upper row one pin longer, offset by half a pitch.
    DSub(usize),
    /// `points -3.5,2 3.5,2 0,-2` (rectangular face) or `round 0,9 -8,-3 8,-3`
    /// (circular face with coding key): the centre of each pin in mm, x right, y down
    /// (appliance couplers, CEE plugs, valve connectors, motor connectors).
    Points { pts: Vec<[f32; 2]>, round: bool },
}

/// Pin layout with an optional protective earth contact beside the pins (`grid 2x8+PE`).
#[derive(Clone, Debug, PartialEq)]
pub struct PinLayout {
    pub layout: Layout,
    pub pe: bool,
}

impl Layout {
    /// Number of pin positions.
    pub fn slots(&self) -> usize {
        match self {
            Layout::Circle { ring, inner, centre } => ring + inner + centre,
            Layout::Row(n) | Layout::Zigzag(n) | Layout::DSub(n) => *n,
            Layout::Grid { rows, cols } | Layout::Cols { rows, cols } => rows * cols,
            Layout::Points { pts, .. } => pts.len(),
        }
    }

    /// One row up to 12 pins, otherwise two rows: for plugs without a pin layout.
    pub fn fallback(n: usize) -> Layout {
        if n <= 12 { Layout::Row(n.max(1)) } else { Layout::Grid { rows: 2, cols: n.div_ceil(2) } }
    }

    /// Default distance of neighbouring pins, mm.
    pub fn default_pitch(&self) -> f32 {
        match self {
            Layout::Circle { .. } => 2.6,
            Layout::Row(_) => 5.08,
            Layout::Zigzag(_) => 2.54,
            Layout::Grid { .. } | Layout::Cols { .. } => 4.2,
            Layout::DSub(_) => 2.77,
            // The smallest distance of two pins.
            Layout::Points { pts, .. } => pts
                .iter()
                .enumerate()
                .flat_map(|(i, a)| pts[i + 1..].iter().map(move |b| (a[0] - b[0]).hypot(a[1] - b[1])))
                .fold(10.0, f32::min)
                .max(1.0),
        }
    }
}

impl PinLayout {
    /// Parses `circle 7+1`, `circle 11+5+1`, `row 8`, `zigzag 10`, `grid 2x8+PE`,
    /// `cols 20x2`, `dsub 9`, `points -3.5,2 3.5,2 0,-2`, `round 0,9 -8,-3 8,-3`.
    pub fn parse(s: &str) -> Option<PinLayout> {
        let s = s.trim().to_ascii_lowercase();
        let (kind, arg) = s.split_once(char::is_whitespace)?;
        if kind == "points" || kind == "round" {
            let pts: Option<Vec<[f32; 2]>> = arg
                .split_whitespace()
                .map(|p| {
                    let (x, y) = p.split_once(',')?;
                    let v = |t: &str| t.parse::<f32>().ok().filter(|v| v.is_finite());
                    Some([v(x)?, v(y)?])
                })
                .collect();
            return pts.filter(|v| !v.is_empty() && v.len() <= 200).map(|pts| PinLayout { layout: Layout::Points { pts, round: kind == "round" }, pe: false });
        }
        let arg: String = arg.chars().filter(|c| !c.is_whitespace()).collect();
        let (arg, pe) = match arg.strip_suffix("+pe") {
            Some(rest) => (rest, true),
            None => (arg.as_str(), false),
        };
        let n = |t: &str| t.trim().parse::<usize>().ok().filter(|n| (1..=200).contains(n));
        let layout = match kind {
            "circle" => {
                let v: Option<Vec<usize>> = arg.split('+').map(n).collect();
                match *v?.as_slice() {
                    [ring] => Layout::Circle { ring, inner: 0, centre: 0 },
                    [ring, centre] if centre <= 2 => Layout::Circle { ring, inner: 0, centre },
                    [ring, inner] => Layout::Circle { ring, inner, centre: 0 },
                    [ring, inner, centre] if inner > 2 && centre <= 2 => Layout::Circle { ring, inner, centre },
                    _ => return None,
                }
            }
            "row" => Layout::Row(n(arg)?),
            "zigzag" => Layout::Zigzag(n(arg)?),
            "dsub" => Layout::DSub(n(arg)?.max(3)),
            "grid" | "cols" => {
                let (r, c) = arg.split_once('x')?;
                let (rows, cols) = (n(r)?, n(c)?);
                if kind == "grid" { Layout::Grid { rows, cols } } else { Layout::Cols { rows, cols } }
            }
            _ => return None,
        };
        Some(PinLayout { layout, pe })
    }

    /// The `Pin layout` of the technical data of a part.
    pub fn of_part(part: &Part) -> Option<PinLayout> {
        part.tech_value("Pin layout").and_then(PinLayout::parse)
    }

    /// Text of the layout as [`PinLayout::parse`] reads it.
    pub fn text(&self) -> String {
        let s = match &self.layout {
            Layout::Circle { ring, inner, centre } => {
                let parts: Vec<String> = [*ring, *inner, *centre].iter().enumerate().filter(|(i, n)| *i == 0 || **n > 0).map(|(_, n)| n.to_string()).collect();
                format!("circle {}", parts.join("+"))
            }
            Layout::Row(n) => format!("row {n}"),
            Layout::Zigzag(n) => format!("zigzag {n}"),
            Layout::Grid { rows, cols } => format!("grid {rows}x{cols}"),
            Layout::Cols { rows, cols } => format!("cols {rows}x{cols}"),
            Layout::DSub(n) => format!("dsub {n}"),
            Layout::Points { pts, round } => {
                let v: Vec<String> = pts.iter().map(|[x, y]| format!("{x},{y}")).collect();
                format!("{} {}", if *round { "round" } else { "points" }, v.join(" "))
            }
        };
        if self.pe { s + "+PE" } else { s }
    }

    /// Number of pin positions with the protective earth contact.
    pub fn slots(&self) -> usize {
        self.layout.slots() + usize::from(self.pe)
    }
}

/// Outline of the mating face around its centre, mm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outline {
    Circle { r: f32 },
    Rect { w: f32, h: f32 },
    /// D-sub shell: wider at the top.
    DSub { top: f32, bottom: f32, h: f32 },
}

impl Outline {
    /// Width and height.
    pub fn size(self) -> (f32, f32) {
        match self {
            Outline::Circle { r } => (2.0 * r, 2.0 * r),
            Outline::Rect { w, h } => (w, h),
            Outline::DSub { top, h, .. } => (top, h),
        }
    }
}

/// Geometry of a mating face: centres of the pin positions relative to the centre
/// of the face (mm, x right, y down), seen on the male side.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub outline: Outline,
    pub slots: Vec<[f32; 2]>,
    /// Radius of a pin.
    pub pin_r: f32,
    /// Centre of the coding key or polarisation notch on the outline.
    pub key: Option<[f32; 2]>,
}

/// Distance of neighbouring pins in the technical data (`Pitch: 2.54 mm`).
pub fn pitch_of(part: &Part) -> Option<f32> {
    let v = part.tech_value("Pitch")?;
    let num: String = v.chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
    num.replace(',', ".").parse::<f32>().ok().filter(|p| *p > 0.0)
}

/// Face of a pin layout; `pitch` 0 takes the default of the layout.
pub fn face(pl: &PinLayout, pitch: f32) -> Face {
    let pitch = if pitch > 0.0 { pitch } else { pl.layout.default_pitch() };
    let pin_r = pitch * 0.28;
    let centred = |i: usize, n: usize| (i as f32 - (n as f32 - 1.0) / 2.0) * pitch;
    // `n` pins on a circle of radius `r`, the first right of the top, counter-clockwise.
    let ring_at = |n: usize, r: f32| -> Vec<[f32; 2]> {
        (0..n)
            .map(|i| {
                let a = (-90.0 + (0.5 - i as f32) * 360.0 / n as f32).to_radians();
                [r * a.cos(), r * a.sin()]
            })
            .collect()
    };
    let (mut slots, outline, mut key) = match &pl.layout {
        &Layout::Circle { ring, inner, centre } => {
            let tau = std::f32::consts::TAU;
            let core = if centre > 0 { pitch * 1.1 } else { pitch * 0.6 };
            let r_in = if inner > 0 { (pitch * inner as f32 / tau).max(core) } else { 0.0 };
            let r = (pitch * ring as f32 / tau).max(if inner > 0 { r_in + pitch } else { core });
            let mut v = ring_at(ring, r);
            v.extend(ring_at(inner, r_in));
            match centre {
                0 => {}
                1 => v.push([0.0, 0.0]),
                _ => v.extend([[-pitch / 2.0, 0.0], [pitch / 2.0, 0.0]]),
            }
            let outer = r + pin_r + pitch * 0.8;
            (v, Outline::Circle { r: outer }, Some([0.0, -outer]))
        }
        &Layout::Row(n) => ((0..n).map(|i| [centred(i, n), 0.0]).collect(), Outline::Rect { w: n as f32 * pitch + pitch * 0.6, h: pitch * 1.6 }, None),
        &Layout::Zigzag(n) => {
            let cols = n.div_ceil(2);
            let v = (0..n).map(|i| [centred(i / 2, cols), (i % 2) as f32 * pitch - pitch / 2.0]).collect();
            let h = pitch * 3.0;
            (v, Outline::Rect { w: cols as f32 * pitch + pitch * 1.6, h }, Some([0.0, h / 2.0]))
        }
        &Layout::Grid { rows, cols } => {
            let v = (0..rows * cols).map(|i| [centred(i % cols, cols), centred(i / cols, rows)]).collect();
            (v, Outline::Rect { w: cols as f32 * pitch + pitch * 0.8, h: rows as f32 * pitch + pitch * 0.8 }, None)
        }
        &Layout::Cols { rows, cols } => {
            let v = (0..rows * cols).map(|i| [centred(i / rows, cols), centred(i % rows, rows)]).collect();
            (v, Outline::Rect { w: cols as f32 * pitch + pitch * 0.8, h: rows as f32 * pitch + pitch * 0.8 }, None)
        }
        Layout::Points { pts, round } => {
            let margin = pin_r + pitch * 0.6;
            if *round {
                let r = pts.iter().map(|[x, y]| x.hypot(*y)).fold(0.0, f32::max) + margin;
                (pts.clone(), Outline::Circle { r }, Some([0.0, -r]))
            } else {
                let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
                for p in pts {
                    for k in 0..2 {
                        lo[k] = lo[k].min(p[k]);
                        hi[k] = hi[k].max(p[k]);
                    }
                }
                // The face is centred on the pins.
                let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
                let v = pts.iter().map(|p| [p[0] - mid[0], p[1] - mid[1]]).collect();
                (v, Outline::Rect { w: hi[0] - lo[0] + 2.0 * margin, h: hi[1] - lo[1] + 2.0 * margin }, None)
            }
        }
        &Layout::DSub(n) => {
            let (top, bottom) = (n.div_ceil(2), n / 2);
            let dy = pitch * 1.03;
            let mut v: Vec<[f32; 2]> = (0..top).map(|i| [centred(i, top), -dy / 2.0]).collect();
            v.extend((0..bottom).map(|i| [centred(i, bottom), dy / 2.0]));
            let w = top as f32 * pitch + pitch * 1.2;
            (v, Outline::DSub { top: w, bottom: w - pitch * 1.6, h: dy + pitch * 1.8 }, None)
        }
    };
    let outline = if pl.pe {
        // The protective earth contact sits right of the pins, the face grows by it.
        let (w, h) = outline.size();
        let x = w / 2.0 + pitch * 0.6;
        for s in &mut slots {
            s[0] -= pitch * 0.6;
        }
        slots.push([x - pitch * 0.6, 0.0]);
        match outline {
            Outline::Circle { r } => {
                key = Some([0.0, -(r + pitch * 0.6)]);
                Outline::Circle { r: r + pitch * 0.6 }
            }
            _ => Outline::Rect { w: w + pitch * 1.2, h },
        }
    } else {
        outline
    };
    Face { outline, slots, pin_r, key }
}

/// Position on the face of each pin designation: by order when the designations fill
/// the face, by number when they are numbers within it (M8 3-pin: 1, 3, 4), otherwise
/// by order up to the number of positions.
pub fn assign(designations: &[String], slots: usize) -> Vec<Option<usize>> {
    let numbers: Option<Vec<usize>> = designations.iter().map(|d| d.trim().parse::<usize>().ok().filter(|n| (1..=slots).contains(n))).collect();
    match numbers {
        Some(v) if designations.len() != slots => v.into_iter().map(|n| Some(n - 1)).collect(),
        _ => (0..designations.len()).map(|i| (i < slots).then_some(i)).collect(),
    }
}

#[cfg(test)]
mod tests;
