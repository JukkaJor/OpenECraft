//! Generic example catalog with function templates.

use crate::{natural_cmp, FnKind, FunctionTemplate, Mesh3d, Model2d, Part, Prim2d, TopGroup};

struct C {
    parts: Vec<Part>,
}

#[allow(clippy::too_many_arguments)]
impl C {
    fn add(
        &mut self,
        number: &str,
        desc: &str,
        group: &str,
        subgroup: &str,
        ident: &str,
        price: f32,
        dims: (f32, f32, f32, f32),
        functions: &[(FnKind, &str)],
    ) -> &mut Part {
        let (top, rest) = number.split_once('.').unwrap_or(("GEN", number));
        let manufacturer = if top == "GEN" { "Generic" } else { top };
        self.parts.push(Part {
            number: number.into(),
            type_number: rest.into(),
            order_number: rest.into(),
            manufacturer: manufacturer.into(),
            supplier: "Generic distribution".into(),
            description: desc.into(),
            top_group: match group {
                "Mounting" => TopGroup::Mechanics,
                "Valves" | "Fluid power components" => TopGroup::Fluid,
                _ => TopGroup::Electrical,
            },
            group: group.into(),
            subgroup: subgroup.into(),
            identifier: ident.into(),
            price,
            width: dims.0,
            height: dims.1,
            depth: dims.2,
            weight: dims.3,
            mounting: if dims.0 > 0.0 { "Mounting rail 35 mm".into() } else { String::new() },
            functions: functions.iter().map(|(k, p)| FunctionTemplate::new(*k, p)).collect(),
            ..Default::default()
        });
        self.parts.last_mut().unwrap()
    }
}

