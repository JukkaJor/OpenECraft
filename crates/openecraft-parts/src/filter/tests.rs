use super::*;

fn part(number: &str, manufacturer: &str, supplier: &str, top_group: TopGroup, group: &str) -> Part {
    Part { number: number.into(), manufacturer: manufacturer.into(), supplier: supplier.into(), top_group, group: group.into(), ..Default::default() }
}

fn sample() -> Vec<Part> {
    vec![
        part("ABB.K1", "ABB", "Wholesale", TopGroup::Electrical, "Relays, contactors"),
        part("SIE.K2", "Siemens", "Wholesale", TopGroup::Electrical, "Relays, contactors"),
        part("SIE.Q1", "Siemens", "Direct", TopGroup::Electrical, "Safety devices"),
        part("FES.V1", "Festo", "", TopGroup::Fluid, "Valves"),
    ]
}

fn numbers(parts: &[Part], f: &PartFilter) -> Vec<String> {
    parts.iter().filter(|p| f.matches(p)).map(|p| p.number.clone()).collect()
}

#[test]
fn empty_filter_matches_everything() {
    let parts = sample();
    let f = PartFilter::default();
    assert!(!f.is_active());
    assert_eq!(numbers(&parts, &f).len(), parts.len());
}

#[test]
fn criteria_are_combined() {
    let parts = sample();
    let mut f = PartFilter { manufacturer: Some("Siemens".into()), ..Default::default() };
    assert_eq!(numbers(&parts, &f), ["SIE.K2", "SIE.Q1"]);
    f.group = Some("Safety devices".into());
    assert_eq!(numbers(&parts, &f), ["SIE.Q1"]);
    f.text = "k2".into();
    assert!(numbers(&parts, &f).is_empty());

    let f = PartFilter { supplier: Some("Wholesale".into()), text: "abb".into(), ..Default::default() };
    assert_eq!(numbers(&parts, &f), ["ABB.K1"]);
    let f = PartFilter { top_group: Some(TopGroup::Fluid), ..Default::default() };
    assert_eq!(numbers(&parts, &f), ["FES.V1"]);
}

#[test]
fn reset_keeps_grouping() {
    let mut f = PartFilter { text: "x".into(), manufacturer: Some("ABB".into()), grouping: Grouping::Manufacturer, ..Default::default() };
    assert!(f.is_active());
    f.reset();
    assert!(!f.is_active());
    assert_eq!(f.grouping, Grouping::Manufacturer);
}

#[test]
fn distinct_values_are_sorted_and_unique() {
    let parts = sample();
    assert_eq!(distinct_values(&parts, |p| &p.manufacturer), ["ABB", "Festo", "Siemens"]);
    assert_eq!(distinct_values(&parts, |p| &p.supplier), ["Direct", "Wholesale"]);
}
