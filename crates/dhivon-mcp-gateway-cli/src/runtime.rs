//! Agent-facing tools-only service. No credentials, payloads, or browser prompts enter stdout.
use crate::{
    aggregation::{AggregateError, Aggregator},
    sync::SyncEngine,
};
use rmcp::{
    RoleServer, ServerHandler, ServiceExt,
    model::*,
    service::{Peer, RequestContext},
};
use sha2::{Digest, Sha256};
use std::{borrow::Cow, sync::Arc, time::Duration};
use tokio::sync::{Semaphore, watch};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub const MAX_IN_FLIGHT: usize = 16;
pub const CALL_TIMEOUT: Duration = Duration::from_secs(60);
const PAGE_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
pub struct GatewayService {
    pub gateway: Arc<Aggregator>,
    slots: Arc<Semaphore>,
    stop: CancellationToken,
    timeout: Duration,
    initialized: watch::Receiver<bool>,
}
impl GatewayService {
    pub fn new(
        gateway: Arc<Aggregator>,
        stop: CancellationToken,
        initialized: watch::Receiver<bool>,
        timeout: Duration,
        concurrency: usize,
    ) -> Result<Self, AggregateError> {
        if timeout.is_zero()
            || timeout > CALL_TIMEOUT
            || concurrency == 0
            || concurrency > MAX_IN_FLIGHT
        {
            return Err(AggregateError::Limit);
        }
        Ok(Self {
            gateway,
            slots: Arc::new(Semaphore::new(concurrency)),
            stop,
            timeout,
            initialized,
        })
    }
    async fn wait_initial(&self) {
        if *self.initialized.borrow() || self.gateway.current().is_err() {
            return;
        }
        let mut ready = self.initialized.clone();
        let _ = tokio::time::timeout(Duration::from_secs(30), async {
            loop { if *ready.borrow() { break; } tokio::select! { _ = self.stop.cancelled() => break, changed = ready.changed() => if changed.is_err() { break; } } }
        }).await;
    }
}
impl ServerHandler for GatewayService {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        )
        .with_protocol_version(ProtocolVersion::V_2025_11_25)
        .with_server_info(Implementation::new(
            "dhivon-mcp-gateway",
            env!("CARGO_PKG_VERSION"),
        ))
    }
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Owned(vec![ProtocolVersion::V_2025_11_25])
    }
    // Authorization and upstream argument/result validation occur at the dispatch boundary.
    // SDK-side get_tool must not turn an authorization failure into a schema error.
    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        self.wait_initial().await;
        let tools = if self.stop.is_cancelled() {
            Vec::new()
        } else {
            self.gateway.list().unwrap_or_default()
        };
        paginate(tools, request.and_then(|p| p.cursor))
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let correlation = Uuid::new_v4();
        let started = std::time::Instant::now();
        if self.stop.is_cancelled() || self.gateway.route(request.name.as_ref()).is_err() {
            tracing::info!(event = "gateway_tool_denied", correlation_id = %correlation, duration_ms = started.elapsed().as_millis() as u64, category = "policy_denied");
            return Err(ErrorData::invalid_params(
                "Tool is unavailable under current authorization",
                None,
            ));
        }
        let _slot = self.slots.clone().try_acquire_owned().map_err(|_| {
            tracing::info!(event = "gateway_tool_failed", correlation_id = %correlation, duration_ms = started.elapsed().as_millis() as u64, category = "concurrency_limit");
            ErrorData::internal_error("Gateway has reached its tool-call concurrency limit", None)
        })?;
        let target = self.gateway.audit_target(request.name.as_ref());
        tracing::info!(event = "gateway_tool_allowed", correlation_id = %correlation, duration_ms = started.elapsed().as_millis() as u64, user_id = target.as_ref().map(|t| t.0.as_str()), mcp_id = target.as_ref().map(|t| t.1.as_str()), original_tool = target.as_ref().map(|t| t.2.as_str()));
        let result = tokio::select! {
            biased;
            _ = self.stop.cancelled() => Err(AggregateError::Cancelled),
            result = self.gateway.call_bounded(request, context.ct, self.timeout) => result,
        };
        let outcome = match &result {
            Ok(result) if result.is_error == Some(true) => "tool_error",
            Ok(_) => "success",
            Err(AggregateError::Denied) => "policy_denied",
            Err(AggregateError::Cancelled) => "cancelled",
            Err(AggregateError::Timeout) => "timeout",
            Err(_) => "upstream_unavailable",
        };
        tracing::info!(event = "gateway_tool_finished", correlation_id = %correlation, duration_ms = started.elapsed().as_millis() as u64, outcome);
        match result {
            Ok(result) => Ok(result.into()),
            Err(AggregateError::Denied) => Err(ErrorData::invalid_params(
                "Tool is unavailable under current authorization",
                None,
            )),
            Err(error) => {
                Ok(CallToolResult::error(vec![ContentBlock::text(error.to_string())]).into())
            }
        }
    }
}

