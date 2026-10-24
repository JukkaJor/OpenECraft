use super::*;

/// The symbols of the IEC 60617 overview (Siemens Capital: isolators / fuses /
/// contactors / overloads, power / meters / transformers / motors, general symbols),
/// by key; wires, links and tags are not symbols.
const OVERVIEW: &[&str] = &[
    // isolators, disconnectors, fuses, contactors, overloads
    "cb1", "main1", "cb2", "cb3", "cb1_tm", "cb2_tm", "cb3_tm", "main3", "main3_trip", "main3_semi", "main3_nc", "isol1_lock", "isol1",
    "isol2", "isol2_lock", "isol3_lock", "isol3", "fuse", "fuse_hbc1", "fuse2", "fuse_hbc2", "fuse3", "fuse_hbc3", "fuse_mcb1", "fuse_mcb2",
    "fuse_mcb3", "fuse_disc1", "fuse_disc2", "fuse_disc3", "fuse_sw1", "fuse_sw2", "fuse_sw3", "fuse_swdisc1", "fuse_swdisc2",
    "fuse_swdisc3", "main_sw1", "main_sw2", "disc3", "mcb1", "mcb2", "mps3", "rcd1", "rcd2", "rcd3", "swdisc1", "swdisc1_trip", "swdisc2",
    "swdisc2_trip", "swdisc3_trip", "swdisc3", "olm1", "olm2", "olm3", "ol1", "ol2", "ol3",
    // power, meters, transformers, motors
    "psu", "psu_dcac", "psu_dcdc", "psu_bidir", "rect_bridge", "rect_b", "rect_c", "rect_d", "voltmeter", "ammeter", "ammeter_reactive",
    "varmeter", "freqmeter", "voltmeter_diff", "salinity_meter", "tachometer", "pf_meter", "phase_meter", "wavemeter", "galvanometer",
    "thermometer", "wattmeter_rec", "watt_var_rec", "hour_meter", "ah_meter", "wh_meter", "wh_uni", "wh_from", "wh_to", "wh_from_to",
    "wh_multi", "wh_excess", "wh_tx", "wh_slave", "wh_slave_print", "wh_maxdem", "wh_maxrec", "varh_meter", "ct", "ct_2core", "ct_2wind",
    "trafo", "trafo1_var", "trafo1_screen", "autotrafo1", "autotrafo1_var", "trafo1_ct", "trafo3", "trafo_3w", "trafo1_taps", "trafo3_taps",
    "autotrafo3", "autotrafo3_star", "trafo3_bank", "trafo3_dd", "trafo3_yyd", "trafo3_yz", "trafo3_yy", "trafo3_dy", "trafo3_yd",
    "motor_dc", "motor_dc_shunt", "motor_ec", "motor1_2", "motor1_series", "motor1_aux", "fan1", "motor3", "motor3_6", "motor3_series",
    "motor3_slip", "motor3_linear", "fan3", "fan3_6",
    // general: terminals, connectors, signals, coils, contacts, switches, sensors, counters
    "term", "term_fuse", "plug_m", "plug_f", "plcbox_di8", "vfd", "temp_controller", "lamp", "lamp_flash", "lamp_trafo", "beacon", "siren",
    "buzzer", "horn", "brake", "clutch", "solenoid", "heater", "coil", "coil_ac", "coil_fast", "coil_latched", "coil_latching", "coil_led",
    "coil_resonance", "coil_thermal", "coil_indicator", "coil_flashing", "coil_suppressor", "coil_remanence", "coil_polarized",
    "coil_ac_resistant", "relay_overvoltage", "relay_undervoltage", "no", "nc", "no_pass", "no_pass_rel", "no_pass_both", "no_early",
    "nc_early", "no_late", "nc_late", "co", "co_mbb", "co_pass", "coil_on", "coil_off", "coil_clock", "no_on", "nc_on", "no_off", "nc_off",
    "coil_on_off", "no_on_off", "nc_on_off", "co_on", "co_off", "co_on_off", "clock_no", "clock_nc", "man_no", "man_nc", "man_no_m",
    "man_nc_m", "pb_no", "pb_nc", "pb_no_m", "pb_nc_m", "pull_no", "pull_nc", "pull_no_m", "pull_nc_m", "key_no", "key_nc", "key_no_m",
    "key_nc_m", "pb_key_no_m", "pb_key_nc_m", "estop_no", "estop", "estop_turn_no", "estop_turn_nc", "estop_pull_no", "estop_pull_nc",
    "estop_key_no", "estop_key_nc", "sel", "sel_nc", "sel_no_m", "sel_nc_m", "sel_no_pa", "sel_no_pb", "sel_nc_pa", "sel_nc_pb",
    "sel_no_m_pd", "sel_nc_m_pd", "sel3_no_sr_off", "sel3_nc_sr_off", "sel3_no_ret_off", "sel3_nc_ret_off", "sel3_no_m_off",
    "sel3_nc_m_off", "sel3_no_sr_mid", "sel3_nc_sr_mid", "sel3_no_sr_mm", "sel3_nc_sr_mm", "sel3_no_m", "sel3_nc_m", "foot_no", "foot_nc",
    "thermo_no", "thermo_nc", "prox_cap3", "prox_cap2", "prox_ind3", "prox_ind2", "prox_mag", "photo_retro4", "photo_retro3",
    "photo_diffuse4", "photo_diffuse3", "photo_tx", "photo_rx4", "photo_rx3", "photo_thru4", "photo_thru3", "touch3", "touch2",
    "pressure_no", "pressure_nc", "flow_no", "flow_nc", "gasflow_no", "gasflow_nc", "level_no", "level_nc", "cam_no", "cam_nc",
    "roller_no", "roller_nc", "camprof_no", "camprof_nc", "co_cam", "co_roller", "co_camprof", "limit", "limit_nc", "counter_1stage",
    "counter_2stage", "counter_pulse", "counter_preset", "counter_reset", "counter_no", "counter_nc", "counter_cam_no", "counter_cam_nc",
    "counter_sw_no", "counter_sw_nc",
];

