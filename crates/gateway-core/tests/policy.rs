use gateway_core::*;
use std::collections::BTreeMap;

fn snapshot() -> ConfigSnapshot {
    ConfigSnapshot {
        schema_version: CONFIG_SCHEMA_VERSION,
        deployment_id: "org".into(),
        user: UserInfo {
            name: "Fixture User".into(),
            id: "u1".into(),
            email: "alice@example.test".into(),
            role: Role::User,
            active: true,
        },
        revision: 1,
        validated_at: 1000,
        expires_at: 4600,
        session_expires_at: 50_000,
        mcps: vec![EffectiveMcp {
            definition: McpDefinition {
                id: "m1".into(),
                name: "Git".into(),
                description: String::new(),
                active: true,
                transport: Transport::Stdio {
                    command: CommandSpec {
                        program: "git-mcp".into(),
                        args: vec![],
                        env: BTreeMap::new(),
                        cwd: None,
                    },
                },
                install: None,
                auth: AuthMode::None {},
                tool_policy: ToolPolicy::Allowlist(vec!["read".into()]),
            },
            enabled: true,
            secret: None,
        }],
    }
}

#[test]
fn grants_are_additive_and_absence_denies() {
    let make = |target| Assignment {
        id: "a".into(),
        mcp_id: "m1".into(),
        target,
    };
    assert!(!has_assignment(&[], "u1", &[], "m1"));
    assert!(has_assignment(
        &[make(GrantTarget::User { id: "u1".into() })],
        "u1",
        &[],
        "m1"
    ));
    assert!(!has_assignment(
        &[make(GrantTarget::User { id: "u2".into() })],
        "u1",
        &[],
        "m1"
    ));
    assert!(has_assignment(
        &[make(GrantTarget::Group { id: "g".into() })],
        "u1",
        &["g".into()],
        "m1"
    ));
    assert!(has_assignment(
        &[make(GrantTarget::All {})],
        "u1",
        &[],
        "m1"
    ));
    assert!(!has_assignment(
        &[make(GrantTarget::All {})],
        "u1",
        &[],
        "other"
    ));
}

#[test]
fn expiry_is_exclusive_and_cannot_be_extended_by_a_read() {
    let cache = snapshot();
    assert!(cache.validate("org", "u1", 4599).is_ok());
    assert_eq!(cache.validate("org", "u1", 4600), Err(PolicyError::Expired));
    assert_eq!(cache.validate("org", "u1", 4601), Err(PolicyError::Expired));
    assert_eq!(cache.expires_at, 4600);
}

#[test]
fn invalid_identity_clock_and_lease_fail_closed() {
    let mut cache = snapshot();
    assert_eq!(
        cache.validate("other", "u1", 1001),
        Err(PolicyError::Identity)
    );
    assert_eq!(
        cache.validate("org", "u2", 1001),
        Err(PolicyError::Identity)
    );
    assert_eq!(
        cache.validate("org", "u1", 999),
        Err(PolicyError::ClockRollback)
    );
    cache.expires_at = 4601;
    assert_eq!(cache.validate("org", "u1", 1001), Err(PolicyError::Invalid));
    cache.expires_at = i64::MAX;
    cache.validated_at = i64::MIN;
    assert_eq!(cache.validate("org", "u1", 1001), Err(PolicyError::Invalid));
}

#[test]
fn every_dispatch_axis_is_checked() {
    let base = snapshot();
    assert!(
        base.authorize_tool("org", "u1", 1001, "m1", "read", true)
            .is_ok()
    );
    assert_eq!(
        base.authorize_tool("org", "u1", 1001, "m1", "write", true),
        Err(PolicyError::ToolDenied)
    );
    assert_eq!(
        base.authorize_tool("org", "u1", 1001, "m1", "read", false),
        Err(PolicyError::AuthRequired)
    );
    assert_eq!(
        base.authorize_tool("org", "u1", 4600, "m1", "read", true),
        Err(PolicyError::Expired)
    );
    assert_eq!(
        base.authorize_tool("org", "u1", 1001, "other", "read", true),
        Err(PolicyError::Unassigned)
    );
    let mut cache = base.clone();
    cache.mcps[0].enabled = false;
    assert_eq!(
        cache.authorize_tool("org", "u1", 1001, "m1", "read", true),
        Err(PolicyError::NotEnabled)
    );
    cache.mcps[0].enabled = true;
    cache.mcps[0].definition.active = false;
    assert_eq!(
        cache.authorize_tool("org", "u1", 1001, "m1", "read", true),
        Err(PolicyError::Unassigned)
    );
    cache.user.active = false;
    assert_eq!(
        cache.authorize_tool("org", "u1", 1001, "m1", "read", true),
        Err(PolicyError::DisabledIdentity)
    );
}

#[test]
fn tool_modes_define_new_tool_behavior() {
    assert!(!ToolPolicy::default().allows("new"));
    assert!(!ToolPolicy::Allowlist(vec!["read".into()]).allows("new"));
    assert!(ToolPolicy::Denylist(vec!["write".into()]).allows("new"));
    assert!(!ToolPolicy::Denylist(vec!["write".into()]).allows("write"));
}

#[test]
fn contracts_round_trip_and_reject_unknown_token_fields() {
    let original = snapshot();
    let json = serde_json::to_value(&original).expect("serialize fixture");
    assert!(json.get("validatedAt").is_some());
    assert_eq!(
        serde_json::from_value::<ConfigSnapshot>(json).expect("valid snapshot"),
        original
    );
    assert!(
        serde_json::from_value::<AuthMode>(
            serde_json::json!({"kind":"bearer","token":"do-not-store"})
        )
        .is_err()
    );
    let mut copy = original.clone();
    copy.mcps.push(copy.mcps[0].clone());
    assert_eq!(copy.validate("org", "u1", 1001), Err(PolicyError::Invalid));
}

#[test]
fn production_url_validation_never_accepts_insecure_or_embedded_credentials() {
    assert!(validate_remote_url("https://mcp.example.com/mcp", false).is_ok());
    assert!(validate_remote_url("http://127.0.0.1:8000/mcp", false).is_err());
    assert!(validate_remote_url("http://127.0.0.1:8000/mcp", true).is_ok());
    assert!(validate_remote_url("http://example.com/mcp", true).is_err());
    assert!(validate_remote_url("https://user:password@example.com/mcp", false).is_err());
    assert!(validate_remote_url("https://example.com/mcp#token", false).is_err());
    assert!(validate_remote_url("file:///private/key", false).is_err());
}

#[test]
fn bearer_binding_and_session_deadline_are_required() {
    let mut cache = snapshot();
    cache.session_expires_at = cache.expires_at - 1;
    assert_eq!(cache.validate("org", "u1", 1001), Err(PolicyError::Invalid));
    cache.session_expires_at = 50_000;
    cache.mcps[0].definition.transport = Transport::Http {
        url: "https://example.com/mcp".into(),
    };
    cache.mcps[0].definition.auth = AuthMode::Bearer {};
    assert_eq!(cache.validate("org", "u1", 1001), Err(PolicyError::Invalid));
    cache.mcps[0].secret = Some(SecretRef {
        id: "s1".into(),
        key: "work".into(),
        version: 1,
    });
    assert!(cache.validate("org", "u1", 1001).is_ok());
    assert!(
        serde_json::from_value::<GrantTarget>(serde_json::json!({"kind":"all", "id":"u2"}))
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(AuthMode::OAuth {
            client_id: "public".into(),
            scopes: vec![]
        })
        .expect("fixture"),
        serde_json::json!({"kind":"oauth", "clientId":"public", "scopes":[]})
    );
}
