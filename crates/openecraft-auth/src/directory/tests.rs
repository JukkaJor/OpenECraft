use super::*;

fn ad() -> Directory {
    Directory {
        id: "corp".into(),
        url: "ldaps://dc1.corp.example".into(),
        bind_dn: "CN=svc,DC=corp,DC=example".into(),
        base_dn: "DC=corp,DC=example".into(),
        ..Default::default()
    }
}

#[test]
fn user_filters_escape_the_name() {
    let d = ad();
    assert_eq!(d.user_filter("anna"), "(&(objectClass=user)(sAMAccountName=anna))");
    assert_eq!(d.user_filter("*)(cn=*"), r"(&(objectClass=user)(sAMAccountName=\2a\29\28cn=\2a))");
    let ldap = Directory { kind: DirectoryKind::Ldap, ..ad() };
    assert_eq!(ldap.user_filter("bob"), "(&(objectClass=person)(uid=bob))");
    let custom = Directory { user_filter: "(mail={user})".into(), ..ad() };
    assert_eq!(custom.user_filter("a@b"), "(mail=a@b)");
}

#[test]
fn bind_names_without_service_account() {
    let upn = Directory { bind_dn: String::new(), user_bind: "{user}@corp.example".into(), ..ad() };
    assert_eq!(upn.user_bind("anna"), "anna@corp.example");
    let dn = Directory { user_bind: "uid={user},ou=people,dc=example".into(), ..upn };
    assert_eq!(dn.user_bind("a,b"), r"uid=a\2cb,ou=people,dc=example");
}

#[test]
fn group_lookups() {
    let d = ad();
    assert_eq!(d.group_filter("CN=Anna,DC=corp", "anna"), None, "memberOf is read");
    let nested = Directory { nested_groups: true, ..ad() };
    assert_eq!(nested.group_filter("CN=Anna (x),DC=corp", "anna").unwrap(), r"(&(objectClass=group)(member:1.2.840.113556.1.4.1941:=CN=Anna \28x\29,DC=corp))");
    let ldap = Directory { kind: DirectoryKind::Ldap, group_filter: "(&(objectClass=posixGroup)(memberUid={user}))".into(), ..ad() };
    assert_eq!(ldap.group_filter("uid=bob", "bob").unwrap(), "(&(objectClass=posixGroup)(memberUid=bob))");
}

#[test]
fn common_names_of_dns() {
    assert_eq!(common_name("CN=Engineering,OU=Groups,DC=corp,DC=example").as_deref(), Some("Engineering"));
    assert_eq!(common_name(r"CN=Smith\, John,OU=People").as_deref(), Some("Smith, John"));
    assert_eq!(common_name(r"cn=M\C3\BCller+uid=7,dc=x").as_deref(), Some("Müller"));
    assert_eq!(common_name("not a dn"), None);
    assert_eq!(common_name("CN=,DC=x"), None);
}

#[test]
fn groups_are_qualified_and_can_limit_sign_in() {
    let d = Directory { allowed_groups: vec!["OpenECraft*".into()], ..ad() };
    let groups = d.qualify(["OpenECraft Users".to_string(), "Staff".into(), "staff".into()]);
    assert_eq!(groups, ["corp:OpenECraft Users", "corp:Staff"]);
    assert!(d.allows(&groups));
    assert!(!d.allows(&d.qualify(["Staff".to_string()])));
    assert!(ad().allows(&[]));
    let qualified = Directory { allowed_groups: vec!["corp:Staff".into()], ..ad() };
    assert!(qualified.allows(&groups));
}

#[test]
fn directories_are_checked() {
    assert!(ad().check().is_ok());
    let bad = [
        Directory { id: "a b".into(), ..ad() },
        Directory { id: "a:b".into(), ..ad() },
        Directory { url: "http://dc1".into(), ..ad() },
        Directory { bind_dn: String::new(), ..ad() },
        Directory { base_dn: String::new(), ..ad() },
    ];
    for d in bad {
        assert!(d.check().is_err(), "{d:?}");
    }
    let p = Directory { title: "Corporate AD".into(), ..ad() }.provider();
    assert_eq!((p.id.as_str(), p.title.as_str(), p.kind.as_str()), ("corp", "Corporate AD", "active_directory"));
    assert_eq!(ad().provider().title, "corp");
}

#[tokio::test]
async fn empty_passwords_never_reach_the_directory() {
    // An empty password would be an anonymous bind, which LDAP servers accept.
    let unreachable = Directory { url: "ldap://127.0.0.1:1".into(), timeout_secs: 1, ..ad() };
    assert_eq!(unreachable.authenticate("anna", "").await, Err(SignInError::Refused));
    assert_eq!(unreachable.authenticate(" ", "pw").await, Err(SignInError::Refused));
    assert_eq!(sign_in(&[], "", "anna", "pw").await, Err(SignInError::Unknown));
}

#[tokio::test]
async fn unreachable_directories_are_reported() {
    let unreachable = Directory { url: "ldap://127.0.0.1:1".into(), timeout_secs: 1, ..ad() };
    assert!(matches!(unreachable.authenticate("anna", "pw").await, Err(SignInError::Unavailable(_))));
    let result = sign_in(std::slice::from_ref(&unreachable), "", "anna", "pw").await;
    assert_eq!(result, Err(SignInError::Unavailable("directory corp is not available".into())));
    assert_eq!(sign_in(&[unreachable], "other", "anna", "pw").await, Err(SignInError::Unknown), "only the named directory");
}