/// Generic example catalog with function templates.
pub fn default_catalog() -> Vec<Part> {
    use FnKind::*;
    let mut c = C { parts: Vec::new() };
    let rc = "Relays, contactors";
    let sd = "Safety devices";
    let sw = "Sensors, switches, pushbuttons";
    let main3 = "1 3 5 2 4 6";

    let p = c.add("GEN.CT-9A-24DC", "Contactor 3P 9 A, coil 24 V DC, 1 NO aux", rc, "Power contactor", "K", 38.5, (45.0, 74.0, 73.0, 0.32), &[(Coil, "A1 A2"), (MainContacts3, main3), (No, "13 14")]);
    p.voltage = "24 V DC".into();
    p.current = "9 A (AC-3)".into();
    p.tech_data = "Power loss: 3 W
Coil power: 3 W
B10d: 1300000".into();
    p.power = "4 kW / 400 V".into();
    p.accessories = vec!["GEN.AUX-2NO2NC".into()];
    let p = c.add("GEN.CT-18A-24DC", "Contactor 3P 18 A, coil 24 V DC, 1 NO + 1 NC aux", rc, "Power contactor", "K", 52.0, (45.0, 84.0, 87.0, 0.45), &[(Coil, "A1 A2"), (MainContacts3, main3), (No, "13 14"), (Nc, "21 22")]);
    p.voltage = "24 V DC".into();
    p.current = "18 A (AC-3)".into();
    p.tech_data = "Power loss: 4.5 W
Coil power: 4.5 W
B10d: 1300000".into();
    p.power = "7.5 kW / 400 V".into();
    p.accessories = vec!["GEN.AUX-2NO2NC".into()];
    c.add("GEN.AUX-2NO2NC", "Auxiliary contact block 2 NO + 2 NC, front mounting", rc, "Auxiliary contact block", "K", 14.6, (45.0, 40.0, 40.0, 0.05), &[(No, "53 54"), (Nc, "61 62"), (Nc, "71 72"), (No, "83 84")]);
    c.add("GEN.AUX-1NO1NC", "Auxiliary contact block 1 NO + 1 NC", rc, "Auxiliary contact block", "K", 9.8, (45.0, 40.0, 40.0, 0.03), &[(No, "53 54"), (Nc, "61 62")]);
    let p = c.add("GEN.RLY-24DC-2CO", "Interface relay 24 V DC, 2 CO, with socket", rc, "Relay", "K", 12.4, (15.5, 78.0, 70.0, 0.04), &[(Coil, "A1 A2"), (Changeover, "11 12 14"), (Changeover, "21 22 24")]);
    p.voltage = "24 V DC".into();
    p.current = "8 A".into();
    p.tech_data = "Power loss: 0.5 W".into();
    let p = c.add("GEN.TMR-24DC-OND", "Timer relay on-delay 24 V DC, 0.05 s - 100 h, 1 CO", rc, "Timer relay", "K", 34.9, (17.5, 90.0, 64.0, 0.07), &[(Coil, "A1 A2"), (Changeover, "15 16 18")]);
    p.voltage = "24 V DC".into();
    p.tech_data = "Power loss: 1 W".into();
    let p = c.add("GEN.TMR-24DC-OND-2", "Timer relay on-delay 24 V DC, 1 NO + 1 NC delayed", rc, "Timer relay", "K", 39.5, (17.5, 90.0, 64.0, 0.07), &[(Coil, "A1 A2"), (NoOnDelay, "17 18"), (NcOnDelay, "25 26")]);
    p.voltage = "24 V DC".into();
    p.tech_data = "Power loss: 1 W".into();
    let p = c.add("GEN.TMR-24DC-OFFD-2", "Timer relay off-delay 24 V DC, 1 NO + 1 NC delayed", rc, "Timer relay", "K", 41.0, (17.5, 90.0, 64.0, 0.07), &[(Coil, "A1 A2"), (NoOffDelay, "17 18"), (NcOffDelay, "25 26")]);
    p.voltage = "24 V DC".into();
    p.tech_data = "Power loss: 1 W".into();
    let p = c.add("GEN.CT-25A-4P-24DC", "Contactor 4P 25 A (AC-1), coil 24 V DC", rc, "Power contactor", "K", 58.0, (45.0, 84.0, 87.0, 0.42), &[(Coil, "A1 A2"), (MainContacts4, "1 3 5 7 2 4 6 8")]);
    p.voltage = "24 V DC".into();
    p.current = "25 A (AC-1)".into();
    p.tech_data = "Power loss: 4 W".into();
    p.accessories = vec!["GEN.AUX-1NO1NC".into()];
    let p = c.add("GEN.ICT-20A-1NO-230AC", "Installation contactor 1P 20 A, 1 NO, coil 230 V AC", rc, "Power contactor", "K", 21.0, (18.0, 85.0, 60.0, 0.1), &[(Coil, "A1 A2"), (MainContact, "1 2")]);
    p.voltage = "230 V AC".into();
    p.current = "20 A (AC-1)".into();
    p.tech_data = "Power loss: 2 W".into();
    let p = c.add("GEN.SAFE-RLY-24DC", "Safety relay 24 V DC, 3 NO + 1 NC, up to PL e", rc, "Safety relay", "K", 168.0, (22.5, 100.0, 120.0, 0.2), &[(Coil, "A1 A2"), (No, "13 14"), (No, "23 24"), (No, "33 34"), (Nc, "41 42")]);
    p.voltage = "24 V DC".into();
    p.tech_data = "Power loss: 2 W
PL: e
PFHd: 2.5E-9
Current consumption: 80 mA".into();

    let p = c.add("GEN.MPS-2.5-4A", "Motor protection switch 2.5 - 4 A", sd, "Motor protection switch", "Q", 61.0, (45.0, 97.0, 97.0, 0.3), &[(MotorProtection, main3)]);
    p.current = "2.5 - 4 A".into();
    p.tech_data = "Power loss: 5 W
Breaking capacity: 50 kA
Instantaneous trip: 13 x In".into();
    p.accessories = vec!["GEN.MPS-AUX-1NO1NC".into()];
    let p = c.add("GEN.MPS-4-6.3A", "Motor protection switch 4 - 6.3 A", sd, "Motor protection switch", "Q", 63.0, (45.0, 97.0, 97.0, 0.3), &[(MotorProtection, main3)]);
    p.current = "4 - 6.3 A".into();
    p.tech_data = "Power loss: 6 W
Breaking capacity: 50 kA
Instantaneous trip: 13 x In".into();
    p.accessories = vec!["GEN.MPS-AUX-1NO1NC".into()];
    c.add("GEN.MPS-AUX-1NO1NC", "Auxiliary switch for motor protection switch, 1 NO + 1 NC", sd, "Auxiliary switch", "Q", 12.9, (9.0, 90.0, 60.0, 0.02), &[(No, "13 14"), (Nc, "21 22")]);
    let p = c.add("GEN.MCB-1P-C6", "Miniature circuit breaker 1P C6", sd, "Circuit breaker", "F", 8.7, (18.0, 90.0, 70.0, 0.12), &[(Breaker1, "1 2")]);
    p.current = "6 A".into();
    p.tech_data = "Power loss: 1.5 W
Characteristic: C
Breaking capacity: 10 kA".into();
    let p = c.add("GEN.MCB-1P-B10", "Miniature circuit breaker 1P B10", sd, "Circuit breaker", "F", 8.7, (18.0, 90.0, 70.0, 0.12), &[(Breaker1, "1 2")]);
    p.current = "10 A".into();
    p.tech_data = "Power loss: 2 W
Characteristic: B
Breaking capacity: 10 kA".into();
    let p = c.add("GEN.MCB-3P-C16", "Miniature circuit breaker 3P C16", sd, "Circuit breaker", "Q", 27.3, (54.0, 90.0, 70.0, 0.36), &[(Breaker3, main3)]);
    p.current = "16 A".into();
    p.tech_data = "Power loss: 7 W
Characteristic: C
Breaking capacity: 10 kA".into();
    // House connection box and NH fuse links (fitted into fuse bases and fuse switch
    // disconnectors: added to their devices, three per device).
    let p = c.add("GEN.HAK-NH2-315A", "House connection box NH2, 3 fuse bases 400 A, PEN terminal, for wall mounting", sd, "House connection box", "F", 165.0, (300.0, 450.0, 160.0, 7.5), &[(Fuse3, main3)]);
    p.voltage = "400 V AC".into();
    p.current = "400 A".into();
    p.mounting = "Wall mounting".into();
    p.mounting_data.wall_mount = true;
    p.tech_data = "Fuse size: NH2\nProtection: IP54\nConnection: up to 240 mm² Al / Cu\nPower loss: 25 W".into();
    for (number, size, amps, price) in [("GEN.NH000-GG-63A", "NH000", 63, 6.8), ("GEN.NH00-GG-50A", "NH00", 50, 6.9), ("GEN.NH00-GG-80A", "NH00", 80, 7.4), ("GEN.NH2-GG-80A", "NH2", 80, 17.5), ("GEN.NH2-GG-250A", "NH2", 250, 24.0)] {
        let p = c.add(number, &format!("NH fuse link size {size}, gG, {amps} A, 500 V AC"), sd, "Fuse link", "F", price, (0.0, 0.0, 0.0, 0.2), &[]);
        p.current = format!("{amps} A");
        p.mounting.clear();
        p.tech_data = format!("Fuse size: {size}\nUtilisation category: gG\nBreaking capacity: 120 kA");
    }
    let p = c.add("GEN.FUSE-10X38-3P-2A", "Fuse holder 3-pole 10 x 38 mm with fuse links 2 A gG, for voltage taps", sd, "Fuse", "F", 24.0, (54.0, 90.0, 60.0, 0.2), &[(Fuse3, main3)]);
    p.current = "2 A".into();
    p.tech_data = "Power loss: 1 W\nUtilisation category: gG".into();
    let p = c.add("GEN.FUSE-PV-2P-15A", "PV fuse holder 2-pole 10 x 38 mm, 1000 V DC, with gPV fuse links 15 A", sd, "Fuse", "F", 32.0, (36.0, 90.0, 60.0, 0.15), &[(Fuse, "1 2"), (Fuse, "3 4")]);
    p.voltage = "1000 V DC".into();
    p.current = "15 A".into();
    p.tech_data = "Power loss: 2 W\nUtilisation category: gPV".into();
    let p = c.add("GEN.FUSE-5X20-2A", "Fuse holder 5x20 mm with fuse 2 A", sd, "Fuse", "F", 4.2, (8.0, 60.0, 50.0, 0.02), &[(Fuse, "1 2")]);
    p.current = "2 A".into();
    p.tech_data = "Power loss: 0.4 W
Utilisation category: gG
Breaking capacity: 1.5 kA".into();
    let p = c.add("GEN.OL-2.8-4A", "Thermal overload relay 2.8 - 4 A, 1 NO + 1 NC", sd, "Overload relay", "F", 44.0, (45.0, 70.0, 80.0, 0.15), &[(Overload, main3), (Nc, "95 96"), (No, "97 98")]);
    p.current = "2.8 - 4 A".into();
    p.tech_data = "Power loss: 3 W".into();
    let p = c.add("GEN.MCB-2P-C16", "Miniature circuit breaker 2P C16", sd, "Circuit breaker", "F", 19.5, (36.0, 90.0, 70.0, 0.24), &[(Breaker2, "1 3 2 4")]);
    p.current = "16 A".into();
    p.tech_data = "Power loss: 4.5 W
Characteristic: C
Breaking capacity: 10 kA".into();
    let p = c.add("GEN.MCB-4P-C32", "Miniature circuit breaker 4P C32", sd, "Circuit breaker", "Q", 48.0, (72.0, 90.0, 70.0, 0.48), &[(Breaker4, "1 3 5 7 2 4 6 8")]);
    p.current = "32 A".into();
    p.tech_data = "Power loss: 12 W
Characteristic: C
Breaking capacity: 10 kA".into();
    let p = c.add("GEN.RCCB-4P-40A-30MA", "Residual current circuit breaker 4P 40 A, 30 mA, type A", sd, "Residual current device", "F", 64.0, (72.0, 90.0, 70.0, 0.4), &[(ResidualCurrent, "1 3 5 N 2 4 6 N'")]);
    p.current = "40 A".into();
    p.tech_data = "Rated residual current: 30 mA\nPower loss: 5 W".into();
    let p = c.add("GEN.FUSE-3P-10X38-32A", "Fuse holder 3P 10x38 mm, 32 A, with fuses", sd, "Fuse", "F", 24.0, (54.0, 81.0, 60.0, 0.25), &[(Fuse3, main3)]);
    p.current = "32 A".into();
    p.tech_data = "Power loss: 9 W
Utilisation category: gG
Breaking capacity: 100 kA".into();
    let p = c.add("GEN.SPD-T2-275", "Surge protective device type 2, 1P, 275 V AC, 20 kA", sd, "Surge protection", "F", 42.0, (18.0, 90.0, 68.0, 0.12), &[(SurgeProtection, "L PE")]);
    p.voltage = "275 V AC".into();
    let p = c.add("GEN.MS-3P-40A", "Main switch 3P 40 A, red / yellow handle, lockable", sd, "Main switch", "Q", 56.0, (48.0, 64.0, 85.0, 0.3), &[(Disconnector3, main3)]);
    p.current = "40 A".into();
    p.mounting = "Screw mounting".into();
    p.tech_data = "Mounting holes: M4: 6,8; 42,8; 6,56; 42,56\nPower loss: 3 W".into();

    c.add("GEN.PB-GN-1NO", "Pushbutton green, 1 NO", sw, "Pushbutton", "S", 11.2, (30.0, 30.0, 45.0, 0.03), &[(PushbuttonNo, "13 14")]).accessories = vec!["GEN.CB-1NO".into(), "GEN.CB-1NC".into()];
    c.add("GEN.PB-RD-1NC", "Pushbutton red, 1 NC", sw, "Pushbutton", "S", 11.2, (30.0, 30.0, 45.0, 0.03), &[(PushbuttonNc, "11 12")]).accessories = vec!["GEN.CB-1NO".into(), "GEN.CB-1NC".into()];
    let p = c.add("GEN.ESTOP-1NC", "Emergency stop mushroom 40 mm, twist release, 1 NC", sw, "Emergency stop", "S", 24.6, (40.0, 40.0, 60.0, 0.06), &[(PushbuttonNc, "11 12")]);
    p.accessories = vec!["GEN.CB-1NC".into()];
    p.tech_data = "B10d: 100000".into();
    c.add("GEN.SEL-2POS-1NO", "Selector switch 2 positions, 1 NO", sw, "Selector switch", "S", 15.9, (30.0, 30.0, 45.0, 0.03), &[(SwitchNo, "13 14")]);
    c.add("GEN.CB-1NO", "Contact block 1 NO for front mounting", sw, "Contact block", "S", 4.9, (10.0, 30.0, 40.0, 0.01), &[(No, "13 14")]);
    c.add("GEN.CB-1NC", "Contact block 1 NC for front mounting", sw, "Contact block", "S", 4.9, (10.0, 30.0, 40.0, 0.01), &[(Nc, "21 22")]);
    let p = c.add("GEN.PROX-M12-PNP", "Inductive proximity sensor M12, PNP NO, Sn 4 mm", sw, "Proximity sensor", "B", 29.0, (12.0, 12.0, 60.0, 0.05), &[(Sensor, "BN BK")]);
    p.voltage = "10 - 30 V DC".into();
    p.tech_data = "Current consumption: 15 mA".into();
    c.add("GEN.LIMIT-1NO1NC", "Position switch roller plunger, 1 NO + 1 NC", sw, "Position switch", "B", 32.0, (31.0, 60.0, 33.0, 0.1), &[(Sensor, "13 14"), (Nc, "21 22")]);
    c.add("GEN.LIMIT-1NC", "Position switch plunger, 1 NC, positive opening", sw, "Position switch", "B", 26.0, (31.0, 60.0, 33.0, 0.1), &[(SensorNc, "11 12")]).tech_data = "B10d: 10000000".into();
    c.add("GEN.SEL-2POS-1NC", "Selector switch 2 positions, 1 NC", sw, "Selector switch", "S", 15.9, (30.0, 30.0, 45.0, 0.03), &[(SwitchNc, "11 12")]);
    c.add("GEN.KEY-2POS-1NO", "Key switch 2 positions, key removable in 0, 1 NO", sw, "Key switch", "S", 27.5, (30.0, 30.0, 50.0, 0.05), &[(SwitchNo, "13 14")]);
    c.add("GEN.PRESS-SW-10BAR", "Pressure switch 1 - 10 bar, G1/4, 1 NO", sw, "Pressure switch", "B", 48.0, (0.0, 0.0, 0.0, 0.2), &[(Sensor, "13 14")]);
    c.add("GEN.TEMP-SW-60C", "Thermostat 0 - 60 °C, 1 NC", sw, "Temperature switch", "B", 19.0, (17.5, 67.0, 38.0, 0.05), &[(SensorNc, "11 12")]);
    c.add("GEN.FLOAT-SW-1NO", "Float switch, cable 5 m, 1 NO", sw, "Level switch", "B", 21.0, (0.0, 0.0, 0.0, 0.3), &[(Sensor, "13 14")]);
    let p = c.add("GEN.PT-10BAR-420", "Pressure transmitter 0 - 10 bar, 4 - 20 mA, 2-wire, G1/4", sw, "Transmitter", "B", 135.0, (0.0, 0.0, 0.0, 0.15), &[(Transmitter, "+ -")]);
    p.voltage = "10 - 30 V DC".into();
    let p = c.add("GEN.TT-PT100-420", "Temperature transmitter Pt100, 0 - 150 °C, 4 - 20 mA", sw, "Transmitter", "B", 118.0, (0.0, 0.0, 0.0, 0.2), &[(Transmitter, "+ -")]);
    p.voltage = "10 - 30 V DC".into();

    let p = c.add("GEN.LMP-GN-24", "Indicator lamp green, LED 24 V", "Signal devices", "Indicator lamp", "P", 9.1, (30.0, 30.0, 45.0, 0.02), &[(Signal, "X1 X2")]);
    p.voltage = "24 V AC/DC".into();
    p.tech_data = "Current consumption: 20 mA".into();
    let p = c.add("GEN.LMP-RD-24", "Indicator lamp red, LED 24 V", "Signal devices", "Indicator lamp", "P", 9.1, (30.0, 30.0, 45.0, 0.02), &[(Signal, "X1 X2")]);
    p.voltage = "24 V AC/DC".into();
    p.tech_data = "Current consumption: 20 mA".into();
    c.add("GEN.HORN-24", "Panel buzzer 24 V, 85 dB", "Signal devices", "Horn", "P", 18.0, (30.0, 30.0, 50.0, 0.03), &[(Signal, "1 2")]);

    let p = c.add("GEN.MOT-0.75KW", "Three-phase motor 0.75 kW 400 V, 1400 rpm", "Motors", "Three-phase motor", "M", 180.0, (0.0, 0.0, 0.0, 9.5), &[(Motor3, "U1 V1 W1 PE")]);
    p.voltage = "400 V".into();
    p.current = "1.9 A".into();
    p.power = "0.75 kW".into();
    let p = c.add("GEN.MOT-1PH-0.25KW", "Single-phase motor 0.25 kW 230 V", "Motors", "Single-phase motor", "M", 140.0, (0.0, 0.0, 0.0, 5.0), &[(Motor1, "L N PE")]);
    p.power = "0.25 kW".into();
    let p = c.add("GEN.MOT-1.5KW-PTC", "Three-phase motor 1.5 kW 400 V, 1420 rpm, PTC thermistors", "Motors", "Three-phase motor", "M", 260.0, (0.0, 0.0, 0.0, 14.0), &[(Motor3, "U1 V1 W1 PE"), (Thermistor, "T1 T2")]);
    p.voltage = "400 V".into();
    p.current = "3.4 A".into();
    p.power = "1.5 kW".into();
    let p = c.add("GEN.MOT-0.55KW-BRK", "Three-phase brake motor 0.55 kW 400 V, brake 24 V DC", "Motors", "Brake motor", "M", 310.0, (0.0, 0.0, 0.0, 11.0), &[(Motor3, "U1 V1 W1 PE"), (Brake, "BD1 BD2")]);
    p.voltage = "400 V".into();
    p.current = "1.5 A".into();
    p.power = "0.55 kW".into();
    let p = c.add("GEN.MOT-DC-24V-60W", "DC motor 24 V 60 W, 3000 rpm", "Motors", "DC motor", "M", 95.0, (0.0, 0.0, 0.0, 1.2), &[(MotorDc, "A1 A2")]);
    p.voltage = "24 V DC".into();
    p.current = "3.2 A".into();
    p.power = "60 W".into();

    let p = c.add("GEN.VFD-0.75KW", "Frequency converter 0.75 kW, 3 x 400 V, EMC filter", "Drives", "Frequency converter", "T", 285.0, (72.0, 200.0, 170.0, 1.8), &[(FrequencyConverter, "L1 L2 L3 PE U V W"), (PlcInput, "DI1"), (Changeover, "11 12 14")]);
    p.voltage = "380 - 480 V AC".into();
    p.current = "2.2 A".into();
    p.power = "0.75 kW".into();
    p.tech_data = "Clearance: 100 mm (above and below)\nPower loss: 35 W".into();
    let p = c.add("GEN.SST-7.5KW", "Soft starter 7.5 kW 400 V, 2-phase controlled, bypass", "Drives", "Soft starter", "Q", 240.0, (45.0, 125.0, 130.0, 0.8), &[(SoftStarter, "1L1 3L2 5L3 2T1 4T2 6T3"), (Coil, "A1 A2"), (No, "13 14")]);
    p.voltage = "24 V AC/DC".into();
    p.current = "16 A".into();
    p.power = "7.5 kW".into();
    p.tech_data = "Power loss: 6 W".into();

    let p = c.add("GEN.CT-150-5A", "Current transformer 150/5 A, class 1, 2.5 VA, for cables up to Ø 28 mm", "Measuring instruments", "Current transformer", "T", 34.0, (70.0, 80.0, 65.0, 0.4), &[(CurrentTransformer, "P1 P2 S1 S2")]);
    p.current = "150 / 5 A".into();
    let p = c.add("GEN.PV-430W", "PV module 430 Wp, monocrystalline, glass-glass, 1722 x 1134 mm", "Power supplies, transformers", "PV module", "G", 118.0, (1134.0, 1722.0, 30.0, 21.5), &[(PvGenerator, "+ -")]);
    p.current = "13.9 A (Isc)".into();
    p.power = "430 Wp".into();
    p.mounting = "Roof".into();
    p.tech_data = "Open-circuit voltage: 39.2 V\nVoltage at maximum power: 32.6 V\nCurrent at maximum power: 13.2 A\nEfficiency: 22 %\nMax. system voltage: 1500 V DC".into();
    let p = c.add("GEN.PLC-CPU", "PLC CPU 24 V DC, 2 x Ethernet (Modbus TCP, EtherCAT master), 1 MB work memory", "PLC", "CPU", "A", 690.0, (60.0, 100.0, 75.0, 0.3), &[(Generic, "L+ M"), (Generic, "X1 X2")]);
    p.voltage = "24 V DC".into();
    p.tech_data = "Power loss: 6 W\nCurrent consumption: 250 mA".into();
    let p = c.add("GEN.ACT-VLV-24V", "Valve actuator 24 V AC/DC, open / close, 90°, 15 Nm, with two end position switches", "Motors", "Valve actuator", "MA", 189.0, (0.0, 0.0, 0.0, 1.1), &[(Actuator, "1 2 3"), (No, "S1 S2"), (No, "S3 S4")]);
    p.voltage = "24 V AC/DC".into();
    p.power = "6 W".into();
    let p = c.add("GEN.LT-HYD-2M", "Hydrostatic level transmitter 0 - 2 m H2O, 4 - 20 mA, 2-wire, 10 m cable", sw, "Transmitter", "B", 245.0, (0.0, 0.0, 0.0, 0.4), &[(Transmitter, "+ -")]);
    p.voltage = "10 - 30 V DC".into();
    let p = c.add("GEN.MOT-4KW", "Three-phase motor 4 kW 400 V, 2880 rpm, IE3, foot-mounted", "Motors", "Three-phase motor", "M", 410.0, (280.0, 280.0, 380.0, 26.0), &[(Motor3, "U1 V1 W1 PE")]);
    p.mounting = "Floor".into();
    p.mounting_data.floor_mount = true;
    p.voltage = "400 V".into();
    p.current = "8 A".into();
    p.power = "4 kW".into();
    let p = c.add("GEN.HP-AW-12KW", "Heat pump air / water 12 kW, 400 V, inverter, flow up to 70 °C, indoor unit", "Heating", "Heat pump", "EQ", 9800.0, (600.0, 1800.0, 650.0, 150.0), &[(FluidComponent, "1 2")]);
    p.mounting = "Floor".into();
    p.mounting_data.floor_mount = true;
    p.voltage = "400 V".into();
    p.current = "10 A".into();
    p.power = "3.4 kW (12 kW heat)".into();
    p.tech_data = "Coefficient of performance: 4.2 (A7/W35)\nControl: enable contact, 0 - 10 V setpoint, fault contact\nPower loss: 150 W".into();
    let p = c.add("GEN.PUMP-CIRC-DN25", "Circulating pump group DN25, high-efficiency wet rotor, 230 V, with shut-off valves and wall bracket", "Fluid power components", "Pump", "GP", 420.0, (250.0, 400.0, 200.0, 7.5), &[(Pump, "S P")]);
    p.mounting = "Wall".into();
    p.mounting_data.wall_mount = true;
    p.voltage = "230 V".into();
    p.current = "0.4 A".into();
    for (number, desc) in [
        ("GEN.TT-PT1000-420", "Temperature transmitter PT1000, 0 - 100 °C, 4 - 20 mA, 2-wire, with immersion sleeve"),
        ("GEN.TT-OUT-420", "Outdoor temperature transmitter -40 - 60 °C, 4 - 20 mA, 2-wire, wall housing IP65"),
        ("GEN.PT-16BAR-420", "Pressure transmitter 0 - 16 bar, 4 - 20 mA, 2-wire, G1/4"),
    ] {
        let p = c.add(number, desc, sw, "Transmitter", "B", 118.0, (0.0, 0.0, 0.0, 0.2), &[(Transmitter, "+ -")]);
        p.voltage = "10 - 30 V DC".into();
    }
    let p = c.add("GEN.RLY-24DC-1NO", "Interface relay 24 V DC, 1 NO 6 A, with socket", rc, "Relay", "K", 9.8, (6.2, 90.0, 70.0, 0.03), &[(Coil, "A1 A2"), (No, "13 14")]);
    p.voltage = "24 V DC".into();
    p.current = "6 A".into();
    p.tech_data = "Power loss: 0.4 W".into();
    let p = c.add("GEN.MOT-2.2KW", "Three-phase motor 2.2 kW 400 V, 2850 rpm, IE3, foot-mounted", "Motors", "Three-phase motor", "M", 290.0, (230.0, 250.0, 330.0, 16.0), &[(Motor3, "U1 V1 W1 PE")]);
    p.mounting = "Floor".into();
    p.mounting_data.floor_mount = true;
    p.voltage = "400 V".into();
    p.current = "4.6 A".into();
    p.power = "2.2 kW".into();
    let p = c.add("GEN.PS-24DC-5A", "Power supply 230 V AC / 24 V DC 5 A", "Power supplies, transformers", "Power supply", "T", 89.0, (40.0, 125.0, 115.0, 0.6), &[(PowerSupply, "L N PE L+ M")]);
    p.voltage = "230 V AC / 24 V DC".into();
    p.current = "5 A".into();
    p.tech_data = "Clearance: 25 mm (convection cooling, above and below)\nPower loss: 13 W".into();
    let p = c.add("GEN.TR-230-24-100VA", "Control transformer 230 V / 24 V 100 VA", "Power supplies, transformers", "Control transformer", "T", 46.0, (78.0, 84.0, 70.0, 1.6), &[(Transformer, "1.1 1.2 2.1 2.2")]);
    p.mounting = "Screw mounting".into();
    p.tech_data = "Mounting holes: M5: 9,12; 69,12; 9,72; 69,72\nPower loss: 12 W".into();
    let p = c.add("GEN.BAT-24V-7AH", "Battery module 24 V DC 7.2 Ah, lead-acid, for DC UPS", "Power supplies, transformers", "Battery", "G", 120.0, (0.0, 0.0, 0.0, 5.5), &[(Battery, "+ -")]);
    p.voltage = "24 V DC".into();
    let p = c.add("GEN.CTR-100-5A", "Current transformer 100 / 5 A, class 1, 2.5 VA", "Measuring instruments", "Current transformer", "T", 28.0, (0.0, 0.0, 0.0, 0.3), &[(CurrentTransformer, "P1 P2 S1 S2")]);
    p.current = "100 A / 5 A".into();
    c.add("GEN.AMM-100-5A", "Ammeter 0 - 100 A for current transformer x / 5 A, 72 x 72 mm", "Measuring instruments", "Ammeter", "P", 32.0, (72.0, 72.0, 55.0, 0.2), &[(Meter, "1 2")]);
    c.add("GEN.VOLT-500V", "Voltmeter 0 - 500 V AC, 72 x 72 mm", "Measuring instruments", "Voltmeter", "P", 30.0, (72.0, 72.0, 55.0, 0.2), &[(Meter, "1 2")]);

    let p = c.add("GEN.SOCKET-DIN-16A", "Socket outlet type F for mounting rail, 16 A 250 V", "Installation devices", "Socket outlet", "X", 14.0, (45.0, 90.0, 60.0, 0.1), &[(SocketOutlet, "L N PE")]);
    p.current = "16 A".into();
    // Made for direct wall mounting in a room (Layout space › Insert › Wall-mounted part).
    let p = c.add("GEN.LIGHT-EM-3W", "Emergency light LED 3 W, 3 h, IP65, for wall mounting", "Installation devices", "Emergency light", "E", 89.0, (350.0, 120.0, 60.0, 0.9), &[(Signal, "L N")]);
    p.voltage = "230 V AC".into();
    p.mounting_data.wall_mount = true;
    let p = c.add("GEN.HEATER-100W", "Enclosure heater 100 W, 230 V AC, with fan", "Installation devices", "Enclosure heater", "E", 75.0, (70.0, 130.0, 70.0, 0.6), &[(Heater, "1 2")]);
    p.voltage = "230 V AC".into();
    p.power = "100 W".into();

    let p = c.add("GEN.TB-2.5", "Feed-through terminal 2.5 mm², grey", "Terminals", "Feed-through terminal", "X", 0.95, (5.2, 48.6, 29.3, 0.006), &[(Terminal, "")]);
    p.tech_data = "End bracket: GEN.EB-35\nEnd plate: GEN.TB-END\nJumper: GEN.JMP-2.5-2, GEN.JMP-2.5-3, GEN.JMP-2.5-10".into();
    p.accessories = vec!["GEN.TB-END".into(), "GEN.EB-35".into(), "GEN.JMP-2.5-2".into(), "GEN.JMP-2.5-3".into(), "GEN.JMP-2.5-10".into()];
    let p = c.add("GEN.TB-PE-2.5", "Protective earth terminal 2.5 mm², green-yellow", "Terminals", "PE terminal", "X", 1.8, (5.2, 48.5, 29.3, 0.01), &[(Terminal, "")]);
    p.tech_data = "Terminal type: PE\nEnd bracket: GEN.EB-35\nEnd plate: GEN.TB-END".into();
    p.accessories = vec!["GEN.TB-END".into(), "GEN.EB-35".into()];
    let p = c.add("GEN.TB-N-2.5", "Neutral terminal 2.5 mm², blue", "Terminals", "N terminal", "X", 1.0, (5.2, 48.6, 29.3, 0.006), &[(Terminal, "")]);
    p.tech_data = "Terminal type: N\nEnd bracket: GEN.EB-35\nEnd plate: GEN.TB-END".into();
    p.accessories = vec!["GEN.TB-END".into(), "GEN.EB-35".into()];
    let p = c.add("GEN.TB-DISC-4", "Disconnect terminal 4 mm², knife disconnect, grey", "Terminals", "Disconnect terminal", "X", 2.6, (6.2, 60.0, 42.0, 0.01), &[(Terminal, "")]);
    p.tech_data = "Terminal type: Disconnect\nEnd bracket: GEN.EB-35".into();
    p.accessories = vec!["GEN.EB-35".into()];
    let p = c.add("GEN.TB-FUSE-4", "Fuse terminal 4 mm² for fuses 5 x 20 mm, with LED 24 V", "Terminals", "Fuse terminal", "X", 3.9, (6.2, 55.9, 57.4, 0.02), &[(Terminal, "")]);
    p.tech_data = "Terminal type: Fuse\nEnd bracket: GEN.EB-35".into();
    p.accessories = vec!["GEN.EB-35".into()];
    // Terminals for the cables between enclosures and from the field (sizes after Phoenix
    // Contact PT 16 N, PTPOWER 35 and PT 6).
    for (number, desc, subgroup, price, dims) in [
        ("GEN.TB-16", "Feed-through terminal 16 mm², grey", "Feed-through terminal", 3.4, (12.2, 75.4, 45.1, 0.03)),
        ("GEN.TB-N-16", "Neutral terminal 16 mm², blue", "N terminal", 3.6, (12.2, 75.4, 45.1, 0.03)),
        ("GEN.TB-PE-16", "Protective earth terminal 16 mm², green-yellow", "PE terminal", 6.8, (12.2, 75.4, 45.1, 0.05)),
        ("GEN.TB-35", "Feed-through terminal 35 mm², grey", "Feed-through terminal", 7.9, (16.0, 91.6, 62.3, 0.06)),
        ("GEN.TB-N-35", "Neutral terminal 35 mm², blue", "N terminal", 8.2, (16.0, 91.6, 62.3, 0.06)),
        ("GEN.TB-PE-35", "Protective earth terminal 35 mm², green-yellow", "PE terminal", 12.5, (16.0, 91.6, 62.3, 0.09)),
        ("GEN.TB-PV-6", "PV feed-through terminal 6 mm², 1000 V DC, grey", "Feed-through terminal", 2.9, (8.2, 57.7, 36.0, 0.02)),
        ("GEN.BB-FEED-185", "Busbar feed-in terminal 1-pole, up to 185 mm², for 60 mm busbar systems", "Feed-through terminal", 38.0, (40.0, 90.0, 70.0, 0.4)),
        ("GEN.TB-240", "Feed-through terminal 240 mm², for cable distribution cabinets", "Feed-through terminal", 24.0, (0.0, 0.0, 0.0, 0.5)),
        ("GEN.TB-PEN-150", "PEN terminal 150 mm², green-yellow with blue marking", "PE terminal", 19.0, (0.0, 0.0, 0.0, 0.4)),
    ] {
        let p = c.add(number, desc, "Terminals", subgroup, "X", price, dims, &[(Terminal, "")]);
        if number == "GEN.BB-FEED-185" {
            p.mounting = "Busbar 60 mm".into();
        } else if dims.0 > 0.0 {
            // On the rail between end brackets, 6 and 16 mm² with an end plate.
            let plate = if number.ends_with("-16") { Some("GEN.TB-END-16") } else if number == "GEN.TB-PV-6" { Some("GEN.TB-END-6") } else { None };
            p.tech_data = match plate {
                Some(plate) => format!("End bracket: GEN.EB-35\nEnd plate: {plate}"),
                None => "End bracket: GEN.EB-35".into(),
            };
            p.accessories = plate.into_iter().chain(["GEN.EB-35"]).map(String::from).collect();
        }
    }
    c.add("GEN.TB-END", "End plate for terminal 2.5 mm²", "Terminals", "Accessories", "X", 0.3, (2.2, 48.6, 29.1, 0.002), &[]);
    c.add("GEN.TB-END-6", "End plate for terminal 6 mm²", "Terminals", "Accessories", "X", 0.4, (2.2, 57.7, 36.0, 0.003), &[]);
    c.add("GEN.TB-END-16", "End plate for terminal 16 mm²", "Terminals", "Accessories", "X", 0.6, (2.2, 75.4, 45.1, 0.005), &[]);
    c.add("GEN.EB-35", "End bracket for TS 35 mounting rail", "Terminals", "End bracket", "X", 1.1, (9.5, 54.6, 26.0, 0.01), &[]);
    for poles in [2, 3, 10] {
        let number = format!("GEN.JMP-2.5-{poles}");
        let desc = format!("Plug-in jumper {poles} poles for terminal 2.5 mm²");
        c.add(&number, &desc, "Terminals", "Jumper", "", 0.25 * poles as f32, (0.0, 0.0, 0.0, 0.001), &[]).tech_data = format!("Poles: {poles}");
    }

    // Marker material (printed by marking software), per piece.
    let mk = "Marking";
    c.add("GEN.MARK-DEV", "Device marker 15 x 5 mm, white, printable", mk, "Device marker", "", 0.08, (0.0, 0.0, 0.0, 0.0), &[]);
    c.add("GEN.MARK-LEG", "Legend plate 27 x 8 mm for 22 mm command devices, printable", mk, "Legend plate", "", 0.35, (0.0, 0.0, 0.0, 0.0), &[]);
    c.add("GEN.MARK-TB-5", "Terminal marker 5.2 mm, white, printable", mk, "Terminal marker", "", 0.02, (0.0, 0.0, 0.0, 0.0), &[]);
    c.add("GEN.MARK-WIRE", "Wire marker sleeve for 0.5 – 2.5 mm², white, printable", mk, "Wire marker", "", 0.03, (0.0, 0.0, 0.0, 0.0), &[]);
    c.add("GEN.MARK-CABLE", "Cable marker for cable Ø 4 – 10 mm, white, printable", mk, "Cable marker", "", 0.12, (0.0, 0.0, 0.0, 0.0), &[]);

    let di: Vec<(FnKind, String)> = (0..16).map(|i| (PlcInput, format!("I{}.{}", i / 8, i % 8))).collect();
    let di: Vec<(FnKind, &str)> = di.iter().map(|(k, s)| (*k, s.as_str())).collect();
    c.add("GEN.PLC-DI16", "PLC digital input module 16 x 24 V DC", "PLC", "Digital input module", "A", 145.0, (25.0, 100.0, 75.0, 0.15), &di);
    let dq: Vec<(FnKind, String)> = (0..16).map(|i| (PlcOutput, format!("Q{}.{}", i / 8, i % 8))).collect();
    let dq: Vec<(FnKind, &str)> = dq.iter().map(|(k, s)| (*k, s.as_str())).collect();
    c.add("GEN.PLC-DO16", "PLC digital output module 16 x 24 V DC 0.5 A", "PLC", "Digital output module", "A", 165.0, (25.0, 100.0, 75.0, 0.15), &dq);
    c.add("GEN.PLC-AI4", "PLC analog input module 4 x 0/4 - 20 mA, 0 - 10 V", "PLC", "Analog input module", "A", 245.0, (25.0, 100.0, 75.0, 0.15), &[(PlcAnalogInput, "IW64"), (PlcAnalogInput, "IW66"), (PlcAnalogInput, "IW68"), (PlcAnalogInput, "IW70")]);
    c.add("GEN.PLC-AQ2", "PLC analog output module 2 x 0/4 - 20 mA, 0 - 10 V", "PLC", "Analog output module", "A", 230.0, (25.0, 100.0, 75.0, 0.15), &[(PlcAnalogOutput, "QW64"), (PlcAnalogOutput, "QW66")]);

    let p = c.add("GEN.SV-24DC", "Solenoid valve coil 24 V DC, 4.8 W", "Valves", "Solenoid valve", "Y", 22.5, (0.0, 0.0, 0.0, 0.1), &[(Valve, "A1 A2")]);
    p.voltage = "24 V DC".into();
    p.power = "4.8 W".into();
    let p = c.add("GEN.VLV-52-24DC", "5/2-way solenoid valve G1/8, spring return, coil 24 V DC", "Valves", "Directional valve", "QM", 68.0, (26.0, 110.0, 64.0, 0.25), &[(DirectionalValve, "4 2 5 1 3"), (Valve, "A1 A2")]);
    p.voltage = "24 V DC".into();
    p.power = "1.5 W".into();
    let p = c.add("GEN.VLV-42-24DC", "4/2-way solenoid valve G1/4, spring return, coil 24 V DC", "Valves", "Directional valve", "QM", 74.0, (32.0, 120.0, 70.0, 0.3), &[(DirectionalValve, "A B P T"), (Valve, "A1 A2")]);
    p.voltage = "24 V DC".into();
    p.power = "2.5 W".into();
    connectors(&mut c);
    let fl = "Fluid power components";
    c.add("GEN.CYL-32-100", "Pneumatic cylinder double-acting, bore 32 mm, stroke 100 mm", fl, "Cylinder", "MM", 82.0, (0.0, 0.0, 0.0, 0.7), &[(Cylinder, "1 2")]);
    c.add("GEN.CYL-20-50-SA", "Pneumatic cylinder single-acting, bore 20 mm, stroke 50 mm", fl, "Cylinder", "MM", 46.0, (0.0, 0.0, 0.0, 0.3), &[(Cylinder, "1")]);
    c.add("GEN.PUMP-HYD-4CC", "Hydraulic gear pump 4 cm³/rev, 200 bar", fl, "Pump", "GP", 190.0, (0.0, 0.0, 0.0, 1.8), &[(Pump, "P S")]);
    c.add("GEN.FR-G14", "Filter regulator G1/4, 5 µm, 0.5 - 10 bar", fl, "Service unit", "HQ", 54.0, (45.0, 160.0, 70.0, 0.4), &[(FluidComponent, "1 2")]);
    c.add("GEN.THROTTLE-G18", "One-way flow control valve G1/8", fl, "Service unit", "RN", 12.0, (0.0, 0.0, 0.0, 0.05), &[(FluidComponent, "1 2")]);
    // Standing on the floor of a room (Layout space › Insert › Floor-standing part).
    for (number, desc, subgroup, ident, price, dims, function) in [
        ("GEN.PUMP-CENT-DN25", "Centrifugal pump DN25, bare shaft, on base frame, without motor", "Pump", "GP", 980.0, (320.0, 420.0, 640.0, 31.0), (Pump, "S P")),
        ("GEN.TANK-200L", "Vessel 200 l, stainless steel, on feet", "Vessel", "CM", 1450.0, (650.0, 1200.0, 650.0, 62.0), (FluidComponent, "N1 N2")),
    ] {
        let p = c.add(number, desc, fl, subgroup, ident, price, dims, &[function]);
        p.mounting = "Floor".into();
        p.mounting_data.floor_mount = true;
    }
    for (number, desc, subgroup, ident, price, dims, function) in [
        ("GEN.PUMP-BOOST-DN32", "Multistage booster pump DN32, 6 m³/h at 50 m, horizontal, bare shaft, on base frame, without motor", "Pump", "GP", 1240.0, (300.0, 330.0, 620.0, 29.0), (Pump, "S P")),
        ("GEN.TANK-1000L", "Storage tank 1000 l, PE, for drinking water, with overflow and inspection opening", "Vessel", "CM", 890.0, (1100.0, 1650.0, 1100.0, 55.0), (FluidComponent, "N1 N2")),
        ("GEN.TANK-PRESS-100L", "Diaphragm pressure vessel 100 l, 10 bar, flow-through, for drinking water", "Vessel", "CM", 420.0, (450.0, 760.0, 450.0, 21.0), (FluidComponent, "N1 N2")),
        ("GEN.TANK-BUFFER-500L", "Buffer storage tank 500 l for hot water, 4 connections, insulated", "Vessel", "CM", 1180.0, (800.0, 1750.0, 800.0, 95.0), (FluidComponent, "N1 N2 N3 N4")),
        ("GEN.COMP-PISTON-450", "Piston compressor 10 bar, 450 l/min, belt-driven, on base frame, without motor", "Compressor", "GQ", 1650.0, (1100.0, 800.0, 500.0, 78.0), (Pump, "S P")),
        ("GEN.RECEIVER-270L", "Air receiver 270 l, 11 bar, vertical, with safety valve, gauge and condensate drain", "Air receiver", "CM", 520.0, (600.0, 1500.0, 600.0, 58.0), (FluidComponent, "1 2")),
        ("GEN.DRYER-REFR-600", "Refrigeration dryer 600 l/min, pressure dew point +3 °C, 230 V AC", "Air dryer", "HQ", 1380.0, (450.0, 750.0, 550.0, 36.0), (FluidComponent, "1 2")),
    ] {
        let p = c.add(number, desc, fl, subgroup, ident, price, dims, &[function]);
        p.mounting = "Floor".into();
        p.mounting_data.floor_mount = true;
    }
    // Pipe fittings screwed to the wall of a room.
    for (number, desc, subgroup, ident, price, dims, pins) in [
        ("GEN.VLV-BALL-DN32", "Ball valve DN32, PN25, brass, with wall bracket", "Shut-off valve", "QM", 48.0, (180.0, 130.0, 150.0, 1.9), "1 2"),
        ("GEN.VLV-MOT-DN32", "Motorised ball valve DN32, PN16, with wall bracket, for valve actuators ISO 5211", "Shut-off valve", "QM", 156.0, (200.0, 140.0, 160.0, 2.4), "1 2"),
        ("GEN.CHECK-DN32", "Check valve DN32, PN16, brass, with wall bracket", "Check valve", "RM", 29.0, (140.0, 90.0, 90.0, 0.8), "1 2"),
        ("GEN.WM-DN32", "Water meter Q3 10 m³/h, DN32, with pulse output and wall bracket", "Flow meter", "BF", 168.0, (260.0, 120.0, 120.0, 2.6), "1 2"),
        ("GEN.HOUSE-ENTRY-DN32", "House service entry for water pipe DN32, wall sleeve with seal and bracket", "Pipe connection", "UM", 210.0, (180.0, 180.0, 140.0, 3.1), "1"),
        ("GEN.MANIFOLD-DN32-4", "Distribution manifold DN32 with 4 shut-off valves DN20, wall bracket", "Pipe connection", "UM", 145.0, (500.0, 160.0, 120.0, 3.8), "1"),
        ("GEN.DRYER-MEMBRANE-G38", "Membrane dryer G3/8 for the point of use, pressure dew point -20 °C, with wall bracket", "Air dryer", "HQ", 310.0, (60.0, 300.0, 70.0, 0.9), "1 2"),
        ("GEN.FR-G38-WALL", "Filter regulator G3/8, 40 µm, 0.5 - 10 bar, with gauge and wall bracket", "Service unit", "AZ", 86.0, (80.0, 220.0, 90.0, 0.8), "1 2"),
        ("GEN.LUB-G38", "Lubricator (oiler) G3/8, proportional oil mist, with wall bracket", "Lubricator", "HQ", 64.0, (70.0, 200.0, 80.0, 0.6), "1 2"),
        ("GEN.VLV-BALL-G12", "Ball valve G1/2 for compressed air, with wall bracket", "Shut-off valve", "QM", 21.0, (90.0, 70.0, 60.0, 0.3), "1 2"),
        ("GEN.COUPLING-NW72", "Quick coupling NW 7.2, self-sealing, wall outlet", "Quick coupling", "XM", 18.0, (60.0, 80.0, 60.0, 0.2), "1"),
        ("GEN.INTAKE-FILTER-G1", "Intake filter G1 with silencer for compressors", "Intake filter", "RN", 34.0, (0.0, 0.0, 0.0, 0.4), "1"),
        ("GEN.VLV-BALL-DN25", "Ball valve DN25, PN40, with wall bracket", "Shut-off valve", "QM", 64.0, (160.0, 120.0, 140.0, 1.4), "1 2"),
        ("GEN.GAUGE-0-10BAR", "Pressure gauge 0 - 10 bar, Ø 100 mm, with wall bracket", "Pressure gauge", "BP", 38.0, (110.0, 140.0, 70.0, 0.4), "1"),
    ] {
        let p = c.add(number, desc, fl, subgroup, ident, price, dims, &[(FluidComponent, pins)]);
        p.mounting = "Wall".into();
        p.mounting_data.wall_mount = true;
        if number == "GEN.MANIFOLD-DN32-4" {
            p.functions = ["IN", "OUT1", "OUT2", "OUT3", "OUT4"].into_iter().map(|pin| FunctionTemplate::new(FluidComponent, pin)).collect();
        }
    }

    c.add("GEN.RES-1K", "Resistor 1 kΩ 0.6 W", "Passive components", "Resistor", "R", 0.1, (0.0, 0.0, 0.0, 0.0), &[(Generic, "1 2")]);
    c.add("GEN.CAP-100N", "Capacitor 100 nF 63 V", "Passive components", "Capacitor", "C", 0.15, (0.0, 0.0, 0.0, 0.0), &[(Generic, "1 2")]);
    c.add("GEN.DIODE-1N4007", "Rectifier diode 1 A 1000 V", "Passive components", "Diode", "V", 0.08, (0.0, 0.0, 0.0, 0.0), &[(Generic, "A K")]);
    c.add("GEN.VAR-275V", "Varistor 275 V AC, 4.5 kA", "Passive components", "Varistor", "R", 0.6, (0.0, 0.0, 0.0, 0.0), &[(Generic, "1 2")]);

    let p = c.add("GEN.RAIL-TS35-1M", "Mounting rail TS 35 x 7.5, perforated, 1 m", "Mounting", "Mounting rail", "", 6.5, (1000.0, 35.0, 7.5, 0.35), &[]);
    p.quantity_unit = "m".into();
    p.mounting.clear();
    for (number, desc, price, dims) in [
        ("GEN.RAIL-TS35X15-1M", "Mounting rail TS 35 x 15, perforated, 1 m", 9.8, (1000.0, 35.0, 15.0, 0.6)),
        ("GEN.RAIL-TS15-1M", "Mounting rail TS 15 x 5.5, 1 m", 5.2, (1000.0, 15.0, 5.5, 0.15)),
        ("GEN.RAIL-G32-1M", "Mounting rail G 32, 1 m", 8.9, (1000.0, 32.0, 15.0, 0.55)),
    ] {
        let p = c.add(number, desc, "Mounting", "Mounting rail", "", price, dims, &[]);
        p.quantity_unit = "m".into();
        p.mounting.clear();
    }
    let p = c.add("GEN.DUCT-40X60", "Cable duct 40 x 60 mm, 2 m", "Mounting", "Cable duct", "", 11.0, (2000.0, 40.0, 60.0, 0.9), &[]);
    p.pack_quantity = 10;
    p.mounting.clear();
    let p = c.add("GEN.DUCT-25X30", "Cable duct 25 x 30 mm, 2 m, for doors", "Mounting", "Cable duct", "", 6.5, (2000.0, 25.0, 30.0, 0.4), &[]);
    p.pack_quantity = 10;
    p.mounting.clear();
    for (number, desc, price, dims) in [
        ("GEN.DUCT-25X40", "Cable duct 25 x 40 mm, 2 m", 7.4, (2000.0, 25.0, 40.0, 0.5)),
        ("GEN.DUCT-40X40", "Cable duct 40 x 40 mm, 2 m", 9.6, (2000.0, 40.0, 40.0, 0.7)),
        ("GEN.DUCT-60X60", "Cable duct 60 x 60 mm, 2 m", 13.5, (2000.0, 60.0, 60.0, 1.1)),
        ("GEN.DUCT-60X80", "Cable duct 60 x 80 mm, 2 m", 15.9, (2000.0, 60.0, 80.0, 1.3)),
        ("GEN.DUCT-80X80", "Cable duct 80 x 80 mm, 2 m", 18.7, (2000.0, 80.0, 80.0, 1.5)),
        ("GEN.DUCT-100X80", "Cable duct 100 x 80 mm, 2 m", 22.4, (2000.0, 100.0, 80.0, 1.8)),
        ("GEN.DUCT-100X120", "Cable duct 100 x 120 mm, 2 m", 31.5, (2000.0, 100.0, 120.0, 2.4)),
    ] {
        let p = c.add(number, desc, "Mounting", "Cable duct", "", price, dims, &[]);
        p.pack_quantity = 10;
        p.mounting.clear();
    }
    let p = c.add("GEN.ENC-600X800", "Steel enclosure 600 x 800 x 250 mm, IP66", "Mounting", "Enclosure", "", 320.0, (600.0, 800.0, 250.0, 28.0), &[]);
    p.mounting.clear();
    // Busbar system: width = length of 1 m, height = width of the system, depth = height above the plate.
    let p = c.add("GEN.BB-3X12X5", "Busbar system 3-pole, Cu 12 x 5 mm, 60 mm centre distance", "Mounting", "Busbar system", "", 38.0, (1000.0, 75.0, 40.0, 1.6), &[]);
    p.quantity_unit = "m".into();
    p.mounting.clear();
    p.tech_data = "Bars: 3\nBar: Cu 12 x 5 mm\nRated current: 200 A".into();
    for (number, desc, price, dims, data) in [
        ("GEN.BB-3X20X5", "Busbar system 3-pole, Cu 20 x 5 mm, 60 mm centre distance", 52.0, (1000.0, 75.0, 40.0, 2.7), "Bars: 3\nBar: Cu 20 x 5 mm\nRated current: 320 A"),
        ("GEN.BB-3X30X10", "Busbar system 3-pole, Cu 30 x 10 mm, 60 mm centre distance", 118.0, (1000.0, 90.0, 50.0, 8.1), "Bars: 3\nBar: Cu 30 x 10 mm\nRated current: 800 A"),
        ("GEN.BB-4X12X5", "Busbar system 4-pole, Cu 12 x 5 mm, 40 mm centre distance", 49.0, (1000.0, 135.0, 40.0, 2.1), "Bars: 4\nBar: Cu 12 x 5 mm\nRated current: 200 A"),
    ] {
        let p = c.add(number, desc, "Mounting", "Busbar system", "", price, dims, &[]);
        p.quantity_unit = "m".into();
        p.mounting.clear();
        p.tech_data = data.into();
    }
    let p = c.add("GEN.BB-HOLDER-3P", "Busbar holder 3-pole for 12 x 5 mm, 60 mm", "Mounting", "Busbar holder", "", 6.4, (75.0, 25.0, 40.0, 0.08), &[]);
    p.mounting.clear();
    enclosure_systems(&mut c);
    fasteners(&mut c);
    accessories(&mut c);
    rack_parts(&mut c);

    // Ferrules (DIN 46228-4 colour code), sold per piece.
    for (cs, color, len) in [
        ("0.34", "turquoise", 8),
        ("0.5", "white", 8),
        ("0.75", "grey", 8),
        ("1.0", "red", 8),
        ("1.5", "black", 8),
        ("2.5", "blue", 8),
        ("4", "grey", 10),
        ("6", "yellow", 12),
        ("10", "red", 12),
        ("16", "blue", 12),
    ] {
        let number = format!("GEN.FER-{cs}");
        // The number stays as it was; the cross-section is written like the others (1 mm²).
        let shown = cs.strip_suffix(".0").unwrap_or(cs);
        let desc = format!("Ferrule {shown} mm², insulated, {color}, {len} mm");
        let p = c.add(&number, &desc, "Wiring material", "Ferrule", "", 0.03, (0.0, 0.0, 0.0, 0.0), &[]);
        p.mounting.clear();
        p.tech_data = format!("Cross-section: {shown} mm²\nColour: {color}\nLength: {len} mm");
    }

    // Cables: sold by the metre; the cores are listed in the technical data.
    for (number, desc, subgroup, price, kg, cores, cs) in [
        ("GEN.CBL-4G1.5", "Power cable 4G1.5, rubber sheathed", "Power cable", 2.4, 0.12, "BN BK GY GNYE", "1.5 mm²"),
        ("GEN.CBL-5G2.5", "Power cable 5G2.5, rubber sheathed", "Power cable", 4.1, 0.2, "BN BK GY BU GNYE", "2.5 mm²"),
        ("GEN.CBL-7G1.0", "Control cable 7G1.0, numbered cores", "Control cable", 2.9, 0.11, "1 2 3 4 5 6 GNYE", "1 mm²"),
        ("GEN.CBL-SENS-3X0.34", "Sensor cable M12 3 x 0.34, PUR", "Control cable", 1.6, 0.04, "BN BU BK", "0.34 mm²"),
        ("GEN.CBL-NAYY-4X150", "Power cable NAYY-J 4 x 150 SE 0.6/1 kV, aluminium, for laying in the ground", "Power cable", 14.5, 2.2, "BN BK GY GNYE", "150 mm²"),
        ("GEN.CBL-NYY-4X150", "Power cable NYY-J 4 x 150 SM 0.6/1 kV", "Power cable", 52.0, 6.3, "BN BK GY GNYE", "150 mm²"),
        ("GEN.CBL-NYY-5X35", "Power cable NYY-J 5 x 35 RM 0.6/1 kV", "Power cable", 19.5, 2.2, "BN BK GY BU GNYE", "35 mm²"),
        ("GEN.CBL-NYY-5X25", "Power cable NYY-J 5 x 25 RM 0.6/1 kV", "Power cable", 14.2, 1.6, "BN BK GY BU GNYE", "25 mm²"),
        ("GEN.CBL-NYY-5X16", "Power cable NYY-J 5 x 16 RM 0.6/1 kV", "Power cable", 9.6, 1.1, "BN BK GY BU GNYE", "16 mm²"),
        ("GEN.CBL-NYM-3X10", "Installation cable NYM-J 3 x 10", "Power cable", 5.8, 0.5, "BN BU GNYE", "10 mm²"),
        ("GEN.CBL-NYM-3X2.5", "Installation cable NYM-J 3 x 2.5", "Power cable", 1.4, 0.15, "BN BU GNYE", "2.5 mm²"),
        ("GEN.CBL-PV-2X6", "Solar cable H1Z2Z2-K 2 x 1 x 6 mm², 1500 V DC, red / black", "Power cable", 2.6, 0.14, "RD BK", "6 mm²"),
        ("GEN.CBL-BAT-2X35", "Battery cable H07V-K 2 x 1 x 35 mm², red / black", "Power cable", 9.8, 0.75, "RD BK", "35 mm²"),
        ("GEN.CBL-BAT-2X70", "Battery cable H07V-K 2 x 1 x 70 mm², red / black", "Power cable", 18.9, 1.45, "RD BK", "70 mm²"),
        ("GEN.CBL-RJ45-CAT6", "Patch cable RJ45 Cat 6, U/UTP, 4 pairs", "Data cable", 0.9, 0.04, "RJ45", "0.14 mm²"),
    ] {
        let p = c.add(number, desc, "Cables", subgroup, "W", price, (0.0, 0.0, 0.0, kg), &[]);
        p.quantity_unit = "m".into();
        p.tech_data = format!("Cores: {cores}\nCross-section: {cs}");
    }
    // Flat cables: ribbon cables for IDC connectors (core 1 marked red) and flat
    // cables for festoon systems; the cores lie side by side.
    for n in [10, 14, 16, 20, 26, 34, 40, 50, 64] {
        let p = c.add(&format!("GEN.CBL-FLAT-{n}"), &format!("Ribbon cable {n} cores, AWG 28, pitch 1.27 mm, grey, core 1 marked red"), "Cables", "Ribbon cable", "W", 0.06 * n as f32, (0.0, 0.0, 0.0, 0.004 * n as f32), &[]);
        p.quantity_unit = "m".into();
        let cores: Vec<String> = (1..=n).map(|i| i.to_string()).collect();
        p.tech_data = format!("Cores: {}\nCross-section: 0.08 mm²\nCable form: Flat\nPitch: 1.27 mm\nThickness: 0.9 mm", cores.join(" "));
    }
    for (number, desc, price, kg, cores, cs, width, thick) in [
        ("GEN.CBL-FLAT-4G1.5", "Flat cable H07VVH6-F 4G1.5 for festoon systems", 2.8, 0.13, "1 2 3 GNYE", "1.5 mm²", 18.0, 5.0),
        ("GEN.CBL-FLAT-7G1.5", "Flat cable H07VVH6-F 7G1.5 for festoon systems", 4.6, 0.22, "1 2 3 4 5 6 GNYE", "1.5 mm²", 30.0, 5.0),
        ("GEN.CBL-FLAT-12G1.0", "Flat control cable 12G1.0 for festoon systems", 6.2, 0.27, "1 2 3 4 5 6 7 8 9 10 11 GNYE", "1 mm²", 42.0, 4.4),
    ] {
        let p = c.add(number, desc, "Cables", "Flat cable", "W", price, (0.0, 0.0, 0.0, kg), &[]);
        p.quantity_unit = "m".into();
        p.tech_data = format!("Cores: {cores}\nCross-section: {cs}\nCable form: Flat\nWidth: {width} mm\nThickness: {thick} mm");
    }

    let mut parts = c.parts;
    parts.iter_mut().for_each(add_generated_models);
    parts.iter_mut().for_each(|p| {
        p.migrate_tech_data();
    });
    parts.sort_by(|a, b| natural_cmp(&a.number, &b.number));
    parts
}

