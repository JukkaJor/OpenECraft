use super::*;

fn part(number: &str, accessories: &[&str], fits: &[&str]) -> Part {
    Part { number: number.into(), accessories: accessories.iter().map(|s| s.to_string()).collect(), fits: fits.iter().map(|s| s.to_string()).collect(), ..Default::default() }
}

#[test]
fn either_side_makes_a_pair() {
    // The RCCB lists the signal contact, the signal contact lists a second RCCB.
    let parts = vec![part("HAG.CDA440D", &["HAG.CZ001"], &[]), part("HAG.CCA216D", &[], &[]), part("HAG.CZ001", &[], &["HAG.CCA216D"]), part("HAG.MBN106", &[], &[])];
    let [cda, cca, cz, mbn] = [&parts[0], &parts[1], &parts[2], &parts[3]];
    assert!(fits(cz, cda) && fits(cz, cca) && !fits(cz, mbn) && !fits(cda, cz));
    let mains: Vec<&str> = mains_of(&parts, cz).iter().map(|p| p.number.as_str()).collect();
    assert_eq!(mains, ["HAG.CCA216D", "HAG.CDA440D"]);
    assert_eq!(accessories_of(&parts, cca).len(), 1);
    assert!(accessories_of(&parts, mbn).is_empty());
    let (acc, of) = related_numbers(&parts, cz);
    assert!(acc.is_empty() && of == ["HAG.CCA216D", "HAG.CDA440D"]);
    // A part is never its own accessory.
    assert!(!fits(&part("A", &["A"], &[]), &part("A", &["A"], &[])));
}

#[test]
fn listed_accessories_keep_their_order() {
    let parts = vec![part("M", &["B", "A"], &[]), part("A", &[], &[]), part("B", &[], &[]), part("C", &[], &["M"])];
    let acc: Vec<&str> = accessories_of(&parts, &parts[0]).iter().map(|p| p.number.as_str()).collect();
    assert_eq!(acc, ["B", "A", "C"]);
}

#[test]
fn mounts_read_keys_and_names() {
    for m in AccessoryMount::ALL {
        assert_eq!(AccessoryMount::from_key(m.key()), Some(m));
        assert_eq!(serde_json::from_str::<AccessoryMount>(&format!("\"{}\"", m.key())).unwrap(), m);
    }
    assert_eq!(AccessoryMount::from_key("Left"), Some(AccessoryMount::Left));
    assert_eq!(AccessoryMount::from_key(""), Some(AccessoryMount::Auto));
    assert_eq!(AccessoryMount::from_key("roof"), None);
    assert!(AccessoryMount::Right.is_side() && !AccessoryMount::Front.is_side());
    assert_eq!(AccessoryMount::from_key("Behind"), Some(AccessoryMount::Behind));
    assert!(!AccessoryMount::Behind.is_side() && !AccessoryMount::Inside.is_side());
}
