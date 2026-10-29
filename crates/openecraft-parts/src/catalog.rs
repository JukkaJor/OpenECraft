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

    let p = c.add("GEN.TB-2.5", "Feed-through terminal 2.5 mm², grey", "Terminals", "Feed-through terminal", "X", 0.95, (5.2, 48.0, 36.0, 0.006), &[(Terminal, "")]);
    p.tech_data = "End bracket: GEN.EB-35\nEnd plate: GEN.TB-END\nJumper: GEN.JMP-2.5-2, GEN.JMP-2.5-3, GEN.JMP-2.5-10".into();
    p.accessories = vec!["GEN.TB-END".into(), "GEN.EB-35".into(), "GEN.JMP-2.5-2".into(), "GEN.JMP-2.5-3".into(), "GEN.JMP-2.5-10".into()];
    let p = c.add("GEN.TB-PE-2.5", "Protective earth terminal 2.5 mm², green-yellow", "Terminals", "PE terminal", "X", 1.8, (5.2, 48.0, 36.0, 0.01), &[(Terminal, "")]);
    p.tech_data = "Terminal type: PE\nEnd bracket: GEN.EB-35\nEnd plate: GEN.TB-END".into();
    p.accessories = vec!["GEN.TB-END".into(), "GEN.EB-35".into()];
    let p = c.add("GEN.TB-N-2.5", "Neutral terminal 2.5 mm², blue", "Terminals", "N terminal", "X", 1.0, (5.2, 48.0, 36.0, 0.006), &[(Terminal, "")]);
    p.tech_data = "Terminal type: N\nEnd bracket: GEN.EB-35\nEnd plate: GEN.TB-END".into();
    p.accessories = vec!["GEN.TB-END".into(), "GEN.EB-35".into()];
    let p = c.add("GEN.TB-DISC-4", "Disconnect terminal 4 mm², knife disconnect, grey", "Terminals", "Disconnect terminal", "X", 2.6, (6.2, 60.0, 42.0, 0.01), &[(Terminal, "")]);
    p.tech_data = "Terminal type: Disconnect\nEnd bracket: GEN.EB-35".into();
    p.accessories = vec!["GEN.EB-35".into()];
    let p = c.add("GEN.TB-FUSE-4", "Fuse terminal 4 mm² for fuses 5 x 20 mm, with LED 24 V", "Terminals", "Fuse terminal", "X", 3.9, (8.2, 70.0, 50.0, 0.02), &[(Terminal, "")]);
    p.tech_data = "Terminal type: Fuse\nEnd bracket: GEN.EB-35".into();
    p.accessories = vec!["GEN.EB-35".into()];
    c.add("GEN.TB-END", "End plate for terminal 2.5 mm²", "Terminals", "Accessories", "X", 0.3, (1.5, 48.0, 36.0, 0.002), &[]);
    c.add("GEN.EB-35", "End bracket for TS 35 mounting rail", "Terminals", "End bracket", "X", 1.1, (9.0, 40.0, 36.0, 0.01), &[]);
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
    for (number, desc, pins) in [
        ("GEN.CON-M12-4", "Connector M12, 4 pins, A-coded, female, field-wireable", 4),
        ("GEN.CON-HD-16", "Heavy duty connector insert 16 pins + PE, 16 A, male", 17),
    ] {
        let functions: Vec<(FnKind, String)> = (1..=pins).map(|i| (PlugPin, if i > 16 { "PE".into() } else { i.to_string() })).collect();
        let functions: Vec<(FnKind, &str)> = functions.iter().map(|(k, s)| (*k, s.as_str())).collect();
        c.add(number, desc, "Connectors", "Plug connector", "X", if pins > 4 { 38.0 } else { 9.5 }, (0.0, 0.0, 0.0, 0.05), &functions);
    }
    let fl = "Fluid power components";
    c.add("GEN.CYL-32-100", "Pneumatic cylinder double-acting, bore 32 mm, stroke 100 mm", fl, "Cylinder", "MM", 82.0, (0.0, 0.0, 0.0, 0.7), &[(Cylinder, "1 2")]);
    c.add("GEN.CYL-20-50-SA", "Pneumatic cylinder single-acting, bore 20 mm, stroke 50 mm", fl, "Cylinder", "MM", 46.0, (0.0, 0.0, 0.0, 0.3), &[(Cylinder, "1")]);
    c.add("GEN.PUMP-HYD-4CC", "Hydraulic gear pump 4 cm³/rev, 200 bar", fl, "Pump", "GP", 190.0, (0.0, 0.0, 0.0, 1.8), &[(Pump, "P S")]);
    c.add("GEN.FR-G14", "Filter regulator G1/4, 5 µm, 0.5 - 10 bar", fl, "Service unit", "HQ", 54.0, (45.0, 160.0, 70.0, 0.4), &[(FluidComponent, "1 2")]);
    c.add("GEN.THROTTLE-G18", "One-way flow control valve G1/8", fl, "Service unit", "RN", 12.0, (0.0, 0.0, 0.0, 0.05), &[(FluidComponent, "1 2")]);
    // Standing on the floor of a room (Layout space › Insert › Floor-standing part).
    for (number, desc, subgroup, ident, price, dims, function) in [
        ("GEN.PUMP-CENT-DN25", "Centrifugal pump DN25, 2.2 kW, on base frame", "Pump", "GP", 980.0, (320.0, 420.0, 640.0, 38.0), (Pump, "S P")),
        ("GEN.TANK-200L", "Vessel 200 l, stainless steel, on feet", "Vessel", "CM", 1450.0, (650.0, 1200.0, 650.0, 62.0), (FluidComponent, "N1 N2")),
    ] {
        let p = c.add(number, desc, fl, subgroup, ident, price, dims, &[function]);
        p.mounting = "Floor".into();
        p.mounting_data.floor_mount = true;
    }
    // Pipe fittings screwed to the wall of a room.
    for (number, desc, subgroup, ident, price, dims, pins) in [
        ("GEN.VLV-BALL-DN25", "Ball valve DN25, PN40, with wall bracket", "Shut-off valve", "QM", 64.0, (160.0, 120.0, 140.0, 1.4), "1 2"),
        ("GEN.GAUGE-0-10BAR", "Pressure gauge 0 - 10 bar, Ø 100 mm, with wall bracket", "Pressure gauge", "BP", 38.0, (110.0, 140.0, 70.0, 0.4), "1"),
    ] {
        let p = c.add(number, desc, fl, subgroup, ident, price, dims, &[(FluidComponent, pins)]);
        p.mounting = "Wall".into();
        p.mounting_data.wall_mount = true;
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
    ] {
        let p = c.add(number, desc, "Cables", subgroup, "W", price, (0.0, 0.0, 0.0, kg), &[]);
        p.quantity_unit = "m".into();
        p.tech_data = format!("Cores: {cores}\nCross-section: {cs}");
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
        ("GEN.RK-PDU-8", "Power distribution unit 19-inch, 8 sockets 230 V, 16 A, 1 U", "XD", 120.0, (1.0, 60.0, 1.4), &[(Generic, "L N")][..], 0.0),
        ("GEN.RK-SERVER-2U", "Industrial server 19-inch, 2 U, redundant power supply", "KF", 4200.0, (2.0, 550.0, 18.0), &[(Generic, "L N")][..], 350.0),
        ("GEN.RK-UPS-2U", "UPS 19-inch, 1500 VA, line interactive, 2 U", "GU", 980.0, (2.0, 450.0, 22.0), &[(Generic, "L N")][..], 40.0),
    ] {
        let p = c.add(number, desc, "General", "19-inch device", ident, price, dims(u, d, kg), functions);
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
    /// Feed-through terminal: thin block with two screws.
    Terminal,
    /// PLC I/O module: tall block with an LED / connector column.
    Plc,
    /// Door / field device (pushbutton, lamp, sensor): round front.
    Round,
}

