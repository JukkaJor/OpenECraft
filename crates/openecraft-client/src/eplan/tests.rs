use super::*;
use openecraft_parts::TopGroup;
use serde_json::json;

fn response(status: u16, body: &str) -> ehttp::Result<ehttp::Response> {
    Ok(ehttp::Response {
        url: "https://portal.test/api".into(),
        ok: (200..300).contains(&status),
        status,
        status_text: "Status".into(),
        headers: ehttp::Headers::default(),
        bytes: body.as_bytes().to_vec(),
    })
}

fn endpoint(token: &str) -> EplanEndpoint {
    EplanEndpoint { url: "https://portal.test/api/".into(), token: token.into() }
}

/// A search answer with a relationship to an included manufacturer.
fn document() -> Value {
    json!({
        "data": [{
            "type": "parts",
            "id": "4711",
            "attributes": {
                "part_number": "SIE.3RT2015-1BB41",
                "type_number": "3RT2015-1BB41",
                "order_number": "3RT20151BB41",
                "designation": { "de_DE": "Leistungsschütz", "en_US": "Power contactor" },
                "description": { "en_US": "AC-3, 3 kW, 24 V DC" },
                "update_date": "2026-10-23T00:00:00+00:00",
                "product_top_group": { "id": "01", "name": "Electrical engineering" },
                "product_group": { "id": "0100020000", "name": "Relays, contactors" },
                "product_sub_group": { "id": "0100020002", "name": "Contactors" },
                "data_types": ["commercial_data", "dxf_data", "2d_graphical_data"],
                "picture_file": { "type": "picturefile", "id": "SIE/3RT2015.jpg", "preview": { "96": "https://cdn.test/96.jpg", "512": "https://cdn.test/512.jpg" } },
                "accessories": [{ "part_number": "SIE.3RT2916-1BB00" }]
            },
            "relationships": { "manufacturer": { "data": { "type": "manufacturers", "id": "SIE" } } }
        }, {
            "type": "parts",
            "id": "4712",
            "attributes": { "part_number": "XYZ.1", "designation": "Valve", "product_group": "Pneumatic specials" }
        }],
        "included": [{ "type": "manufacturers", "id": "SIE", "attributes": { "short_name": "SIE", "long_name": "Siemens AG" } }],
        "meta": { "total": 1234 }
    })
}

#[test]
fn requests_of_the_portal() {
    let e = endpoint(" abc123 ");
    let info = e.info();
    assert_eq!(info.url, "https://portal.test/api/parts?page%5Blimit%5D=1");
    assert_eq!(info.headers.get("Authorization"), Some("Bearer PAT:abc123"));
    assert_eq!(endpoint("PAT:abc").info().headers.get("Authorization"), Some("Bearer PAT:abc"));
    assert_eq!(endpoint("eyJhbGciOi.x.y").info().headers.get("Authorization"), Some("Bearer eyJhbGciOi.x.y"));
    assert_eq!(endpoint("").info().headers.get("Authorization"), None);

    let q = PartQuery { q: Some("contactor 24V".into()), manufacturer: Some("SIE".into()), sort: Sort::Manufacturer, descending: true, offset: Some(50), limit: Some(25), ..Default::default() };
    assert_eq!(
        e.search(&q).url,
        "https://portal.test/api/parts?search=contactor%2024V&filter%5Bmanufacturer%5D=SIE&sort=-manufacturer.short_name&page%5Boffset%5D=50&page%5Blimit%5D=25"
    );
    let numbers = PartQuery { numbers: vec!["A.1".into(), "B.2".into()], ..Default::default() };
    let req = e.search(&numbers);
    assert_eq!((req.method.as_str(), req.url.as_str(), req.body.as_slice()), ("POST", "https://portal.test/api/parts", b"A.1,B.2".as_slice()));
    assert_eq!(req.headers.get("Content-Type"), Some("text/plain"));
    assert_eq!(e.part("A/1").url, "https://portal.test/api/parts/A%2F1");
}

#[test]
fn files_keep_the_token_on_the_portal() {
    let e = endpoint("abc");
    let own = e.file("download/dxf_data/part/4711");
    assert_eq!(own.url, "https://portal.test/api/download/dxf_data/part/4711");
    assert!(own.headers.get("Authorization").is_some());
    let other = e.file("https://cdn.test/512.jpg");
    assert_eq!(other.url, "https://cdn.test/512.jpg");
    assert_eq!(other.headers.get("Authorization"), None);
    assert!(e.needs_token("download/dxf_data/part/4711") && !e.needs_token("https://cdn.test/512.jpg"));
}

#[test]
fn product_groups_become_the_catalog_uid() {
    let uid = |top: Option<TopGroup>, group: Option<&str>, sub: Option<&str>| {
        catalog_uid(&PartQuery { top_group: top, group: group.map(Into::into), subgroup: sub.map(Into::into), ..Default::default() })
    };
    assert_eq!(uid(None, None, None), None);
    assert_eq!(uid(Some(TopGroup::Fluid), None, None).as_deref(), Some("02"));
    assert_eq!(uid(None, Some("Relays, contactors"), None).as_deref(), Some("010002"));
    assert_eq!(uid(Some(TopGroup::Electrical), Some("Relays, contactors"), Some("Contactors")).as_deref(), Some("0100020002"));
    assert_eq!(uid(None, Some("Unknown group"), None), None);
    let q = PartQuery { group: Some("Relays, contactors".into()), ..Default::default() };
    assert!(query_string(&q).contains("filter%5Bcatalog_eplan%5D=010002"));
}

