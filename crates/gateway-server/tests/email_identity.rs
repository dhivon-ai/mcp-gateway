mod common;
use common::{Harness, PASSWORD};
use serde_json::{Value, json};

async fn revision(h: &Harness, admin: &str) -> u64 {
    h.request("GET", "/api/admin/configuration", Some(admin), None)
        .await
        .2["revision"]
        .as_u64()
        .expect("revision")
}
async fn login(h: &Harness, email: &str) -> (u16, Value) {
    let (s, _, v) = h
        .request(
            "POST",
            "/api/endpoint/login",
            None,
            Some(json!({"email":email,"password":PASSWORD})),
        )
        .await;
    (s, v)
}
#[tokio::test]
#[ignore = "requires isolated TEST_DATABASE_URL"]
async fn email_only_login_profiles_uniqueness_and_revocation() {
    let h = Harness::new().await;
    let admin = h.bootstrap().await;
    let create = |email: &str, name: &str, rev: u64| json!({"expectedRevision":rev,"email":email,"name":name,"password":PASSWORD,"role":"user"});
    let (s, _, alice) = h
        .request(
            "POST",
            "/api/admin/users",
            Some(&admin.token),
            Some(create(
                " Alice+Dev@Example.Test ",
                "  Alice Developer  ",
                revision(&h, &admin.token).await,
            )),
        )
        .await;
    assert_eq!(s, 200);
    assert_eq!(alice["email"], "alice+dev@example.test");
    assert_eq!(alice["name"], "Alice Developer");
    assert!(alice.get("password").is_none() && alice.get("username").is_none());
    assert_eq!(
        h.request(
            "POST",
            "/api/admin/users",
            Some(&admin.token),
            Some(create(
                "ALICE+DEV@example.test",
                "Other name",
                revision(&h, &admin.token).await
            ))
        )
        .await
        .0,
        400
    );
    for bad in [
        "alice",
        "a@@example.test",
        ".a@example.test",
        "a..b@example.test",
        "a@-bad.test",
        "a@b..test",
        "a@exa mple.test",
        "a\n@example.test",
    ] {
        assert_eq!(
            h.request(
                "POST",
                "/api/admin/users",
                Some(&admin.token),
                Some(create(bad, "Valid name", revision(&h, &admin.token).await))
            )
            .await
            .0,
            400
        );
    }
    for bad in [
        "".to_owned(),
        "   ".to_owned(),
        "x".repeat(201),
        "Alice\nDeveloper".to_owned(),
    ] {
        assert_eq!(
            h.request(
                "POST",
                "/api/admin/users",
                Some(&admin.token),
                Some(create(
                    "other@example.test",
                    &bad,
                    revision(&h, &admin.token).await
                ))
            )
            .await
            .0,
            400
        );
    }
    let (s, session) = login(&h, " ALICE+DEV@EXAMPLE.TEST ").await;
    assert_eq!(s, 200);
    let token = session["token"].as_str().expect("token");
    assert_eq!(login(&h, "Alice Developer").await.0, 401);
    assert_eq!(login(&h, "alice").await.0, 401);
    // The old field is not a compatibility alias.
    assert_eq!(
        h.request(
            "POST",
            "/api/endpoint/login",
            None,
            Some(json!({"username":"alice","password":PASSWORD}))
        )
        .await
        .0,
        400
    );
    let id = alice["id"].as_str().expect("user id");
    let (_, _, group) = h
        .request(
            "POST",
            "/api/admin/groups",
            Some(&admin.token),
            Some(json!({"name":"Engineering","memberIds":[id]})),
        )
        .await;
    let rev = revision(&h, &admin.token).await;
    let body = json!({"expectedRevision":rev,"name":"Alice Display","email":"Changed@Example.Test","role":"user","active":true});
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/users/{id}"),
            Some(token),
            Some(body.clone())
        )
        .await
        .0,
        403
    );
    let (s, _, edited) = h
        .request(
            "PUT",
            &format!("/api/admin/users/{id}"),
            Some(&admin.token),
            Some(body.clone()),
        )
        .await;
    assert_eq!(s, 200);
    assert_eq!(edited["id"], alice["id"]);
    assert_eq!(edited["name"], "Alice Display");
    assert_eq!(edited["email"], "changed@example.test");
    assert_eq!(
        h.request("GET", "/api/config", Some(token), None).await.0,
        401
    );
    assert_eq!(login(&h, "alice+dev@example.test").await.0, 401);
    assert_eq!(login(&h, "Changed@Example.Test").await.0, 200);
    assert_eq!(login(&h, "Alice Display").await.0, 401);
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/users/{id}"),
            Some(&admin.token),
            Some(body)
        )
        .await
        .0,
        409
    );
    let (_, _, groups) = h
        .request("GET", "/api/admin/groups", Some(&admin.token), None)
        .await;
    assert_eq!(groups[0]["id"], group["id"]);
    assert_eq!(groups[0]["memberIds"], json!([id]));
    let (_, _, users) = h
        .request("GET", "/api/admin/users", Some(&admin.token), None)
        .await;
    assert!(!users.to_string().contains(PASSWORD));
    assert!(!users.to_string().contains("password"));
    // Hold the row after lookup/password verification until email changes.
    let mut tx = h.state.pool.begin().await.expect("owned concurrent edit");
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(uuid::Uuid::parse_str(id).expect("UUID"))
        .execute(&mut *tx)
        .await
        .expect("lock profile");
    let (result, ()) = tokio::join!(login(&h, "changed@example.test"), async {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'SELECT id,email,name,role,active FROM users WHERE id=%')").fetch_one(&h.state.pool).await.expect("owned login fence witness");
                if blocked { break; }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }).await.expect("login reached final identity fence");
        sqlx::query("UPDATE users SET email='new-login@example.test' WHERE id=$1")
            .bind(uuid::Uuid::parse_str(id).expect("UUID"))
            .execute(&mut *tx)
            .await
            .expect("concurrent email edit");
        tx.commit().await.expect("commit email edit");
    });
    assert_eq!(
        result.0, 401,
        "the old email cannot mint a session after the rename"
    );
    h.cleanup().await;
}
#[tokio::test]
#[ignore = "requires isolated TEST_DATABASE_URL"]
async fn populated_old_identity_migration_is_rejected_without_data_loss() {
    let h = Harness::new().await;
    let mut tx = h.state.pool.begin().await.expect("owned transaction");
    sqlx::query("CREATE SCHEMA legacy_email_test")
        .execute(&mut *tx)
        .await
        .expect("owned schema");
    sqlx::query("SET LOCAL search_path=legacy_email_test")
        .execute(&mut *tx)
        .await
        .expect("owned schema scope");
    sqlx::raw_sql(include_str!("../migrations/0001_identity.sql"))
        .execute(&mut *tx)
        .await
        .expect("old schema");
    let id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO users(id,username,password_hash,role) VALUES($1,'legacy','fixture-hash','admin')").bind(id).execute(&mut *tx).await.expect("old fixture account");
    sqlx::query("SAVEPOINT before_identity_change")
        .execute(&mut *tx)
        .await
        .expect("savepoint");
    let error = sqlx::raw_sql(include_str!("../migrations/0004_email_identity.sql"))
        .execute(&mut *tx)
        .await
        .expect_err("populated old schema must reject");
    assert!(error.to_string().contains("fresh pre-release database"));
    sqlx::query("ROLLBACK TO SAVEPOINT before_identity_change")
        .execute(&mut *tx)
        .await
        .expect("recover transaction");
    let old: (uuid::Uuid, String) = sqlx::query_as("SELECT id,username FROM users")
        .fetch_one(&mut *tx)
        .await
        .expect("old data preserved");
    assert_eq!(old, (id, "legacy".into()));
    tx.rollback().await.expect("remove owned schema");
    h.cleanup().await;
}
