//! National symbol sets: GB (China), GOST (Russia and the CIS) and JIS (Japan).
//!
//! GB/T 4728 and JIS C 0617 adopt the graphical symbols of IEC 60617, so the sets take
//! the IEC symbols and give them the device letters and names of the national
//! standards: GB/T 7159 (`KM`, `KA`, `QF`, `SB`, `HL` …), GOST 2.710 (`KM`, `KK`, `QF`,
//! `SB`, `HL` …) and JIS C 0401 (`MC`, `MCCB`, `PBS`, `PL`, `THR` …). The GOST set
//! uses the same graphics; the proportions of the ESKD drawings (GOST 2.755, 2.756)
//! are not reproduced.

use super::{Category, SymbolDef};

/// (key, IEC symbol it is drawn like, device letter, name)
type Entry = (&'static str, &'static str, &'static str, &'static str);

const GB: &[Entry] = &[
    ("gb_coil_km", "coil", "KM", "Contactor coil KM (GB)"),
    ("gb_coil_ka", "coil", "KA", "Intermediate relay coil KA (GB)"),
    ("gb_coil_kt", "coil_on", "KT", "Time relay coil KT (GB)"),
    ("gb_main3", "main3", "KM", "Contactor main contacts KM (GB)"),
    ("gb_no", "no", "KA", "NO contact KA (GB)"),
    ("gb_nc", "nc", "KA", "NC contact KA (GB)"),
    ("gb_no_on", "no_on", "KT", "NO contact, on-delay KT (GB)"),
    ("gb_nc_on", "nc_on", "KT", "NC contact, on-delay KT (GB)"),
    ("gb_pb_no", "pb_no", "SB", "Push button NO SB (GB)"),
    ("gb_pb_nc", "pb_nc", "SB", "Push button NC SB (GB)"),
    ("gb_estop", "estop", "SB", "Emergency stop SB (GB)"),
    ("gb_sel", "sel", "SA", "Selector switch SA (GB)"),
    ("gb_limit", "limit", "SQ", "Position switch NO SQ (GB)"),
    ("gb_limit_nc", "limit_nc", "SQ", "Position switch NC SQ (GB)"),
    ("gb_fuse", "fuse", "FU", "Fuse FU (GB)"),
    ("gb_cb1", "cb1", "QF", "Circuit breaker 1-pole QF (GB)"),
    ("gb_cb3", "cb3", "QF", "Circuit breaker 3-pole QF (GB)"),
    ("gb_disc3", "disc3", "QS", "Disconnector 3-pole QS (GB)"),
    ("gb_ol3", "ol3", "FR", "Thermal relay FR (GB)"),
    ("gb_ol_nc", "ol_nc", "FR", "Thermal relay NC contact FR (GB)"),
    ("gb_motor3", "motor3", "M", "Motor 3-phase M (GB)"),
    ("gb_lamp", "lamp", "HL", "Indicator lamp HL (GB)"),
    ("gb_horn", "horn", "HA", "Horn / buzzer HA (GB)"),
    ("gb_trafo", "trafo", "TC", "Control transformer TC (GB)"),
    ("gb_valve", "valve", "YV", "Solenoid valve coil YV (GB)"),
    ("gb_term", "term", "XT", "Terminal XT (GB)"),
];

const GOST: &[Entry] = &[
    ("gost_coil_km", "coil", "KM", "Contactor coil KM (GOST)"),
    ("gost_coil_k", "coil", "K", "Relay coil K (GOST)"),
    ("gost_coil_kt", "coil_on", "KT", "Time relay coil KT (GOST)"),
    ("gost_main3", "main3", "KM", "Contactor main contacts KM (GOST)"),
    ("gost_no", "no", "K", "NO contact K (GOST)"),
    ("gost_nc", "nc", "K", "NC contact K (GOST)"),
    ("gost_no_on", "no_on", "KT", "NO contact, on-delay KT (GOST)"),
    ("gost_nc_on", "nc_on", "KT", "NC contact, on-delay KT (GOST)"),
    ("gost_pb_no", "pb_no", "SB", "Push button NO SB (GOST)"),
    ("gost_pb_nc", "pb_nc", "SB", "Push button NC SB (GOST)"),
    ("gost_estop", "estop", "SB", "Emergency stop SB (GOST)"),
    ("gost_sel", "sel", "SA", "Selector switch SA (GOST)"),
    ("gost_limit", "limit", "SQ", "Position switch NO SQ (GOST)"),
    ("gost_limit_nc", "limit_nc", "SQ", "Position switch NC SQ (GOST)"),
    ("gost_fuse", "fuse", "FU", "Fuse FU (GOST)"),
    ("gost_cb1", "cb1", "QF", "Circuit breaker 1-pole QF (GOST)"),
    ("gost_cb3", "cb3", "QF", "Circuit breaker 3-pole QF (GOST)"),
    ("gost_disc3", "disc3", "QS", "Disconnector 3-pole QS (GOST)"),
    ("gost_ol3", "ol3", "KK", "Thermal relay KK (GOST)"),
    ("gost_ol_nc", "ol_nc", "KK", "Thermal relay NC contact KK (GOST)"),
    ("gost_motor3", "motor3", "M", "Motor 3-phase M (GOST)"),
    ("gost_lamp", "lamp", "HL", "Indicator lamp HL (GOST)"),
    ("gost_horn", "horn", "HA", "Horn / buzzer HA (GOST)"),
    ("gost_trafo", "trafo", "T", "Control transformer T (GOST)"),
    ("gost_valve", "valve", "YA", "Solenoid valve coil YA (GOST)"),
    ("gost_term", "term", "XT", "Terminal XT (GOST)"),
];

