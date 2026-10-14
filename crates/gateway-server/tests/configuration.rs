mod common;
use common::{Harness, PASSWORD};
use gateway_core::{ConfigSnapshot, McpDefinition};
use serde_json::{Value, json};
use uuid::Uuid;

fn mcp() -> Value {
    json!({"id":Uuid::new_v4(),"name":"Fixture MCP","description":"Controlled upstream","active":true,"transport":{"kind":"http","url":"http://127.0.0.1:18081/mcp"},"install":null,"auth":{"kind":"none"},"toolPolicy":{"mode":"allowlist","tools":[]}})
}
async fn revision(h: &Harness, admin: &str) -> u32 {
    let (s, _, v) = h
        .request("GET", "/api/admin/configuration", Some(admin), None)
        .await;
    assert_eq!(s, 200);
    v["revision"].as_u64().expect("revision") as u32
}
async fn snapshot(h: &Harness, token: &str) -> ConfigSnapshot {
    let (s, _, v) = h.request("GET", "/api/config", Some(token), None).await;
    assert_eq!(s, 200);
    serde_json::from_value(v).expect("snapshot")
}
async fn grant(h: &Harness, admin: &str, mcp: &str, target: Value) -> String {
    let (s, _, v) = h
        .request(
            "POST",
            "/api/admin/assignments",
            Some(admin),
            Some(json!({"expectedRevision":revision(h,admin).await,"mcpId":mcp,"target":target})),
        )
        .await;
    assert_eq!(s, 200);
    v["id"].as_str().expect("assignment id").into()
}
async fn revoke(h: &Harness, admin: &str, id: &str) {
    assert_eq!(
        h.request(
            "DELETE",
            &format!("/api/admin/assignments/{id}"),
            Some(admin),
            Some(json!({"expectedRevision":revision(h,admin).await}))
        )
        .await
        .0,
        200
    );
}
async fn enable(h: &Harness, token: &str, id: &str) -> ConfigSnapshot {
    let old = snapshot(h, token).await;
    let (s, _, v) = h
        .request(
            "PUT",
            &format!("/api/settings/{id}"),
            Some(token),
            Some(json!({"expectedRevision":old.revision,"enabled":true,"secretId":null})),
        )
        .await;
    assert_eq!(s, 200);
    serde_json::from_value(v).expect("confirmed snapshot")
}

