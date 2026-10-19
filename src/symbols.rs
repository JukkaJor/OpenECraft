//! Built-in IEC style symbol library.
//!
//! Symbols are defined in local millimetre coordinates in vertical orientation
//! (current flows top → bottom). Connection points lie on the 4 mm grid relative to
//! the symbol origin so that placed symbols connect cleanly to wires.

use crate::geom::{p, P, R};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactKind {
    No,
    Nc,
    Changeover,
    MainNo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Func {
    /// Main function of a device. `mirror` = draws a contact image (coils, overloads).
    Main { mirror: bool },
    /// Auxiliary function, belongs to the main function with the same tag.
    Aux(ContactKind),
    /// Terminal: tag = terminal strip, label = terminal number.
    Terminal,
    /// PLC connection point: tag = PLC card, label = address.
    Plc,
    /// Potential connection point: label = potential name. No device tag.
    Potential,
    /// Interruption point (page connector): label = name. No device tag.
    Interruption,
}

impl Func {
    pub fn has_tag(self) -> bool {
        !matches!(self, Func::Potential | Func::Interruption)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Coils,
    Contacts,
    Switches,
    Protection,
    Loads,
    Terminals,
    Plc,
    Potentials,
    Passive,
}

impl Category {
    pub const ALL: [Category; 9] = [
        Category::Coils,
        Category::Contacts,
        Category::Switches,
        Category::Protection,
        Category::Loads,
        Category::Terminals,
        Category::Plc,
        Category::Potentials,
        Category::Passive,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Category::Coils => "Coils / Relays",
            Category::Contacts => "Contacts",
            Category::Switches => "Switches / Sensors",
            Category::Protection => "Protective devices",
            Category::Loads => "Motors / Loads",
            Category::Terminals => "Terminals",
            Category::Plc => "PLC",
            Category::Potentials => "Potentials / Connectors",
            Category::Passive => "Passive components",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Label,
    Tag,
}

#[derive(Clone, Debug)]
pub enum Prim {
    Line { a: P, b: P, dashed: bool },
    Poly { pts: Vec<P>, closed: bool, fill: bool },
    Circle { c: P, r: f32, fill: bool },
    Text { pos: P, text: String, size: f32, anchor: Anchor },
    /// Instance property drawn at this position (stays horizontal when rotated).
    Field { kind: FieldKind, pos: P, size: f32, anchor: Anchor },
}

#[derive(Clone, Debug)]
pub struct PinDef {
    pub pos: P,
    pub label: &'static str,
}

#[derive(Clone, Debug)]
pub struct SymbolDef {
    pub key: &'static str,
    pub name: &'static str,
    pub category: Category,
    /// Default identifying letter (IEC 81346 style).
    pub letter: &'static str,
    pub func: Func,
    pub prims: Vec<Prim>,
    pub pins: Vec<PinDef>,
    /// Pins that are internally connected (terminals).
    pub bridges: Vec<(usize, usize)>,
    pub default_label: &'static str,
    /// Contact numbering (13/14, 21/22, ...) is assigned automatically.
    pub auto_number: bool,
    /// Draw tag / function text next to the symbol.
    pub show_tag: bool,
    pub show_pin_labels: bool,
    pub bounds: R,
}

impl SymbolDef {
    pub fn pin_world(&self, idx: usize, pos: P, rot: u8, mirror: bool) -> P {
        pos + self.pins[idx].pos.transform(rot, mirror)
    }

    pub fn world_bounds(&self, pos: P, rot: u8, mirror: bool) -> R {
        let corners = [
            self.bounds.min,
            p(self.bounds.max.x, self.bounds.min.y),
            self.bounds.max,
            p(self.bounds.min.x, self.bounds.max.y),
        ];
        let mut r = R::nothing();
        for c in corners {
            r.extend(pos + c.transform(rot, mirror));
        }
        r
    }

    pub fn contact_kind(&self) -> Option<ContactKind> {
        match self.func {
            Func::Aux(k) => Some(k),
            _ => None,
        }
    }
}

struct B {
    prims: Vec<Prim>,
    pins: Vec<PinDef>,
}

impl B {
    fn new() -> B {
        B { prims: Vec::new(), pins: Vec::new() }
    }
    fn l(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> &mut Self {
        self.prims.push(Prim::Line { a: p(x1, y1), b: p(x2, y2), dashed: false });
        self
    }
    fn dl(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> &mut Self {
        self.prims.push(Prim::Line { a: p(x1, y1), b: p(x2, y2), dashed: true });
        self
    }
    fn rect(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, fill: bool) -> &mut Self {
        self.prims.push(Prim::Poly {
            pts: vec![p(x1, y1), p(x2, y1), p(x2, y2), p(x1, y2)],
            closed: true,
            fill,
        });
        self
    }
    fn poly(&mut self, pts: &[(f32, f32)], closed: bool, fill: bool) -> &mut Self {
        self.prims.push(Prim::Poly {
            pts: pts.iter().map(|&(x, y)| p(x, y)).collect(),
            closed,
            fill,
        });
        self
    }
    fn circle(&mut self, x: f32, y: f32, r: f32, fill: bool) -> &mut Self {
        self.prims.push(Prim::Circle { c: p(x, y), r, fill });
        self
    }
    /// Arc from angle `a0` to `a1` in degrees (0° = +x, 90° = down).
    fn arc(&mut self, x: f32, y: f32, r: f32, a0: f32, a1: f32) -> &mut Self {
        let n = 12;
        let pts = (0..=n)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
                p(x + r * a.cos(), y + r * a.sin())
            })
            .collect();
        self.prims.push(Prim::Poly { pts, closed: false, fill: false });
        self
    }
    fn text(&mut self, x: f32, y: f32, t: &str, size: f32, anchor: Anchor) -> &mut Self {
        self.prims.push(Prim::Text { pos: p(x, y), text: t.into(), size, anchor });
        self
    }
    fn field(&mut self, kind: FieldKind, x: f32, y: f32, size: f32, anchor: Anchor) -> &mut Self {
        self.prims.push(Prim::Field { kind, pos: p(x, y), size, anchor });
        self
    }
    fn pin(&mut self, x: f32, y: f32, label: &'static str) -> &mut Self {
        self.pins.push(PinDef { pos: p(x, y), label });
        self
    }

    // --- reusable sub-drawings -------------------------------------------------

    /// Normally open contact at x.
    fn no(&mut self, x: f32) -> &mut Self {
        self.l(x, -4.0, x, -2.0).l(x, 2.0, x, 4.0).l(x, 2.0, x - 2.0, -2.4)
    }
    /// Normally closed contact at x.
    fn nc(&mut self, x: f32) -> &mut Self {
        self.l(x, -4.0, x, -2.0)
            .l(x, -2.0, x + 1.8, -2.0)
            .l(x, 2.0, x, 4.0)
            .l(x, 2.0, x + 2.3, -2.6)
    }
    /// Push button actuator, linked to the contact blade at `bx`.
    fn push_actuator(&mut self, bx: f32) -> &mut Self {
        self.dl(bx, 0.0, -5.0, 0.0)
            .l(-5.0, -1.5, -5.0, 1.5)
            .l(-5.0, -1.5, -6.0, -1.5)
            .l(-5.0, 1.5, -6.0, 1.5)
    }
    /// Circuit breaker cross at the fixed contact of a pole at x.
    fn breaker_x(&mut self, x: f32) -> &mut Self {
        self.l(x - 0.7, -2.7, x + 0.7, -1.3).l(x - 0.7, -1.3, x + 0.7, -2.7)
    }
}

struct Def {
    key: &'static str,
    name: &'static str,
    category: Category,
    letter: &'static str,
    func: Func,
    label: &'static str,
    auto_number: bool,
    show_tag: bool,
    show_pin_labels: bool,
    bridges: Vec<(usize, usize)>,
}

fn def(key: &'static str, name: &'static str, category: Category, letter: &'static str, func: Func) -> Def {
    Def {
        key,
        name,
        category,
        letter,
        func,
        label: "",
        auto_number: false,
        show_tag: func.has_tag(),
        show_pin_labels: matches!(func, Func::Main { .. } | Func::Aux(_)),
        bridges: Vec::new(),
    }
}

impl Def {
    fn label(mut self, l: &'static str) -> Self {
        self.label = l;
        self
    }
    fn auto(mut self) -> Self {
        self.auto_number = true;
        self
    }
    fn build(self, b: &mut B) -> SymbolDef {
        let mut bounds = R::nothing();
        for prim in &b.prims {
            match prim {
                Prim::Line { a, b, .. } => {
                    bounds.extend(*a);
                    bounds.extend(*b);
                }
                Prim::Poly { pts, .. } => pts.iter().for_each(|q| bounds.extend(*q)),
                Prim::Circle { c, r, .. } => {
                    bounds.extend(p(c.x - r, c.y - r));
                    bounds.extend(p(c.x + r, c.y + r));
                }
                Prim::Text { .. } | Prim::Field { .. } => {}
            }
        }
        for pin in &b.pins {
            bounds.extend(pin.pos);
        }
        SymbolDef {
            key: self.key,
            name: self.name,
            category: self.category,
            letter: self.letter,
            func: self.func,
            prims: std::mem::take(&mut b.prims),
            pins: std::mem::take(&mut b.pins),
            bridges: self.bridges,
            default_label: self.label,
            auto_number: self.auto_number,
            show_tag: self.show_tag,
            show_pin_labels: self.show_pin_labels,
            bounds: bounds.expand(0.5),
        }
    }
}

const MAIN: Func = Func::Main { mirror: false };
const MAIN_MIRROR: Func = Func::Main { mirror: true };

fn build_library() -> Vec<SymbolDef> {
    use Anchor::*;
    use Category as C;
    let mut v = Vec::new();
    let mut b = B::new();

    // ---------------------------------------------------------------- coils
    b.l(0.0, -4.0, 0.0, -2.0).rect(-3.0, -2.0, 3.0, 2.0, false).l(0.0, 2.0, 0.0, 4.0);
    b.pin(0.0, -4.0, "A1").pin(0.0, 4.0, "A2");
    v.push(def("coil", "Relay / contactor coil", C::Coils, "K", MAIN_MIRROR).build(&mut b));

    b.l(0.0, -4.0, 0.0, -2.0).rect(-3.0, -2.0, 3.0, 2.0, false).l(0.0, 2.0, 0.0, 4.0);
    b.rect(-6.0, -2.0, -3.0, 2.0, false).l(-6.0, -2.0, -3.0, 2.0).l(-6.0, 2.0, -3.0, -2.0);
    b.pin(0.0, -4.0, "A1").pin(0.0, 4.0, "A2");
    v.push(def("coil_on", "Timer coil, on-delay", C::Coils, "K", MAIN_MIRROR).build(&mut b));

    b.l(0.0, -4.0, 0.0, -2.0).rect(-3.0, -2.0, 3.0, 2.0, false).l(0.0, 2.0, 0.0, 4.0);
    b.rect(-6.0, -2.0, -3.0, 2.0, true);
    b.pin(0.0, -4.0, "A1").pin(0.0, 4.0, "A2");
    v.push(def("coil_off", "Timer coil, off-delay", C::Coils, "K", MAIN_MIRROR).build(&mut b));

    b.l(0.0, -4.0, 0.0, -2.0).rect(-3.0, -2.0, 3.0, 2.0, false).l(-3.0, 2.0, 3.0, -2.0).l(0.0, 2.0, 0.0, 4.0);
    b.pin(0.0, -4.0, "A1").pin(0.0, 4.0, "A2");
    v.push(def("valve", "Solenoid valve coil", C::Coils, "Y", MAIN).build(&mut b));

    // ---------------------------------------------------------------- contacts
    b.no(0.0).pin(0.0, -4.0, "13").pin(0.0, 4.0, "14");
    v.push(def("no", "NO contact", C::Contacts, "K", Func::Aux(ContactKind::No)).auto().build(&mut b));

    b.nc(0.0).pin(0.0, -4.0, "21").pin(0.0, 4.0, "22");
    v.push(def("nc", "NC contact", C::Contacts, "K", Func::Aux(ContactKind::Nc)).auto().build(&mut b));

    b.l(0.0, 4.0, 0.0, 2.0).l(0.0, 2.0, 2.3, -2.6); // common + blade
    b.l(0.0, -4.0, 0.0, -2.0); // NO
    b.l(4.0, -4.0, 4.0, -2.0).l(4.0, -2.0, 1.8, -2.0); // NC
    b.pin(0.0, 4.0, "11").pin(4.0, -4.0, "12").pin(0.0, -4.0, "14");
    v.push(
        def("co", "Changeover contact", C::Contacts, "K", Func::Aux(ContactKind::Changeover))
            .auto()
            .build(&mut b),
    );

    for x in [0.0, 8.0, 16.0] {
        b.no(x).arc(x, -1.3, 0.7, 180.0, 360.0);
    }
    b.dl(-0.9, 0.0, 15.1, 0.0);
    b.pin(0.0, -4.0, "1").pin(8.0, -4.0, "3").pin(16.0, -4.0, "5");
    b.pin(0.0, 4.0, "2").pin(8.0, 4.0, "4").pin(16.0, 4.0, "6");
    v.push(def("main3", "Main contacts 3-pole", C::Contacts, "K", Func::Aux(ContactKind::MainNo)).build(&mut b));

    b.no(0.0).arc(0.0, -1.3, 0.7, 180.0, 360.0).pin(0.0, -4.0, "1").pin(0.0, 4.0, "2");
    v.push(def("main1", "Main contact 1-pole", C::Contacts, "K", Func::Aux(ContactKind::MainNo)).build(&mut b));

    b.nc(0.0).dl(0.96, 0.0, -5.0, 0.0).rect(-8.0, -1.5, -5.0, 1.5, false);
    b.poly(&[(-7.6, 1.0), (-7.0, 1.0), (-7.0, -1.0), (-6.0, -1.0), (-6.0, 1.0), (-5.4, 1.0)], false, false);
    b.pin(0.0, -4.0, "95").pin(0.0, 4.0, "96");
    v.push(def("ol_nc", "Overload aux contact NC", C::Contacts, "F", Func::Aux(ContactKind::Nc)).build(&mut b));

    // ---------------------------------------------------------------- switches
    b.no(0.0).push_actuator(-0.9).pin(0.0, -4.0, "13").pin(0.0, 4.0, "14");
    v.push(def("pb_no", "Push button NO", C::Switches, "S", MAIN).build(&mut b));

    b.nc(0.0).push_actuator(0.96).pin(0.0, -4.0, "11").pin(0.0, 4.0, "12");
    v.push(def("pb_nc", "Push button NC", C::Switches, "S", MAIN).build(&mut b));

    b.nc(0.0).dl(0.96, 0.0, -4.0, 0.0).l(-4.0, -1.8, -4.0, 1.8).arc(-4.0, 0.0, 1.8, 90.0, 270.0);
    b.pin(0.0, -4.0, "11").pin(0.0, 4.0, "12");
    v.push(def("estop", "Emergency stop NC", C::Switches, "S", MAIN).build(&mut b));

    b.no(0.0).dl(-0.9, 0.0, -4.0, 0.0);
    b.poly(&[(-4.0, -1.5), (-5.0, -1.5), (-5.0, 1.5), (-6.0, 1.5)], false, false);
    b.pin(0.0, -4.0, "13").pin(0.0, 4.0, "14");
    v.push(def("sel", "Selector switch NO", C::Switches, "S", MAIN).build(&mut b));

    b.no(0.0).dl(-0.9, 0.0, -5.0, 0.0);
    b.poly(&[(-5.0, 0.0), (-6.5, -1.5), (-8.0, 0.0), (-6.5, 1.5)], true, false);
    b.pin(0.0, -4.0, "BN").pin(0.0, 4.0, "BK");
    v.push(def("prox", "Proximity sensor NO", C::Switches, "B", MAIN).build(&mut b));

    b.no(0.0).dl(-0.9, 0.0, -4.0, 0.0);
    b.poly(&[(-4.0, -1.5), (-6.0, 0.0), (-4.0, 1.5)], false, false);
    b.pin(0.0, -4.0, "13").pin(0.0, 4.0, "14");
    v.push(def("limit", "Position switch NO", C::Switches, "B", MAIN).build(&mut b));

    // ---------------------------------------------------------------- protection
    b.l(0.0, -4.0, 0.0, 4.0).rect(-1.2, -2.6, 1.2, 2.6, false);
    b.pin(0.0, -4.0, "1").pin(0.0, 4.0, "2");
    v.push(def("fuse", "Fuse", C::Protection, "F", MAIN).build(&mut b));

    b.no(0.0).breaker_x(0.0).dl(-0.9, 0.0, -5.0, 0.0).rect(-9.0, -1.5, -5.0, 1.5, false);
    b.text(-7.0, 0.0, "I>", 1.6, Center);
    b.pin(0.0, -4.0, "1").pin(0.0, 4.0, "2");
    v.push(def("cb1", "Circuit breaker 1-pole", C::Protection, "F", MAIN).build(&mut b));

    for x in [0.0, 8.0, 16.0] {
        b.no(x).breaker_x(x);
    }
    b.dl(-5.0, 0.0, 15.1, 0.0).rect(-9.0, -1.5, -5.0, 1.5, false).text(-7.0, 0.0, "I>", 1.6, Center);
    b.pin(0.0, -4.0, "1").pin(8.0, -4.0, "3").pin(16.0, -4.0, "5");
    b.pin(0.0, 4.0, "2").pin(8.0, 4.0, "4").pin(16.0, 4.0, "6");
    v.push(def("cb3", "Circuit breaker 3-pole", C::Protection, "Q", MAIN).build(&mut b));

    for x in [0.0, 8.0, 16.0] {
        b.no(x).breaker_x(x);
    }
    b.dl(-5.0, 0.0, 15.1, 0.0).rect(-13.0, -1.5, -5.0, 1.5, false).text(-9.0, 0.0, "I>  ϑ>", 1.6, Center);
    b.pin(0.0, -4.0, "1").pin(8.0, -4.0, "3").pin(16.0, -4.0, "5");
    b.pin(0.0, 4.0, "2").pin(8.0, 4.0, "4").pin(16.0, 4.0, "6");
    v.push(def("mps3", "Motor protection switch", C::Protection, "Q", MAIN).build(&mut b));

    b.rect(-2.0, -2.5, 18.0, 2.5, false);
    for x in [0.0, 8.0, 16.0] {
        b.l(x, -4.0, x, -2.5).l(x, 2.5, x, 4.0);
        b.poly(&[(x, -2.5), (x, -1.0), (x + 1.2, -1.0), (x + 1.2, 1.0), (x, 1.0), (x, 2.5)], false, false);
    }
    b.pin(0.0, -4.0, "1").pin(8.0, -4.0, "3").pin(16.0, -4.0, "5");
    b.pin(0.0, 4.0, "2").pin(8.0, 4.0, "4").pin(16.0, 4.0, "6");
    v.push(def("ol3", "Thermal overload relay", C::Protection, "F", MAIN_MIRROR).build(&mut b));

    // ---------------------------------------------------------------- loads
    {
        let (cx, cy, r) = (8.0f32, 8.0f32, 6.0f32);
        let lead = |b: &mut B, x: f32| {
            let start = p(x, 0.0);
            let d = start - p(cx, cy);
            let e = p(cx, cy) + d * (r / d.len());
            b.l(x, -4.0, x, 0.0).l(x, 0.0, e.x, e.y);
        };
        lead(&mut b, 0.0);
        lead(&mut b, 16.0);
        b.l(8.0, -4.0, 8.0, cy - r);
        b.l(24.0, -4.0, 24.0, cy).l(24.0, cy, cx + r, cy);
        b.circle(cx, cy, r, false).text(cx, cy - 1.2, "M", 3.5, Center).text(cx, cy + 2.4, "3~", 2.2, Center);
        b.pin(0.0, -4.0, "U1").pin(8.0, -4.0, "V1").pin(16.0, -4.0, "W1").pin(24.0, -4.0, "PE");
        v.push(def("motor3", "Motor 3-phase", C::Loads, "M", MAIN).build(&mut b));
    }
    {
        let (cx, cy, r) = (4.0f32, 8.0f32, 5.0f32);
        let lead = |b: &mut B, x: f32| {
            let start = p(x, 3.0);
            let d = start - p(cx, cy);
            let e = p(cx, cy) + d * (r / d.len());
            b.l(x, -4.0, x, 3.0).l(x, 3.0, e.x, e.y);
        };
        lead(&mut b, 0.0);
        lead(&mut b, 8.0);
        b.l(16.0, -4.0, 16.0, cy).l(16.0, cy, cx + r, cy);
        b.circle(cx, cy, r, false).text(cx, cy - 1.0, "M", 3.0, Center).text(cx, cy + 2.0, "1~", 2.0, Center);
        b.pin(0.0, -4.0, "L").pin(8.0, -4.0, "N").pin(16.0, -4.0, "PE");
        v.push(def("motor1", "Motor 1-phase", C::Loads, "M", MAIN).build(&mut b));
    }

    let d = 2.5 * std::f32::consts::FRAC_1_SQRT_2;
    b.l(0.0, -4.0, 0.0, -2.5).circle(0.0, 0.0, 2.5, false).l(-d, -d, d, d).l(-d, d, d, -d).l(0.0, 2.5, 0.0, 4.0);
    b.pin(0.0, -4.0, "X1").pin(0.0, 4.0, "X2");
    v.push(def("lamp", "Indicator lamp", C::Loads, "P", MAIN).build(&mut b));

    b.l(0.0, -4.0, 0.0, -2.0).poly(&[(-2.5, -2.0), (2.5, -2.0), (2.5, 0.5), (-2.5, 0.5)], true, false);
    b.arc(0.0, 0.5, 2.5, 0.0, 180.0).l(0.0, 3.0, 0.0, 4.0);
    b.pin(0.0, -4.0, "1").pin(0.0, 4.0, "2");
    v.push(def("horn", "Horn / buzzer", C::Loads, "P", MAIN).build(&mut b));

    b.rect(-3.0, 0.0, 19.0, 12.0, false).l(-3.0, 12.0, 19.0, 0.0);
    b.text(3.0, 3.5, "~", 3.5, Center).text(13.0, 8.5, "=", 3.5, Center);
    b.l(0.0, -4.0, 0.0, 0.0).l(8.0, -4.0, 8.0, 0.0).l(16.0, -4.0, 16.0, 0.0);
    b.l(0.0, 12.0, 0.0, 16.0).l(8.0, 12.0, 8.0, 16.0);
    b.pin(0.0, -4.0, "L").pin(8.0, -4.0, "N").pin(16.0, -4.0, "PE");
    b.pin(0.0, 16.0, "L+").pin(8.0, 16.0, "M");
    v.push(def("psu", "Power supply AC/DC", C::Loads, "T", MAIN).build(&mut b));

    b.circle(4.0, 3.0, 3.6, false).circle(4.0, 7.0, 3.6, false);
    b.l(0.0, -4.0, 0.0, 0.0).l(0.0, 0.0, 1.6, 0.4).l(8.0, -4.0, 8.0, 0.0).l(8.0, 0.0, 6.4, 0.4);
    b.l(0.0, 14.0, 0.0, 10.0).l(0.0, 10.0, 1.6, 9.6).l(8.0, 14.0, 8.0, 10.0).l(8.0, 10.0, 6.4, 9.6);
    b.l(0.0, 14.0, 0.0, 16.0).l(8.0, 14.0, 8.0, 16.0);
    b.pin(0.0, -4.0, "1.1").pin(8.0, -4.0, "1.2").pin(0.0, 16.0, "2.1").pin(8.0, 16.0, "2.2");
    v.push(def("trafo", "Control transformer", C::Loads, "T", MAIN).build(&mut b));

    // ---------------------------------------------------------------- terminals
    b.l(0.0, -4.0, 0.0, -1.0).circle(0.0, 0.0, 1.0, false).l(0.0, 1.0, 0.0, 4.0);
    b.field(FieldKind::Tag, -1.6, 0.0, 2.0, Right).field(FieldKind::Label, 1.6, 0.0, 2.2, Left);
    b.pin(0.0, -4.0, "").pin(0.0, 4.0, "");
    let mut t = def("term", "Terminal", C::Terminals, "X", Func::Terminal).label("1");
    t.show_tag = false;
    t.bridges = vec![(0, 1)];
    v.push(t.build(&mut b));

    b.l(0.0, -4.0, 0.0, -1.0).circle(0.0, 0.0, 1.0, false).l(0.0, 1.0, 0.0, 4.0);
    b.l(-1.6, -1.6, -0.7, -0.7).l(-1.6, 1.6, -0.7, 0.7);
    b.field(FieldKind::Tag, -2.2, 0.0, 2.0, Right).field(FieldKind::Label, 1.6, 0.0, 2.2, Left);
    b.pin(0.0, -4.0, "").pin(0.0, 4.0, "");
    let mut t = def("term_pe", "PE terminal", C::Terminals, "X", Func::Terminal).label("PE");
    t.show_tag = false;
    t.bridges = vec![(0, 1)];
    v.push(t.build(&mut b));

    // ---------------------------------------------------------------- PLC
    b.l(0.0, -4.0, 0.0, 0.0).rect(-5.0, 0.0, 5.0, 6.0, false).poly(&[(-5.0, 0.0), (-3.5, 1.5), (-5.0, 3.0)], false, false);
    b.field(FieldKind::Label, 0.0, 3.2, 2.0, Center);
    b.field(FieldKind::Tag, 5.6, 4.5, 1.8, Left);
    b.pin(0.0, -4.0, "");
    let mut t = def("plc_in", "PLC digital input", C::Plc, "A", Func::Plc).label("I0.0");
    t.show_tag = false;
    v.push(t.build(&mut b));

    b.l(0.0, 0.0, 0.0, 4.0).rect(-5.0, -6.0, 5.0, 0.0, false).poly(&[(-3.5, -6.0), (-5.0, -4.5), (-3.5, -3.0)], false, false);
    b.field(FieldKind::Label, 0.0, -2.8, 2.0, Center);
    b.field(FieldKind::Tag, 5.6, -4.5, 1.8, Left);
    b.pin(0.0, 4.0, "");
    let mut t = def("plc_out", "PLC digital output", C::Plc, "A", Func::Plc).label("Q0.0");
    t.show_tag = false;
    v.push(t.build(&mut b));

    // ---------------------------------------------------------------- potentials
    for (key, name, label) in [
        ("pot_l1", "Potential L1", "L1"),
        ("pot_l2", "Potential L2", "L2"),
        ("pot_l3", "Potential L3", "L3"),
        ("pot_n", "Potential N", "N"),
        ("pot_pe", "Potential PE", "PE"),
        ("pot_24", "Potential +24V", "+24V"),
        ("pot_0v", "Potential 0V", "0V"),
        ("pot", "Potential (custom)", "P1"),
    ] {
        b.l(0.0, 0.0, 0.0, 4.0).poly(&[(-1.2, 0.0), (1.2, 0.0), (0.0, 2.0)], true, true);
        b.field(FieldKind::Label, 0.0, -1.6, 2.5, Center);
        b.pin(0.0, 4.0, "");
        v.push(def(key, name, C::Potentials, "", Func::Potential).label(label).build(&mut b));
    }

    b.l(0.0, -4.0, 0.0, 0.0).l(-2.5, 0.0, 2.5, 0.0).l(-1.6, 1.0, 1.6, 1.0).l(-0.7, 2.0, 0.7, 2.0);
    b.pin(0.0, -4.0, "");
    v.push(def("ground", "Protective earth", C::Potentials, "", Func::Potential).label("PE").build(&mut b));

    b.l(0.0, 0.0, 0.0, 2.6).poly(&[(-1.5, 1.2), (0.0, 2.7), (1.5, 1.2)], false, false);
    b.field(FieldKind::Label, 2.2, 1.6, 2.5, Left);
    b.pin(0.0, 0.0, "");
    v.push(def("intr", "Interruption point", C::Potentials, "", Func::Interruption).label("1").build(&mut b));

    // ---------------------------------------------------------------- passive
    b.l(0.0, -4.0, 0.0, -3.0).rect(-1.2, -3.0, 1.2, 3.0, false).l(0.0, 3.0, 0.0, 4.0);
    b.pin(0.0, -4.0, "1").pin(0.0, 4.0, "2");
    v.push(def("res", "Resistor", C::Passive, "R", MAIN).build(&mut b));

    b.l(0.0, -4.0, 0.0, -0.8).l(-2.5, -0.8, 2.5, -0.8).l(-2.5, 0.8, 2.5, 0.8).l(0.0, 0.8, 0.0, 4.0);
    b.pin(0.0, -4.0, "1").pin(0.0, 4.0, "2");
    v.push(def("cap", "Capacitor", C::Passive, "C", MAIN).build(&mut b));

    b.l(0.0, -4.0, 0.0, -1.5).poly(&[(-1.8, -1.5), (1.8, -1.5), (0.0, 1.5)], true, false);
    b.l(-1.8, 1.5, 1.8, 1.5).l(0.0, 1.5, 0.0, 4.0);
    b.pin(0.0, -4.0, "A").pin(0.0, 4.0, "K");
    v.push(def("diode", "Diode", C::Passive, "V", MAIN).build(&mut b));

    v
}

pub fn library() -> &'static [SymbolDef] {
    static LIB: OnceLock<Vec<SymbolDef>> = OnceLock::new();
    LIB.get_or_init(build_library)
}

pub fn get(key: &str) -> Option<&'static SymbolDef> {
    library().iter().find(|d| d.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_on_grid_and_keys_unique() {
        let lib = library();
        let mut keys: Vec<_> = lib.iter().map(|d| d.key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), lib.len());
        for d in lib {
            for pin in &d.pins {
                assert_eq!(pin.pos.x % crate::geom::GRID, 0.0, "{} pin x off grid", d.key);
                assert_eq!(pin.pos.y % crate::geom::GRID, 0.0, "{} pin y off grid", d.key);
            }
        }
    }
}
