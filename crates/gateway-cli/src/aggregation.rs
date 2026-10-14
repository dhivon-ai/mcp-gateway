//! Tools-only aggregation. Every listing and dispatch consumes current committed policy.
use crate::{
    cache::{SnapshotCache, ValidatedSnapshot},
    upstream_http::BoundedHttpClient,
    wire::{
        BoundedRead, BoundedTransport, Direction, MAX_MESSAGE_BYTES, MAX_TOOL_BYTES,
        MAX_TOOLS_PER_MCP,
    },
};
use futures::{StreamExt, future::BoxFuture};
use gateway_core::{AuthMode, EffectiveMcp, InventoryReport, Transport};
use rmcp::{
    ClientHandler, RoleClient, ServiceExt,
    model::*,
    service::{NotificationContext, Peer, RunningService},
    transport::{
        StreamableHttpClientTransport, async_rw::AsyncRwTransport,
        streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zeroize::Zeroizing;

pub const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_UPSTREAMS: usize = 32;
pub const MAX_AGGREGATE_TOOLS: usize = 2048;
pub const MAX_AGGREGATE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AggregateError {
    #[error("Current configuration does not authorize this tool")]
    Denied,
    #[error("Upstream MCP authentication is required")]
    Authentication,
    #[error("Upstream authentication was rejected; replace the key or reconnect while online")]
    AuthenticationRejected,
    #[error("Upstream access token expired; reconnect while online")]
    TokenExpired,
    #[error("Upstream MCP is unavailable")]
    Unavailable,
    #[error("Upstream MCP returned invalid or unsupported protocol data")]
    Protocol,
    #[error("The MCP resource limit was reached")]
    Limit,
    #[error("The MCP operation was cancelled")]
    Cancelled,
    #[error("The MCP tool call reached its deadline")]
    Timeout,
    #[error("The upstream executable or working directory is missing")]
    MissingExecutable,
    #[error("The upstream executable cannot run with current permissions")]
    ExecutionDenied,
}
type AggregateResult<T> = Result<T, AggregateError>;

/// Native-only authorization context. It has no Debug/Serialize/browser representation.
pub struct AuthLease {
    pub access: Option<Zeroizing<String>>,
    pub binding: String,
    pub expires_at: Option<i64>,
}
pub trait CredentialResolver: Send + Sync {
    fn resolve<'a>(
        &'a self,
        snapshot: &'a ValidatedSnapshot,
        mcp: &'a EffectiveMcp,
    ) -> BoxFuture<'a, AggregateResult<AuthLease>>;
    fn cached_ready(&self, _: &ValidatedSnapshot, mcp: &EffectiveMcp, binding: &str) -> bool {
        matches!(mcp.definition.auth, AuthMode::None {}) && binding == "none"
    }
    fn rejected(&self, _: Uuid, _: &str, _: &str) {}
}
pub struct NoCredentials;
impl CredentialResolver for NoCredentials {
    fn resolve<'a>(
        &'a self,
        _: &'a ValidatedSnapshot,
        mcp: &'a EffectiveMcp,
    ) -> BoxFuture<'a, AggregateResult<AuthLease>> {
        Box::pin(async move {
            if !matches!(mcp.definition.auth, AuthMode::None {}) {
                return Err(AggregateError::Authentication);
            }
            Ok(AuthLease {
                access: None,
                binding: "none".into(),
                expires_at: None,
            })
        })
    }
}

