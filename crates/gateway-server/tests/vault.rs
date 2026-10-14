mod common;
use common::{Harness, PASSWORD};
use gateway_core::{ConfigSnapshot, crypto::KeyRing};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

async fn snap(h: &Harness, token: &str) -> ConfigSnapshot {
    let (s, _, v) = h.request("GET", "/api/config", Some(token), None).await;
    assert_eq!(s, 200);
    serde_json::from_value(v).expect("config")
}
async fn assigned(h: &Harness, admin: &str, auth: Value) -> String {
    let id = Uuid::new_v4().to_string();
    let rev = snap(h, admin).await.revision;
    let def = json!({"id":id,"name":"Protected fixture","description":"Test auth","active":true,"transport":{"kind":"http","url":"http://127.0.0.1:18081/mcp"},"install":null,"auth":auth,"toolPolicy":{"mode":"denylist","tools":[]}});
    assert_eq!(
        h.request(
            "POST",
            "/api/admin/mcps",
            Some(admin),
            Some(json!({"expectedRevision":rev,"definition":def}))
        )
        .await
        .0,
        200
    );
    assert_eq!(h.request("POST","/api/admin/assignments",Some(admin),Some(json!({"expectedRevision":snap(h,admin).await.revision,"mcpId":id,"target":{"kind":"all"}}))).await.0,200);
    id
}
async fn set(h: &Harness, token: &str, mcp: &str, enabled: bool, secret: Option<&str>) {
    assert_eq!(h.request("PUT",&format!("/api/settings/{mcp}"),Some(token),Some(json!({"expectedRevision":snap(h,token).await.revision,"enabled":enabled,"secretId":secret}))).await.0,200);
}
async fn resolve(h: &Harness, token: &str, mcp: &str, installation: &str) -> (u16, Value) {
    let(s,_,v)=h.request("POST",&format!("/api/endpoint/credentials/{mcp}"),Some(token),Some(json!({"expectedRevision":snap(h,token).await.revision,"installationId":installation}))).await;
    (s, v)
}

