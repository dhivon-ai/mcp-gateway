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
pub struct Harness {
    pub state: AppState,
    pub app: Router,
    base: PgPool,
    name: String,
}
impl Harness {
    pub async fn new() -> Self {
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
                public_url: "http://127.0.0.1:18080".into(),
                allow_loopback_http: true,
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
        value: Option<Value>,
    ) -> (u16, HeaderMap, Value) {
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
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("host", "127.0.0.1:18080");
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
        gateway_server::identity::bootstrap(&self.state, "admin", PASSWORD.into())
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
                Some(json!({"username":name,"password":password})),
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
                Some(json!({"username":name,"password":PASSWORD,"role":"user"})),
            )
            .await;
        assert_eq!(status, 200);
        value["id"].as_str().expect("user id").into()
    }
}