#[derive(Clone)]
struct UpstreamEvents {
    changed: Arc<Notify>,
}
impl ClientHandler for UpstreamEvents {
    fn get_info(&self) -> ClientConfig {
        ClientConfig::new(
            ClientCapabilities::default(),
            Implementation::new("mcp-gateway", env!("CARGO_PKG_VERSION")),
        )
        .with_protocol_version(ProtocolVersion::V_2025_11_25)
    }
    async fn on_tool_list_changed(&self, _: NotificationContext<RoleClient>) {
        self.changed.notify_one();
    }
}
pub struct Connection {
    pub(crate) peer: Peer<RoleClient>,
    service: tokio::sync::Mutex<Option<RunningService<RoleClient, UpstreamEvents>>>,
    #[cfg(target_os = "macos")]
    owner: tokio::sync::Mutex<Option<crate::owned_process::OwnedProcess>>,
    pub(crate) stop: CancellationToken,
    tools: Mutex<Vec<Tool>>,
    fingerprint: String,
    definition_hash: String,
    auth_expiry: Option<i64>,
    auth_binding: String,
    auth_rejected: Arc<std::sync::atomic::AtomicBool>,
}
impl Connection {
    async fn close(&self) {
        self.stop.cancel();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        #[cfg(target_os = "macos")]
        if let Some(owner) = self.owner.lock().await.take() {
            let _ = owner.terminate(Duration::from_secs(4)).await;
        }
        if let Some(service) = self.service.lock().await.take() {
            let _ = tokio::time::timeout_at(deadline, service.cancel()).await;
        }
    }
    async fn discover(&self) -> AggregateResult<Vec<Tool>> {
        let mut cursor = None;
        let mut cursors = HashSet::new();
        let mut names = HashSet::new();
        let mut tools = Vec::new();
        let mut bytes = 0usize;
        for _ in 0..32 {
            let page = self
                .peer
                .list_tools(
                    cursor
                        .take()
                        .map(|cursor| PaginatedRequestParams::default().with_cursor(Some(cursor))),
                )
                .await
                .map_err(|_| AggregateError::Protocol)?;
            for tool in page.tools {
                let size = serde_json::to_vec(&tool)
                    .map_err(|_| AggregateError::Protocol)?
                    .len();
                bytes = bytes.checked_add(size).ok_or(AggregateError::Limit)?;
                if !names.insert(tool.name.to_string()) {
                    return Err(AggregateError::Protocol);
                }
                if size > MAX_TOOL_BYTES
                    || bytes > MAX_MESSAGE_BYTES
                    || tools.len() >= MAX_TOOLS_PER_MCP
                {
                    return Err(AggregateError::Limit);
                }
                tools.push(tool);
            }
            match page.next_cursor {
                None => return Ok(tools),
                Some(next) if next.len() <= 1024 && cursors.insert(next.clone()) => {
                    cursor = Some(next)
                }
                _ => return Err(AggregateError::Protocol),
            }
        }
        Err(AggregateError::Limit)
    }
}
#[derive(Clone)]
struct Route {
    mcp_id: String,
    original: String,
    tool: Tool,
    connection: Arc<Connection>,
}
#[derive(Default)]
struct State {
    connections: BTreeMap<String, Arc<Connection>>,
    routes: BTreeMap<String, Route>,
    statuses: BTreeMap<String, &'static str>,
}

