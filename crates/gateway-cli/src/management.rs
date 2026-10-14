//! Local cookie/CSRF identity boundary. P7 plugs compiled assets into this router.
use crate::{EndpointError, Result, central::CentralClient, epoch, session::SessionManager};
use axum::{
    Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use gateway_core::LoginRequest;
use serde_json::json;
use std::{
    collections::{BTreeMap, HashMap},
    net::SocketAddr,
    sync::Arc,
};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ManagementAssets {
    pub index_html: Arc<str>,
    /// Keys are exact /assets/ relative paths; no runtime filesystem access.
    pub assets: BTreeMap<String, EmbeddedAsset>,
}
#[derive(Clone)]
pub struct EmbeddedAsset {
    pub content_type: &'static str,
    pub bytes: Arc<[u8]>,
}
#[derive(Clone)]
pub struct ManagementState {
    pub sessions: SessionManager,
    pub central: CentralClient,
    authority: String,
    cookie_name: String,
    origin: String,
    browser_sessions: Arc<Mutex<HashMap<String, BrowserSession>>>,
    assets: Option<ManagementAssets>,
}
struct BrowserSession {
    csrf: String,
    expires_at: i64,
}
impl ManagementState {
    pub fn new(
        sessions: SessionManager,
        central: CentralClient,
        address: SocketAddr,
        assets: Option<ManagementAssets>,
    ) -> Result<Self> {
        if !address.ip().is_loopback() || !address.is_ipv4() || address.port() == 0 {
            return Err(EndpointError::Invalid);
        }
        let authority = address.to_string();
        Ok(Self {
            sessions,
            central,
            origin: format!("http://{authority}"),
            authority,
            cookie_name: format!("gateway_local_{}", address.port()),
            browser_sessions: Arc::new(Mutex::new(HashMap::new())),
            assets,
        })
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
}
fn denied() -> Response {
    (
        StatusCode::FORBIDDEN,
        axum::Json(json!({"code":"forbidden","message":"Local browser authorization required"})),
    )
        .into_response()
}
fn error(e: EndpointError) -> Response {
    let (status, code) = match e {
        EndpointError::Rejected | EndpointError::LoginRequired => {
            (StatusCode::UNAUTHORIZED, "login_required")
        }
        EndpointError::Offline => (StatusCode::SERVICE_UNAVAILABLE, "offline"),
        EndpointError::IdentityChanged => (StatusCode::CONFLICT, "identity_changed"),
        EndpointError::Invalid => (StatusCode::BAD_REQUEST, "invalid_request"),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "endpoint_unavailable"),
    };
    (
        status,
        axum::Json(json!({"code":code,"message": e.to_string()})),
    )
        .into_response()
}
fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|s| s.split(';'))
        .filter_map(|s| {
            s.trim()
                .split_once('=')
                .filter(|(key, _)| *key == name)
                .map(|(_, value)| value)
        });
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    Some(value)
}
async fn guard(State(state): State<ManagementState>, request: Request, next: Next) -> Response {
    let headers = request.headers();
    if headers.get_all(header::HOST).iter().count() != 1
        || headers.get(header::HOST).and_then(|s| s.to_str().ok()) != Some(state.authority.as_str())
    {
        return denied();
    }
    // OAuth is a distinct narrowly validated handler. It never authenticates a management request.
    let callback = request.uri().path() == "/oauth/callback";
    if headers.get_all(header::ORIGIN).iter().count() > 1
        || headers
            .get(header::ORIGIN)
            .is_some_and(|s| s.to_str().ok() != Some(state.origin.as_str()))
    {
        return denied();
    }
    if !callback
        && headers
            .get("sec-fetch-site")
            .is_some_and(|s| s != "same-origin" && s != "none")
    {
        return denied();
    }
    let mutation = !matches!(
        *request.method(),
        axum::http::Method::GET | axum::http::Method::HEAD
    );
    if mutation
        && headers.get(header::ORIGIN).and_then(|s| s.to_str().ok()) != Some(state.origin.as_str())
    {
        return denied();
    }
    if request.uri().path().starts_with("/api/") {
        let Some(local_cookie) = cookie(headers, &state.cookie_name) else {
            return denied();
        };
        let Ok(now) = epoch() else {
            return denied();
        };
        let sessions = state.browser_sessions.lock().await;
        let Some(session) = sessions.get(local_cookie).filter(|s| now < s.expires_at) else {
            return denied();
        };
        if mutation
            && headers.get("x-csrf-token").and_then(|s| s.to_str().ok())
                != Some(session.csrf.as_str())
        {
            return denied();
        }
    }
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response.headers_mut().insert("content-security-policy", HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"));
    response
}
fn random() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| EndpointError::Invalid)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
async fn page(State(state): State<ManagementState>) -> Response {
    let result = async {
        let now = epoch()?;
        let token = random()?;
        let csrf = random()?;
        let mut sessions = state.browser_sessions.lock().await;
        sessions.retain(|_, s| s.expires_at > now);
        if sessions.len() >= 128 {
            return Err(EndpointError::Invalid);
        }
        sessions.insert(
            token.clone(),
            BrowserSession {
                csrf,
                expires_at: now + 3600,
            },
        );
        let html = state
            .assets
            .as_ref()
            .map_or(DEVELOPMENT_PAGE, |a| a.index_html.as_ref());
        let mut response = Html(html.to_owned()).into_response();
        response.headers_mut().insert(
            header::SET_COOKIE,
            HeaderValue::from_str(&format!(
                "{}={token}; Path=/; Max-Age=3600; HttpOnly; SameSite=Strict",
                state.cookie_name
            ))
            .map_err(|_| EndpointError::Invalid)?,
        );
        Ok(response)
    }
    .await;
    result.unwrap_or_else(error)
}
async fn context(State(state): State<ManagementState>, headers: HeaderMap) -> Response {
    match state.sessions.status() {
        Ok(status) => {
            let sessions = state.browser_sessions.lock().await;
            let csrf = cookie(&headers, &state.cookie_name)
                .and_then(|c| sessions.get(c))
                .map(|s| s.csrf.clone());
            axum::Json(json!({"status":status,"csrfToken":csrf,"serverUrl":state.central.origin()}))
                .into_response()
        }
        Err(e) => error(e),
    }
}
async fn status(State(state): State<ManagementState>) -> Response {
    match state.sessions.status() {
        Ok(s) => axum::Json(s).into_response(),
        Err(e) => error(e),
    }
}
async fn login(State(state): State<ManagementState>, request: Request) -> Response {
    if request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|s| s.to_str().ok())
        != Some("application/json")
    {
        return error(EndpointError::Invalid);
    }
    let bytes = match axum::body::to_bytes(request.into_body(), 4096).await {
        Ok(b) => b,
        Err(_) => return error(EndpointError::Invalid),
    };
    let input = match serde_json::from_slice::<LoginRequest>(&bytes) {
        Ok(i) => i,
        Err(_) => return error(EndpointError::Invalid),
    };
    match state.sessions.login(&state.central, input).await {
        Ok(()) => status(State(state)).await,
        Err(e) => error(e),
    }
}
async fn logout(State(state): State<ManagementState>, headers: HeaderMap) -> Response {
    match state.sessions.logout().await {
        Ok(()) => {
            if let Some(c) = cookie(&headers, &state.cookie_name) {
                state.browser_sessions.lock().await.remove(c);
            }
            let mut response = StatusCode::NO_CONTENT.into_response();
            response.headers_mut().insert(
                header::SET_COOKIE,
                match HeaderValue::from_str(&format!(
                    "{}=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict",
                    state.cookie_name
                )) {
                    Ok(value) => value,
                    Err(_) => return error(EndpointError::Invalid),
                },
            );
            response
        }
        Err(e) => error(e),
    }
}
async fn oauth_callback() -> Response {
    // P5 must validate pending state, issuer and PKCE here; no broad API-guard exemption.
    (
        StatusCode::BAD_REQUEST,
        axum::Json(json!({"code":"invalid_callback","message":"No pending OAuth authorization"})),
    )
        .into_response()
}
async fn asset(State(state): State<ManagementState>, Path(path): Path<String>) -> Response {
    let Some(asset) = state.assets.as_ref().and_then(|a| a.assets.get(&path)) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(content_type) = HeaderValue::from_str(asset.content_type) else {
        return error(EndpointError::Invalid);
    };
    let mut response = Body::from(asset.bytes.to_vec()).into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    response
}
pub fn router(state: ManagementState) -> Router {
    Router::new()
        .route("/", get(page))
        .route("/assets/{*path}", get(asset))
        .route("/api/context", get(context))
        .route("/api/status", get(status))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/oauth/callback", get(oauth_callback))
        .fallback(|| async { (StatusCode::NOT_FOUND, Body::empty()) })
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}
const DEVELOPMENT_PAGE: &str = "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>MCP Gateway development endpoint</title></head><body><h1>MCP Gateway development endpoint</h1><p>Protected native identity APIs are available. The local user interface will be integrated in Phase 7.</p><p>This page establishes a one-hour HttpOnly local cookie. GET /api/context provides a CSRF challenge. POST JSON username/password to /api/auth/login with X-CSRF-Token and the exact local Origin. Credentials remain native; GET /api/status contains redacted metadata only. POST /api/auth/logout clears the active identity.</p></body></html>";