/// Gives a part without models a simple 2D front view and 3D model of its dimensions
/// (nothing for parts without dimensions).
pub fn add_generated_models(p: &mut Part) {
    if p.model_2d.is_some() || p.model_3d.is_some() {
        return;
    }
    if let Some((m2, m3)) = generated_models(p) {
        p.model_2d = Some(m2.into());
        p.model_3d = Some(m3.into());
    }
}

/// The standard sizes of the enclosure systems with their components (side panels,
/// rear walls, plinths, gland plates).
fn enclosure_systems(c: &mut C) {
    use crate::mechanics::*;
    let mut seen: Vec<String> = Vec::new();
    let mut add = |c: &mut C, number: String, desc: String, subgroup: &str, price: f32, dims: (f32, f32, f32, f32), tech: String| {
        if seen.contains(&number) {
            return;
        }
        seen.push(number.clone());
        let p = c.add(&number, &desc, "Mounting", subgroup, "", (price * 100.0).round() / 100.0, dims, &[]);
        p.mounting.clear();
        p.tech_data = tech;
    };
    for s in SYSTEMS {
        for &(w, h, d) in s.sizes {
            let area = w * h / 1e6;
            let price = s.price.0.max(s.price.1 * area);
            let kg = area * if s.material == "plastic" { 6.0 } else { 9.0 } * (1.0 + d / 1000.0) + if s.bayable { 25.0 } else { 0.0 };
            let tech = format!("Enclosure system: {}\nProtection: {}\nWall: {} mm", s.id, s.ip, s.wall);
            add(c, body_number(s, w, h, d), format!("{} {w:.0} x {h:.0} x {d:.0} mm, {}", s.body, s.ip), "Enclosure", price, (w, h, d, kg), tech);
            add(c, gland_plate_number(w, d), format!("Gland plate {w:.0} x {d:.0} mm, sheet steel, with seal"), "Gland plate", 18.0 + 60.0 * w * d / 1e6, (w, d, 2.0, 0.0), String::new());
            if s.bayable {
                add(c, side_number(s, h, d), format!("Side panel {h:.0} x {d:.0} mm for floor-standing enclosures, pair"), "Side panel", 95.0 + 80.0 * h * d / 1e6, (d, h, s.wall, 0.0), String::new());
                add(c, rear_number(s, w, h), format!("Rear wall {w:.0} x {h:.0} mm, screw-fastened"), "Rear wall", 70.0 + 60.0 * w * h / 1e6, (w, h, s.wall, 0.0), String::new());
            }
            if s.id == RACK_SYSTEM {
                let u = rack_units(h);
                add(c, rack_rail_number(u), format!("19-inch mounting rails {u} U, pair, with cage nut holes"), "19-inch rail", 30.0 + 2.5 * u as f32, (15.0, u as f32 * RACK_UNIT, 20.0, 0.0), String::new());
            }
            if s.plinth > 0.0 {
                add(c, plinth_number(w, d, s.plinth), format!("Plinth {w:.0} x {d:.0} mm, {:.0} mm high, corner pieces and trims", s.plinth), "Plinth", 85.0 + 0.1 * (w + d), (w, s.plinth, d, 0.0), String::new());
            }
        }
    }
    let p = c.add(BAYING_KIT, "Baying kit for floor-standing enclosures (6 clamps, seal)", "Mounting", "Baying kit", "", 24.0, (0.0, 0.0, 0.0, 0.4), &[]);
    p.mounting.clear();
    let p = c.add(RAIL_STANDOFF, "Mounting rail standoff 40 mm, M5", "Mounting", "Rail standoff", "", 1.2, (20.0, 20.0, 40.0, 0.02), &[]);
    p.mounting.clear();
}

