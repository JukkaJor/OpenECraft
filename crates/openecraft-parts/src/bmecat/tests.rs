use super::*;

const CATALOG_2005: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<BMECAT version="2005" xmlns="http://www.bmecat.org/bmecat/2005">
  <HEADER>
    <CATALOG><LANGUAGE>eng</LANGUAGE><CATALOG_ID>C1</CATALOG_ID><CURRENCY>EUR</CURRENCY></CATALOG>
    <SUPPLIER><SUPPLIER_NAME>Elektro Großhandel GmbH</SUPPLIER_NAME></SUPPLIER>
  </HEADER>
  <T_NEW_CATALOG>
    <PRODUCT mode="new">
      <SUPPLIER_PID>4711</SUPPLIER_PID>
      <PRODUCT_DETAILS>
        <DESCRIPTION_SHORT>Contactor 3P 9 A, coil 24 V DC</DESCRIPTION_SHORT>
        <DESCRIPTION_LONG>Power contactor AC-3</DESCRIPTION_LONG>
        <INTERNATIONAL_PID type="gtin">4011209000000</INTERNATIONAL_PID>
        <MANUFACTURER_PID>CT9-24DC</MANUFACTURER_PID>
        <MANUFACTURER_NAME>Acme Controls AG</MANUFACTURER_NAME>
      </PRODUCT_DETAILS>
      <PRODUCT_FEATURES>
        <REFERENCE_FEATURE_SYSTEM_NAME>ETIM-9.0</REFERENCE_FEATURE_SYSTEM_NAME>
        <REFERENCE_FEATURE_GROUP_ID>EC000066</REFERENCE_FEATURE_GROUP_ID>
        <REFERENCE_FEATURE_GROUP_NAME>Power contactor</REFERENCE_FEATURE_GROUP_NAME>
        <FEATURE><FNAME>EF000008</FNAME><FVALUE>45</FVALUE><FUNIT>MMT</FUNIT></FEATURE>
        <FEATURE><FNAME>EF000040</FNAME><FVALUE>7.4</FVALUE><FUNIT>CMT</FUNIT></FEATURE>
        <FEATURE><FNAME>Depth</FNAME><FVALUE>73</FVALUE><FUNIT>MMT</FUNIT></FEATURE>
        <FEATURE><FNAME>EF000001</FNAME><FDESCR>Rated control supply voltage</FDESCR><FVALUE>24</FVALUE><FUNIT>VLT</FUNIT></FEATURE>
        <FEATURE><FNAME>Rated operational current AC-3</FNAME><FVALUE>9</FVALUE><FUNIT>AMP</FUNIT></FEATURE>
        <FEATURE><FNAME>Power loss</FNAME><FVALUE>2,4</FVALUE><FUNIT>WTT</FUNIT></FEATURE>
        <FEATURE><FNAME>EF000123</FNAME><FVALUE>1</FVALUE><FVALUE>2</FVALUE></FEATURE>
        <FEATURE><FNAME>EF000999</FNAME><FVALUE>-</FVALUE></FEATURE>
      </PRODUCT_FEATURES>
      <PRODUCT_FEATURES>
        <REFERENCE_FEATURE_SYSTEM_NAME>ECLASS-12.0</REFERENCE_FEATURE_SYSTEM_NAME>
        <REFERENCE_FEATURE_GROUP_ID>27371001</REFERENCE_FEATURE_GROUP_ID>
        <FEATURE><FNAME>Gewicht</FNAME><FVALUE>320</FVALUE><FUNIT>GRM</FUNIT></FEATURE>
      </PRODUCT_FEATURES>
      <PRODUCT_ORDER_DETAILS><ORDER_UNIT>C62</ORDER_UNIT><NO_CU_PER_OU>1</NO_CU_PER_OU><PRICE_QUANTITY>1</PRICE_QUANTITY></PRODUCT_ORDER_DETAILS>
      <PRODUCT_PRICE_DETAILS>
        <PRODUCT_PRICE price_type="net_customer"><PRICE_AMOUNT>30.00</PRICE_AMOUNT></PRODUCT_PRICE>
        <PRODUCT_PRICE price_type="net_list"><PRICE_AMOUNT>38.50</PRICE_AMOUNT><PRICE_CURRENCY>EUR</PRICE_CURRENCY></PRODUCT_PRICE>
      </PRODUCT_PRICE_DETAILS>
      <MIME_INFO>
        <MIME><MIME_TYPE>image/jpeg</MIME_TYPE><MIME_SOURCE>ct9.jpg</MIME_SOURCE><MIME_PURPOSE>normal</MIME_PURPOSE></MIME>
        <MIME><MIME_TYPE>application/pdf</MIME_TYPE><MIME_SOURCE>ct9.pdf</MIME_SOURCE><MIME_PURPOSE>data_sheet</MIME_PURPOSE></MIME>
      </MIME_INFO>
    </PRODUCT>
    <PRODUCT><PRODUCT_DETAILS><DESCRIPTION_SHORT>No id</DESCRIPTION_SHORT></PRODUCT_DETAILS></PRODUCT>
  </T_NEW_CATALOG>
