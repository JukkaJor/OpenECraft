use super::*;

#[test]
fn tables_have_unique_numbers_in_order() {
    for table in [TOP_GROUPS, GROUPS, SUBGROUPS] {
        assert!(table.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(table[0], (0, "Undefined"));
        assert!(table.iter().all(|(_, n)| !n.trim().is_empty() && n.trim() == *n));
    }
    assert_eq!(group_name(2), Some("Relays, contactors"));
    assert_eq!(group_name(26), Some("PLC"));
    assert_eq!(subgroup_name(194), Some("Circuit breaker"));
    assert_eq!(group_name(27), None);
}

#[test]
fn top_groups_match_the_eplan_numbers() {
    for t in TopGroup::ALL {
        assert_eq!(name_of(TOP_GROUPS, t.eplan_id()), Some(t.title()));
        assert_eq!(TopGroup::from_eplan_id(t.eplan_id()), Some(t));
    }
    assert_eq!(TopGroup::from_eplan_id(0), None);
}

#[test]
fn names_and_own_groups_map_to_numbers() {
    assert_eq!(group_id("relays, CONTACTORS"), Some(2));
    assert_eq!(group_id("Safety devices"), Some(6));
    assert_eq!(group_id("unknown"), None);
    assert_eq!(subgroup_id("Contactors"), Some(2));
    // Every group of the example catalog has an EPLAN number.
    for p in crate::default_catalog() {
        assert!(group_id(&p.group).is_some(), "{}: {}", p.number, p.group);
    }
}

#[test]
fn classify_and_apply_round_trip() {
    let part = Part { top_group: TopGroup::Fluid, group: "Valves".into(), subgroup: "Directional control valves".into(), ..Default::default() };
    let c = classify(&part);
    assert_eq!(c, Classification { top_group: 2, group: 44, subgroup: 118 });
    let mut other = Part { group: "kept".into(), ..Default::default() };
    apply(&mut other, Classification { top_group: 3, group: 0, subgroup: 140 });
    assert_eq!((other.top_group, other.group.as_str(), other.subgroup.as_str()), (TopGroup::Mechanics, "kept", "Mounting rail"));
    apply(&mut other, c);
    assert_eq!(classify(&other), c);
}
