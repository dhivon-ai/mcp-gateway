use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRequest, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use gateway_core::{ApiError, ErrorCode, UiContext};
use serde::de::DeserializeOwned;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, Semaphore};
use uuid::Uuid;

pub mod identity;
#[derive(Clone)]
pub struct Options {
    pub public_url: String,
    pub allow_loopback_http: bool,
}
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub options: Options,
    pub deployment_id: Uuid,
    pub(crate) authority: String,
    pub(crate) origin: String,
    pub(crate) secure_cookies: bool,
    pub(crate) dummy_hash: String,
    pub(crate) attempts: Arc<Mutex<HashMap<String, (i64, u32)>>>,
    pub(crate) password_slots: Arc<Semaphore>,
}
pub fn epoch() -> Result<i64, AppError> {
    let value = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::internal())?
        .as_secs();
    i64::try_from(value).map_err(|_| AppError::internal())
}
impl AppState {
    pub async fn connect(database_url: &str, options: Options) -> anyhow::Result<Self> {
        let url = gateway_core::validate_remote_url(
            &options.public_url,
            options.allow_loopback_http,
        )
        .map_err(|_| {
            anyhow::anyhow!(
                "Public URL must use verified HTTPS or explicitly allowed loopback test HTTP"
            )
        })?;
        let host = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Missing public hostname"))?;
        if url.path() != "/" || url.query().is_some() {
            anyhow::bail!("Public URL must be an origin without a path or query");
        }
        let authority = match url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_string(),
        };
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(5))
            .after_connect(|conn, _| {
                Box::pin(async move {
                    sqlx::query("SET statement_timeout = '5s'")
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(database_url)
            .await
            .map_err(|_| anyhow::anyhow!("Unable to connect to configured PostgreSQL database"))?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        sqlx::query("INSERT INTO deployment(singleton,id) VALUES(true,$1) ON CONFLICT DO NOTHING")
            .bind(Uuid::new_v4())
            .execute(&pool)
            .await?;
        let deployment_id = sqlx::query_scalar("SELECT id FROM deployment WHERE singleton=true")
            .fetch_one(&pool)
            .await?;
        let dummy_hash =
            identity::hash_password("non-user-dummy-verification-password".into()).await?;
        Ok(Self {
            pool,
            options,
            deployment_id,
            authority,
            origin: url.origin().ascii_serialization(),
            secure_cookies: url.scheme() == "https",
            dummy_hash,
            attempts: Default::default(),
            password_slots: Arc::new(Semaphore::new(16)),
        })
    }
}
pub struct Input<T>(pub T);
impl<S, T> FromRequest<S> for Input<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = AppError;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|_| AppError::bad("Invalid JSON request."))
    }
}
pub struct AppError {
    pub status: StatusCode,
    pub code: ErrorCode,
    pub message: &'static str,
    pub revision: Option<u32>,
}
impl AppError {
    pub fn new(status: StatusCode, code: ErrorCode, message: &'static str) -> Self {
        Self {
            status,
            code,
            message,
            revision: None,
        }
    }
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::Internal,
            "The server could not complete the request.",
        )
    }
    pub fn bad(message: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorCode::InvalidRequest, message)
    }
    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            ErrorCode::Unauthorized,
            "Login is required.",
        )
    }
    pub fn forbidden() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            ErrorCode::Forbidden,
            "This action is not permitted.",
        )
    }
    pub fn conflict(message: &'static str) -> Self {
        Self::new(StatusCode::CONFLICT, ErrorCode::Conflict, message)
    }
}
impl std::fmt::Debug for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppError({}, {})", self.status, self.message)
    }
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message)
    }
}
impl std::error::Error for AppError {}
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        if e.as_database_error()
            .is_some_and(|d| d.is_unique_violation())
        {
            Self::conflict("That entry already exists.")
        } else {
            tracing::warn!(event = "database_operation_failed");
            Self::internal()
        }
    }
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiError {
                code: self.code,
                message: self.message.into(),
                revision: self.revision,
            }),
        )
            .into_response()
    }
}
async fn origin_guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let host = request.headers().get("host").and_then(|h| h.to_str().ok());
    if !host.is_some_and(|h| h.eq_ignore_ascii_case(&state.authority)) {
        return AppError::bad("Unexpected request host.").into_response();
    }
    if request
        .headers()
        .get("origin")
        .is_some_and(|h| h.to_str().ok() != Some(state.origin.as_str()))
        || request
            .headers()
            .get("sec-fetch-site")
            .is_some_and(|h| h == "cross-site")
    {
        return AppError::forbidden().into_response();
    }
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "cache-control",
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}
async fn health(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"status":"ok"})))
}
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/api/context", get(identity::context))
        .route("/api/auth/login", post(identity::browser_login))
        .route("/api/endpoint/login", post(identity::endpoint_login))
        .route("/api/auth/logout", post(identity::logout))
        .route(
            "/api/admin/users",
            get(identity::list_users).post(identity::create_user),
        )
        .route("/api/admin/users/{id}", put(identity::edit_user))
        .route(
            "/api/admin/users/{id}/password",
            put(identity::reset_password),
        )
        .route(
            "/api/admin/groups",
            get(identity::list_groups).post(identity::create_group),
        )
        .route(
            "/api/admin/groups/{id}",
            put(identity::edit_group).delete(identity::delete_group),
        )
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), origin_guard))
        .with_state(state)
}
pub(crate) fn empty_context(state: &AppState) -> UiContext {
    UiContext {
        mode: "admin".into(),
        user: None,
        server_url: Some(state.options.public_url.clone()),
        online: true,
        expires_at: None,
        csrf_token: None,
    }
}
pub(crate) async fn lock_revision(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<i64, AppError> {
    Ok(
        sqlx::query_scalar("SELECT revision FROM deployment WHERE singleton=true FOR UPDATE")
            .fetch_one(&mut **tx)
            .await?,
    )
}
pub(crate) async fn bump_revision(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), AppError> {
    sqlx::query("UPDATE deployment SET revision=revision+1 WHERE singleton=true")
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub(crate) fn uuid(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::bad("Invalid identifier."))
}
