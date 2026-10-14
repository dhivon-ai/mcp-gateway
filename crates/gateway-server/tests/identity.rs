mod common;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use common::{Harness, PASSWORD};
use serde_json::json;
use tower::ServiceExt;

#[derive(Clone, Default)]
struct CapturedLogs(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
impl std::io::Write for CapturedLogs {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("log lock").extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedLogs {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[tokio::test]
#[ignore = "requires isolated TEST_DATABASE_URL"]
async fn bootstrap_roles_revocation_and_membership() {
    let h = Harness::new().await;
    assert_eq!(
        h.request("GET", "/api/admin/users", None, None).await.0,
        401
    );
    assert_eq!(
        h.request(
            "POST",
            "/api/endpoint/login",
            None,
            Some(json!({"username":"admin","password":PASSWORD}))
        )
        .await
        .0,
        401
    );
    assert!(
        gateway_server::identity::bootstrap(&h.state, "admin", "short".into())
            .await
            .is_err()
    );
    let admin = h.bootstrap().await;
    assert!(
        gateway_server::identity::bootstrap(&h.state, "another", PASSWORD.into())
            .await
            .is_err()
    );
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE username='admin'")
        .fetch_one(&h.state.pool)
        .await
        .expect("password hash");
    assert!(hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
    assert!(!hash.contains(PASSWORD));
    let alice_id = h.user(&admin.token, "alice").await;
    let bob_id = h.user(&admin.token, "bob").await;
    let stale = json!({"role":"user","active":false,"expectedRevision":0});
    assert_eq!(
        h.custom(
            "PUT",
            &format!("/api/admin/users/{alice_id}"),
            Some(&admin.token),
            Some(stale),
            &[]
        )
        .await
        .0,
        409
    );
    assert_eq!(
        h.custom(
            "PUT",
            &format!("/api/admin/users/{alice_id}"),
            Some(&admin.token),
            Some(json!({"role":"user","active":false})),
            &[]
        )
        .await
        .0,
        400
    );
    let alice = h.login("alice", PASSWORD).await;
    let bob = h.login("bob", PASSWORD).await;
    let hashes: Vec<Vec<u8>> = sqlx::query_scalar("SELECT token_hash FROM sessions")
        .fetch_all(&h.state.pool)
        .await
        .expect("opaque hashes");
    assert!(hashes.iter().all(|h| h.len() == 32));
    assert!(
        hashes
            .iter()
            .all(|h| h.as_slice() != alice.token.as_bytes())
    );
    for (method, path, body) in [
        ("GET", "/api/admin/users", None),
        (
            "POST",
            "/api/admin/users",
            Some(json!({"username":"attack","password":PASSWORD,"role":"admin"})),
        ),
        (
            "POST",
            "/api/admin/groups",
            Some(json!({"name":"attack","memberIds":[]})),
        ),
    ] {
        assert_eq!(
            h.request(method, path, Some(&alice.token), body).await.0,
            403
        );
    }
    let (status, _, user_response) = h
        .request("GET", "/api/admin/users", Some(&admin.token), None)
        .await;
    assert_eq!(status, 200);
    assert!(!user_response.to_string().contains("password"));
    assert!(!user_response.to_string().contains(&hash));
    let (status, _, group) = h
        .request(
            "POST",
            "/api/admin/groups",
            Some(&admin.token),
            Some(json!({"name":"Engineering","memberIds":[alice_id,bob_id]})),
        )
        .await;
    assert_eq!(status, 200);
    let group_path = format!(
        "/api/admin/groups/{}",
        group["id"].as_str().expect("group id")
    );
    assert_eq!(
        h.request(
            "PUT",
            &group_path,
            Some(&admin.token),
            Some(json!({"name":"Platform","memberIds":[alice_id,alice_id]}))
        )
        .await
        .0,
        400
    );
    assert_eq!(
        h.request(
            "PUT",
            &group_path,
            Some(&admin.token),
            Some(json!({"name":"Platform","memberIds":[alice_id]}))
        )
        .await
        .0,
        200
    );
    let (_, _, groups) = h
        .request("GET", "/api/admin/groups", Some(&admin.token), None)
        .await;
    assert_eq!(groups[0]["memberIds"], json!([alice_id]));
    assert_eq!(
        h.request("DELETE", &group_path, Some(&bob.token), None)
            .await
            .0,
        403
    );
    assert_eq!(
        h.request("DELETE", &group_path, Some(&admin.token), None)
            .await
            .0,
        204
    );
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/users/{}", admin.user.id),
            Some(&admin.token),
            Some(json!({"role":"user","active":false}))
        )
        .await
        .0,
        409
    );
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/users/{alice_id}/password"),
            Some(&admin.token),
            Some(json!({"password":"changed-password-alice"}))
        )
        .await
        .0,
        204
    );
    assert_eq!(
        h.request("GET", "/api/context", Some(&alice.token), None)
            .await
            .0,
        401
    );
    assert_eq!(
        h.request("GET", "/api/context", Some(&bob.token), None)
            .await
            .0,
        200
    );
    assert_eq!(
        h.request(
            "POST",
            "/api/endpoint/login",
            None,
            Some(json!({"username":"alice","password":PASSWORD}))
        )
        .await
        .0,
        401
    );
    let alice = h.login("alice", "changed-password-alice").await;
    assert_eq!(
        h.request(
            "PUT",
            &format!("/api/admin/users/{alice_id}"),
            Some(&admin.token),
            Some(json!({"role":"user","active":false}))
        )
        .await
        .0,
        200
    );
    assert_eq!(
        h.request("GET", "/api/context", Some(&alice.token), None)
            .await
            .0,
        401
    );
    assert_eq!(
        h.request("POST", "/api/auth/logout", Some(&bob.token), None)
            .await
            .0,
        204
    );
    assert_eq!(
        h.request("GET", "/api/context", Some(&bob.token), None)
            .await
            .0,
        401
    );
    let bob = h.login("bob", PASSWORD).await;
    sqlx::query("UPDATE sessions SET expires_at=0 WHERE user_id=$1")
        .bind(uuid::Uuid::parse_str(&bob.user.id).expect("user UUID"))
        .execute(&h.state.pool)
        .await
        .expect("expire");
    assert_eq!(
        h.request("GET", "/api/context", Some(&bob.token), None)
            .await
            .0,
        401
    );
    h.cleanup().await;
}

