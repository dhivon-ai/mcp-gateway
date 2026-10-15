#![allow(dead_code)]
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Request},
};
use gateway_core::EndpointSession;
use gateway_server::{AppState, Options};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub const PASSWORD: &str = "test-password-for-identity";

#[derive(Clone, Default)]
pub struct CapturedLogs(pub std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
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
pub struct Harness {
    pub state: AppState,
    pub app: Router,
    base: PgPool,
    name: String,
}
impl Harness {
    pub async fn new() -> Self {
        Self::with_public_url("http://127.0.0.1:18080").await
    }
    pub async fn with_public_url(public_url: &str) -> Self {
        let raw = std::env::var("TEST_DATABASE_URL")
            .expect("Set TEST_DATABASE_URL to the isolated gateway_test database");
        let mut url = url::Url::parse(&raw).expect("test DSN");
        assert!(
            matches!(url.host_str(), Some("127.0.0.1" | "localhost")),
            "Test database must be on loopback"
        );
        assert_eq!(
            url.path(),
            "/gateway_test",
            "Only the fixture gateway_test database is allowed"
        );
        let base = PgPool::connect(&raw).await.expect("fixture PostgreSQL");
        let name = format!("gateway_test_{}", Uuid::new_v4().simple());
        // Identifier is a fixed prefix plus locally generated hexadecimal UUID.
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&base)
            .await
            .expect("create owned database");
        url.set_path(&name);
        let state = AppState::connect(
            url.as_str(),
            Options {
                public_url: public_url.into(),
                allow_loopback_http: true,
                vault: gateway_core::crypto::KeyRing::new(
                    "fixture-key".into(),
                    std::collections::BTreeMap::from([("fixture-key".into(), [42; 32])]),
                )
                .expect("fixture vault key"),
            },
        )
        .await
        .expect("migrations");
        let app = gateway_server::router(state.clone());
        Self {
            state,
            app,
            base,
            name,
        }
    }
    pub async fn cleanup(self) {
        self.state.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP DATABASE {} WITH (FORCE)",
            self.name
        )))
        .execute(&self.base)
        .await
        .expect("remove owned test database");
        self.base.close().await;
    }
    pub async fn request(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        mut value: Option<Value>,
    ) -> (u16, HeaderMap, Value) {
        // Identity writes carry the same revision contract as catalog writes.
        // Tests that exercise stale/missing revisions use custom() explicitly.
        if method != "GET"
            && (path.starts_with("/api/admin/users") || path.starts_with("/api/admin/groups"))
        {
            let revision: i64 =
                sqlx::query_scalar("SELECT revision FROM deployment WHERE singleton=true")
                    .fetch_one(&self.state.pool)
                    .await
                    .expect("fixture revision");
            if value.is_none() {
                value = Some(json!({}));
            }
            if let Some(Value::Object(ref mut object)) = value {
                object.entry("expectedRevision").or_insert(json!(revision));
            }
        }
        if method == "PUT"
            && path.starts_with("/api/admin/users/")
            && !path.ends_with("/password")
            && let Some(Value::Object(ref mut object)) = value
        {
            let id =
                Uuid::parse_str(path.rsplit('/').next().expect("fixture user ID")).expect("UUID");
            let (email, name): (String, String) =
                sqlx::query_as("SELECT email,name FROM users WHERE id=$1")
                    .bind(id)
                    .fetch_one(&self.state.pool)
                    .await
                    .expect("fixture profile");
            object.entry("email").or_insert(json!(email));
            object.entry("name").or_insert(json!(name));
        }
        self.custom(method, path, token, value, &[]).await
    }
    pub async fn custom(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        value: Option<Value>,
        headers: &[(&str, &str)],
    ) -> (u16, HeaderMap, Value) {
        let mut builder = Request::builder().method(method).uri(path).header(
            "host",
            url::Url::parse(&self.state.options.public_url)
                .expect("fixture public URL")
                .authority(),
        );
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        let body = if let Some(value) = value {
            builder = builder.header("content-type", "application/json");
            Body::from(serde_json::to_vec(&value).expect("serialize"))
        } else {
            Body::empty()
        };
        let response = self
            .app
            .clone()
            .oneshot(builder.body(body).expect("request"))
            .await
            .expect("router");
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .expect("bounded response");
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).expect("JSON response")
        };
        (status, headers, value)
    }
    pub async fn bootstrap(&self) -> EndpointSession {
        gateway_server::identity::bootstrap(
            &self.state,
            "admin@example.test",
            "admin",
            PASSWORD.into(),
        )
        .await
        .expect("bootstrap");
        self.login("admin", PASSWORD).await
    }
    pub async fn login(&self, name: &str, password: &str) -> EndpointSession {
        let (status, _, value) = self
            .request(
                "POST",
                "/api/endpoint/login",
                None,
                Some(json!({"email":format!("{name}@example.test"),"password":password})),
            )
            .await;
        assert_eq!(status, 200, "login response status");
        serde_json::from_value(value).expect("endpoint session")
    }
    pub async fn user(&self, admin: &str, name: &str) -> String {
        let (status, _, value) = self
            .request(
                "POST",
                "/api/admin/users",
                Some(admin),
                Some(json!({"name":name,"email":format!("{name}@example.test"),"password":PASSWORD,"role":"user"})),
            )
            .await;
        assert_eq!(status, 200);
        value["id"].as_str().expect("user id").into()
    }
}