/// Parts for 19-inch racks, as wide as a 19-inch panel and a whole number of rack
/// units high (technical data `Rack units`): network and power devices with a device
/// tag, blank panels, shelves and cable management without one.
fn rack_parts(c: &mut C) {
    use crate::mechanics::{RACK_UNIT, RACK_WIDTH};
    use FnKind::*;
    let dims = |u: f32, d: f32, kg: f32| (RACK_WIDTH, u * RACK_UNIT, d, kg);
    for (number, desc, ident, price, (u, d, kg), functions, power) in [
        ("GEN.RK-SWITCH-24P", "Network switch 19-inch, 24 x RJ45 1 Gbit/s, 4 x SFP, managed, 1 U", "KF", 690.0, (1.0, 250.0, 3.2), &[(Generic, "L N")][..], 25.0),
        ("GEN.RK-PATCH-24P", "Patch panel 19-inch, 24 x RJ45 Cat 6A, shielded, 1 U", "XD", 95.0, (1.0, 120.0, 1.1), &[][..], 0.0),
        ("GEN.RK-PDU-8", "Power distribution unit 19-inch, 8 sockets 230 V, 16 A, 1 U", "XD", 120.0, (1.0, 60.0, 1.4), &[(SocketOutlet, "L N PE")][..], 0.0),
        ("GEN.RK-SERVER-2U", "Industrial server 19-inch, 2 U, redundant power supply", "KF", 4200.0, (2.0, 550.0, 18.0), &[(Generic, "L N")][..], 350.0),
        ("GEN.RK-UPS-2U", "UPS 19-inch, 1500 VA, line interactive, 2 U", "GU", 980.0, (2.0, 450.0, 22.0), &[(Generic, "L N")][..], 40.0),
    ] {
        let p = c.add(number, desc, "General", "19-inch device", ident, price, dims(u, d, kg), functions);
        // The power inlet at the rear, on the right, its cable up along the rack.
        let (w, h) = (RACK_WIDTH, u * RACK_UNIT);
        let pins: Vec<String> = p.functions.iter().flat_map(|f| f.pin_list()).collect();
        p.connection_points = (pins.iter().enumerate())
            .map(|(i, pin)| crate::ConnectionPoint { pin: pin.clone(), pos: [w * 0.8 + 10.0 * i as f32, h / 2.0, 0.0], exit: crate::Exit::Up })
            .collect();
        p.voltage = if functions.is_empty() { String::new() } else { "230 V AC".into() };
        p.tech_data = if power > 0.0 { format!("Rack units: {u:.0}
Power loss: {power:.0} W") } else { format!("Rack units: {u:.0}") };
    }
    for (number, desc, price, (u, d, kg)) in [
        ("GEN.RK-BLANK-1U", "Blank panel 19-inch, 1 U, sheet steel", 6.5, (1.0, 10.0, 0.3)),
        ("GEN.RK-BLANK-2U", "Blank panel 19-inch, 2 U, sheet steel", 8.9, (2.0, 10.0, 0.5)),
        ("GEN.RK-SHELF-1U", "Shelf 19-inch, 1 U, 400 mm deep, max. 25 kg", 38.0, (1.0, 400.0, 2.4)),
        ("GEN.RK-CABLE-1U", "Cable management panel 19-inch with 5 brush rings, 1 U", 21.0, (1.0, 70.0, 0.6)),
    ] {
        let p = c.add(number, desc, "Mounting", "19-inch accessory", "", price, dims(u, d, kg), &[]);
        p.mounting.clear();
        p.tech_data = format!("Rack units: {u:.0}
Faces: Plate");
    }
}

/// Screws, nuts, washers and rivet nuts of the fastening systems.
fn fasteners(c: &mut C) {
    use crate::mechanics::*;
    for &(t, len) in THREADS {
        let m: f32 = t[1..].parse().unwrap_or(5.0);
        for (number, desc, subgroup, price) in [
            (screw_number(t), format!("Combination screw {t} x {len}, ISO 7045, steel zinc plated"), "Screw", 0.04 * m),
            (nut_number(t), format!("Hexagon nut {t}, ISO 4032, steel zinc plated"), "Nut", 0.02 * m),
            (washer_number(t), format!("Washer {t}, ISO 7089, steel zinc plated"), "Washer", 0.01 * m),
            (rivet_nut_number(t), format!("Rivet nut {t}, steel zinc plated, flat head"), "Rivet nut", 0.06 * m),
        ] {
            let p = c.add(&number, &desc, "Mounting", subgroup, "", (price * 100.0).round() / 100.0, (0.0, 0.0, 0.0, 0.0), &[]);
            p.pack_quantity = 100;
            p.mounting.clear();
        }
    }
}

/// Enclosure accessories placed without a device tag: fans, filters, glands, lights,
/// handles. Technical data: `Cutout` (opening in the face), `Outside` (protrusion
/// outside the face), `Faces` (where it can be mounted), `Fastening`.
fn accessories(c: &mut C) {
    for (number, desc, subgroup, price, dims, tech) in [
        ("GEN.FAN-FILTER-120", "Filter fan 120 m³/h, 230 V AC, 204 x 204 mm, IP54", "Filter fan", 98.0, (204.0, 204.0, 105.0, 1.6), "Cutout: 177 x 177\nOutside: 25\nAir flow: 120 m³/h\nFaces: Left, Right, Door, Rear\nFastening: Clip"),
        ("GEN.FAN-OUTLET-120", "Outlet filter for filter fan 120 m³/h, 204 x 204 mm", "Outlet filter", 32.0, (204.0, 204.0, 30.0, 0.4), "Cutout: 177 x 177\nOutside: 25\nFaces: Left, Right, Door, Rear\nFastening: Clip"),
        ("GEN.GLAND-M20", "Cable gland M20 x 1.5, polyamide, IP68, cable Ø 6 – 12 mm", "Cable gland", 1.4, (30.0, 30.0, 12.0, 0.01), "Cutout: Ø20.5\nOutside: 20\nFaces: Bottom, Roof, Left, Right, Rear\nFastening: Clip\nClamping range min: 6\nClamping range max: 12"),
        ("GEN.GLAND-M25", "Cable gland M25 x 1.5, polyamide, IP68, cable Ø 9 – 17 mm", "Cable gland", 1.9, (36.0, 36.0, 13.0, 0.02), "Cutout: Ø25.5\nOutside: 24\nFaces: Bottom, Roof, Left, Right, Rear\nFastening: Clip\nClamping range min: 9\nClamping range max: 17"),
        ("GEN.GLAND-M32", "Cable gland M32 x 1.5, polyamide, IP68, cable Ø 13 – 21 mm", "Cable gland", 2.8, (46.0, 46.0, 15.0, 0.03), "Cutout: Ø32.5\nOutside: 28\nFaces: Bottom, Roof, Left, Right, Rear\nFastening: Clip\nClamping range min: 13\nClamping range max: 21"),
        ("GEN.LIGHT-LED-350", "Enclosure LED light 5 W, 230 V AC, 350 mm, with door switch input", "Enclosure light", 46.0, (350.0, 30.0, 25.0, 0.2), "Mounting holes: M4: 10,15; 340,15\nFaces: Plate, Roof, Left, Right, Door"),
        ("GEN.PE-BAR-10", "PE busbar 10 x 3 mm with 10 screw terminals, 200 mm", "PE busbar", 9.5, (200.0, 30.0, 30.0, 0.15), "Mounting holes: M5: 10,15; 190,15\nFaces: Plate, Bottom, Left, Right"),
        ("GEN.CLAMP-RAIL-500", "Cable clamp rail C 30, 500 mm, for strain relief", "Cable clamp rail", 7.8, (500.0, 30.0, 15.0, 0.3), "Mounting holes: M6: 15,15; 250,15; 485,15\nFaces: Plate, Bottom, Left, Right, Rear"),
        ("GEN.HANDLE-SWING", "Swing handle, lockable, with double bit insert", "Door handle", 29.0, (40.0, 160.0, 30.0, 0.35), "Cutout: 25 x 110\nOutside: 35\nFaces: Door\nFastening: Clip"),
        ("GEN.DOC-HOLDER-A4", "Document holder A4, plastic, for the inside of doors", "Document holder", 12.0, (240.0, 230.0, 30.0, 0.2), "Mounting holes: M4: 15,15; 225,15\nFaces: Door, Left, Right\nFastening: Rivet nut"),
    ] {
        let p = c.add(number, desc, "Mounting", subgroup, "", price, dims, &[]);
        p.mounting.clear();
        p.tech_data = tech.into();
    }
}

/// Shape family of the generated example models.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    /// DIN rail device: base, raised front, screw terminals top and bottom.
    Din { toggle: bool, display: bool },
    /// Feed-through terminal: a slice with the stepped profile of a terminal block.
    Terminal,
    /// End plate of a terminal block: the profile without openings.
    EndPlate,
    /// End bracket: low clamp on the rail with its screw.
    EndBracket,
    /// PLC I/O module: tall block with an LED / connector column.
    Plc,
    /// Contactor or relay: DIN device with a dark front cap and the switch position
    /// indicator.
    Contactor,
    /// Door / field device (pushbutton, lamp, sensor): round front.
    Round,
}

fn shape_of(p: &Part) -> Option<Shape> {
    if p.width <= 0.0 || p.height <= 0.0 || p.depth <= 0.0 || p.group == "Mounting" {
        return None;
    }
    Some(match p.subgroup.as_str() {
        "Feed-through terminal" | "PE terminal" | "N terminal" | "Disconnect terminal" | "Fuse terminal" => Shape::Terminal,
        "Accessories" if p.group == "Terminals" => Shape::EndPlate,
        "End bracket" => Shape::EndBracket,
        "Digital input module" | "Digital output module" | "Analog input module" | "Analog output module" | "CPU" => Shape::Plc,
        "Circuit breaker" | "Residual current device" | "Motor protection switch" | "Auxiliary switch" | "Main switch" => Shape::Din { toggle: true, display: false },
        "Power supply" | "Timer relay" | "Safety relay" | "Frequency converter" | "Soft starter" => Shape::Din { toggle: false, display: true },
        "Temperature switch" | "Current transformer" | "Socket outlet" => Shape::Din { toggle: false, display: false },
        "Power contactor" | "Relay" | "Auxiliary contact block" | "Overload relay" => Shape::Contactor,
        "Energy meter" | "Battery monitor" | "Measuring transducer" => Shape::Din { toggle: false, display: true },
        _ if matches!(p.group.as_str(), "Sensors, switches, pushbuttons" | "Signal devices" | "Measuring instruments") => Shape::Round,
        _ => Shape::Din { toggle: false, display: false },
    })
}

/// Simple 2D front view and 3D model with exactly the part's mounting dimensions.
fn generated_models(p: &Part) -> Option<(Model2d, Mesh3d)> {
    if p.group == "Mounting" {
        return mounting::model(p);
    }
    if p.width > 0.0 && p.height > 0.0 && p.depth > 0.0 {
        if let Some(m) = models::model(p) {
            return Some(m);
        }
    }
    let shape = shape_of(p)?;
    let (w, h, d) = (p.width, p.height, p.depth);
    let mut m3 = Mesh3d::default();
    let mut prims: Vec<Prim2d> = vec![Prim2d::Rect { min: [0.0, 0.0], max: [w, h], fill: false }];
    // 2D uses y down from the top, 3D y up from the bottom.
    let rect = |prims: &mut Vec<Prim2d>, x0: f32, y0: f32, x1: f32, y1: f32, fill: bool| {
        prims.push(Prim2d::Rect { min: [x0, h - y1], max: [x1, h - y0], fill })
    };
    match shape {
        Shape::Din { toggle, display } => {
            let base = d * 0.6;
            m3.add_painted_box([0.0, 0.0, 0.0], [w, h, base], models::BODY);
            // Raised front in the middle third
            let (f0, f1) = (h * 0.3, h * 0.7);
            m3.add_painted_box([w * 0.05, f0, base], [w * 0.95, f1, if toggle || display { d * 0.92 } else { d }], models::BODY);
            rect(&mut prims, w * 0.05, f0, w * 0.95, f1, false);
            // Screw terminals on the steps above and below the front, one per connection
            // point of the row
            let points = p.connection_points_or_default().0;
            for (exit, cy) in [(crate::Exit::Up, h * 0.85), (crate::Exit::Down, h * 0.15)] {
                let xs: Vec<f32> = points.iter().filter(|c| c.exit == exit).map(|c| c.pos[0]).collect();
                let r = (w / xs.len().max(1) as f32 * 0.28).min(3.0);
                for cx in xs {
                    m3.add_painted_box([cx - r, cy - r, base], [cx + r, cy + r, base + 1.5], models::DARK);
                    prims.push(Prim2d::Circle { c: [cx, h - cy], r, fill: false });
                    prims.push(Prim2d::Line { a: [cx - r * 0.7, h - cy], b: [cx + r * 0.7, h - cy] });
                }
            }
            if toggle {
                let (tw, th) = ((w * 0.35).min(8.0), h * 0.12);
                let (cx, cy) = (w / 2.0, h / 2.0);
                m3.add_painted_box([cx - tw / 2.0, cy - th / 2.0, d * 0.92], [cx + tw / 2.0, cy + th / 2.0, d], models::BLACK);
                rect(&mut prims, cx - tw / 2.0, cy - th / 2.0, cx + tw / 2.0, cy + th / 2.0, true);
            }
            if display {
                let (cx, cy) = (w / 2.0, h * 0.58);
                let (dw, dh) = (w * 0.6, h * 0.08);
                m3.add_painted_box([cx - dw / 2.0, cy - dh / 2.0, d * 0.92], [cx + dw / 2.0, cy + dh / 2.0, d], models::DISPLAY);
                rect(&mut prims, cx - dw / 2.0, cy - dh / 2.0, cx + dw / 2.0, cy + dh / 2.0, false);
                prims.push(Prim2d::Circle { c: [cx, h - h * 0.42], r: (w * 0.08).min(3.0), fill: false });
            }
        }
        Shape::Terminal | Shape::EndPlate => {
            // Grey; protective earth green with a yellow marker field, neutral blue; end
            // plates darker, without openings.
            let (color, marker) = match p.subgroup.as_str() {
                "PE terminal" => ([84, 160, 64], [232, 204, 48]),
                "N terminal" => ([66, 118, 196], models::WHITE),
                _ if shape == Shape::EndPlate => ([118, 122, 128], [118, 122, 128]),
                _ => ([168, 172, 174], models::WHITE),
            };
            // Screw terminals where the part says so, push-in otherwise.
            let clamp = if p.description.to_lowercase().contains("screw") { Clamp::Screw } else { Clamp::PushIn };
            terminal_slice(&mut m3, w, h, d, color, (shape == Shape::Terminal).then_some((marker, clamp)));
            if shape == Shape::Terminal {
                let r = (w * 0.3).min(2.0);
                for cy in [h * 0.2, h * 0.8] {
                    prims.push(Prim2d::Circle { c: [w / 2.0, h - cy], r, fill: false });
                }
                rect(&mut prims, w * 0.15, h * 0.42, w * 0.85, h * 0.58, true);
            }
            rect(&mut prims, 0.0, h * 0.28, w, h * 0.72, false);
        }
        Shape::EndBracket => {
            // Dark clamp over the rail with a raised middle and its clamping screw.
            let c = [92, 96, 104];
            m3.add_painted_box([0.0, 0.0, 0.0], [w, h, d * 0.3], c);
            m3.add_painted_box([0.0, h * 0.1, d * 0.3], [w, h * 0.9, d * 0.55], c);
            m3.add_painted_box([w * 0.1, h * 0.3, d * 0.55], [w * 0.9, h * 0.7, d * 0.94], c);
            let s = (w * 0.3).min(2.5);
            m3.add_painted_box([w / 2.0 - s, h / 2.0 - s, d * 0.94], [w / 2.0 + s, h / 2.0 + s, d], models::STEEL);
            rect(&mut prims, 0.0, h * 0.1, w, h * 0.9, false);
            rect(&mut prims, w * 0.1, h * 0.3, w * 0.9, h * 0.7, false);
            prims.push(Prim2d::Circle { c: [w / 2.0, h / 2.0], r: s, fill: false });
        }
        Shape::Plc => {
            // Dark housing, the front connector with its LED column and terminal block.
            m3.add_painted_box([0.0, 0.0, 0.0], [w, h, d * 0.85], models::DARK);
            m3.add_painted_box([w * 0.15, h * 0.08, d * 0.85], [w * 0.85, h * 0.92, d * 0.97], [104, 110, 120]);
            rect(&mut prims, w * 0.15, h * 0.08, w * 0.85, h * 0.92, false);
            for i in 0..8 {
                let y = h * 0.14 + i as f32 * h * 0.095;
                m3.add_painted_box([w * 0.25, h - y - h * 0.03, d * 0.97], [w * 0.4, h - y, d], [72, 196, 92]);
                m3.add_painted_box([w * 0.5, h - y - h * 0.05, d * 0.97], [w * 0.78, h - y, d * 0.99], [60, 150, 72]);
                prims.push(Prim2d::Rect { min: [w * 0.25, y], max: [w * 0.4, y + h * 0.03], fill: true });
                prims.push(Prim2d::Line { a: [w * 0.5, y + h * 0.015], b: [w * 0.75, y + h * 0.015] });
            }
        }
        Shape::Contactor => {
            m3.add_painted_box([0.0, 0.0, 0.0], [w, h, d * 0.7], models::BODY);
            let points = p.connection_points_or_default().0;
            for (exit, cy) in [(crate::Exit::Up, h * 0.94), (crate::Exit::Down, h * 0.06)] {
                let xs: Vec<f32> = points.iter().filter(|c| c.exit == exit).map(|c| c.pos[0]).collect();
                let r = (w / xs.len().max(1) as f32 * 0.3).min(3.0);
                for cx in xs {
                    m3.add_painted_box([cx - r, cy - h * 0.04, d * 0.7], [cx + r, cy + h * 0.04, d * 0.72], models::DARK);
                }
            }
            m3.add_painted_box([w * 0.06, h * 0.2, d * 0.7], [w * 0.94, h * 0.8, d * 0.96], [112, 116, 122]);
            m3.add_painted_box([w * 0.38, h * 0.44, d * 0.96], [w * 0.62, h * 0.56, d], [232, 232, 228]);
            m3.add_painted_box([w * 0.15, h * 0.66, d * 0.96], [w * 0.85, h * 0.74, d * 0.98], [228, 228, 224]);
            rect(&mut prims, w * 0.06, h * 0.2, w * 0.94, h * 0.8, false);
            rect(&mut prims, w * 0.38, h * 0.44, w * 0.62, h * 0.56, true);
        }
        Shape::Round => {
            m3.add_painted_box([0.0, 0.0, 0.0], [w, h, d * 0.8], models::DARK);
            m3.add_painted_box([w * 0.15, h * 0.15, d * 0.8], [w * 0.85, h * 0.85, d], [188, 192, 198]);
            prims.push(Prim2d::Circle { c: [w / 2.0, h / 2.0], r: w.min(h) * 0.42, fill: false });
            prims.push(Prim2d::Circle { c: [w / 2.0, h / 2.0], r: w.min(h) * 0.3, fill: true });
        }
    }
    Some((Model2d { width: w, height: h, prims }, m3))
}

/// How the conductors of a terminal are clamped.
#[derive(Clone, Copy, PartialEq)]
enum Clamp {
    /// Push-in: the wire entry in the end face, the pusher on the step behind it.
    PushIn,
    /// Screw: the wire entry in the end face, the screw in an opening of the step.
    Screw,
}

/// Height of the end of a terminal (wire entries), of its steps (pushers or screws) and
/// of the middle, from the rail to the front, as parts of the depth.
const TERMINAL_PROFILE: [f32; 3] = [0.6, 0.82, 1.0];

/// A terminal slice (x across, y along with the rail in the middle, z to the front):
/// low ends with the wire entries, steps with the pushers or the recessed screws, the
/// high middle with the marker field and the jumper shafts (`front`: the marker colour
/// and the clamp; none on an end plate). Above the foot a hairline gap is left on both
/// sides, so that the slices of a strip are seen.
fn terminal_slice(m: &mut Mesh3d, w: f32, h: f32, d: f32, c: [u8; 3], front: Option<([u8; 3], Clamp)>) {
    let [end, step, _] = TERMINAL_PROFILE.map(|f| f * d);
    let g = (w * 0.03).min(0.15);
    let (x0, x1) = (g, w - g);
    let (hx0, hx1) = (w * 0.22, w * 0.78);
    let mut b = |min: [f32; 3], max: [f32; 3], c: [u8; 3]| m.add_painted_box(min, max, c);
    // Foot over the rail (the whole width: the slices touch there).
    b([0.0, 0.0, 0.0], [w, h, d * 0.25], c);
    // Ends (y from both sides): the wire entry in the end face, the screw step behind it.
    let mirror = |y: f32, upper: bool| if upper { h - y } else { y };
    for upper in [false, true] {
        let y = |a: f32, z: f32| {
            let (p, q) = (mirror(a, upper), mirror(z, upper));
            (p.min(q), p.max(q))
        };
        let (e0, e1) = y(0.0, h * 0.14);
        let (s0, s1) = y(h * 0.14, h * 0.28);
        let Some((_, clamp)) = front else {
            b([x0, e0, d * 0.25], [x1, e1, end], c);
            b([x0, s0, d * 0.25], [x1, s1, step], c);
            continue;
        };
        // Wire entry in the end face: walls beside, below and above the opening, its
        // dark inside.
        let (o0, o1) = y(h * 0.03, h * 0.14);
        let (z0, z1) = (d * 0.3, end - d * 0.1);
        b([x0, e0, d * 0.25], [hx0, e1, end], c);
        b([hx1, e0, d * 0.25], [x1, e1, end], c);
        b([hx0, e0, d * 0.25], [hx1, e1, z0], c);
        b([hx0, e0, z1], [hx1, e1, end], c);
        b([hx0, o0, z0], [hx1, o1, z1], DARK_INSIDE);
        match clamp {
            // The orange pusher on the step.
            Clamp::PushIn => {
                let (q0, q1) = y(h * 0.16, h * 0.26);
                b([x0, s0, d * 0.25], [x1, s1, step - d * 0.06], c);
                b([x0, s0, step - d * 0.06], [x1, q0, step], c);
                b([x0, q1, step - d * 0.06], [x1, s1, step], c);
                b([x0, q0, step - d * 0.06], [hx0, q1, step], c);
                b([hx1, q0, step - d * 0.06], [x1, q1, step], c);
                b([hx0, q0, step - d * 0.06], [hx1, q1, step - d * 0.01], PUSHER);
            }
            // Walls around the screw opening, the screw head with its slot at its bottom.
            Clamp::Screw => {
                let (q0, q1) = y(h * 0.15, h * 0.24);
                let floor = step - d * 0.12;
                b([x0, s0, d * 0.25], [x1, s1, floor], c);
                b([x0, s0, floor], [hx0, s1, step], c);
                b([hx1, s0, floor], [x1, s1, step], c);
                b([hx0, s0, floor], [hx1, q0, step], c);
                b([hx0, q1, floor], [hx1, s1, step], c);
                b([hx0, q0, floor - d * 0.02], [hx1, q1, floor], models::STEEL);
                let (qm, sl) = ((q0 + q1) / 2.0, h * 0.008);
                b([hx0, qm - sl, floor], [hx1, qm + sl, floor + d * 0.01], models::DARK);
            }
        }
    }
    // The high middle with the marker field and the jumper shafts beside it.
    let (m0, m1) = (h * 0.28, h * 0.72);
    match front {
        Some((mc, _)) => {
            b([x0, m0, d * 0.25], [x1, m1, d * 0.96], c);
            b([x0, m0, d * 0.96], [x1, h * 0.4, d], c);
            b([x0, h * 0.6, d * 0.96], [x1, m1, d], c);
            b([w * 0.15, h * 0.4, d * 0.96], [w * 0.85, h * 0.6, d], mc);
            for y in [h * 0.33, h * 0.67] {
                b([hx0, y - h * 0.015, d * 0.96], [hx1, y + h * 0.015, d * 0.965], DARK_INSIDE);
            }
        }
        None => b([x0, m0, d * 0.25], [x1, m1, d], c),
    }
}

/// The inside of the openings of a terminal (wire entries, jumper shafts).
const DARK_INSIDE: [u8; 3] = [46, 48, 52];
/// The pushers of push-in terminals.
const PUSHER: [u8; 3] = [236, 116, 34];

mod models;
mod mounting;

pub use models::{nozzles, shaft, Nozzle, Shaft};

/// Plug connectors with the `Pin layout` of their mating face: circular (M8, M12, GX
/// aviation connectors), D-sub, IDC sockets and box headers for ribbon cables,
/// dual-row housings, pluggable terminal blocks, RJ45 and heavy duty inserts; each
/// male and female.
fn connectors(c: &mut C) {
    let mut add = |number: String, desc: String, subgroup: &str, price: f32, labels: Vec<String>, tech: String| {
        let functions: Vec<(FnKind, &str)> = labels.iter().map(|l| (FnKind::PlugPin, l.as_str())).collect();
        let p = c.add(&number, &desc, "Connectors", subgroup, "X", price, (0.0, 0.0, 0.0, 0.02), &functions);
        p.mounting.clear();
        p.tech_data = tech;
    };
    let numbered = |n: usize| -> Vec<String> { (1..=n).map(|i| i.to_string()).collect() };
    let with_pe = |n: usize| -> Vec<String> { numbered(n).into_iter().chain(["PE".to_string()]).collect() };
    let genders = [("M", "male", "panel socket"), ("F", "female", "cable plug")];
    let circle = |ring_max: usize, n: usize| if n <= ring_max { format!("circle {n}") } else { format!("circle {}+1", n - 1) };
    // GX aviation connectors: the male part is the panel socket, the female one the cable plug.
    for (series, sizes, ring_max, amps, price) in [
        ("GX12", &[2, 3, 4, 5, 6, 7][..], 5, 5, 2.4),
        ("GX16", &[2, 3, 4, 5, 6, 7, 8, 9, 10], 6, 5, 3.1),
        ("GX20", &[2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 15], 7, 7, 4.2),
    ] {
        for &n in sizes {
            for (suffix, gender, kind) in genders {
                let desc = format!("Aviation connector {series}, {n} pins, {gender}, {kind}, {amps} A, IP55");
                let tech = format!("Pin layout: {}\nGender: {gender}\nSeries: {series}", circle(ring_max, n));
                add(format!("GEN.CON-{series}-{n}{suffix}"), desc, "Circular connector", price, numbered(n), tech);
            }
        }
    }
    // M8 and M12 sensor / actuator connectors (IEC 61076-2-101 / -104).
    for (series, labels, layout, coding, price) in [
        ("M8-3", "1 3 4", "circle 4", "A", 6.9),
        ("M8-4", "1 2 3 4", "circle 4", "A", 7.4),
        ("M12-4", "1 2 3 4", "circle 4", "A", 9.5),
        ("M12-5", "1 2 3 4 5", "circle 4+1", "A", 10.2),
        ("M12-8", "1 2 3 4 5 6 7 8", "circle 7+1", "A", 13.8),
        ("M12-12", "1 2 3 4 5 6 7 8 9 10 11 12", "circle 11+1", "A", 18.5),
        ("M12-4D", "1 2 3 4", "circle 4", "D", 14.9),
    ] {
        let labels: Vec<String> = labels.split(' ').map(String::from).collect();
        let n = labels.len();
        let size = series.split('-').next().unwrap_or(series);
        for (suffix, gender, _) in genders {
            // The female M12 4-pin keeps its number of earlier versions.
            let number = if series == "M12-4" && suffix == "F" { "GEN.CON-M12-4".to_string() } else { format!("GEN.CON-{series}{suffix}") };
            let desc = format!("Connector {size}, {n} pins, {coding}-coded, {gender}, field-wireable");
            let tech = format!("Pin layout: {layout}\nCoding: {coding}\nGender: {gender}\nSeries: {size}");
            add(number, desc, "Circular connector", price, labels.clone(), tech);
        }
    }
    // D-sub connectors.
    for (shell, n, price) in [("E", 9, 1.8), ("A", 15, 2.4), ("B", 25, 3.2), ("C", 37, 4.6)] {
        for (suffix, gender, _) in genders {
            let desc = format!("D-sub connector D{shell}-{n}, {n} pins, {gender}, solder cup, with metal hood");
            let tech = format!("Pin layout: dsub {n}\nPitch: 2.77 mm\nGender: {gender}\nSeries: D-sub");
            add(format!("GEN.CON-DSUB-{n}{suffix}"), desc, "D-sub connector", price, numbered(n), tech);
        }
    }
    // IDC sockets (on the ribbon cable) and the box headers they plug onto (DIN 41651).
    for n in [10, 14, 16, 20, 26, 34, 40, 50, 64] {
        let tech = |gender: &str| format!("Pin layout: zigzag {n}\nPitch: 2.54 mm\nTermination: IDC\nGender: {gender}\nSeries: IDC 2.54");
        let desc = format!("IDC socket {n} pins (2 x {}), pitch 2.54 mm, for ribbon cable 1.27 mm, with strain relief", n / 2);
        add(format!("GEN.CON-IDC-{n}F"), desc, "IDC connector", 0.6 + 0.02 * n as f32, numbered(n), tech("female"));
        let desc = format!("Box header {n} pins (2 x {}), pitch 2.54 mm, straight, polarised, for PCB", n / 2);
        add(format!("GEN.CON-BOX-{n}M"), desc, "IDC connector", 0.4 + 0.015 * n as f32, numbered(n), tech("male"));
    }
    // Dual-row wire-to-board housings, pitch 3.0 mm.
    for n in [2, 4, 6, 8, 10, 12, 16, 24] {
        let tech = |gender: &str| format!("Pin layout: grid 2x{}\nPitch: 3.0 mm\nGender: {gender}\nSeries: Dual row 3.0", n / 2);
        add(format!("GEN.CON-DR3-{n}F"), format!("Receptacle housing dual row, pitch 3.0 mm, {n} pins, female, with crimp contacts"), "Wire-to-board connector", 0.5 + 0.05 * n as f32, numbered(n), tech("female"));
        add(format!("GEN.CON-DR3-{n}M"), format!("Header dual row, pitch 3.0 mm, {n} pins, male, for PCB"), "Wire-to-board connector", 0.4 + 0.04 * n as f32, numbered(n), tech("male"));
    }
    // Pluggable terminal blocks, pitch 5.08 mm.
    for n in 2..=12 {
        let tech = |gender: &str| format!("Pin layout: row {n}\nPitch: 5.08 mm\nGender: {gender}\nSeries: Pluggable 5.08");
        add(format!("GEN.CON-PTB-{n}F"), format!("Pluggable terminal block, pitch 5.08 mm, {n} poles, screw connection, female"), "Pluggable terminal block", 0.5 + 0.25 * n as f32, numbered(n), tech("female"));
        add(format!("GEN.CON-PTB-{n}M"), format!("PCB header for pluggable terminal blocks, pitch 5.08 mm, {n} poles, male"), "Pluggable terminal block", 0.3 + 0.15 * n as f32, numbered(n), tech("male"));
    }
    // RJ45: jack and field-wireable plug.
    for (suffix, gender, desc) in [("F", "female", "RJ45 jack Cat 6A, shielded, 8 contacts, for panel mounting"), ("M", "male", "RJ45 plug Cat 6A, shielded, 8 contacts, field-wireable")] {
        add(format!("GEN.CON-RJ45-{suffix}"), desc.into(), "Data connector", 6.5, numbered(8), format!("Pin layout: row 8\nPitch: 1.02 mm\nGender: {gender}\nSeries: RJ45"));
    }
    // Heavy duty connector inserts.
    for (n, layout, amps, price) in [(3, "row 3+PE", 16, 14.0), (10, "grid 2x5+PE", 16, 26.0), (16, "grid 2x8+PE", 16, 38.0), (24, "grid 2x12+PE", 16, 52.0)] {
        for (suffix, gender, _) in genders {
            // The male 16-pin insert keeps its number of earlier versions.
            let number = if n == 16 && suffix == "M" { "GEN.CON-HD-16".to_string() } else { format!("GEN.CON-HD-{n}{suffix}") };
            let desc = format!("Heavy duty connector insert {n} pins + PE, {amps} A, {gender}");
            add(number, desc, "Heavy duty connector", price, with_pe(n), format!("Pin layout: {layout}\nGender: {gender}\nSeries: Heavy duty"));
        }
    }
}

#[cfg(test)]
mod tests;