#[tokio::test]
#[ignore = "requires isolated TEST_DATABASE_URL"]
async fn browser_security_redaction_and_throttling() {
    let logs = CapturedLogs::default();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(logs.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let h = Harness::new().await;
    let admin = h.bootstrap().await;
    let (_, headers, context) = h.request("GET", "/api/context", None, None).await;
    let preauth = headers
        .get("set-cookie")
        .expect("preauth cookie")
        .to_str()
        .expect("cookie")
        .split(';')
        .next()
        .expect("cookie pair");
    let csrf = context["csrfToken"].as_str().expect("CSRF token");
    assert!(
        headers
            .get("set-cookie")
            .expect("cookie")
            .to_str()
            .expect("cookie")
            .contains("HttpOnly; SameSite=Strict")
    );
    let login = json!({"username":"admin","password":PASSWORD});
    assert_eq!(
        h.custom(
            "POST",
            "/api/auth/login",
            None,
            Some(login.clone()),
            &[("cookie", preauth)]
        )
        .await
        .0,
        403
    );
    let (status, headers, value) = h
        .custom(
            "POST",
            "/api/auth/login",
            None,
            Some(login.clone()),
            &[("cookie", preauth), ("x-csrf-token", csrf)],
        )
        .await;
    assert_eq!(status, 200);
    assert!(value.get("token").is_none());
    assert!(!value.to_string().contains(PASSWORD));
    let session = headers
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with("gateway_session="))
        .expect("session cookie")
        .split(';')
        .next()
        .expect("cookie");
    let csrf = value["csrfToken"].as_str().expect("session CSRF");
    assert_eq!(
        h.custom(
            "POST",
            "/api/endpoint/login",
            None,
            Some(login),
            &[("cookie", session)]
        )
        .await
        .0,
        403
    );
    assert_eq!(
        h.custom(
            "POST",
            "/api/auth/logout",
            None,
            None,
            &[("cookie", session)]
        )
        .await
        .0,
        403
    );
    assert_eq!(
        h.custom(
            "POST",
            "/api/auth/logout",
            None,
            None,
            &[("cookie", session), ("x-csrf-token", csrf)]
        )
        .await
        .0,
        204
    );
    // A revoked browser cookie must recover a fresh login challenge without waiting twelve hours.
    let (code, fresh_headers, fresh_context) = h
        .custom("GET", "/api/context", None, None, &[("cookie", session)])
        .await;
    assert_eq!(code, 200);
    assert!(fresh_context["user"].is_null());
    let fresh_csrf = fresh_context["csrfToken"]
        .as_str()
        .expect("fresh preauth challenge");
    let fresh_preauth = fresh_headers
        .get_all("set-cookie")
        .iter()
        .filter_map(|h| h.to_str().ok())
        .find(|s| s.starts_with("gateway_preauth="))
        .expect("preauth cookie")
        .split(';')
        .next()
        .expect("pair");
    assert!(fresh_headers.get_all("set-cookie").iter().any(|h| {
        h.to_str()
            .is_ok_and(|s| s.starts_with("gateway_session=;") && s.contains("Max-Age=0"))
    }));
    assert_eq!(
        h.custom(
            "POST",
            "/api/auth/login",
            None,
            Some(json!({"username":"admin","password":PASSWORD})),
            &[("cookie", fresh_preauth), ("x-csrf-token", fresh_csrf)]
        )
        .await
        .0,
        200
    );
    assert_eq!(
        h.custom(
            "GET",
            "/api/context",
            Some(&admin.token),
            None,
            &[("origin", "https://evil.example")]
        )
        .await
        .0,
        403
    );
    let request = Request::builder()
        .uri("/api/context")
        .header("host", "evil.example")
        .body(Body::empty())
        .expect("request");
    assert_eq!(
        h.app
            .clone()
            .oneshot(request)
            .await
            .expect("router")
            .status(),
        400
    );
    let request = Request::builder()
        .method("POST")
        .uri("/api/endpoint/login")
        .header("host", "127.0.0.1:18080")
        .header("content-type", "application/json")
        .body(Body::from(
            "{\"username\":\"bad\",\"password\":\"sensitive-invalid-json",
        ))
        .expect("request");
    let response = h.app.clone().oneshot(request).await.expect("router");
    assert_eq!(response.status(), 400);
    let bytes = to_bytes(response.into_body(), 10000).await.expect("body");
    assert!(!String::from_utf8_lossy(&bytes).contains("sensitive-invalid-json"));
    for _ in 0..5 {
        assert_eq!(
            h.request(
                "POST",
                "/api/endpoint/login",
                None,
                Some(json!({"username":"unknown","password":"wrong-password"}))
            )
            .await
            .0,
            401
        );
    }
    assert_eq!(
        h.request(
            "POST",
            "/api/endpoint/login",
            None,
            Some(json!({"username":"unknown","password":"wrong-password"}))
        )
        .await
        .0,
        429
    );
    let captured = String::from_utf8(logs.0.lock().expect("logs").clone()).expect("UTF8 logs");
    assert!(captured.contains("session_created"));
    for sensitive in [
        PASSWORD,
        admin.token.as_str(),
        csrf,
        "wrong-password",
        "sensitive-invalid-json",
    ] {
        assert!(!captured.contains(sensitive), "Sensitive data in logs");
    }
    h.cleanup().await;
}
