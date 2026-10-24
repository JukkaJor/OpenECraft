use super::*;
use openecraft_parts::{default_catalog, FnKind};
use openecraft_parts::portal::catalog_id;
use openecraft_parts::productgroups::group_id;

#[test]
fn bundled_catalogs_are_consistent() {
    let bundles = all().unwrap();
    assert_eq!(bundles.len(), FILES.len());
    let mut numbers: Vec<String> = default_catalog().into_iter().map(|p| p.number).collect();
    for (bundle, id) in bundles.iter().zip(ids()) {
        let c = &bundle.catalog;
        assert_eq!((c.id.as_str(), catalog_id(&c.name)), (id, id.to_string()));
        assert!(!c.manufacturer.is_empty() && !c.version.is_empty() && !bundle.parts.is_empty(), "{id}");
        for p in &bundle.parts {
            assert!(!numbers.contains(&p.number), "duplicate {}", p.number);
            numbers.push(p.number.clone());
            let (prefix, rest) = p.number.split_once('.').unwrap_or_default();
            assert!(!prefix.is_empty() && rest == p.order_number, "{}: number is not MANUFACTURER.ORDER", p.number);
            assert_eq!(p.manufacturer, c.manufacturer, "{}", p.number);
            assert_eq!(p.catalog, c.id, "{}", p.number);
            assert!(group_id(&p.group).is_some(), "{}: unknown group {}", p.number, p.group);
            assert!(!p.description.is_empty() && !p.identifier.is_empty(), "{}", p.number);
            // Terminals take their designation from the device tag; accessories have no functions.
            assert!(p.functions.iter().all(|f| f.kind == FnKind::Terminal || !f.pin_list().is_empty()), "{}: function template", p.number);
            assert!(!p.functions.is_empty() || matches!(p.subgroup.as_str(), "Accessories" | "End bracket"), "{}: no functions", p.number);
            for a in &p.accessories {
                assert!(bundle.parts.iter().any(|q| &q.number == a), "{}: accessory {a} missing", p.number);
            }
            assert_eq!(p.model_3d.is_some(), p.width > 0.0, "{}: generated model", p.number);
        }
    }
}

#[test]
fn unknown_catalogs_are_refused() {
    let err = load("nobody").unwrap_err();
    assert!(err.contains("siemens"), "{err}");
    assert_eq!(load("SIEMENS").unwrap().catalog.id, "siemens");
}