/// Bounded stateless cursors bind offsets to the exact ordered catalog; changed catalogs require restart.
pub fn paginate(tools: Vec<Tool>, cursor: Option<String>) -> Result<ListToolsResult, ErrorData> {
    let hash = catalog_hash(&tools)?;
    let offset = match cursor {
        None => 0,
        Some(cursor) => {
            if cursor.len() > 80 {
                return Err(ErrorData::invalid_params(
                    "Tool catalog cursor is invalid; restart discovery",
                    None,
                ));
            }
            let (expected, index) = cursor.split_once(':').ok_or_else(|| {
                ErrorData::invalid_params("Tool catalog cursor is invalid; restart discovery", None)
            })?;
            if expected != hash || index.is_empty() || !index.bytes().all(|b| b.is_ascii_digit()) {
                return Err(ErrorData::invalid_params(
                    "Tool catalog changed; restart discovery",
                    None,
                ));
            }
            let offset: usize = index.parse().map_err(|_| {
                ErrorData::invalid_params("Tool catalog cursor is invalid; restart discovery", None)
            })?;
            if offset == 0 || offset >= tools.len() {
                return Err(ErrorData::invalid_params(
                    "Tool catalog cursor is invalid; restart discovery",
                    None,
                ));
            }
            offset
        }
    };
    let total = tools.len();
    let mut bytes = 0usize;
    let mut page = Vec::new();
    for tool in tools.into_iter().skip(offset) {
        let size = serde_json::to_vec(&tool)
            .map_err(|_| ErrorData::internal_error("Tool catalog cannot be encoded", None))?
            .len();
        if size > PAGE_BYTES {
            return Err(ErrorData::internal_error(
                "Tool exceeds the catalog page limit",
                None,
            ));
        }
        if !page.is_empty() && bytes.saturating_add(size) > PAGE_BYTES {
            break;
        }
        bytes += size;
        page.push(tool);
    }
    let next = offset + page.len();
    Ok(ListToolsResult {
        tools: page,
        next_cursor: (next < total).then(|| format!("{hash}:{next}")),
        ..Default::default()
    })
}
fn catalog_hash(tools: &[Tool]) -> Result<String, ErrorData> {
    let bytes = serde_json::to_vec(tools)
        .map_err(|_| ErrorData::internal_error("Tool catalog cannot be encoded", None))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Reconciliation runs separately from the policy monitor: a slow startup cannot hide learned revocation.
pub async fn coordinate(
    gateway: Arc<Aggregator>,
    engine: SyncEngine,
    peer: Peer<RoleServer>,
    stop: CancellationToken,
    initialized: watch::Sender<bool>,
) {
    let mut sync = engine.authorization_changes();
    let mut interval = tokio::time::interval(Duration::from_millis(250));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut work: Option<tokio_util::task::AbortOnDropHandle<Result<(), AggregateError>>> = None;
    let mut dirty = true;
    let mut signature = None;
    let mut advertised = String::new();
    loop {
        if stop.is_cancelled() {
            break;
        }
        gateway.retire_unusable().await;
        let current = gateway.current().ok();
        let next = current.as_ref().map(|latest| {
            (
                latest.generation,
                latest.snapshot.revision,
                latest.snapshot.validated_at,
                latest.snapshot.expires_at,
            )
        });
        if next != signature {
            signature = next;
            dirty = true;
        }
        let tools = gateway.list().unwrap_or_default();
        let hash = catalog_hash(&tools).unwrap_or_default();
        if hash != advertised {
            advertised = hash;
            let _ =
                tokio::time::timeout(Duration::from_millis(250), peer.notify_tool_list_changed())
                    .await;
        }
        if dirty && work.is_none() && current.is_some() {
            dirty = false;
            let gateway = gateway.clone();
            work = Some(tokio_util::task::AbortOnDropHandle::new(tokio::spawn(
                async move { gateway.reconcile().await },
            )));
        }
        tokio::select! {
            biased;
            _ = stop.cancelled() => break,
            finished = async { match work.as_mut() { Some(work) => Some(work.await), None => std::future::pending().await } } => {
                work = None; initialized.send_replace(true);
                if !matches!(finished, Some(Ok(Ok(())))) { tracing::info!(event = "gateway_discovery_failed", category = "authorization_or_upstream_unavailable"); }
            },
            _ = gateway.changed.notified() => dirty = true,
            changed = sync.changed() => if changed.is_err() { break; },
            _ = interval.tick() => (),
        }
    }
    if let Some(work) = work {
        work.abort();
        let _ = work.await;
    }
    initialized.send_replace(true);
}

pub async fn serve_stdio(
    engine: SyncEngine,
    gateway: Arc<Aggregator>,
    stop: CancellationToken,
    timeout: Duration,
    concurrency: usize,
) -> Result<(), AggregateError> {
    let (ready, initialized) = watch::channel(false);
    let service = GatewayService::new(
        gateway.clone(),
        stop.clone(),
        initialized,
        timeout,
        concurrency,
    )?;
    #[cfg(target_os = "macos")]
    let (read, write) = crate::native_stdio::streams().map_err(|_| AggregateError::Unavailable)?;
    #[cfg(not(target_os = "macos"))]
    let (read, write) = (tokio::io::stdin(), tokio::io::stdout());
    let transport = crate::wire::BoundedTransport::new(
        rmcp::transport::async_rw::AsyncRwTransport::new_server(
            crate::wire::BoundedRead::new(read, crate::wire::Direction::Agent),
            write,
        ),
    );
    let serving = service.serve(transport);
    tokio::pin!(serving);
    let running = tokio::select! { _ = stop.cancelled() => return Ok(()), result = tokio::time::timeout(Duration::from_secs(30), &mut serving) => result.map_err(|_| AggregateError::Timeout)?.map_err(|_| AggregateError::Protocol)? };
    let peer = running.peer().clone();
    let mut coordinator = tokio::spawn(coordinate(
        gateway.clone(),
        engine,
        peer,
        stop.clone(),
        ready,
    ));
    let cancellation = running.cancellation_token();
    let mut waiting = tokio::spawn(running.waiting());
    tokio::select! { _ = stop.cancelled() => cancellation.cancel(), _ = &mut waiting => () }
    stop.cancel();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    coordinator.abort();
    let _ = tokio::time::timeout_at(deadline, &mut coordinator).await;
    let _ = tokio::time::timeout_at(deadline, gateway.close()).await;
    if !waiting.is_finished() {
        let _ = tokio::time::timeout_at(deadline, &mut waiting).await;
        waiting.abort();
    }
    Ok(())
}