#[tokio::test]
#[ignore = "requires isolated TEST_DATABASE_URL"]
async fn owned_write_only_secrets_and_encrypted_credential_versions() {
    let logs = common::CapturedLogs::default();
    let _guard = tracing::subscriber::set_default(
        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_max_level(tracing::Level::INFO)
            .with_writer(logs.clone())
            .finish(),
    );
    let mut h = Harness::new().await;
    let admin = h.bootstrap().await;
    h.user(&admin.token, "alice").await;
    h.user(&admin.token, "bob").await;
    let alice = h.login("alice", PASSWORD).await;
    let bob = h.login("bob", PASSWORD).await;
    let token_value = "vault-test-private-bearer-value";
    let (s, _, meta) = h
        .request(
            "POST",
            "/api/secrets",
            Some(&alice.token),
            Some(json!({"key":"work-token","value":token_value,"expectedVersion":null})),
        )
        .await;
    assert_eq!(s, 200);
    assert!(!meta.to_string().contains(token_value));
    let id = meta["id"].as_str().expect("secret id");
    let path = format!("/api/secrets/{id}");
    assert_eq!(
        h.request("GET", &path, Some(&alice.token), None).await.0,
        405
    );
    let (_, _, list) = h
        .request("GET", "/api/secrets", Some(&alice.token), None)
        .await;
    assert_eq!(list.as_array().expect("metadata").len(), 1);
    assert!(list[0].get("value").is_none());
    let (_, _, list) = h
        .request("GET", "/api/secrets", Some(&admin.token), None)
        .await;
    assert_eq!(list, json!([]));
    let (_, _, list) = h
        .request("GET", "/api/secrets", Some(&bob.token), None)
        .await;
    assert_eq!(list, json!([]));
    assert_eq!(
        h.request(
            "PUT",
            &path,
            Some(&bob.token),
            Some(json!({"key":"work-token","value":"cross-user-attack","expectedVersion":1}))
        )
        .await
        .0,
        403
    );
    assert_eq!(
        h.request(
            "PUT",
            &format!("{path}/key"),
            Some(&admin.token),
            Some(json!({"key":"attack","expectedVersion":1}))
        )
        .await
        .0,
        403
    );
    assert_eq!(
        h.request(
            "DELETE",
            &path,
            Some(&bob.token),
            Some(json!({"expectedVersion":1}))
        )
        .await
        .0,
        403
    );
    let ciphertext: Vec<u8> = sqlx::query_scalar("SELECT ciphertext FROM user_secrets WHERE id=$1")
        .bind(Uuid::parse_str(id).expect("id"))
        .fetch_one(&h.state.pool)
        .await
        .expect("encrypted column");
    assert!(!String::from_utf8_lossy(&ciphertext).contains(token_value));
    let mcp = assigned(&h, &admin.token, json!({"kind":"bearer"})).await;
    let install = Uuid::new_v4().to_string();
    let (_, headers, context) = h.request("GET", "/api/context", None, None).await;
    let preauth = headers
        .get("set-cookie")
        .expect("cookie")
        .to_str()
        .expect("cookie")
        .split(';')
        .next()
        .expect("cookie");
    let csrf = context["csrfToken"].as_str().expect("CSRF");
    let (s, headers, context) = h
        .custom(
            "POST",
            "/api/auth/login",
            None,
            Some(json!({"username":"alice","password":PASSWORD})),
            &[("cookie", preauth), ("x-csrf-token", csrf)],
        )
        .await;
    assert_eq!(s, 200);
    let cookie = headers
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|s| s.starts_with("gateway_session="))
        .expect("cookie")
        .split(';')
        .next()
        .expect("cookie");
    let csrf = context["csrfToken"].as_str().expect("CSRF");
    assert_eq!(h.custom("POST",&format!("/api/endpoint/credentials/{mcp}"),None,Some(json!({"expectedRevision":snap(&h,&alice.token).await.revision,"installationId":install})),&[("cookie",cookie),("x-csrf-token",csrf)]).await.0,403);
    assert_eq!(resolve(&h, &alice.token, &mcp, &install).await.0, 403);
    assert_eq!(h.request("PUT",&format!("/api/settings/{mcp}"),Some(&alice.token),Some(json!({"expectedRevision":snap(&h,&alice.token).await.revision,"enabled":true,"secretId":null}))).await.0,400);
    assert_eq!(h.request("PUT",&format!("/api/settings/{mcp}"),Some(&bob.token),Some(json!({"expectedRevision":snap(&h,&bob.token).await.revision,"enabled":true,"secretId":id}))).await.0,403);
    set(&h, &alice.token, &mcp, true, Some(id)).await;
    let (s, credential) = resolve(&h, &alice.token, &mcp, &install).await;
    assert_eq!(s, 200);
    assert!(credential["token"] == token_value);
    assert_eq!(credential["version"], 1);
    assert_eq!(
        h.request(
            "DELETE",
            &path,
            Some(&alice.token),
            Some(json!({"expectedVersion":1}))
        )
        .await
        .0,
        409
    );
    let (s, _, renamed) = h
        .request(
            "PUT",
            &format!("{path}/key"),
            Some(&alice.token),
            Some(json!({"key":"renamed-token","expectedVersion":1})),
        )
        .await;
    assert_eq!(s, 200);
    assert_eq!(renamed["version"], 2);
    assert_eq!(renamed["id"], id);
    assert_eq!(
        snap(&h, &alice.token).await.mcps[0]
            .secret
            .as_ref()
            .expect("binding")
            .key,
        "renamed-token"
    );
    let (s, v) = resolve(&h, &alice.token, &mcp, &install).await;
    assert_eq!(s, 200);
    assert!(v["token"] == token_value);
    assert_eq!(v["version"], 2);
    assert_eq!(
        h.request(
            "PUT",
            &path,
            Some(&alice.token),
            Some(json!({"key":"renamed-token","value":"stale-update","expectedVersion":1}))
        )
        .await
        .0,
        409
    );
    let replacement = "vault-test-replaced-bearer-value";
    assert_eq!(
        h.request(
            "PUT",
            &path,
            Some(&alice.token),
            Some(json!({"key":"renamed-token","value":replacement,"expectedVersion":2}))
        )
        .await
        .0,
        200
    );
    let (s, v) = resolve(&h, &alice.token, &mcp, &install).await;
    assert_eq!(s, 200);
    assert!(v["token"] == replacement);
    assert_eq!(v["version"], 3);
    // A wrong/missing master key yields a redacted failure; metadata stays readable.
    let mut wrong = h.state.clone();
    wrong.options.vault = KeyRing::new(
        "missing".into(),
        BTreeMap::from([("missing".into(), [1; 32])]),
    )
    .expect("wrong key");
    h.app = gateway_server::router(wrong);
    let (s, v) = resolve(&h, &alice.token, &mcp, &install).await;
    assert_eq!(s, 503);
    assert!(!v.to_string().contains(replacement));
    h.app = gateway_server::router(h.state.clone());
    // Version substitution in PostgreSQL cannot decrypt an old ciphertext.
    sqlx::query("UPDATE user_secrets SET version=4 WHERE id=$1")
        .bind(Uuid::parse_str(id).expect("id"))
        .execute(&h.state.pool)
        .await
        .expect("tamper fixture");
    assert_eq!(resolve(&h, &alice.token, &mcp, &install).await.0, 503);
    sqlx::query("UPDATE user_secrets SET version=3 WHERE id=$1")
        .bind(Uuid::parse_str(id).expect("id"))
        .execute(&h.state.pool)
        .await
        .expect("restore fixture");
    h.state.options.vault = KeyRing::new(
        "new-key".into(),
        BTreeMap::from([
            ("fixture-key".into(), [42; 32]),
            ("new-key".into(), [7; 32]),
        ]),
    )
    .expect("rotation keys");
    assert_eq!(
        gateway_server::vault::rotate_keys(&h.state)
            .await
            .expect("rotate"),
        1
    );
    assert_eq!(
        gateway_server::vault::rotate_keys(&h.state)
            .await
            .expect("repeat rotation"),
        0
    );
    h.state.options.vault = KeyRing::new(
        "new-key".into(),
        BTreeMap::from([("new-key".into(), [7; 32])]),
    )
    .expect("new key only");
    h.app = gateway_server::router(h.state.clone());
    let (s, v) = resolve(&h, &alice.token, &mcp, &install).await;
    assert_eq!(s, 200);
    assert!(v["token"] == replacement);
    assert_eq!(v["version"], 3);
    set(&h, &alice.token, &mcp, false, None).await;
    assert_eq!(resolve(&h, &alice.token, &mcp, &install).await.0, 403);
    assert_eq!(
        h.request(
            "DELETE",
            &path,
            Some(&alice.token),
            Some(json!({"expectedVersion":3}))
        )
        .await
        .0,
        204
    );
    let captured = String::from_utf8(logs.0.lock().expect("logs").clone()).expect("logs");
    assert!(captured.contains("credential_resolved"));
    for sensitive in [
        token_value,
        replacement,
        PASSWORD,
        alice.token.as_str(),
        bob.token.as_str(),
    ] {
        assert!(!captured.contains(sensitive), "Sensitive data in logs");
    }
    h.cleanup().await;
}