pub struct Aggregator {
    pub cache: SnapshotCache,
    generation: Uuid,
    allow_fixture_http: bool,
    credentials: Arc<dyn CredentialResolver>,
    state: Mutex<State>,
    update: tokio::sync::Mutex<()>,
    pub changed: Arc<Notify>,
    startup: Duration,
}
impl Aggregator {
    pub fn new(cache: SnapshotCache, allow_fixture_http: bool) -> AggregateResult<Self> {
        let generation = cache
            .sessions
            .load()
            .map_err(|_| AggregateError::Denied)?
            .metadata
            .generation;
        Ok(Self {
            cache,
            generation,
            allow_fixture_http,
            credentials: Arc::new(NoCredentials),
            state: Mutex::new(State::default()),
            update: tokio::sync::Mutex::new(()),
            changed: Arc::new(Notify::new()),
            startup: STARTUP_TIMEOUT,
        })
    }
    pub fn with_credentials(mut self, credentials: Arc<dyn CredentialResolver>) -> Self {
        self.credentials = credentials;
        self
    }
    pub fn with_startup_timeout(mut self, timeout: Duration) -> AggregateResult<Self> {
        if timeout.is_zero() || timeout > STARTUP_TIMEOUT {
            return Err(AggregateError::Limit);
        }
        self.startup = timeout;
        Ok(self)
    }
    pub fn current(&self) -> AggregateResult<ValidatedSnapshot> {
        let latest = self.cache.latest().map_err(|_| AggregateError::Denied)?;
        if latest.generation != self.generation {
            return Err(AggregateError::Denied);
        }
        Ok(latest)
    }
    pub fn statuses(&self) -> AggregateResult<BTreeMap<String, &'static str>> {
        self.current()?;
        Ok(self
            .state
            .lock()
            .map_err(|_| AggregateError::Unavailable)?
            .statuses
            .clone())
    }
    /// One bounded wave; no policy/map lock is held during network or child startup.
    pub async fn reconcile(&self) -> AggregateResult<()> {
        let _update = self.update.lock().await;
        let snapshot = self.current()?;
        let previous = self
            .state
            .lock()
            .map_err(|_| AggregateError::Unavailable)?
            .connections
            .clone();
        let mut statuses = BTreeMap::new();
        let mut candidates = Vec::new();
        for mcp in &snapshot.snapshot.mcps {
            if !mcp.enabled || !mcp.definition.active {
                continue;
            }
            if candidates.len() >= MAX_UPSTREAMS {
                statuses.insert(mcp.definition.id.clone(), "resource_limit");
                continue;
            }
            candidates.push(mcp.clone());
        }
        let work = futures::stream::iter(candidates)
            .map(|mcp| {
                let previous = &previous;
                let snapshot = &snapshot;
                async move {
                    let id = mcp.definition.id.clone();
                    let outcome = self
                        .startup_bound(&mcp, async {
                            let auth = self.credentials.resolve(snapshot, &mcp).await?;
                            if auth.expires_at.is_some_and(|expiry| snapshot.now >= expiry) {
                                return Err(AggregateError::Authentication);
                            }
                            let definition_hash = definition_hash(snapshot.generation, &mcp)?;
                            let fingerprint = digest(
                                &serde_json::to_vec(&(
                                    &definition_hash,
                                    &auth.binding,
                                    auth.expires_at,
                                ))
                                .map_err(|_| AggregateError::Protocol)?,
                            );
                            let connection = if let Some(old) = previous.get(&id).filter(|old| {
                                old.fingerprint == fingerprint
                                    && !old.stop.is_cancelled()
                                    && !old.peer.is_transport_closed()
                            }) {
                                old.clone()
                            } else {
                                self.connect(&mcp, auth, fingerprint, definition_hash)
                                    .await?
                            };
                            match connection.discover().await {
                                Ok(tools) => {
                                    *connection
                                        .tools
                                        .lock()
                                        .map_err(|_| AggregateError::Unavailable)? = tools;
                                    Ok(connection)
                                }
                                Err(error) => {
                                    connection.close().await;
                                    Err(error)
                                }
                            }
                        })
                        .await;
                    (id, outcome)
                }
            })
            .buffer_unordered(4);
        tokio::pin!(work);
        let mut connections = BTreeMap::new();
        let mut routes = BTreeMap::new();
        let mut blocked_names = HashSet::new();
        let mut total_bytes = 0usize;
        while let Some((id, outcome)) = work.next().await {
            let connection = match outcome {
                Ok(connection) => connection,
                Err(error) => {
                    statuses.insert(id, category(error));
                    continue;
                }
            };
            if self.guard_mcp(&id, &connection.definition_hash).is_err() {
                connection.close().await;
                statuses.insert(id, "unavailable");
                continue;
            }
            let tools = connection
                .tools
                .lock()
                .map_err(|_| AggregateError::Unavailable)?
                .clone();
            let mut additions = Vec::new();
            let mut local_names = HashSet::new();
            let mut added_bytes = 0usize;
            let mut rejected = false;
            for mut tool in tools {
                let original = tool.name.to_string();
                let name = alias(&id, &original);
                tool.name = name.clone().into();
                let size = serde_json::to_vec(&tool)
                    .map_err(|_| AggregateError::Protocol)?
                    .len();
                if blocked_names.contains(&name)
                    || routes.contains_key(&name)
                    || !local_names.insert(name.clone())
                {
                    // Neither side of an ambiguous name remains callable.
                    routes.remove(&name);
                    blocked_names.insert(name);
                    rejected = true;
                    break;
                }
                if routes.len() + additions.len() >= MAX_AGGREGATE_TOOLS
                    || total_bytes.saturating_add(added_bytes).saturating_add(size)
                        > MAX_AGGREGATE_BYTES
                {
                    rejected = true;
                    break;
                }
                added_bytes += size;
                additions.push((
                    name,
                    Route {
                        mcp_id: id.clone(),
                        original,
                        tool,
                        connection: connection.clone(),
                    },
                ));
            }
            if rejected {
                connection.close().await;
                statuses.insert(id, "resource_limit");
                continue;
            }
            total_bytes += added_bytes;
            routes.extend(additions);
            self.report_inventory(&id, &connection);
            statuses.insert(id.clone(), "available");
            connections.insert(id, connection);
        }
        if self.current().is_err() {
            close_connections(connections.into_values().collect()).await;
            self.close().await;
            return Err(AggregateError::Denied);
        }
        {
            let mut state = self.state.lock().map_err(|_| AggregateError::Unavailable)?;
            state.connections = connections.clone();
            state.routes = routes;
            state.statuses = statuses;
        }
        close_connections(
            previous
                .iter()
                .filter(|(id, old)| {
                    connections
                        .get(*id)
                        .is_none_or(|new| !Arc::ptr_eq(new, old))
                })
                .map(|(_, old)| old.clone())
                .collect(),
        )
        .await;
        Ok(())
    }
    fn guard_mcp(&self, id: &str, expected: &str) -> AggregateResult<()> {
        let latest = self.current()?;
        let mcp = latest
            .snapshot
            .mcps
            .iter()
            .find(|mcp| mcp.definition.id == id && mcp.definition.active && mcp.enabled)
            .ok_or(AggregateError::Denied)?;
        if definition_hash(latest.generation, mcp)? != expected {
            return Err(AggregateError::Denied);
        }
        Ok(())
    }
    async fn startup_bound<T>(
        &self,
        mcp: &EffectiveMcp,
        work: impl Future<Output = AggregateResult<T>>,
    ) -> AggregateResult<T> {
        let expected = definition_hash(self.generation, mcp)?;
        let start = self.cache.sample().map_err(|_| AggregateError::Denied)?;
        let budget = u64::try_from(self.startup.as_nanos()).map_err(|_| AggregateError::Limit)?;
        let awake_deadline = tokio::time::Instant::now() + self.startup;
        tokio::pin!(work);
        loop {
            self.guard_mcp(&mcp.definition.id, &expected)?;
            if !self
                .cache
                .sample()
                .and_then(|now| now.elapsed_since(start))
                .is_ok_and(|elapsed| elapsed < budget)
            {
                return Err(AggregateError::Unavailable);
            }
            tokio::select! {
                result = &mut work => {
                    self.guard_mcp(&mcp.definition.id, &expected)?;
                    if !self.cache.sample().and_then(|now| now.elapsed_since(start)).is_ok_and(|elapsed| elapsed < budget) { return Err(AggregateError::Unavailable); }
                    return result;
                }
                _ = tokio::time::sleep(Duration::from_millis(50)) => ()
                ,_ = tokio::time::sleep_until(awake_deadline) => return Err(AggregateError::Unavailable)
            }
        }
    }
    pub fn list(&self) -> AggregateResult<Vec<Tool>> {
        let latest = self.current()?;
        let definitions = latest
            .snapshot
            .mcps
            .iter()
            .map(|mcp| {
                Ok((
                    mcp.definition.id.as_str(),
                    (mcp, definition_hash(latest.generation, mcp)?),
                ))
            })
            .collect::<AggregateResult<BTreeMap<_, _>>>()?;
        let state = self.state.lock().map_err(|_| AggregateError::Unavailable)?;
        let ready: BTreeMap<_, _> = state
            .connections
            .iter()
            .map(|(id, connection)| {
                let ready = !connection
                    .auth_rejected
                    .load(std::sync::atomic::Ordering::Acquire)
                    && definitions.get(id.as_str()).is_some_and(|(mcp, _)| {
                        self.credentials
                            .cached_ready(&latest, mcp, &connection.auth_binding)
                    });
                (id, ready)
            })
            .collect();
        let tools = state
            .routes
            .values()
            .filter(|route| {
                ready.get(&route.mcp_id) == Some(&true)
                    && !route.connection.stop.is_cancelled()
                    && !route.connection.peer.is_transport_closed()
                    && route
                        .connection
                        .auth_expiry
                        .is_none_or(|expiry| latest.now < expiry)
                    && definitions
                        .get(route.mcp_id.as_str())
                        .is_some_and(|(mcp, hash)| {
                            *hash == route.connection.definition_hash
                                && gateway_core::authorize_effective_tool(
                                    mcp,
                                    &route.original,
                                    true,
                                )
                                .is_ok()
                        })
            })
            .map(|route| route.tool.clone())
            .collect();
        if !self
            .cache
            .proof_usable(&latest)
            .map_err(|_| AggregateError::Denied)?
        {
            return Err(AggregateError::Denied);
        }
        Ok(tools)
    }
    pub(crate) fn route(&self, name: &str) -> AggregateResult<(String, Arc<Connection>)> {
        let latest = self.current()?;
        let state = self.state.lock().map_err(|_| AggregateError::Unavailable)?;
        let route = state
            .routes
            .get(name)
            .filter(|route| permitted(&latest, route, self.credentials.as_ref()))
            .ok_or(AggregateError::Denied)?;
        if !self
            .cache
            .proof_usable(&latest)
            .map_err(|_| AggregateError::Denied)?
        {
            return Err(AggregateError::Denied);
        }
        Ok((route.original.clone(), route.connection.clone()))
    }
    pub(crate) fn audit_target(&self, name: &str) -> Option<(String, String, String)> {
        let latest = self.current().ok()?;
        let state = self.state.lock().ok()?;
        let route = state.routes.get(name)?;
        Some((
            latest.snapshot.user.id,
            route.mcp_id.clone(),
            route.original.clone(),
        ))
    }
    pub async fn call(&self, params: CallToolRequestParams) -> AggregateResult<CallToolResult> {
        self.call_bounded(params, CancellationToken::new(), Duration::from_secs(60))
            .await
    }
    /// Caller cancellation is independent of the current native policy and sleep-aware deadline.
    pub async fn call_bounded(
        &self,
        mut params: CallToolRequestParams,
        cancelled: CancellationToken,
        timeout: Duration,
    ) -> AggregateResult<CallToolResult> {
        if timeout.is_zero() || timeout > Duration::from_secs(60) {
            return Err(AggregateError::Limit);
        }
        if cancelled.is_cancelled() {
            return Err(AggregateError::Cancelled);
        }
        let start = self.cache.sample().map_err(|_| AggregateError::Denied)?;
        let awake_deadline = tokio::time::Instant::now() + timeout;
        let budget = u64::try_from(timeout.as_nanos()).map_err(|_| AggregateError::Limit)?;
        let name = params.name.to_string();
        let (original, connection) = self.route(&name)?;
        let (checked_original, checked_connection) = self.route(&name)?;
        if original != checked_original || !Arc::ptr_eq(&connection, &checked_connection) {
            return Err(AggregateError::Denied);
        }
        params.name = original.into();
        let guard = || -> AggregateResult<()> {
            if cancelled.is_cancelled() || connection.stop.is_cancelled() {
                return Err(AggregateError::Cancelled);
            }
            let (_, current) = self.route(&name)?;
            if !Arc::ptr_eq(&connection, &current) {
                return Err(AggregateError::Denied);
            }
            if !self
                .cache
                .sample()
                .and_then(|now| now.elapsed_since(start))
                .is_ok_and(|elapsed| elapsed < budget)
                || tokio::time::Instant::now() >= awake_deadline
            {
                return Err(AggregateError::Timeout);
            }
            Ok(())
        };
        guard()?;
        let sending = connection.peer.send_cancellable_request(
            ClientRequest::CallToolRequest(CallToolRequest::new(params)),
            rmcp::service::PeerRequestOptions::no_options(),
        );
        tokio::pin!(sending);
        let handle = loop {
            guard()?;
            tokio::select! {
                biased;
                _ = cancelled.cancelled() => return Err(AggregateError::Cancelled),
                _ = connection.stop.cancelled() => return Err(AggregateError::Cancelled),
                _ = tokio::time::sleep_until(awake_deadline) => return Err(AggregateError::Timeout),
                result = &mut sending => break result.map_err(|_| AggregateError::Unavailable)?,
                _ = tokio::time::sleep(Duration::from_millis(50)) => (),
            }
        };
        let mut cancellation = CancelRequest {
            peer: connection.peer.clone(),
            id: Some(handle.id.clone()),
        };
        let response = handle.await_response();
        tokio::pin!(response);
        loop {
            guard()?;
            tokio::select! {
                biased;
                _ = cancelled.cancelled() => return Err(AggregateError::Cancelled),
                _ = connection.stop.cancelled() => return Err(AggregateError::Cancelled),
                _ = tokio::time::sleep_until(awake_deadline) => return Err(AggregateError::Timeout),
                result = &mut response => {
                    // A completed response cannot cross a learned revocation or expired deadline.
                    guard()?;
                    cancellation.id = None;
                    return match result { Ok(ServerResult::CallToolResult(result)) => Ok(result), _ => Err(AggregateError::Unavailable) };
                },
                _ = tokio::time::sleep(Duration::from_millis(50)) => (),
            }
        }
    }
    /// Remove routes and signal calls immediately, then close only retired owned upstreams.
    pub async fn retire_unusable(&self) {
        let latest = self.current().ok();
        let retired = if let Ok(mut state) = self.state.lock() {
            let ids: Vec<_> = state
                .connections
                .iter()
                .filter(|(id, connection)| {
                    latest.as_ref().is_none_or(|latest| {
                        connection
                            .auth_rejected
                            .load(std::sync::atomic::Ordering::Acquire)
                            || connection.stop.is_cancelled()
                            || connection.peer.is_transport_closed()
                            || connection
                                .auth_expiry
                                .is_some_and(|expiry| latest.now >= expiry)
                            || latest
                                .snapshot
                                .mcps
                                .iter()
                                .find(|mcp| {
                                    mcp.definition.id == **id
                                        && mcp.enabled
                                        && mcp.definition.active
                                })
                                .is_none_or(|mcp| {
                                    !definition_hash(latest.generation, mcp)
                                        .is_ok_and(|hash| hash == connection.definition_hash)
                                        || !self.credentials.cached_ready(
                                            latest,
                                            mcp,
                                            &connection.auth_binding,
                                        )
                                })
                    })
                })
                .map(|(id, _)| id.clone())
                .collect();
            let mut retired = Vec::new();
            for id in ids {
                if let Some(connection) = state.connections.remove(&id) {
                    connection.stop.cancel();
                    retired.push(connection);
                }
                state.statuses.insert(id, "unavailable");
            }
            state
                .routes
                .retain(|_, route| !route.connection.stop.is_cancelled());
            retired
        } else {
            Vec::new()
        };
        close_connections(retired).await;
    }
    pub async fn close(&self) {
        let connections = match self.state.lock() {
            Ok(mut state) => {
                state.routes.clear();
                std::mem::take(&mut state.connections)
            }
            Err(_) => return,
        };
        close_connections(connections.into_values().collect()).await;
    }
    fn report_inventory(&self, id: &str, connection: &Connection) {
        let Ok(session) = self.cache.sessions.load() else {
            return;
        };
        if session.metadata.generation != self.generation {
            return;
        }
        let Ok(tools) = connection.tools.lock() else {
            return;
        };
        let report = InventoryReport {
            mcp_id: id.into(),
            names: tools.iter().map(|tool| tool.name.to_string()).collect(),
        };
        let sessions = self.cache.sessions.clone();
        tokio::spawn(async move {
            if let Ok(body) = serde_json::to_value(report) {
                let _ = sessions
                    .control_request_for(
                        &session,
                        reqwest::Method::POST,
                        "/api/endpoint/inventory",
                        Some(&body),
                    )
                    .await;
            }
        });
    }
    async fn connect(
        &self,
        mcp: &EffectiveMcp,
        auth: AuthLease,
        fingerprint: String,
        definition_hash: String,
    ) -> AggregateResult<Arc<Connection>> {
        self.guard_mcp(&mcp.definition.id, &definition_hash)?;
        let rejected = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let handler = UpstreamEvents {
            changed: self.changed.clone(),
        };
        #[cfg(target_os = "macos")]
        let mut owner = None;
        let service = match &mcp.definition.transport {
            Transport::Http { url } => {
                let mut client = BoundedHttpClient::new(url, self.allow_fixture_http)
                    .map_err(|_| AggregateError::Unavailable)?;
                let signal = rejected.clone();
                let credentials = self.credentials.clone();
                let changed = self.changed.clone();
                let generation = self.generation;
                let id = mcp.definition.id.clone();
                let binding = auth.binding.clone();
                client = client.with_rejection_handler(Arc::new(move || {
                    signal.store(true, std::sync::atomic::Ordering::Release);
                    credentials.rejected(generation, &id, &binding);
                    changed.notify_one();
                }));
                if let Some(access) = auth.access {
                    client = client
                        .with_access(access)
                        .map_err(|_| AggregateError::Authentication)?;
                }
                let config = StreamableHttpClientTransportConfig::with_uri(client.uri())
                    .reinit_on_expired_session(false)
                    .max_sse_event_size(MAX_MESSAGE_BYTES);
                handler
                    .serve(StreamableHttpClientTransport::with_client(client, config))
                    .await
                    .map_err(|_| {
                        if rejected.load(std::sync::atomic::Ordering::Acquire) {
                            AggregateError::AuthenticationRejected
                        } else {
                            AggregateError::Unavailable
                        }
                    })?
            }
            Transport::Stdio { command } => {
                #[cfg(target_os = "macos")]
                {
                    let mut process = crate::owned_process::OwnedProcess::spawn_protocol(command)
                        .map_err(|error| match error {
                        crate::EndpointError::MissingExecutable => {
                            AggregateError::MissingExecutable
                        }
                        crate::EndpointError::ExecutionDenied => AggregateError::ExecutionDenied,
                        _ => AggregateError::Unavailable,
                    })?;
                    let (read, write) = process
                        .take_protocol()
                        .map_err(|_| AggregateError::Unavailable)?;
                    owner = Some(process);
                    handler
                        .serve(BoundedTransport::new(AsyncRwTransport::new_client(
                            BoundedRead::new(read, Direction::Upstream),
                            write,
                        )))
                        .await
                        .map_err(|_| AggregateError::Unavailable)?
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = command;
                    return Err(AggregateError::Unavailable);
                }
            }
        };
        if service.peer().peer_info().is_none_or(|info| {
            info.protocol_version != ProtocolVersion::V_2025_11_25
                || info.capabilities.tools.is_none()
        }) {
            let _ = service.cancel().await;
            return Err(AggregateError::Protocol);
        }
        Ok(Arc::new(Connection {
            peer: service.peer().clone(),
            service: tokio::sync::Mutex::new(Some(service)),
            #[cfg(target_os = "macos")]
            owner: tokio::sync::Mutex::new(owner),
            stop: CancellationToken::new(),
            tools: Mutex::new(Vec::new()),
            fingerprint,
            definition_hash,
            auth_expiry: auth.expires_at,
            auth_binding: auth.binding,
            auth_rejected: rejected,
        }))
    }
}
fn permitted(
    latest: &ValidatedSnapshot,
    route: &Route,
    credentials: &dyn CredentialResolver,
) -> bool {
    if route
        .connection
        .auth_rejected
        .load(std::sync::atomic::Ordering::Acquire)
        || route.connection.stop.is_cancelled()
        || route.connection.peer.is_transport_closed()
        || route
            .connection
            .auth_expiry
            .is_some_and(|expiry| latest.now >= expiry)
    {
        return false;
    }
    latest
        .snapshot
        .authorize_tool(
            &latest.snapshot.deployment_id,
            &latest.snapshot.user.id,
            latest.now,
            &route.mcp_id,
            &route.original,
            true,
        )
        .ok()
        .is_some_and(|mcp| {
            definition_hash(latest.generation, mcp)
                .is_ok_and(|hash| hash == route.connection.definition_hash)
                && credentials.cached_ready(latest, mcp, &route.connection.auth_binding)
        })
}
fn definition_hash(generation: Uuid, mcp: &EffectiveMcp) -> AggregateResult<String> {
    Ok(digest(
        &serde_json::to_vec(&(
            generation,
            &mcp.definition.transport,
            &mcp.definition.auth,
            &mcp.secret,
        ))
        .map_err(|_| AggregateError::Protocol)?,
    ))
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
/// Fixed ASCII <=128 bytes, independent of policy/schema changes. Any route collision is rejected.
pub fn alias(mcp_id: &str, original: &str) -> String {
    format!(
        "gw_{}_{}",
        &digest(mcp_id.as_bytes())[..32],
        digest(original.as_bytes())
    )
}
async fn close_connections(connections: Vec<Arc<Connection>>) {
    let work =
        futures::stream::iter(connections).for_each_concurrent(None, |connection| async move {
            connection.close().await;
        });
    let _ = tokio::time::timeout(Duration::from_secs(5), work).await;
}

/// Dropping a handler future still sends best-effort cancellation for its exact upstream request.
struct CancelRequest {
    peer: Peer<RoleClient>,
    id: Option<RequestId>,
}
impl Drop for CancelRequest {
    fn drop(&mut self) {
        let Some(id) = self.id.take() else {
            return;
        };
        let peer = self.peer.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                let notification = CancelledNotificationParam::new(
                    Some(id),
                    Some("Gateway authorization, deadline, or caller cancellation".into()),
                );
                let _ = tokio::time::timeout(
                    Duration::from_millis(250),
                    peer.notify_cancelled(notification),
                )
                .await;
            });
        }
    }
}
fn category(error: AggregateError) -> &'static str {
    match error {
        AggregateError::Authentication => "auth_required",
        AggregateError::AuthenticationRejected => "auth_rejected",
        AggregateError::TokenExpired => "token_expired",
        AggregateError::Protocol => "invalid_upstream",
        AggregateError::Limit => "resource_limit",
        AggregateError::MissingExecutable => "missing_executable",
        AggregateError::ExecutionDenied => "execution_denied",
        _ => "unavailable",
    }
}
