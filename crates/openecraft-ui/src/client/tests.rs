use super::*;

#[test]
fn poll_updates_status_and_info() {
    let mut client = ServerClient { pending: 2, ..Default::default() };
    {
        let mut inbox = client.inbox.lock().unwrap();
        inbox.push(Reply::Info(Ok(ServerInfo { name: "srv".into(), writable: true, ..Default::default() })));
        inbox.push(Reply::Deleted("A".into(), Ok(())));
    }
    let replies = client.poll();
    assert_eq!(replies.len(), 2);
    assert!(!client.busy());
    assert!(client.writable());
    assert_eq!(client.status, "Deleted part A");
    assert!(!client.failed());

    client.inbox.lock().unwrap().push(Reply::Info(Err("connection failed: refused".into())));
    client.poll();
    assert!(client.failed());
    assert!(client.info.is_none());

    client.inbox.lock().unwrap().push(Reply::Deleted("B".into(), Ok(())));
    client.poll();
    assert!(!client.failed(), "a later success clears the error");
}

#[test]
fn signing_in_and_out_changes_the_token() {
    use openecraft_parts::api::Session;
    use openecraft_parts::portal::{Role, User};
    let mut client = ServerClient { pending: 3, token: "old".into(), info: Some(ServerInfo::default()), ..Default::default() };
    let user = User { name: "ann".into(), role: Role::Editor, manufacturer: String::new(), ..Default::default() };
    client.inbox.lock().unwrap().push(Reply::SignedIn(Ok(Session { token: "session".into(), user: user.clone(), ..Default::default() })));
    client.poll();
    assert_eq!(client.token, "session");
    assert_eq!(client.user(), Some(&user));
    client.inbox.lock().unwrap().push(Reply::SignedIn(Err("wrong user name or password".into())));
    client.poll();
    assert_eq!(client.token, "session", "a failed sign-in keeps the session");
    assert!(client.failed());
    client.inbox.lock().unwrap().push(Reply::SignedOut);
    client.poll();
    assert!(client.token.is_empty() && client.user().is_none());
}

#[test]
fn each_kind_keeps_its_address_and_token() {
    let mut client = ServerClient { token: "admin".into(), info: Some(ServerInfo::default()), ..Default::default() };
    client.set_kind(PortalKind::Eplan);
    assert_eq!((client.url.as_str(), client.token.as_str()), (openecraft_client::eplan::EPLAN_URL, ""));
    assert!(client.read_only() && client.info.is_none());
    client.token = "pat".into();
    client.set_url_of(PortalKind::OpenECraft, "http://other:8090".into());
    assert_eq!(client.url_of(PortalKind::OpenECraft), "http://other:8090");
    client.set_kind(PortalKind::OpenECraft);
    assert_eq!((client.url.as_str(), client.token.as_str()), ("http://other:8090", "admin"));
    client.set_kind(PortalKind::Eplan);
    assert_eq!(client.token, "pat");
}

#[test]
fn the_eplan_data_portal_refuses_changes() {
    let ctx = egui::Context::default();
    let mut client = ServerClient::default();
    client.set_kind(PortalKind::Eplan);
    client.catalogs(&ctx);
    client.save(&ctx, &Part::default());
    let replies = client.poll();
    assert!(matches!(replies[0], Reply::Catalogs(Err(_))) && matches!(replies[1], Reply::Saved(Err(_))));
    assert!(client.failed() && !client.busy());
    assert_eq!(client.file_link("download/dxf_data/part/1"), None);
}