#[tokio::test]
#[ignore = "requires isolated TEST_DATABASE_URL"]
async fn additive_grants_last_removal_and_policy_edit() {
    let h = Harness::new().await;
    let admin = h.bootstrap().await;
    let alice_id = h.user(&admin.token, "alice").await;
    let bob_id = h.user(&admin.token, "bob").await;
    let alice = h.login("alice", PASSWORD).await;
    let bob = h.login("bob", PASSWORD).await;
    let mut def = mcp();
    let id = def["id"].as_str().expect("mcp ID").to_string();
    let path = format!("/api/settings/{id}");
    let rev = revision(&h, &admin.token).await;
    assert_eq!(
        h.request(
            "POST",
            "/api/admin/mcps",
            Some(&alice.token),
            Some(json!({"expectedRevision":rev,"definition":def}))
        )
        .await
        .0,
        403
    );
    assert_eq!(
        h.request(
            "POST",
            "/api/admin/mcps",
            Some(&admin.token),
            Some(json!({"expectedRevision":rev,"definition":def}))
        )
        .await
        .0,
        200
    );
    assert!(snapshot(&h, &alice.token).await.mcps.is_empty());
    assert_eq!(h.request("PUT",&path,Some(&alice.token),Some(json!({"expectedRevision":revision(&h,&admin.token).await,"enabled":true,"secretId":null}))).await.0,403);
    assert_eq!(
        h.request(
            "POST",
            "/api/endpoint/inventory",
            Some(&alice.token),
            Some(json!({"mcpId":id,"names":["echo"]}))
        )
        .await
        .0,
        403
    );
    let (_, _, group) = h
        .request(
            "POST",
            "/api/admin/groups",
            Some(&admin.token),
            Some(json!({"name":"Engineering","memberIds":[alice_id]})),
        )
        .await;
    let group_id = group["id"].as_str().expect("group id");
    let user_grant = grant(&h, &admin.token, &id, json!({"kind":"user","id":alice_id})).await;
    let group_grant = grant(&h, &admin.token, &id, json!({"kind":"group","id":group_id})).await;
    let all_grant = grant(&h, &admin.token, &id, json!({"kind":"all"})).await;
    assert!(!snapshot(&h, &alice.token).await.mcps[0].enabled);
    assert!(!snapshot(&h, &bob.token).await.mcps[0].enabled);
    let enabled = enable(&h, &alice.token, &id).await;
    assert!(enabled.mcps[0].enabled);
    assert_eq!(enabled.expires_at - enabled.validated_at, 3600);
    assert!(enabled.expires_at <= enabled.session_expires_at);
    assert!(
        enabled
            .authorize_tool(
                &enabled.deployment_id,
                &alice_id,
                enabled.validated_at,
                &id,
                "echo",
                true
            )
            .is_err()
    );
    assert_eq!(
        h.request(
            "POST",
            "/api/endpoint/inventory",
            Some(&alice.token),
            Some(json!({"mcpId":id,"names":["echo","echo","new_tool"]}))
        )
        .await
        .0,
        204
    );
    let current = snapshot(&h, &alice.token).await;
    assert_eq!(current.revision, enabled.revision);
    assert!(current.mcps[0].enabled);
    assert_eq!(
        current.mcps[0].definition.tool_policy,
        enabled.mcps[0].definition.tool_policy
    );
    let (_, _, admin_state) = h
        .request("GET", "/api/admin/configuration", Some(&admin.token), None)
        .await;
    assert_eq!(
        admin_state["inventory"][0]["names"],
        json!(["echo", "new_tool"])
    );
    def["toolPolicy"] = json!({"mode":"denylist","tools":["fail"]});
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/mcps/{id}"),
            Some(&admin.token),
            Some(json!({"expectedRevision":revision(&h,&admin.token).await,"definition":def}))
        )
        .await
        .0,
        200
    );
    let current = snapshot(&h, &alice.token).await;
    assert!(current.mcps[0].enabled);
    assert!(
        current
            .authorize_tool(
                &current.deployment_id,
                &alice_id,
                current.validated_at,
                &id,
                "new_tool",
                true
            )
            .is_ok()
    );
    assert!(
        current
            .authorize_tool(
                &current.deployment_id,
                &alice_id,
                current.validated_at,
                &id,
                "fail",
                true
            )
            .is_err()
    );
    revoke(&h, &admin.token, &user_grant).await;
    assert!(snapshot(&h, &alice.token).await.mcps[0].enabled);
    revoke(&h, &admin.token, &group_grant).await;
    assert!(snapshot(&h, &alice.token).await.mcps[0].enabled);
    revoke(&h, &admin.token, &all_grant).await;
    assert!(snapshot(&h, &alice.token).await.mcps.is_empty());
    let group_grant = grant(&h, &admin.token, &id, json!({"kind":"group","id":group_id})).await;
    assert!(!snapshot(&h, &alice.token).await.mcps[0].enabled);
    enable(&h, &alice.token, &id).await;
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/groups/{group_id}"),
            Some(&admin.token),
            Some(json!({"name":"Engineering","memberIds":[bob_id]}))
        )
        .await
        .0,
        200
    );
    assert!(snapshot(&h, &alice.token).await.mcps.is_empty());
    assert!(!snapshot(&h, &bob.token).await.mcps[0].enabled);
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/groups/{group_id}"),
            Some(&admin.token),
            Some(json!({"name":"Engineering","memberIds":[alice_id]}))
        )
        .await
        .0,
        200
    );
    assert!(!snapshot(&h, &alice.token).await.mcps[0].enabled);
    enable(&h, &alice.token, &id).await;
    assert_eq!(
        h.request(
            "DELETE",
            &format!("/api/admin/groups/{group_id}"),
            Some(&admin.token),
            None
        )
        .await
        .0,
        204
    );
    assert!(snapshot(&h, &alice.token).await.mcps.is_empty());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM assignments WHERE id=$1")
        .bind(Uuid::parse_str(&group_grant).expect("UUID"))
        .fetch_one(&h.state.pool)
        .await
        .expect("cascade");
    assert_eq!(count, 0);
    grant(&h, &admin.token, &id, json!({"kind":"user","id":alice_id})).await;
    enable(&h, &alice.token, &id).await;
    def["active"] = json!(false);
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/mcps/{id}"),
            Some(&admin.token),
            Some(json!({"expectedRevision":revision(&h,&admin.token).await,"definition":def}))
        )
        .await
        .0,
        200
    );
    assert!(snapshot(&h, &alice.token).await.mcps.is_empty());
    def["active"] = json!(true);
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/mcps/{id}"),
            Some(&admin.token),
            Some(json!({"expectedRevision":revision(&h,&admin.token).await,"definition":def}))
        )
        .await
        .0,
        200
    );
    assert!(!snapshot(&h, &alice.token).await.mcps[0].enabled);
    enable(&h, &alice.token, &id).await;
    def["transport"]["url"] = json!("http://127.0.0.1:18082/mcp");
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/mcps/{id}"),
            Some(&admin.token),
            Some(json!({"expectedRevision":revision(&h,&admin.token).await,"definition":def}))
        )
        .await
        .0,
        200
    );
    assert!(!snapshot(&h, &alice.token).await.mcps[0].enabled);
    assert_eq!(
        h.request(
            "DELETE",
            &format!("/api/admin/mcps/{id}"),
            Some(&admin.token),
            Some(json!({"expectedRevision":revision(&h,&admin.token).await}))
        )
        .await
        .0,
        200
    );
    assert!(snapshot(&h, &alice.token).await.mcps.is_empty());
    h.cleanup().await;
}