fn shape_of(p: &Part) -> Option<Shape> {
    if p.width <= 0.0 || p.height <= 0.0 || p.depth <= 0.0 || p.group == "Mounting" {
        return None;
    }
    Some(match p.subgroup.as_str() {
        "Feed-through terminal" | "PE terminal" | "N terminal" | "Disconnect terminal" | "Fuse terminal" | "Accessories" | "End bracket" => Shape::Terminal,
        "Digital input module" | "Digital output module" | "Analog input module" | "Analog output module" | "CPU" => Shape::Plc,
        "Circuit breaker" | "Residual current device" | "Motor protection switch" | "Auxiliary switch" | "Main switch" => Shape::Din { toggle: true, display: false },
        "Power supply" | "Timer relay" | "Safety relay" | "Frequency converter" | "Soft starter" => Shape::Din { toggle: false, display: true },
        "Temperature switch" => Shape::Din { toggle: false, display: false },
        _ if matches!(p.group.as_str(), "Sensors, switches, pushbuttons" | "Signal devices" | "Measuring instruments") => Shape::Round,
        _ => Shape::Din { toggle: false, display: false },
    })
}

/// Simple 2D front view and 3D model with exactly the part's mounting dimensions.
fn generated_models(p: &Part) -> Option<(Model2d, Mesh3d)> {
    if p.group == "Mounting" {
        return mounting::model(p);
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
            m3.add_box([0.0, 0.0, 0.0], [w, h, base]);
            // Raised front in the middle third
            let (f0, f1) = (h * 0.3, h * 0.7);
            m3.add_box([w * 0.05, f0, base], [w * 0.95, f1, if toggle || display { d * 0.92 } else { d }]);
            rect(&mut prims, w * 0.05, f0, w * 0.95, f1, false);
            // Screw terminals on the steps above and below the front
            let n = ((w / 9.0).floor() as usize).clamp(1, 6);
            let pitch = w / n as f32;
            for i in 0..n {
                let cx = pitch * (i as f32 + 0.5);
                let r = (pitch * 0.28).min(3.0);
                for cy in [h * 0.15, h * 0.85] {
                    m3.add_box([cx - r, cy - r, base], [cx + r, cy + r, base + 1.5]);
                    prims.push(Prim2d::Circle { c: [cx, h - cy], r, fill: false });
                    prims.push(Prim2d::Line { a: [cx - r * 0.7, h - cy], b: [cx + r * 0.7, h - cy] });
                }
            }
            if toggle {
                let (tw, th) = ((w * 0.35).min(8.0), h * 0.12);
                let (cx, cy) = (w / 2.0, h / 2.0);
                m3.add_box([cx - tw / 2.0, cy - th / 2.0, d * 0.92], [cx + tw / 2.0, cy + th / 2.0, d]);
                rect(&mut prims, cx - tw / 2.0, cy - th / 2.0, cx + tw / 2.0, cy + th / 2.0, true);
            }
            if display {
                let (cx, cy) = (w / 2.0, h * 0.58);
                let (dw, dh) = (w * 0.6, h * 0.08);
                m3.add_box([cx - dw / 2.0, cy - dh / 2.0, d * 0.92], [cx + dw / 2.0, cy + dh / 2.0, d]);
                rect(&mut prims, cx - dw / 2.0, cy - dh / 2.0, cx + dw / 2.0, cy + dh / 2.0, false);
                prims.push(Prim2d::Circle { c: [cx, h - h * 0.42], r: (w * 0.08).min(3.0), fill: false });
            }
        }
        Shape::Terminal => {
            m3.add_box([0.0, 0.0, 0.0], [w, h, d * 0.7]);
            m3.add_box([w * 0.1, h * 0.35, d * 0.7], [w * 0.9, h * 0.65, d]);
            let r = (w * 0.3).min(2.0);
            for cy in [h * 0.18, h * 0.82] {
                prims.push(Prim2d::Circle { c: [w / 2.0, h - cy], r, fill: false });
            }
            rect(&mut prims, w * 0.1, h * 0.35, w * 0.9, h * 0.65, false);
        }
        Shape::Plc => {
            m3.add_box([0.0, 0.0, 0.0], [w, h, d * 0.85]);
            m3.add_box([w * 0.15, h * 0.08, d * 0.85], [w * 0.85, h * 0.92, d]);
            rect(&mut prims, w * 0.15, h * 0.08, w * 0.85, h * 0.92, false);
            for i in 0..8 {
                let y = h * 0.14 + i as f32 * h * 0.095;
                prims.push(Prim2d::Rect { min: [w * 0.25, y], max: [w * 0.4, y + h * 0.03], fill: true });
                prims.push(Prim2d::Line { a: [w * 0.5, y + h * 0.015], b: [w * 0.75, y + h * 0.015] });
            }
        }
        Shape::Round => {
            m3.add_box([0.0, 0.0, 0.0], [w, h, d * 0.8]);
            m3.add_box([w * 0.15, h * 0.15, d * 0.8], [w * 0.85, h * 0.85, d]);
            prims.push(Prim2d::Circle { c: [w / 2.0, h / 2.0], r: w.min(h) * 0.42, fill: false });
            prims.push(Prim2d::Circle { c: [w / 2.0, h / 2.0], r: w.min(h) * 0.3, fill: true });
        }
    }
    Some((Model2d { width: w, height: h, prims }, m3))
}

mod mounting;

#[cfg(test)]
mod tests;