#[test]
fn the_iec_overview_is_complete() {
    let missing: Vec<_> = OVERVIEW.iter().filter(|k| get(k).is_none()).collect();
    assert!(missing.is_empty(), "missing symbols: {missing:?}");
}

#[test]
fn symbols_have_a_function_kind() {
    // Devices without a template kind of their own get the generic definition.
    const GENERIC: [&str; 3] = ["counter_1stage", "counter_2stage", "temp_controller"];
    let without: Vec<_> = library()
        .iter()
        .filter(|d| matches!(d.category, C::Coils | C::Contacts | C::Switches | C::Sensors | C::Protection | C::Loads | C::Measuring | C::Power))
        .filter(|d| crate::parts::kind_of_symbol(d.key).is_none() && !GENERIC.contains(&d.key))
        .map(|d| d.key)
        .collect();
    assert!(without.is_empty(), "{without:?}");
    assert_eq!(kind("cb3_tm"), Some(FnKind::Breaker3));
    assert_eq!(kind("trafo3_dy"), Some(FnKind::Transformer));
    assert_eq!(crate::parts::kind_of_symbol("estop_turn_nc"), Some(FnKind::PushbuttonNc));
    assert_eq!(kind("coil"), None, "built-in symbols keep their own mapping");
    // Every symbol of this library with a kind can be assigned a function definition of it.
    for (key, k) in KINDS.get().unwrap() {
        assert_eq!(crate::functions::default_for_symbol(key).kind, *k, "{key}");
    }
}

#[test]
fn multi_pole_symbols_number_their_poles() {
    for key in ["cb3_tm", "fuse_swdisc3", "isol3_lock", "rcd3", "olm3", "main3_trip"] {
        let labels: Vec<_> = get(key).unwrap().pins.iter().map(|p| p.label).collect();
        assert_eq!(labels, ["1", "3", "5", "2", "4", "6"], "{key}");
    }
    let co = get("co_on").unwrap();
    assert_eq!(co.contact_kind(), Some(ContactKind::Changeover));
    assert_eq!(co.pins.len(), 3);
    assert_eq!(get("trafo3_yyd").unwrap().pins.len(), 9);
}

#[test]
fn push_and_pull_operators_differ() {
    // Pushing: the bracket opens towards the contact (its back at x = -6); pulling:
    // away from it (its back at x = -5).
    let back = |key: &str| {
        get(key).unwrap().prims.iter().any(|prim| matches!(prim, Prim::Line { a, b, dashed: false } if a.x == -6.0 && b.x == -6.0 && a.y == -1.5 && b.y == 1.5))
    };
    assert!(back("pb_no") && back("pb_nc") && back("pb_no_m"));
    assert!(!back("pull_no") && !back("pull_nc_m"));
}