#[test]
fn parts_of_the_portal_become_parts() {
    let found = with_lang_en(|| parts(&document()));
    assert_eq!(found.len(), 2);
    let p = &found[0];
    assert_eq!(p.number, "SIE.3RT2015-1BB41");
    assert_eq!((p.type_number.as_str(), p.order_number.as_str(), p.manufacturer.as_str()), ("3RT2015-1BB41", "3RT20151BB41", "SIE"));
    assert_eq!((p.description.as_str(), p.note.as_str()), ("Power contactor", "AC-3, 3 kW, 24 V DC"));
    assert_eq!((p.top_group, p.group.as_str(), p.subgroup.as_str()), (TopGroup::Electrical, "Relays, contactors", "Contactors"));
    assert_eq!(p.accessories, vec!["SIE.3RT2916-1BB00".to_string()]);
    assert_eq!((p.modified, p.revision), (1_792_713_600, 1_792_713_600));
    let files: Vec<(&str, &str, AttachmentKind)> = p.attachments.iter().map(|a| (a.id.as_str(), a.name.as_str(), a.kind)).collect();
    assert_eq!(
        files,
        vec![
            ("download/commercial_data/part/4711", "SIE.3RT2015-1BB41_commercial_data.csv", AttachmentKind::Other),
            ("download/dxf_data/part/4711", "SIE.3RT2015-1BB41_dxf.zip", AttachmentKind::Drawing),
            ("https://cdn.test/512.jpg", "3RT2015.jpg", AttachmentKind::Image),
        ]
    );
    // Plain texts, unknown groups kept by name, no data types: commercial data only.
    let q = &found[1];
    assert_eq!((q.description.as_str(), q.group.as_str(), q.attachments.len()), ("Valve", "Pneumatic specials", 1));
    // The user's language is preferred.
    let german = openecraft_i18n::with_lang(openecraft_i18n::Lang::De, || parts(&document()));
    assert_eq!(german[0].description, "Leistungsschütz");
}

fn with_lang_en<R>(f: impl FnOnce() -> R) -> R {
    openecraft_i18n::with_lang(openecraft_i18n::Lang::En, f)
}

#[test]
fn answers_become_parts_server_answers() {
    let body = document().to_string();
    let info = decode_info(response(200, &body)).unwrap();
    assert_eq!((info.name.as_str(), info.parts, info.writable, info.user.is_none()), (EPLAN_NAME, 1234, false, true));

    let query = PartQuery { manufacturer: Some("SIE".into()), offset: Some(50), ..Default::default() };
    let r = with_lang_en(|| decode_search(&query, response(200, &body))).unwrap();
    assert_eq!((r.total, r.offset, r.parts.len()), (1234, 50, 2));
    let names: Vec<&str> = r.facets.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, FACETS);
    assert!(r.facet("manufacturer").unwrap().values.iter().any(|v| v.value == "SIE" && v.count == 1));

    // Loading by number keeps only the requested parts.
    let numbers = PartQuery { numbers: vec!["XYZ.1".into()], ..Default::default() };
    let r = decode_search(&numbers, response(200, &body)).unwrap();
    assert_eq!((r.total, r.parts[0].number.as_str()), (1, "XYZ.1"));

    // Without a count, a next link promises one more part.
    let paged = json!({ "data": [], "links": { "next": "https://portal.test/api/parts?page[offset]=10" } }).to_string();
    assert_eq!(decode_search(&PartQuery { offset: Some(10), ..Default::default() }, response(200, &paged)).unwrap().total, 11);

    let one = json!({ "data": document()["data"][1] }).to_string();
    assert_eq!(decode_part(response(200, &one)).unwrap().map(|p| p.number), Some("XYZ.1".into()));
    assert_eq!(decode_part(response(404, "")).unwrap(), None);
    assert_eq!(decode_file(response(200, "a;b")).unwrap(), b"a;b".to_vec());
}

#[test]
fn errors_are_readable() {
    let denied = with_lang_en(|| decode_info(response(401, "{}")).unwrap_err());
    assert_eq!(denied, "access denied – check the personal access token");
    let errors = json!({ "errors": [{ "status": "422", "title": "Invalid", "detail": "page[limit] too large" }, { "title": "Second" }] }).to_string();
    assert_eq!(decode_search(&PartQuery::default(), response(422, &errors)).unwrap_err(), "page[limit] too large; Second");
    assert_eq!(decode_file(response(500, "oops")).unwrap_err(), "500 Status");
    assert!(decode_info(Err("refused".into())).unwrap_err().contains("refused"));
    assert!(decode_info(response(200, "<html>")).is_err());
}
