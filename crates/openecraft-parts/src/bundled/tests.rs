use super::*;
use crate::{default_catalog, FnKind};
use crate::portal::catalog_id;
use crate::productgroups::group_id;

#[test]
fn bundled_catalogs_are_consistent() {
    let bundles = all().unwrap();
    assert_eq!(bundles.len(), FILES.len());
    let mut numbers: Vec<String> = default_catalog().into_iter().map(|p| p.number).collect();
    for (bundle, id) in bundles.iter().zip(ids()) {
        let c = &bundle.catalog;
        assert_eq!((c.id.as_str(), catalog_id(id)), (id, id.to_string()));
        assert!(!c.manufacturer.is_empty() && !c.version.is_empty() && !bundle.parts.is_empty(), "{id}");
        for p in &bundle.parts {
            assert!(!numbers.contains(&p.number), "duplicate {}", p.number);
            numbers.push(p.number.clone());
            // MANUFACTURER.ORDER, or a number made from the type if the order number is unknown.
            let (prefix, rest) = p.number.split_once('.').unwrap_or_default();
            assert!(!prefix.is_empty() && (rest == p.order_number || p.order_number.is_empty()), "{}: number is not MANUFACTURER.ORDER", p.number);
            assert!(!rest.is_empty() && !rest.contains(['/', ' ']), "{}: number", p.number);
            assert_eq!(p.manufacturer, c.manufacturer, "{}", p.number);
            assert_eq!(p.catalog, c.id, "{}", p.number);
            assert!(group_id(&p.group).is_some(), "{}: unknown group {}", p.number, p.group);
            // Mounting material (ducts) is placed without a device tag.
            let mounting = p.group == "Mounting";
            assert!(!p.description.is_empty() && (mounting || !p.identifier.is_empty()), "{}", p.number);
            // Terminals take their designation from the device tag; accessories have no functions.
            assert!(p.functions.iter().all(|f| f.kind == FnKind::Terminal || !f.pin_list().is_empty()), "{}: function template", p.number);
            assert!(!p.functions.is_empty() || mounting || matches!(p.subgroup.as_str(), "Accessories" | "End bracket"), "{}: no functions", p.number);
            for a in &p.accessories {
                assert!(bundle.parts.iter().any(|q| &q.number == a), "{}: accessory {a} missing", p.number);
            }
            // Every part with dimensions has a generated model, mounting material too.
            assert_eq!(p.model_3d.is_some(), p.width > 0.0 && p.height > 0.0 && p.depth > 0.0, "{}: generated model", p.number);
            // Units are written alike in every catalog (the facets list each spelling).
            let texts = format!("{} {}", p.description, p.tech_data);
            assert!(!texts.contains(" mm2") && !texts.contains(".0 mm²"), "{}: write cross-sections as 2.5 mm²", p.number);
        }
    }
}

#[test]
fn unknown_catalogs_are_refused() {
    let err = load("nobody").unwrap_err();
    assert!(err.contains("siemens"), "{err}");
    assert_eq!(load("SIEMENS").unwrap().catalog.id, "siemens");
}

#[test]
fn victron_devices_are_wall_mountable() {
    let victron = load("victron").unwrap();
    let wall: Vec<&str> = victron.parts.iter().filter(|p| p.wall_mountable()).map(|p| p.number.as_str()).collect();
    assert!(wall.contains(&"VIC.PMP482305012") && wall.contains(&"VIC.SCC115035210"), "{wall:?}");
    // The BMV-712 display goes into a front panel.
    let bmv = victron.parts.iter().find(|p| p.number == "VIC.BAM030712000").unwrap();
    assert!(!bmv.wall_mountable() && bmv.mounting_data.door_cutout.is_some());
    assert!(victron.parts.iter().all(|p| p.model_3d.is_some()));
}
