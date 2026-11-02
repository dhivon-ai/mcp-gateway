//! Local cookie/CSRF identity boundary. P7 plugs compiled assets into this router.
use crate::{EndpointError, Result, central::CentralClient, epoch, session::SessionManager};
use axum::{
    Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post, put},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use dhivon_mcp_gateway_core::LoginRequest;
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
    pub sync: Option<crate::sync::SyncEngine>,
    pub installs: Option<crate::install::InstallManager>,
    pub oauth: Option<crate::oauth_control::OAuthControl>,
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
            sessions: sessions.for_server(&central),
            central,
            sync: None,
            installs: None,
            oauth: None,
            origin: format!("http://{authority}"),
            authority,
            cookie_name: format!("dhivon_mcp_gateway_local_{}", address.port()),
            browser_sessions: Arc::new(Mutex::new(HashMap::new())),
            assets,
        })
    }
    pub fn with_sync(mut self, sync: crate::sync::SyncEngine) -> Result<Self> {
        if sync.cache.sessions.store.root() != self.sessions.store.root() {
            return Err(EndpointError::Invalid);
        }
        if !sync.cache.sessions.is_selected_for(&self.central) {
            return Err(EndpointError::ServerMismatch);
        }
        self.installs = Some(crate::install::InstallManager::new(
            crate::bridge::ControlBridge::new(sync.clone()),
        ));
        self.oauth = Some(crate::oauth_control::OAuthControl::new(
            sync.clone(),
            format!("{}/oauth/callback", self.origin),
            self.central.origin().starts_with("http://"),
        ));
        self.sync = Some(sync);
        Ok(self)
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
    let error = crate::bridge::BridgeError::from(e);
    (error.status, axum::Json(error.body)).into_response()
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
    // Radix positioning/focus uses inline style attributes; scripts remain same-origin only.
    response.headers_mut().insert("content-security-policy", HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"));
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
        let mut response =
            Html(html.replacen("<html", "<html data-gateway-mode=\"local\"", 1)).into_response();
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
fn redacted_status(state: &ManagementState) -> Result<serde_json::Value> {
    let status = match &state.sync {
        Some(sync) => serde_json::to_value(sync.status()?),
        None => serde_json::to_value(state.sessions.status()?),
    };
    status.map_err(|_| EndpointError::Invalid)
}
async fn context(State(state): State<ManagementState>, headers: HeaderMap) -> Response {
    match redacted_status(&state) {
        Ok(status) => {
            let sessions = state.browser_sessions.lock().await;
            let csrf = cookie(&headers, &state.cookie_name)
                .and_then(|c| sessions.get(c))
                .map(|s| s.csrf.clone());
            axum::Json(json!({"mode":"local","status":status,"csrfToken":csrf,"serverUrl":state.central.origin()}))
                .into_response()
        }
        Err(e) => error(e),
    }
}
async fn status(State(state): State<ManagementState>) -> Response {
    match redacted_status(&state) {
        Ok(s) => axum::Json(s).into_response(),
        Err(e) => error(e),
    }
}
async fn refresh(State(state): State<ManagementState>, request: Request) -> Response {
    bridge_response(
        async {
            let action = input::<()>(request).await?;
            let generation = crate::bridge::ControlBridge::generation(&action.expected_generation)?;
            state.sessions.guard_generation(generation)?;
            let sync = state.sync.as_ref().ok_or(EndpointError::SyncRequired)?;
            sync.refresh().await?;
            state.sessions.guard_generation(generation)?;
            sync.status().map_err(Into::into)
        }
        .await,
    )
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
        Ok(()) => {
            if let Some(sync) = &state.sync {
                let _ = sync.refresh().await;
            }
            status(State(state)).await
        }
        Err(e) => error(e),
    }
}
async fn logout(State(state): State<ManagementState>, headers: HeaderMap) -> Response {
    let Some(generation) = headers
        .get("x-endpoint-generation")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| uuid::Uuid::parse_str(v).ok())
    else {
        return error(EndpointError::Invalid);
    };
    if let Err(e) = state.sessions.guard_generation(generation) {
        return error(e);
    }
    if let Some(installs) = &state.installs {
        installs.shutdown_generation(&generation.to_string()).await;
    }
    if let Some(oauth) = &state.oauth {
        oauth.cancel_generation(generation).await;
    }
    // Check again after bounded child shutdown; never revoke a concurrent new login.
    if let Err(e) = state.sessions.guard_generation(generation) {
        return error(e);
    }
    match state.sessions.logout_generation(generation).await {
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
async fn oauth_callback(State(state): State<ManagementState>, request: Request) -> Response {
    let Some(oauth) = &state.oauth else {
        return error(EndpointError::Invalid);
    };
    let raw = format!("{}{}", state.origin, request.uri());
    match oauth.callback(&raw).await {
        Ok(_) => Html("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>OAuth connected</title></head><body><h1>OAuth connected</h1><p>Close this window and return to MCP Gateway. Enable the MCP explicitly when ready.</p></body></html>").into_response(),
        Err(e) => error(e),
    }
}
fn oauth_manager(
    state: &ManagementState,
) -> crate::bridge::BridgeResult<&crate::oauth_control::OAuthControl> {
    state
        .oauth
        .as_ref()
        .ok_or_else(|| EndpointError::SyncRequired.into())
}
async fn oauth_start(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    request: Request,
) -> Response {
    bridge_response(
        async {
            let action = input::<crate::oauth_control::ConnectInput>(request).await?;
            oauth_manager(&state)?
                .start(&action.expected_generation, &id, action.input)
                .await
                .map_err(Into::into)
        }
        .await,
    )
}
async fn oauth_status(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    bridge_response(
        async {
            oauth_manager(&state)?
                .status(generation(&headers)?, &id)
                .await
                .map_err(Into::into)
        }
        .await,
    )
}
async fn oauth_disconnect(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    request: Request,
) -> Response {
    bridge_response(
        async {
            let action = input::<crate::oauth_control::DisconnectInput>(request).await?;
            oauth_manager(&state)?
                .disconnect(&action.expected_generation, &id, action.input)
                .await?;
            Ok(json!({"confirmed":true}))
        }
        .await,
    )
}
async fn oauth_cancel(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    request: Request,
) -> Response {
    bridge_response(
        async {
            let action = input::<serde_json::Value>(request).await?;
            if action.input != json!({}) {
                return Err(crate::bridge::BridgeError::invalid());
            }
            oauth_manager(&state)?
                .cancel(&action.expected_generation, &id)
                .await?;
            Ok(json!({"cancelled":true}))
        }
        .await,
    )
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

fn action_manager(
    state: &ManagementState,
) -> crate::bridge::BridgeResult<&crate::install::InstallManager> {
    state
        .installs
        .as_ref()
        .ok_or_else(|| EndpointError::SyncRequired.into())
}
fn bridge_response<T: serde::Serialize>(result: crate::bridge::BridgeResult<T>) -> Response {
    match result {
        Ok(value) => axum::Json(value).into_response(),
        Err(e) => (e.status, axum::Json(e.body)).into_response(),
    }
}
fn generation(headers: &HeaderMap) -> crate::bridge::BridgeResult<&str> {
    headers
        .get("x-endpoint-generation")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(crate::bridge::BridgeError::invalid)
}
async fn input<T: serde::de::DeserializeOwned>(
    request: Request,
) -> crate::bridge::BridgeResult<dhivon_mcp_gateway_core::LocalAction<T>> {
    if request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        != Some("application/json")
    {
        return Err(crate::bridge::BridgeError::invalid());
    }
    let bytes = axum::body::to_bytes(request.into_body(), 131072)
        .await
        .map_err(|_| crate::bridge::BridgeError::invalid())?;
    serde_json::from_slice(&bytes).map_err(|_| crate::bridge::BridgeError::invalid())
}
async fn catalog(State(state): State<ManagementState>, headers: HeaderMap) -> Response {
    bridge_response((|| action_manager(&state)?.catalog(generation(&headers)?))())
}
async fn secrets(State(state): State<ManagementState>, headers: HeaderMap) -> Response {
    bridge_response(
        async {
            action_manager(&state)?
                .bridge
                .secrets(generation(&headers)?)
                .await
        }
        .await,
    )
}
async fn settings(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    request: Request,
) -> Response {
    bridge_response(
        async {
            let action = input::<dhivon_mcp_gateway_core::SettingsUpdate>(request).await?;
            action_manager(&state)?
                .bridge
                .settings(&action.expected_generation, &id, action.input)
                .await
        }
        .await,
    )
}
async fn secret_create(State(state): State<ManagementState>, request: Request) -> Response {
    bridge_response(
        async {
            let action = input::<dhivon_mcp_gateway_core::SecretWrite>(request).await?;
            action_manager(&state)?
                .bridge
                .secret(
                    &action.expected_generation,
                    reqwest::Method::POST,
                    None,
                    false,
                    action.input,
                )
                .await
        }
        .await,
    )
}
async fn secret_replace(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    request: Request,
) -> Response {
    bridge_response(
        async {
            let action = input::<dhivon_mcp_gateway_core::SecretWrite>(request).await?;
            action_manager(&state)?
                .bridge
                .secret(
                    &action.expected_generation,
                    reqwest::Method::PUT,
                    Some(&id),
                    false,
                    action.input,
                )
                .await
        }
        .await,
    )
}
async fn secret_rename(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    request: Request,
) -> Response {
    bridge_response(
        async {
            let action = input::<dhivon_mcp_gateway_core::SecretRename>(request).await?;
            action_manager(&state)?
                .bridge
                .secret(
                    &action.expected_generation,
                    reqwest::Method::PUT,
                    Some(&id),
                    true,
                    action.input,
                )
                .await
        }
        .await,
    )
}
async fn secret_delete(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    request: Request,
) -> Response {
    bridge_response(
        async {
            let action = input::<dhivon_mcp_gateway_core::VersionInput>(request).await?;
            action_manager(&state)?
                .bridge
                .secret(
                    &action.expected_generation,
                    reqwest::Method::DELETE,
                    Some(&id),
                    false,
                    action.input,
                )
                .await
        }
        .await,
    )
}
async fn install_start(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    request: Request,
) -> Response {
    let result = async {
        let action = input::<dhivon_mcp_gateway_core::InstallRequest>(request).await?;
        action_manager(&state)?
            .start(&action.expected_generation, &id, action.input)
            .await
    }
    .await;
    match result {
        Ok(progress) => (StatusCode::ACCEPTED, axum::Json(progress)).into_response(),
        Err(e) => (e.status, axum::Json(e.body)).into_response(),
    }
}
async fn install_progress(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    bridge_response((|| {
        action_manager(&state)?.progress(generation(&headers)?, &id)
    })())
}
async fn install_cancel(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    request: Request,
) -> Response {
    bridge_response(
        async {
            let action = input::<()>(request).await?;
            action_manager(&state)?.cancel(&action.expected_generation, &id)?;
            Ok(serde_json::json!({"cancellationRequested":true}))
        }
        .await,
    )
}
async fn install_enable(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    request: Request,
) -> Response {
    bridge_response(
        async {
            let action = input::<dhivon_mcp_gateway_core::SettingsUpdate>(request).await?;
            action_manager(&state)?
                .enable(&action.expected_generation, &id, action.input)
                .await
        }
        .await,
    )
}
pub fn router(state: ManagementState) -> Router {
    Router::new()
        .route("/", get(page))
        .route("/assets/{*path}", get(asset))
        .route("/api/context", get(context))
        .route("/api/status", get(status))
        .route("/api/refresh", post(refresh))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/catalog", get(catalog))
        .route("/api/settings/{id}", put(settings))
        .route("/api/secrets", get(secrets).post(secret_create))
        .route(
            "/api/secrets/{id}",
            put(secret_replace).delete(secret_delete),
        )
        .route("/api/secrets/{id}/key", put(secret_rename))
        .route("/api/mcps/{id}/install", post(install_start))
        .route("/api/installations/{id}", get(install_progress))
        .route("/api/installations/{id}/cancel", post(install_cancel))
        .route("/api/installations/{id}/enable", post(install_enable))
        .route("/api/oauth/{id}/connect", post(oauth_start))
        .route("/api/oauth/{id}/status", get(oauth_status))
        .route("/api/oauth/{id}/disconnect", post(oauth_disconnect))
        .route("/api/oauth/flows/{id}/cancel", post(oauth_cancel))
        .route("/oauth/callback", get(oauth_callback))
        .fallback(|| async { (StatusCode::NOT_FOUND, Body::empty()) })
        .layer(DefaultBodyLimit::max(131072))
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}
const DEVELOPMENT_PAGE: &str = "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>MCP Gateway development endpoint</title></head><body><h1>MCP Gateway development endpoint</h1><p>Protected native identity APIs are available. The local user interface will be integrated in Phase 7.</p><p>This page establishes a one-hour HttpOnly local cookie. GET /api/context provides a CSRF challenge. POST JSON email/password to /api/auth/login with X-CSRF-Token and the exact local Origin. Credentials remain native; GET /api/status contains redacted metadata only. POST /api/auth/logout clears the active identity.</p></body></html>";
