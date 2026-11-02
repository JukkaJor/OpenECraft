//! Mechanical catalog data shared by the catalog and the panel layout: the standard
//! enclosure systems with their sizes and the part numbers of their components, the
//! dimensions of 19-inch racks (IEC 60297) and the fasteners (screws, nuts, washers,
//! rivet nuts) of the fastening systems.

/// An enclosure system of the generic catalog.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SystemData {
    /// Stable key stored in projects ("compact", "baying", …).
    pub id: &'static str,
    /// Part number prefix of the system's components ("GEN.CE").
    pub prefix: &'static str,
    /// Catalog description of the enclosure body.
    pub body: &'static str,
    /// Standard sizes (width × height × depth, mm).
    pub sizes: &'static [(f32, f32, f32)],
    /// Sheet thickness of walls and door.
    pub wall: f32,
    /// Free edge on the inside of walls and door (folds, frame) where nothing can be
    /// mounted.
    pub flange: f32,
    /// Frame profile (floor-standing systems), 0 for monobloc enclosures.
    pub frame: f32,
    /// Default plinth height, 0 without plinth.
    pub plinth: f32,
    /// Enclosures can be bayed side by side (side panels are separate parts).
    pub bayable: bool,
    /// Material key of the climate calculation ("sheet_steel", "plastic", …).
    pub material: &'static str,
    pub ip: &'static str,
    /// Price of the smallest size, per m² of front for larger ones.
    pub price: (f32, f32),
}

pub const SYSTEMS: &[SystemData] = &[
    SystemData {
        id: "compact",
        prefix: "GEN.CE",
        body: "Compact enclosure, sheet steel, single door, with mounting plate",
        sizes: &[(380.0, 600.0, 210.0), (500.0, 500.0, 210.0), (600.0, 760.0, 210.0), (760.0, 760.0, 300.0), (800.0, 1000.0, 300.0), (1000.0, 1200.0, 300.0)],
        wall: 1.5,
        flange: 20.0,
        frame: 0.0,
        plinth: 0.0,
        bayable: false,
        material: "sheet_steel",
        ip: "IP66",
        price: (190.0, 520.0),
    },
    SystemData {
        id: "baying",
        prefix: "GEN.TS",
        body: "Floor-standing baying enclosure, frame with door, roof and mounting plate",
        sizes: &[(600.0, 1800.0, 500.0), (600.0, 2000.0, 500.0), (800.0, 2000.0, 500.0), (800.0, 2000.0, 600.0), (1000.0, 2000.0, 600.0), (1200.0, 2000.0, 600.0)],
        wall: 1.5,
        flange: 40.0,
        frame: 40.0,
        plinth: 100.0,
        bayable: true,
        material: "sheet_steel",
        ip: "IP55",
        price: (780.0, 640.0),
    },
    SystemData {
        id: "terminal_box",
        prefix: "GEN.KL",
        body: "Terminal box, sheet steel, screwed cover",
        sizes: &[(150.0, 150.0, 80.0), (200.0, 300.0, 120.0), (300.0, 300.0, 120.0), (400.0, 200.0, 120.0)],
        wall: 1.2,
        flange: 15.0,
        frame: 0.0,
        plinth: 0.0,
        bayable: false,
        material: "sheet_steel",
        ip: "IP66",
        price: (38.0, 300.0),
    },
    SystemData {
        id: "plastic",
        prefix: "GEN.PC",
        body: "Plastic enclosure, polycarbonate, hinged cover with mounting plate",
        sizes: &[(200.0, 300.0, 130.0), (300.0, 400.0, 180.0), (400.0, 600.0, 200.0)],
        wall: 3.5,
        flange: 15.0,
        frame: 0.0,
        plinth: 0.0,
        bayable: false,
        material: "plastic",
        ip: "IP67",
        price: (45.0, 380.0),
    },
    SystemData {
        id: HOUSE_CONNECTION_SYSTEM,
        prefix: "GEN.HAK",
        body: "House connection box, glass-fibre reinforced polyester, RAL 7035, hinged sealable cover, with mounting plate",
        sizes: &[(255.0, 415.0, 144.0), (410.0, 720.0, 185.0)],
        wall: 4.0,
        flange: 15.0,
        frame: 0.0,
        plinth: 0.0,
        bayable: false,
        material: "plastic",
        ip: "IP54",
        price: (68.0, 290.0),
    },
    SystemData {
        id: RACK_SYSTEM,
        prefix: "GEN.RK",
        body: "19-inch rack, frame with perforated door and two pairs of 19-inch mounting rails",
        sizes: &[(600.0, 640.0, 450.0), (600.0, 1200.0, 600.0), (600.0, 2000.0, 800.0), (600.0, 2000.0, 1000.0), (800.0, 2000.0, 1000.0), (800.0, 2200.0, 1200.0)],
        wall: 1.5,
        flange: 40.0,
        frame: 40.0,
        plinth: 0.0,
        bayable: true,
        material: "sheet_steel",
        ip: "IP20",
        price: (420.0, 480.0),
    },
];