#[tokio::test]
#[ignore = "requires isolated TEST_DATABASE_URL"]
async fn managed_oauth_scopes_refresh_lease_and_atomic_rotation() {
    let logs = common::CapturedLogs::default();
    let _guard = tracing::subscriber::set_default(
        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_max_level(tracing::Level::INFO)
            .with_writer(logs.clone())
            .finish(),
    );
    let mut h = Harness::new().await;
    let admin = h.bootstrap().await;
    h.user(&admin.token, "alice").await;
    h.user(&admin.token, "bob").await;
    let alice = h.login("alice", PASSWORD).await;
    let bob = h.login("bob", PASSWORD).await;
    let mcp = assigned(
        &h,
        &admin.token,
        json!({"kind":"oauth","clientId":"gateway-fixture-client","scopes":["tools:read"]}),
    )
    .await;
    let installation = Uuid::new_v4().to_string();
    let other_installation = Uuid::new_v4().to_string();
    let path = format!("/api/endpoint/oauth/{mcp}");
    let lease_path = format!("{path}/lease");
    let credentials = json!({"accessToken":"oauth-vault-private-access","refreshToken":"oauth-vault-private-refresh","expiresAt":gateway_server::epoch().expect("time")+300,"issuer":"http://127.0.0.1:18081","resource":"http://127.0.0.1:18081/mcp","clientId":"gateway-fixture-client","scopes":["tools:read"]});
    let mut body = json!({"expectedRevision":snap(&h,&alice.token).await.revision,"installationId":installation,"expectedVersion":null,"refreshLease":null,"credentials":credentials});
    let mut invalid = body.clone();
    invalid["credentials"]["resource"] = json!("https://unrelated.example/mcp");
    assert_eq!(
        h.request("PUT", &path, Some(&alice.token), Some(invalid))
            .await
            .0,
        400
    );
    let mut invalid = body.clone();
    invalid["credentials"]["clientId"] = json!("different-client");
    assert_eq!(
        h.request("PUT", &path, Some(&alice.token), Some(invalid))
            .await
            .0,
        400
    );
    let (s, _, v) = h
        .request("PUT", &path, Some(&alice.token), Some(body.clone()))
        .await;
    assert_eq!(s, 200);
    assert!(v.get("accessToken").is_none());
    assert!(v.get("refreshToken").is_none());
    set(&h, &alice.token, &mcp, true, None).await;
    set(&h, &bob.token, &mcp, true, None).await;
    assert_eq!(resolve(&h, &bob.token, &mcp, &installation).await.0, 428);
    assert_eq!(
        resolve(&h, &alice.token, &mcp, &other_installation).await.0,
        428
    );
    let (s, v) = resolve(&h, &alice.token, &mcp, &installation).await;
    assert_eq!(s, 200);
    assert!(v["credentials"]["accessToken"] == "oauth-vault-private-access");
    assert_eq!(v["version"], 1);
    let ciphertext: Vec<u8> =
        sqlx::query_scalar("SELECT ciphertext FROM oauth_credentials WHERE user_id=$1")
            .bind(Uuid::parse_str(&alice.user.id).expect("id"))
            .fetch_one(&h.state.pool)
            .await
            .expect("encrypted OAuth");
    assert!(!String::from_utf8_lossy(&ciphertext).contains("oauth-vault-private"));
    let (s, _, status) = h
        .request(
            "GET",
            &format!("/api/oauth/{mcp}/status/{installation}"),
            Some(&alice.token),
            None,
        )
        .await;
    assert_eq!(s, 200);
    assert!(!status.to_string().contains("oauth-vault-private"));
    assert_eq!(status["connected"], true);
    let req = json!({"installationId":installation,"expectedVersion":1});
    let (a, b) = tokio::join!(
        h.request("POST", &lease_path, Some(&alice.token), Some(req.clone())),
        h.request("POST", &lease_path, Some(&alice.token), Some(req.clone()))
    );
    assert!(matches!((a.0, b.0), (200, 409) | (409, 200)));
    let lease = if a.0 == 200 { a.2 } else { b.2 };
    let lease_id = lease["leaseId"].as_str().expect("lease");
    body["expectedVersion"] = json!(1);
    body["expectedRevision"] = json!(snap(&h, &alice.token).await.revision);
    assert_eq!(
        h.request("PUT", &path, Some(&alice.token), Some(body.clone()))
            .await
            .0,
        409
    );
    // A canceled refresh releases coordination without modifying either token.
    assert_eq!(
        h.request(
            "DELETE",
            &lease_path,
            Some(&alice.token),
            Some(json!({"installationId":installation,"leaseId":lease_id}))
        )
        .await
        .0,
        204
    );
    let (s, v) = resolve(&h, &alice.token, &mcp, &installation).await;
    assert_eq!(s, 200);
    assert_eq!(v["version"], 1);
    let (s, _, lease) = h
        .request("POST", &lease_path, Some(&alice.token), Some(req))
        .await;
    assert_eq!(s, 200);
    let lease_id = lease["leaseId"].as_str().expect("lease");
    let mut bad = body.clone();
    bad["refreshLease"] = json!(Uuid::new_v4());
    assert_eq!(
        h.request("PUT", &path, Some(&alice.token), Some(bad))
            .await
            .0,
        409
    );
    body["refreshLease"] = json!(lease_id);
    body["credentials"]["accessToken"] = json!("oauth-vault-rotated-access");
    body["credentials"]["refreshToken"] = json!("oauth-vault-rotated-refresh");
    assert_eq!(
        h.request("PUT", &path, Some(&alice.token), Some(body.clone()))
            .await
            .0,
        200
    );
    let (s, v) = resolve(&h, &alice.token, &mcp, &installation).await;
    assert_eq!(s, 200);
    assert_eq!(v["version"], 2);
    assert!(v["credentials"]["refreshToken"] == "oauth-vault-rotated-refresh");
    body["expectedRevision"] = json!(snap(&h, &alice.token).await.revision);
    assert_eq!(
        h.request("PUT", &path, Some(&alice.token), Some(body))
            .await
            .0,
        409
    );
    let (s, _, expired) = h
        .request(
            "POST",
            &lease_path,
            Some(&alice.token),
            Some(json!({"installationId":installation,"expectedVersion":2})),
        )
        .await;
    assert_eq!(s, 200);
    sqlx::query("UPDATE oauth_credentials SET refresh_until=0 WHERE user_id=$1")
        .bind(Uuid::parse_str(&alice.user.id).expect("user"))
        .execute(&h.state.pool)
        .await
        .expect("expire lease");
    let mut expired_write = json!({"expectedRevision":snap(&h,&alice.token).await.revision,"installationId":installation,"expectedVersion":2,"refreshLease":expired["leaseId"],"credentials":credentials});
    expired_write["credentials"]["accessToken"] = json!("must-not-overwrite-expired-lease");
    assert_eq!(
        h.request("PUT", &path, Some(&alice.token), Some(expired_write))
            .await
            .0,
        409
    );
    h.state.options.vault = KeyRing::new(
        "new-key".into(),
        BTreeMap::from([
            ("fixture-key".into(), [42; 32]),
            ("new-key".into(), [7; 32]),
        ]),
    )
    .expect("rotation keys");
    assert_eq!(
        gateway_server::vault::rotate_keys(&h.state)
            .await
            .expect("rotate OAuth"),
        1
    );
    h.state.options.vault = KeyRing::new(
        "new-key".into(),
        BTreeMap::from([("new-key".into(), [7; 32])]),
    )
    .expect("new key only");
    h.app = gateway_server::router(h.state.clone());
    let (s, v) = resolve(&h, &alice.token, &mcp, &installation).await;
    assert_eq!(s, 200);
    assert_eq!(v["version"], 2);
    assert!(v["credentials"]["accessToken"] == "oauth-vault-rotated-access");
    assert_eq!(h.request("DELETE",&path,Some(&alice.token),Some(json!({"expectedRevision":snap(&h,&alice.token).await.revision,"installationId":installation,"expectedVersion":2}))).await.0,204);
    assert!(!snap(&h, &alice.token).await.mcps[0].enabled);
    let captured = String::from_utf8(logs.0.lock().expect("logs").clone()).expect("logs");
    assert!(captured.contains("oauth_credentials_stored"));
    for sensitive in [
        "oauth-vault-private-access",
        "oauth-vault-private-refresh",
        "oauth-vault-rotated-access",
        "oauth-vault-rotated-refresh",
        PASSWORD,
        alice.token.as_str(),
    ] {
        assert!(!captured.contains(sensitive), "Sensitive data in logs");
    }
    h.cleanup().await;
}

#[test]
fn external_key_files_require_private_permissions_and_valid_data() {
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let path = std::env::temp_dir().join(format!(
        "mcp-gateway-vault-fixture-{}",
        Uuid::new_v4().simple()
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&path).expect("owned fixture key file");
    file.write_all(&KeyRing::generate_key_file("fixture").expect("key"))
        .expect("write");
    file.sync_all().expect("durable file");
    drop(file);
    assert!(gateway_server::vault::load_key_file(&path).is_ok());
    #[cfg(unix)]
    {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .expect("fixture permissions");
        assert!(gateway_server::vault::load_key_file(&path).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .expect("restore permissions");
    }
    std::fs::write(&path, b"invalid-key-data").expect("corrupt fixture");
    assert!(gateway_server::vault::load_key_file(&path).is_err());
    std::fs::remove_file(&path).expect("owned cleanup");
    assert!(gateway_server::vault::load_key_file(&path).is_err());
}
