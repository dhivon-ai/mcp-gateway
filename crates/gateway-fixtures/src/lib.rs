//! Isolated, credential-free fixture MCP and OAuth services. Never deploy publicly.
use axum::{
    Json, Router,
    extract::{Form, Query, State},
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::*,
    service::RequestContext,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct EchoServer;

impl EchoServer {
    fn tools() -> Vec<Tool> {
        let schema = json!({"type":"object","properties":{"message":{"type":"string","maxLength":4096},"delayMs":{"type":"integer","minimum":0,"maximum":120000}},"required":["message"],"additionalProperties":false});
        vec![
            Tool::new(
                "echo",
                "Echo a fixture message",
                schema.as_object().cloned().unwrap_or_default(),
            ),
            Tool::new(
                "fail",
                "Return a fixture tool error",
                serde_json::Map::from_iter([("type".into(), json!("object"))]),
            ),
        ]
    }
}

impl ServerHandler for EchoServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2025_11_25)
            .with_server_info(Implementation::new("gateway-fixture", "0.1.0"))
    }
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Owned(vec![ProtocolVersion::V_2025_11_25])
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        Self::tools().into_iter().find(|t| t.name == name)
    }
    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let cursor = request.and_then(|p| p.cursor);
        let mut tools = Self::tools();
        match cursor.as_deref() {
            None => Ok(ListToolsResult {
                tools: vec![tools.remove(0)],
                next_cursor: Some("1".into()),
                ..Default::default()
            }),
            Some("1") => Ok(ListToolsResult {
                tools: vec![tools.remove(1)],
                ..Default::default()
            }),
            _ => Err(ErrorData::invalid_params("invalid cursor", None)),
        }
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        match request.name.as_ref() {
            "echo" => {
                let args: EchoArgs =
                    serde_json::from_value(Value::Object(request.arguments.unwrap_or_default()))
                        .map_err(|_| ErrorData::invalid_params("message is required", None))?;
                if args.message.len() > 4096 || args.delay_ms.unwrap_or_default() > 120_000 {
                    return Err(ErrorData::invalid_params(
                        "fixture argument bound exceeded",
                        None,
                    ));
                }
                tokio::time::sleep(Duration::from_millis(args.delay_ms.unwrap_or_default())).await;
                Ok(CallToolResult::success(vec![ContentBlock::text(args.message)]).into())
            }
            "fail" => Ok(CallToolResult::error(vec![ContentBlock::text("fixture failure")]).into()),
            _ => Err(ErrorData::invalid_params("unknown fixture tool", None)),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EchoArgs {
    message: String,
    delay_ms: Option<u64>,
}

#[derive(Clone)]
pub struct FixtureState {
    pub base: String,
    pub auth: String,
    pub bearer: String,
    codes: Arc<Mutex<HashMap<String, Code>>>,
    tokens: Arc<Mutex<HashMap<String, i64>>>,
    refresh: Arc<Mutex<HashMap<String, String>>>,
}
#[derive(Clone)]
struct Code {
    challenge: String,
    redirect: String,
    expires_at: i64,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl FixtureState {
    pub fn new(base: String, auth: String) -> Self {
        Self {
            base,
            auth,
            bearer: "fixture-token".into(),
            codes: Default::default(),
            tokens: Default::default(),
            refresh: Default::default(),
        }
    }
}

async fn require_auth(
    State(state): State<FixtureState>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let candidate = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let valid = match state.auth.as_str() {
        "none" => true,
        "bearer" => candidate.is_some_and(|v| v == state.bearer),
        "oauth" => {
            if let Some(token) = candidate {
                state
                    .tokens
                    .lock()
                    .await
                    .get(token)
                    .is_some_and(|e| *e > now())
            } else {
                false
            }
        }
        _ => false,
    };
    if valid {
        return next.run(request).await;
    }
    let challenge = format!(
        "Bearer resource_metadata=\"{}/.well-known/oauth-protected-resource\"",
        state.base
    );
    let mut response = (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error":"unauthorized"})),
    )
        .into_response();
    if let Ok(header) = HeaderValue::from_str(&challenge) {
        response.headers_mut().insert("www-authenticate", header);
    }
    response
}

async fn protected_metadata(State(s): State<FixtureState>) -> Json<Value> {
    Json(
        json!({"resource":format!("{}/mcp",s.base),"authorization_servers":[s.base],"scopes_supported":["tools"]}),
    )
}
async fn issuer_metadata(State(s): State<FixtureState>) -> Json<Value> {
    Json(
        json!({"issuer":s.base,"authorization_endpoint":format!("{}/authorize",s.base),"token_endpoint":format!("{}/token",s.base),"response_types_supported":["code"],"grant_types_supported":["authorization_code","refresh_token"],"token_endpoint_auth_methods_supported":["none"],"code_challenge_methods_supported":["S256"],"authorization_response_iss_parameter_supported":true,"scopes_supported":["tools"]}),
    )
}

#[derive(Deserialize)]
struct AuthorizeQuery {
    client_id: String,
    redirect_uri: String,
    response_type: String,
    code_challenge: String,
    code_challenge_method: String,
    state: String,
    resource: String,
}
async fn authorize(State(s): State<FixtureState>, Query(q): Query<AuthorizeQuery>) -> Response {
    let Ok(mut redirect) = url::Url::parse(&q.redirect_uri) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if q.client_id != "gateway-fixture-client"
        || q.response_type != "code"
        || q.code_challenge_method != "S256"
        || q.code_challenge.len() < 43
        || q.resource != format!("{}/mcp", s.base)
        || q.state.is_empty()
        || redirect.scheme() != "http"
        || !matches!(redirect.host_str(), Some("127.0.0.1" | "localhost"))
        || redirect.path() != "/oauth/callback"
        || redirect.fragment().is_some()
        || !redirect.username().is_empty()
        || redirect.password().is_some()
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let code = Uuid::new_v4().to_string();
    s.codes.lock().await.insert(
        code.clone(),
        Code {
            challenge: q.code_challenge,
            redirect: q.redirect_uri,
            expires_at: now() + 300,
        },
    );
    redirect
        .query_pairs_mut()
        .append_pair("code", &code)
        .append_pair("state", &q.state)
        .append_pair("iss", &s.base);
    Redirect::to(redirect.as_str()).into_response()
}

#[derive(Deserialize)]
struct TokenForm {
    grant_type: String,
    client_id: String,
    resource: String,
    code: Option<String>,
    code_verifier: Option<String>,
    redirect_uri: Option<String>,
    refresh_token: Option<String>,
}
async fn token(State(s): State<FixtureState>, Form(q): Form<TokenForm>) -> Response {
    if q.client_id != "gateway-fixture-client" || q.resource != format!("{}/mcp", s.base) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"invalid_request"})),
        )
            .into_response();
    }
    let valid = match q.grant_type.as_str() {
        "authorization_code" => {
            let code = if let Some(code) = &q.code {
                s.codes.lock().await.remove(code)
            } else {
                None
            };
            code.is_some_and(|c| {
                c.expires_at > now()
                    && q.redirect_uri.as_deref() == Some(c.redirect.as_str())
                    && q.code_verifier.as_ref().is_some_and(|v| {
                        URL_SAFE_NO_PAD.encode(Sha256::digest(v.as_bytes())) == c.challenge
                    })
            })
        }
        "refresh_token" => {
            if let Some(refresh) = &q.refresh_token {
                s.refresh.lock().await.remove(refresh).is_some()
            } else {
                false
            }
        }
        _ => false,
    };
    if !valid {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"invalid_grant"})),
        )
            .into_response();
    }
    let access = Uuid::new_v4().to_string();
    let refresh = Uuid::new_v4().to_string();
    s.tokens.lock().await.insert(access.clone(), now() + 30);
    s.refresh
        .lock()
        .await
        .insert(refresh.clone(), access.clone());
    Json(json!({"access_token":access,"refresh_token":refresh,"token_type":"Bearer","expires_in":30,"scope":"tools"})).into_response()
}

pub fn http_router(state: FixtureState, host: String) -> Router {
    let service = StreamableHttpService::new(
        || Ok(EchoServer),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_allowed_hosts(vec![host]),
    );
    let mcp = Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));
    mcp.merge(
        Router::new()
            .route("/healthz", get(|| async { StatusCode::OK }))
            .route(
                "/.well-known/oauth-protected-resource",
                get(protected_metadata),
            )
            .route(
                "/.well-known/oauth-protected-resource/mcp",
                get(protected_metadata),
            )
            .route(
                "/.well-known/oauth-authorization-server",
                get(issuer_metadata),
            )
            .route("/authorize", get(authorize))
            .route("/token", post(token))
            .with_state(state),
    )
}