/// The system of the 19-inch racks.
pub const RACK_SYSTEM: &str = "rack19";
/// The system of the house connection boxes (the utility's fuses at the building entry).
pub const HOUSE_CONNECTION_SYSTEM: &str = "house_connection";
/// Height of a rack unit (1 U), mm.
pub const RACK_UNIT: f32 = 44.45;
/// Width of a 19-inch front panel with its mounting ears, mm.
pub const RACK_WIDTH: f32 = 482.6;
/// Clear width between the mounting rails, mm.
pub const RACK_OPENING: f32 = 450.0;
/// Space at the top and the bottom of a rack outside its rack units (at least), mm.
pub const RACK_MARGIN: f32 = 50.0;
/// Distance of the front mounting rails behind the door, mm.
pub const RACK_FRONT: f32 = 100.0;

/// Rack units of a rack as high as `height` mm.
pub fn rack_units(height: f32) -> u32 {
    ((height - 2.0 * RACK_MARGIN) / RACK_UNIT).floor().max(0.0) as u32
}

/// A pair of 19-inch mounting rails as long as `units` rack units.
pub fn rack_rail_number(units: u32) -> String {
    format!("GEN.RK-RAIL-{units}U")
}

pub fn system(id: &str) -> Option<&'static SystemData> {
    SYSTEMS.iter().find(|s| s.id == id)
}

fn mm(v: f32) -> String {
    format!("{:.0}", v.round())
}

/// Enclosure body (frame or monobloc with door and mounting plate).
pub fn body_number(s: &SystemData, w: f32, h: f32, d: f32) -> String {
    format!("{}-{}X{}X{}", s.prefix, mm(w), mm(h), mm(d))
}

/// Side panel (one side) of a floor-standing system.
pub fn side_number(s: &SystemData, h: f32, d: f32) -> String {
    format!("{}-SP-{}X{}", s.prefix, mm(h), mm(d))
}

/// Rear wall of a floor-standing system.
pub fn rear_number(s: &SystemData, w: f32, h: f32) -> String {
    format!("{}-RW-{}X{}", s.prefix, mm(w), mm(h))
}

/// Gland plate (bottom) for the enclosure footprint.
pub fn gland_plate_number(w: f32, d: f32) -> String {
    format!("GEN.GP-{}X{}", mm(w), mm(d))
}

/// Plinth for the enclosure footprint.
pub fn plinth_number(w: f32, d: f32, height: f32) -> String {
    format!("GEN.PL-{}X{}-{}", mm(w), mm(d), mm(height))
}

/// Kit joining two bayed enclosures (clamps, seal).
pub const BAYING_KIT: &str = "GEN.TS-BAY";

/// Standoff raising a mounting rail above the plate (one per fixing point).
pub const RAIL_STANDOFF: &str = "GEN.RAIL-STANDOFF";

/// Thread sizes of the fasteners in the catalog: (thread, screw length).
pub const THREADS: &[(&str, u32)] = &[("M3", 8), ("M4", 10), ("M5", 12), ("M6", 16), ("M8", 20), ("M10", 25)];

/// Normalised thread ("m5" → "M5") if it is one of [`THREADS`].
pub fn thread(spec: &str) -> Option<&'static str> {
    let s = spec.trim().to_uppercase();
    THREADS.iter().map(|t| t.0).find(|t| *t == s)
}

pub fn screw_number(thread: &str) -> String {
    let len = THREADS.iter().find(|t| t.0 == thread).map(|t| t.1).unwrap_or(10);
    format!("GEN.SCREW-{thread}X{len}")
}

pub fn nut_number(thread: &str) -> String {
    format!("GEN.NUT-{thread}")
}

pub fn washer_number(thread: &str) -> String {
    format!("GEN.WASHER-{thread}")
}

pub fn rivet_nut_number(thread: &str) -> String {
    format!("GEN.RIVNUT-{thread}")
}

/// Subgroups of the enclosure accessories (placed in the panel layout without a device
/// tag).
pub const ACCESSORY_SUBGROUPS: &[&str] = &[
    "Filter fan",
    "Outlet filter",
    "Cable gland",
    "Enclosure light",
    "PE busbar",
    "Cable clamp rail",
    "Door handle",
    "Document holder",
];

#[cfg(test)]
mod tests;