const JIS: &[Entry] = &[
    ("jis_coil_mc", "coil", "MC", "Electromagnetic contactor coil MC (JIS)"),
    ("jis_coil_r", "coil", "R", "Auxiliary relay coil R (JIS)"),
    ("jis_coil_tlr", "coil_on", "TLR", "Time-lag relay coil TLR (JIS)"),
    ("jis_main3", "main3", "MC", "Contactor main contacts MC (JIS)"),
    ("jis_no", "no", "R", "NO contact R (JIS)"),
    ("jis_nc", "nc", "R", "NC contact R (JIS)"),
    ("jis_no_on", "no_on", "TLR", "NO contact, on-delay TLR (JIS)"),
    ("jis_nc_on", "nc_on", "TLR", "NC contact, on-delay TLR (JIS)"),
    ("jis_pb_no", "pb_no", "PBS", "Push button NO PBS (JIS)"),
    ("jis_pb_nc", "pb_nc", "PBS", "Push button NC PBS (JIS)"),
    ("jis_estop", "estop", "PBS", "Emergency stop PBS (JIS)"),
    ("jis_sel", "sel", "COS", "Selector switch COS (JIS)"),
    ("jis_limit", "limit", "LS", "Position switch NO LS (JIS)"),
    ("jis_limit_nc", "limit_nc", "LS", "Position switch NC LS (JIS)"),
    ("jis_fuse", "fuse", "F", "Fuse F (JIS)"),
    ("jis_cb1", "cb1", "MCCB", "Circuit breaker 1-pole MCCB (JIS)"),
    ("jis_cb3", "cb3", "MCCB", "Circuit breaker 3-pole MCCB (JIS)"),
    ("jis_disc3", "disc3", "DS", "Disconnector 3-pole DS (JIS)"),
    ("jis_ol3", "ol3", "THR", "Thermal relay THR (JIS)"),
    ("jis_ol_nc", "ol_nc", "THR", "Thermal relay NC contact THR (JIS)"),
    ("jis_motor3", "motor3", "M", "Motor 3-phase M (JIS)"),
    ("jis_lamp", "lamp", "PL", "Pilot lamp PL (JIS)"),
    ("jis_horn", "buzzer", "BZ", "Buzzer BZ (JIS)"),
    ("jis_trafo", "trafo", "T", "Control transformer T (JIS)"),
    ("jis_valve", "valve", "SV", "Solenoid valve coil SV (JIS)"),
    ("jis_term", "term", "TB", "Terminal block TB (JIS)"),
];

/// Adds the national sets, drawn like the IEC symbols already in `v`.
pub(super) fn add(v: &mut Vec<SymbolDef>) {
    for (category, entries) in [(Category::Gb, GB), (Category::Gost, GOST), (Category::Jis, JIS)] {
        for &(key, iec, letter, name) in entries {
            let Some(base) = v.iter().find(|d| d.key == iec) else { continue };
            v.push(SymbolDef { key, name, category, letter, ..base.clone() });
        }
    }
}

/// The IEC symbol a symbol of a national set is drawn like.
pub fn iec_equivalent(key: &str) -> Option<&'static str> {
    [GB, GOST, JIS].into_iter().flatten().find(|e| e.0 == key).map(|e| e.1)
}

#[cfg(test)]
mod tests;