</BMECAT>"#;

#[test]
fn products_with_etim_and_eclass_features() {
    assert!(is_bmecat(CATALOG_2005));
    let parts = from_bmecat(CATALOG_2005).unwrap();
    assert_eq!(parts.len(), 1);
    let p = &parts[0];
    assert_eq!((p.number.as_str(), p.type_number.as_str(), p.order_number.as_str()), ("ACME.CT9-24DC", "CT9-24DC", "4711"));
    assert_eq!((p.manufacturer.as_str(), p.supplier.as_str()), ("Acme Controls AG", "Elektro Großhandel GmbH"));
    assert_eq!((p.description.as_str(), p.note.as_str(), p.group.as_str()), ("Contactor 3P 9 A, coil 24 V DC", "Power contactor AC-3", "Power contactor"));
    assert_eq!((p.width, p.height, p.depth), (45.0, 74.0, 73.0));
    assert!((p.weight - 0.32).abs() < 1e-6);
    assert_eq!((p.current.as_str(), p.voltage.as_str()), ("9 A", "24 V"));
    assert_eq!((p.price, p.currency.as_str(), p.datasheet.as_str()), (38.5, "EUR", "ct9.pdf"));
    assert_eq!(p.tech_value("Power loss"), Some("2,4 W"));
    assert_eq!(p.tech_value("ETIM class"), Some("EC000066 Power contactor (ETIM-9.0)"));
    assert_eq!(p.tech_value("eCl@ss class"), Some("27371001 (ECLASS-12.0)"));
    assert_eq!(p.tech_value("GTIN"), Some("4011209000000"));
    assert_eq!(p.tech_value("EF000123"), Some("1, 2"));
    assert_eq!(p.tech_value("EF000999"), None);
    assert_eq!(p.tech_data.matches("Power loss").count(), 1);
    // The general parts import recognises the format.
    assert_eq!(crate::import(CATALOG_2005).unwrap(), parts);
}

#[test]
fn bmecat_1_2_articles() {
    let text = r#"<BMECAT version="1.2"><HEADER><CATALOG><CURRENCY>CHF</CURRENCY></CATALOG><SUPPLIER><SUPPLIER_NAME>Shop</SUPPLIER_NAME></SUPPLIER></HEADER>
      <T_NEW_CATALOG><ARTICLE><SUPPLIER_AID>A-1</SUPPLIER_AID><ARTICLE_DETAILS><DESCRIPTION_SHORT>Terminal 2.5</DESCRIPTION_SHORT><EAN>123</EAN></ARTICLE_DETAILS>
      <ARTICLE_FEATURES><REFERENCE_FEATURE_SYSTEM_NAME>ETIM-5.0</REFERENCE_FEATURE_SYSTEM_NAME><REFERENCE_FEATURE_GROUP_ID>EC000897</REFERENCE_FEATURE_GROUP_ID>
      <FEATURE><FNAME>Breite</FNAME><FVALUE>5,2</FVALUE><FUNIT>MMT</FUNIT></FEATURE></ARTICLE_FEATURES>
      <ARTICLE_PRICE_DETAILS><ARTICLE_PRICE price_type="net_list"><PRICE_AMOUNT>0.95</PRICE_AMOUNT></ARTICLE_PRICE></ARTICLE_PRICE_DETAILS></ARTICLE></T_NEW_CATALOG></BMECAT>"#;
    let parts = crate::import(text).unwrap();
    let p = &parts[0];
    assert_eq!((p.number.as_str(), p.supplier.as_str(), p.width, p.price, p.currency.as_str()), ("SHOP.A-1", "Shop", 5.2, 0.95, "CHF"));
    assert_eq!(p.tech_value("GTIN"), Some("123"));
}

#[test]
fn other_documents_are_refused() {
    assert!(!is_bmecat("Part number;Description\n"));
    assert!(from_bmecat("<OTHER/>").is_err());
    assert!(from_bmecat("<BMECAT><T_NEW_CATALOG/></BMECAT>").unwrap_err().contains("no products"));
}