#[tokio::test]
#[ignore = "requires isolated TEST_DATABASE_URL"]
async fn concurrent_settings_conflict_and_restart_durability() {
    let h = Harness::new().await;
    let admin = h.bootstrap().await;
    h.user(&admin.token, "alice").await;
    let alice = h.login("alice", PASSWORD).await;
    let second = h.login("alice", PASSWORD).await;
    let def = mcp();
    let id = def["id"].as_str().expect("mcp ID");
    let rev = revision(&h, &admin.token).await;
    assert_eq!(
        h.request(
            "POST",
            "/api/admin/mcps",
            Some(&admin.token),
            Some(json!({"expectedRevision":rev,"definition":def}))
        )
        .await
        .0,
        200
    );
    grant(&h, &admin.token, id, json!({"kind":"all"})).await;
    let before = snapshot(&h, &alice.token).await;
    let path = format!("/api/settings/{id}");
    let body = json!({"expectedRevision":before.revision,"enabled":true,"secretId":null});
    let (a, b) = tokio::join!(
        h.request("PUT", &path, Some(&alice.token), Some(body.clone())),
        h.request("PUT", &path, Some(&second.token), Some(body))
    );
    assert!(matches!((a.0, b.0), (200, 409) | (409, 200)));
    let conflict = if a.0 == 409 { a.2 } else { b.2 };
    assert_eq!(conflict["snapshot"]["mcps"][0]["enabled"], true);
    assert_eq!(conflict["revision"], before.revision + 1);
    let confirmed = snapshot(&h, &second.token).await;
    assert!(confirmed.mcps[0].enabled);
    let url: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&h.state.pool)
        .await
        .expect("database");
    let base = std::env::var("TEST_DATABASE_URL").expect("DSN");
    let mut url_dsn = url::Url::parse(&base).expect("DSN");
    url_dsn.set_path(&url);
    let restarted = gateway_server::AppState::connect(url_dsn.as_str(), h.state.options.clone())
        .await
        .expect("restart and repeat migrations");
    assert_eq!(restarted.deployment_id, h.state.deployment_id);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM user_settings")
        .fetch_one(&restarted.pool)
        .await
        .expect("persisted opt-ins");
    assert_eq!(count, 1);
    restarted.pool.close().await;
    let response = h
        .request(
            "PUT",
            &path,
            Some(&second.token),
            Some(json!({"expectedRevision":confirmed.revision,"enabled":false,"secretId":null})),
        )
        .await;
    assert_eq!(response.0, 200);
    assert!(!snapshot(&h, &alice.token).await.mcps[0].enabled);
    let invalid = McpDefinition {
        transport: gateway_core::Transport::Http {
            url: "http://remote.example/mcp".into(),
        },
        ..serde_json::from_value(def.clone()).expect("definition")
    };
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/mcps/{id}"),
            Some(&admin.token),
            Some(json!({"expectedRevision":revision(&h,&admin.token).await,"definition":invalid}))
        )
        .await
        .0,
        400
    );
    let mut invalid = def.clone();
    invalid["auth"] = json!({"kind":"bearer","token":"must-never-persist"});
    let (s, _, v) = h
        .request(
            "PUT",
            &format!("/api/admin/mcps/{id}"),
            Some(&admin.token),
            Some(json!({"expectedRevision":revision(&h,&admin.token).await,"definition":invalid})),
        )
        .await;
    assert_eq!(s, 400);
    assert!(!v.to_string().contains("must-never-persist"));

    // A concurrent opt-in cannot silently resurrect a revoked grant.
    let env: serde_json::Map<String, Value> = (0..64)
        .map(|n| (format!("KEY_{n}"), json!("a".repeat(16384))))
        .collect();
    let mut oversized = def.clone();
    oversized["transport"] = json!({"kind":"stdio","command":{"program":"/usr/bin/true","args":[],"env":env,"cwd":null}});
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/mcps/{id}"),
            Some(&admin.token),
            Some(
                json!({"expectedRevision":revision(&h,&admin.token).await,"definition":oversized})
            )
        )
        .await
        .0,
        400
    );
    let (_, _, admin_state) = h
        .request("GET", "/api/admin/configuration", Some(&admin.token), None)
        .await;
    let assignment = admin_state["assignments"][0]["id"]
        .as_str()
        .expect("assignment");
    let rev = revision(&h, &admin.token).await;
    let delete_path = format!("/api/admin/assignments/{assignment}");
    let (enable_result, revoke_result) = tokio::join!(
        h.request(
            "PUT",
            &path,
            Some(&alice.token),
            Some(json!({"expectedRevision":rev,"enabled":true,"secretId":null}))
        ),
        h.request(
            "DELETE",
            &delete_path,
            Some(&admin.token),
            Some(json!({"expectedRevision":rev}))
        )
    );
    assert!(matches!(
        (enable_result.0, revoke_result.0),
        (200, 409) | (409, 200)
    ));
    if revoke_result.0 == 409 {
        revoke(&h, &admin.token, assignment).await;
    }
    assert!(snapshot(&h, &alice.token).await.mcps.is_empty());
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM user_settings")
        .fetch_one(&h.state.pool)
        .await
        .expect("cleared opt-ins");
    assert_eq!(remaining, 0);
    h.cleanup().await;
}

