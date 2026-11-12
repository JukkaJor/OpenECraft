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

fn checked(client: &ServerClient, token: &str, warning: Option<String>) -> Reply {
    Reply::Checked(Box::new(CheckedConnection {
        selection: client.connection(),
        connection: Connection::new(client.kind, &client.url, token),
        generation: client.check_generation,
        result: Ok(ServerInfo { parts: 17, ..Default::default() }),
        warning,
    }))
}

#[test]
fn session_cache_is_endpoint_specific_and_explicit_tokens_win() {
    let mut client = ServerClient::default();
    client.set_kind(PortalKind::Eplan);
    assert!(!client.remember_pat);
    client.inbox.lock().unwrap().push(checked(&client, "saved-fixture", None));
    client.poll();
    let cached = client.request_connection().unwrap();
    assert_eq!(cached, Connection::new(client.kind, &client.url, "saved-fixture"));
    // Multiple ordinary requests require no store access or re-check.
    assert_eq!(client.request_connection().unwrap(), cached);
    client.token = "explicit-fixture".into();
    assert_eq!(client.request_connection().unwrap(), client.connection());
    client.token.clear();
    client.url = "https://other.test/api".into();
    assert_eq!(client.request_connection().unwrap(), client.connection());
}

#[test]
fn save_warning_keeps_info_and_a_usable_session_without_exposing_the_token() {
    let other = ServerClient { token: "private-fixture".into(), ..Default::default() };
    assert!(!format!("{:?}", checked(&other, "private-fixture", None)).contains("private-fixture"));
    let mut client = ServerClient::default();
    client.set_kind(PortalKind::Eplan);
    let reply = checked(&client, "private-fixture", Some("Could not save the shared Data Portal token (Windows error 1312)".into()));
    assert!(!format!("{reply:?}").contains("private-fixture"));
    client.inbox.lock().unwrap().push(reply);
    let replies = client.poll();
    assert!(matches!(replies.as_slice(), [Reply::Info(Ok(_))]), "parts manager must still receive the Info notification");
    assert!(!client.failed());
    assert_eq!(client.info.as_ref().unwrap().parts, 17);
    assert!(client.status.contains("Connected:") && client.status.contains("Could not save"));
    assert!(!client.status.contains("private-fixture"));
    assert_eq!(client.request_connection().unwrap(), Connection::new(client.kind, &client.url, "private-fixture"));
}

struct ForgetStore(bool);
impl CredentialStore for ForgetStore {
    fn load(&self, _: &str) -> Result<Option<String>, credentials::CredentialError> { panic!("ordinary GUI requests must use the cache") }
    fn save(&self, _: &str, _: &str) -> Result<bool, credentials::CredentialError> { panic!("forget must not save") }
    fn forget(&self, _: &str) -> Result<bool, credentials::CredentialError> { Ok(self.0) }
}

#[test]
fn forgetting_reports_absence_and_invalidates_pending_checks() {
    for removed in [false, true] {
        let mut client = ServerClient::default();
        client.set_kind(PortalKind::Eplan);
        let old = checked(&client, "old-fixture", None);
        client.forget_saved_pat_with(&ForgetStore(removed));
        assert_eq!(client.status, if removed { "Saved Data Portal token removed" } else { "No saved Data Portal token for this endpoint" });
        client.inbox.lock().unwrap().push(old);
        client.poll();
        assert!(client.info.is_none() && client.cached_connection.is_none());
        assert_eq!(client.request_connection().unwrap(), client.connection());
    }
}

#[test]
fn pending_check_cannot_follow_a_changed_endpoint_or_explicit_token() {
    for change_token in [false, true] {
        let mut client = ServerClient::default();
        client.set_kind(PortalKind::Eplan);
        let old = checked(&client, "old-fixture", None);
        if change_token { client.token = "replacement-fixture".into(); } else { client.url = "https://other.test/api".into(); }
        client.inbox.lock().unwrap().push(old);
        client.poll();
        assert!(client.info.is_none() && client.cached_connection.is_none());
    }
}
