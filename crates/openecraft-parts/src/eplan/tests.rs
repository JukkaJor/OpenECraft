use super::*;

const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<partsmanagement>
  <part P_ARTICLE_PARTNR="SIE.3RT2015-1BB41" P_ARTICLE_TYPENR="3RT2015-1BB41" P_ARTICLE_MANUFACTURER="SIE"
        P_ARTICLE_DESCR1="de_DE@Leistungsschütz;en_US@Power contactor;" P_ARTICLE_NOTE="de_DE@Spule 24 V DC;"
        P_ARTICLE_SALESPRICE_1="42,50" P_ARTICLE_WIDTH="45" P_ARTICLE_HEIGHT="57.5" P_ARTICLE_DEPTH="73" P_ARTICLE_WEIGHT="0,2"
        P_ARTICLE_VOLTAGE="24 V DC" P_ARTICLE_PRODUCTTOPGROUP="1" P_ARTICLE_PRODUCTGROUP="1" P_ARTICLE_PRODUCTSUBGROUP="0" />
  <part P_ARTICLE_PARTNR="PXC.UT2,5" P_ARTICLE_DESCR1="Feed-through terminal">
    <variant P_ARTICLE_VARIANT="1" P_ARTICLE_PARTNR="PXC.UT2,5-BU" P_ARTICLE_DESCR1="Feed-through terminal, blue" />
    <variant P_ARTICLE_VARIANT="2" P_ARTICLE_PARTNR="PXC.UT2,5-GY" />
  </part>
  <part P_ARTICLE_TYPENR="no number" />
</partsmanagement>"#;

#[test]
fn multi_language_texts() {
    assert_eq!(language_text("de_DE@Schütz;en_US@Contactor;"), "Contactor");
    assert_eq!(language_text("fr_FR@Contacteur;de_DE@Schütz;"), "Schütz");
    assert_eq!(language_text("fr_FR@Contacteur;"), "Contacteur");
    assert_eq!(language_text(" plain "), "plain");
    assert_eq!(language_text("mail@example.org"), "example.org", "anything with @ is a language text");
}

#[test]
fn parts_and_variants_are_read() {
    assert!(is_eplan_xml(SAMPLE) && !is_eplan_xml("Number;Description") && !is_eplan_xml("<BMECAT/>"));
    let parts = from_eplan_xml(SAMPLE).unwrap();
    assert_eq!(parts.len(), 3, "one part and two variants; the part without number is left out");
    let c = &parts[0];
    assert_eq!((c.number.as_str(), c.type_number.as_str(), c.manufacturer.as_str()), ("SIE.3RT2015-1BB41", "3RT2015-1BB41", "SIE"));
    assert_eq!((c.description.as_str(), c.note.as_str()), ("Power contactor", "Spule 24 V DC"));
    assert_eq!((c.price, c.width, c.height, c.depth, c.weight), (42.5, 45.0, 57.5, 73.0, 0.2));
    assert_eq!(c.voltage, "24 V DC");
    assert_eq!(c.top_group, crate::TopGroup::Electrical);
    assert_eq!(c.group, "General");
    assert_eq!(parts[1].number, "PXC.UT2,5-BU");
    assert_eq!(parts[1].description, "Feed-through terminal, blue");
    assert_eq!(parts[2].description, "Feed-through terminal", "variants take what they do not set from the part");
    assert!(from_eplan_xml("<partsmanagement/>").is_err());
    // The generic import recognises the format.
    assert_eq!(crate::import(SAMPLE).unwrap().len(), 3);
}

#[test]
fn round_trip_through_eplan_xml() {
    let parts = from_eplan_xml(SAMPLE).unwrap();
    let xml = to_eplan_xml(&parts);
    assert!(xml.contains("P_ARTICLE_DESCR1=\"en_US@Power contactor;\""));
    assert!(xml.contains("P_ARTICLE_PRODUCTTOPGROUP=\"1\""));
    let back = from_eplan_xml(&xml).unwrap();
    assert_eq!(back.len(), parts.len());
    for (a, b) in parts.iter().zip(&back) {
        assert_eq!((&a.number, &a.description, a.price, a.width, &a.group), (&b.number, &b.description, b.price, b.width, &b.group));
    }
}
