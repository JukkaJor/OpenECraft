use super::*;

fn part(number: &str, manufacturer: &str, group: &str, tech: &str) -> Part {
    Part { number: number.into(), manufacturer: manufacturer.into(), group: group.into(), tech_data: tech.into(), ..Default::default() }
}

fn parts() -> Vec<Part> {
    vec![
        Part { voltage: "24 V DC".into(), catalog: "abb_2026".into(), ..part("ABB.K1", "ABB", "Relays, contactors", "Poles: 3") },
        Part { voltage: "230 V AC".into(), ..part("ABB.K2", "ABB", "Relays, contactors", "Poles: 4") },
        Part { voltage: "24 V DC".into(), ..part("SIE.K3", "Siemens", "Relays, contactors", "Poles: 3\nWidth class: 45 mm") },
        Part { lifecycle: Lifecycle::Discontinued, successor: "SIE.Q2".into(), ..part("SIE.Q1", "Siemens", "Protection devices", "") },
    ]
}

fn values(f: &Facet) -> Vec<(&str, usize)> {
    f.values.iter().map(|v| (v.value.as_str(), v.count)).collect()
}

#[test]
fn attributes_combine_fields_and_technical_data() {
    let p = Part { voltage: " 24 V ".into(), tech_data: "Poles: 3\nno value:\nVoltage: 48 V\nfree text".into(), ..Default::default() };
    assert_eq!(attributes(&p), vec![("Voltage".to_string(), "24 V".to_string()), ("Poles".to_string(), "3".to_string())]);
}

#[test]
fn every_word_of_the_text_must_occur() {
    let q = PartQuery { q: Some("siemens relays".into()), ..Default::default() };
    assert_eq!(q.apply(&parts()).1.iter().map(|p| p.number.as_str()).collect::<Vec<_>>(), ["SIE.K3"]);
    let q = PartQuery { q: Some("abb_2026".into()), ..Default::default() };
    assert_eq!(q.apply(&parts()).0, 1);
}

#[test]
fn facets_ignore_their_own_filter() {
    let q = PartQuery { manufacturer: Some("abb".into()), ..Default::default() };
    let r = q.search(&parts(), 7);
    assert_eq!((r.revision, r.total), (7, 2));
    // All manufacturers stay selectable …
    assert_eq!(values(r.facet("manufacturer").unwrap()), [("ABB", 2), ("Siemens", 2)]);
    // … the other facets count the ABB parts only.
    assert_eq!(values(r.facet("group").unwrap()), [("Relays, contactors", 2)]);
    assert_eq!(values(r.facet("Voltage").unwrap()), [("24 V DC", 1), ("230 V AC", 1)]);
    assert_eq!(values(r.facet("lifecycle").unwrap()), [("Active", 2)]);
}

#[test]
fn attribute_filters_and_facets() {
    let q = PartQuery { attributes: vec![("voltage".into(), "24 v dc".into()), ("Poles".into(), "3".into())], ..Default::default() };
    let r = q.search(&parts(), 1);
    assert_eq!(r.parts.iter().map(|p| p.number.as_str()).collect::<Vec<_>>(), ["ABB.K1", "SIE.K3"]);
    // Poles facet: all 24 V parts, whatever their poles.
    assert_eq!(values(r.facet("Poles").unwrap()), [("3", 2)]);
    assert_eq!(values(r.facet("Voltage").unwrap()), [("24 V DC", 2)]);
    // Most frequent attributes first.
    assert_eq!(r.attributes.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["Poles", "Voltage", "Width class"]);
}

#[test]
fn lifecycle_numbers_and_revision_filters() {
    let mut list = parts();
    list[1].revision = 5;
    let q = PartQuery { lifecycle: Some(Lifecycle::Discontinued), ..Default::default() };
    assert_eq!(q.apply(&list).1[0].number, "SIE.Q1");
    let q = PartQuery { numbers: vec!["ABB.K2".into(), "X".into()], ..Default::default() };
    assert_eq!(q.apply(&list).0, 1);
    let q = PartQuery { changed_since: Some(4), ..Default::default() };
    assert_eq!(q.apply(&list).1[0].number, "ABB.K2");
    let q = PartQuery { top_group: Some(TopGroup::Fluid), ..Default::default() };
    assert_eq!(q.apply(&list).0, 0);
}

#[test]
fn sorting_paging_and_brief_results() {
    let mut list = parts();
    list[0].price = 9.0;
    list[2].price = 1.0;
    list[0].model_3d = Some(Default::default());
    let q = PartQuery { sort: Sort::Price, descending: true, limit: Some(2), brief: true, ..Default::default() };
    let (total, page) = q.apply(&list);
    assert_eq!(total, 4);
    assert_eq!(page.iter().map(|p| p.number.as_str()).collect::<Vec<_>>(), ["ABB.K1", "SIE.K3"]);
    assert!(page[0].model_3d.is_none());
}

