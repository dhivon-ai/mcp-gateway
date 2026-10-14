//! Success-only complete refresh and sleep-aware scheduling; connectivity never renews a lease.
use crate::{
    EndpointError, Result,
    cache::{CacheStatus, SnapshotCache},
    central::bounded_body,
    clock::{ClockSample, NANOS_PER_SECOND},
    session::{NativeSession, RedactedStatus, SessionManager},
};
use gateway_core::ConfigSnapshot;
use serde::Serialize;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{Notify, watch};

pub type FetchFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(NativeSession, ConfigSnapshot)>> + Send + 'a>>;
pub trait ConfigTransport: Send + Sync {
    fn fetch<'a>(&'a self, sessions: &'a SessionManager) -> FetchFuture<'a>;
}
pub struct NativeTransport;
impl ConfigTransport for NativeTransport {
    fn fetch<'a>(&'a self, sessions: &'a SessionManager) -> FetchFuture<'a> {
        Box::pin(async move {
            let (session, response) = sessions
                .request(reqwest::Method::GET, "/api/config", None)
                .await?;
            if response.status() != reqwest::StatusCode::OK {
                return Err(EndpointError::Response);
            }
            let bytes = bounded_body(response).await?;
            let snapshot = serde_json::from_slice::<ConfigSnapshot>(&bytes)
                .map_err(|_| EndpointError::Response)?;
            Ok((session, snapshot))
        })
    }
}
#[derive(Default)]
struct SyncState {
    online: bool,
    last_error: Option<&'static str>,
    last_attempt: Option<ClockSample>,
    last_generation: Option<uuid::Uuid>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointStatus {
    pub identity: RedactedStatus,
    pub configuration: CacheStatus,
    /// Last refresh result; all writes must still perform authenticated online requests.
    pub online: bool,
    pub last_sync_error: Option<&'static str>,
}
#[derive(Clone)]
pub struct SyncEngine {
    pub cache: SnapshotCache,
    transport: Arc<dyn ConfigTransport>,
    state: Arc<Mutex<SyncState>>,
    operation: Arc<tokio::sync::Mutex<()>>,
    wake: Arc<Notify>,
    changes: watch::Sender<CacheStatus>,
}
impl SyncEngine {
    pub fn new(cache: SnapshotCache, transport: Arc<dyn ConfigTransport>) -> Self {
        let initial = CacheStatus {
            state: "sync_required",
            revision: None,
            validated_at: None,
            expires_at: None,
            remaining_seconds: None,
        };
        let (changes, _) = watch::channel(initial);
        Self {
            cache,
            transport,
            state: Arc::new(Mutex::new(SyncState::default())),
            operation: Arc::new(tokio::sync::Mutex::new(())),
            wake: Arc::new(Notify::new()),
            changes,
        }
    }
    pub fn authorization_changes(&self) -> watch::Receiver<CacheStatus> {
        self.changes.subscribe()
    }
    /// A native reachability/wake signal or confirmed local login may request an immediate retry.
    pub fn notify_reconnected(&self) {
        self.wake.notify_one();
    }
    pub async fn refresh(&self) -> Result<()> {
        let _operation = self.operation.lock().await;
        let start = match self.cache.sample() {
            Ok(start) => start,
            Err(error) => {
                let mut state = self.state.lock().map_err(|_| EndpointError::Invalid)?;
                state.online = false;
                state.last_error = Some(error_code(&error));
                drop(state);
                self.publish();
                return Err(error);
            }
        };
        let generation = self.cache.sessions.store.read()?.generation;
        let result = match tokio::time::timeout(
            Duration::from_secs(10),
            self.transport.fetch(&self.cache.sessions),
        )
        .await
        {
            Ok(Ok((session, snapshot))) => self.cache.install(&session, snapshot, start),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(EndpointError::Offline),
        };
        {
            let mut state = self.state.lock().map_err(|_| EndpointError::Invalid)?;
            state.online = result.is_ok();
            state.last_error = result.as_ref().err().map(error_code);
            state.last_attempt = Some(start);
            state.last_generation = Some(generation);
        }
        self.publish();
        match &result {
            Ok(()) => tracing::info!(event = "endpoint_sync_succeeded"),
            Err(e) => tracing::info!(event = "endpoint_sync_failed", category = error_code(e)),
        }
        result
    }
    pub fn status(&self) -> Result<EndpointStatus> {
        let mut identity = self.cache.sessions.status()?;
        let configuration = self.cache.status();
        identity.configuration = configuration.state;
        if configuration.state == "server_mismatch" {
            identity.state = "server_mismatch";
            identity.identity = None;
            identity.generation = None;
            identity.session_expires_at = None;
        }
        let state = self.state.lock().map_err(|_| EndpointError::Invalid)?;
        Ok(EndpointStatus {
            online: state.online && identity.state != "server_mismatch",
            identity,
            configuration,
            last_sync_error: state.last_error,
        })
    }
    fn publish(&self) {
        self.changes.send_replace(self.cache.status());
    }
    pub fn due(&self) -> Result<bool> {
        let now = self.cache.sample()?;
        let metadata = self.cache.sessions.store.read()?;
        let generation = metadata.generation;
        if !self.cache.sessions.matches_selection(&metadata) {
            return Ok(false);
        }
        let state = self.state.lock().map_err(|_| EndpointError::Invalid)?;
        let Some(previous) = state.last_attempt else {
            return Ok(true);
        };
        if state.last_generation != Some(generation) {
            return Ok(true);
        }
        if metadata.active.is_none() {
            return Ok(false);
        }
        let interval = if state.online {
            gateway_core::REFRESH_INTERVAL_SECONDS
        } else {
            1
        };
        match now.elapsed_since(previous) {
            Ok(elapsed) => Ok(elapsed >= interval * NANOS_PER_SECOND),
            Err(_) => Ok(true),
        }
    }
    pub async fn poll_if_due(&self) -> Result<bool> {
        if self.due()? {
            self.refresh().await?;
            return Ok(true);
        }
        self.publish();
        Ok(false)
    }
    /// Startup refresh, 60-second native-clock cadence, fast offline recovery, and explicit wake/reconnect hints.
    pub async fn run(&self, stop: watch::Receiver<bool>) {
        let _ = self.refresh().await;
        self.run_scheduled(stop).await;
    }
    /// Continue cadence when the caller already completed the startup refresh.
    pub async fn run_scheduled(&self, mut stop: watch::Receiver<bool>) {
        loop {
            if *stop.borrow() {
                return;
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(1)) => { let _ = self.poll_if_due().await; },
                _ = self.wake.notified() => { let _ = self.refresh().await; },
                changed = stop.changed() => { if changed.is_err() || *stop.borrow() { return; } },
            }
        }
    }
}
fn error_code(error: &EndpointError) -> &'static str {
    match error {
        EndpointError::LoginRequired | EndpointError::Rejected => "login_required",
        EndpointError::Offline => "offline",
        EndpointError::Response => "invalid_response",
        EndpointError::CacheExpired => "expired",
        EndpointError::ClockUncertain => "clock_uncertain",
        EndpointError::RevisionRegression => "revision_regression",
        EndpointError::IdentityChanged => "identity_changed",
        EndpointError::ServerMismatch => "server_mismatch",
        _ => "invalid_cache",
    }
}