#[tokio::test]
#[ignore = "requires isolated TEST_DATABASE_URL"]
async fn administrator_collections_exceed_ten_thousand_rows() {
    let h = Harness::new().await;
    let admin = h.bootstrap().await;
    let def = mcp();
    let mcp_id = Uuid::parse_str(def["id"].as_str().expect("MCP id")).expect("UUID");
    assert_eq!(
        h.request(
            "POST",
            "/api/admin/mcps",
            Some(&admin.token),
            Some(json!({"expectedRevision":revision(&h,&admin.token).await,"definition":def}))
        )
        .await
        .0,
        200
    );
    // Bulk fixture rows obey normal schema constraints; avoid hashing10,001 passwords.
    sqlx::query("INSERT INTO users(id,username,password_hash,role) SELECT gen_random_uuid(),'fixture-user-'||n, password_hash,'user' FROM users CROSS JOIN generate_series(1,10001) n WHERE username='admin'")
        .execute(&h.state.pool).await.expect("fixture users");
    sqlx::query("INSERT INTO groups(id,name) SELECT gen_random_uuid(),'Fixture group '||n FROM generate_series(1,10001) n")
        .execute(&h.state.pool).await.expect("fixture groups");
    sqlx::query("INSERT INTO assignments(id,mcp_id,target_kind,target_group) SELECT gen_random_uuid(),$1,'group',id FROM groups")
        .bind(mcp_id).execute(&h.state.pool).await.expect("fixture assignments");
    let (status, _, users) = h
        .request("GET", "/api/admin/users", Some(&admin.token), None)
        .await;
    assert_eq!(status, 200);
    let users: Vec<gateway_core::UserInfo> = serde_json::from_value(users).expect("users contract");
    assert_eq!(users.len(), 10002);
    let (status, _, groups) = h
        .request("GET", "/api/admin/groups", Some(&admin.token), None)
        .await;
    assert_eq!(status, 200);
    let groups: Vec<gateway_core::GroupInfo> =
        serde_json::from_value(groups).expect("groups contract");
    assert_eq!(groups.len(), 10001);
    let (status, _, config) = h
        .request("GET", "/api/admin/configuration", Some(&admin.token), None)
        .await;
    assert_eq!(status, 200);
    let config: gateway_core::AdminConfiguration =
        serde_json::from_value(config).expect("configuration contract");
    assert_eq!(config.assignments.len(), 10001);
    h.cleanup().await;
}