#[test]
fn update_check_finds_newer_and_discontinued_parts() {
    let mut local = parts();
    for p in &mut local {
        p.revision = 3;
    }
    local.push(part("OWN.X", "Own", "", ""));
    let mut remote = local.clone();
    remote[0].description = "new".into();
    remote[0].revision = 4;
    // Local change after the server version: kept.
    local[1].description = "mine".into();
    remote[3].lifecycle = Lifecycle::Discontinued;
    remote[3].revision = 4;
    local[3].lifecycle = Lifecycle::Active;
    remote.remove(2);
    remote.pop();
    let updates = check_updates(&local, &remote);
    let found: Vec<(&str, UpdateKind)> = updates.iter().map(|u| (u.number.as_str(), u.kind)).collect();
    assert_eq!(found, [("ABB.K1", UpdateKind::Changed), ("SIE.K3", UpdateKind::Removed), ("SIE.Q1", UpdateKind::Discontinued)]);
    assert_eq!(updates[2].successor, "SIE.Q2");
    assert_eq!(apply_updates(&mut local, &remote, &updates), 2);
    assert_eq!(local[0].description, "new");
    assert!(local.iter().any(|p| p.number == "SIE.K3"));
    assert!(check_updates(&local, &remote).iter().all(|u| u.kind == UpdateKind::Removed));
}

#[test]
fn manufacturers_edit_their_own_parts_only() {
    let user = |role, m: &str| User { name: "u".into(), role, manufacturer: m.into(), ..Default::default() };
    assert!(user(Role::Manufacturer, "ABB").may_edit("abb "));
    assert!(!user(Role::Manufacturer, "ABB").may_edit("Siemens"));
    assert!(!user(Role::Manufacturer, "").may_edit(""));
    assert!(user(Role::Editor, "").may_edit("Siemens"));
    assert!(!user(Role::Reader, "ABB").may_edit("ABB"));
    assert!(Role::Admin > Role::Editor && Role::Editor > Role::Manufacturer);
}

#[test]
fn roles_give_rights() {
    use crate::access::{Grant, Policy, Right, Subject, SERVER};
    let maker = User { name: "max".into(), role: Role::Manufacturer, manufacturer: "ABB".into(), groups: vec!["corp:Purchasing".into()], ..Default::default() };
    let access = maker.access();
    assert!(access.allows(SERVER, PartsRight::Read));
    assert!(access.allows("ABB", PartsRight::EditCatalogs));
    assert!(!access.allows("Siemens", PartsRight::DeleteParts));
    assert!(!access.allows(SERVER, PartsRight::Administer));
    let policy = Policy::<PartsRight> { grants: vec![Grant::new(Subject::Group("corp:Purchasing".into()), "editor", "Sie*")], ..Default::default() };
    assert!(policy.access(&maker.identity(), &maker.grants()).allows("Siemens", PartsRight::DeleteParts));
    for role in Role::ALL {
        assert_eq!(PartsRight::role(role.name()), Some(role.rights()));
    }
    assert_eq!(PartsRight::ROLES, Role::ALL.map(Role::name));
    let admin = User { role: Role::Admin, ..Default::default() };
    assert!(admin.access().allows(SERVER, PartsRight::Administer));
}

#[test]
fn attachment_kinds_and_media_types() {
    assert_eq!(AttachmentKind::guess("K1 datasheet.PDF"), AttachmentKind::Datasheet);
    assert_eq!(AttachmentKind::guess("manual_de.pdf"), AttachmentKind::Manual);
    assert_eq!(AttachmentKind::guess("model.stp"), AttachmentKind::Model3d);
    assert_eq!(AttachmentKind::guess("contactor.x_t"), AttachmentKind::Model3d);
    assert_eq!(AttachmentKind::guess("motor-macros.json"), AttachmentKind::Macro);
    assert_eq!(AttachmentKind::guess("photo.jpeg"), AttachmentKind::Image);
    assert_eq!(content_type("a.PDF"), "application/pdf");
    assert_eq!(content_type("noext"), "application/octet-stream");
}

#[test]
fn keys_and_catalog_ids() {
    for l in Lifecycle::ALL {
        assert_eq!(lifecycle_from_key(&format!("{l:?}")), Some(l));
    }
    assert_eq!(top_group_from_key("Process"), Some(TopGroup::Process));
    assert_eq!(catalog_id(" ABB Contactors 2026! "), "abb_contactors_2026");
}

#[test]
fn roles_and_file_kinds_are_read_in_either_case() {
    assert_eq!(serde_json::from_str::<Role>(r#""editor""#).unwrap(), Role::Editor);
    assert_eq!(serde_json::from_str::<Role>(r#""Editor""#).unwrap(), Role::Editor);
    assert_eq!(serde_json::from_str::<AttachmentKind>(r#""model3d""#).unwrap(), AttachmentKind::Model3d);
    assert_eq!(serde_json::to_string(&Role::Admin).unwrap(), r#""Admin""#);
}
