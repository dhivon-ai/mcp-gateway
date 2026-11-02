//! Explicit native installation; receipts describe side effects, never authorization.
use crate::{
    EndpointError, Result,
    bridge::{BridgeError, BridgeResult, ControlBridge},
    clock::NANOS_PER_SECOND,
    epoch,
    store::check_private,
};
use base64::Engine;
use dhivon_mcp_gateway_core::{
    CommandSpec, InstallProgress, InstallRequest, InstallState, LocalCatalog, SettingsUpdate,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::watch;
use uuid::Uuid;

pub const INSTALL_TIMEOUT: Duration = Duration::from_secs(300);
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
#[derive(Clone)]
pub struct InstallManager {
    pub bridge: ControlBridge,
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    timeout: Duration,
    shutdown: Duration,
    #[cfg(all(test, target_os = "macos"))]
    cleanup_hook: Option<Arc<CleanupHook>>,
}
struct Job {
    cancel: watch::Sender<bool>,
    done: watch::Receiver<bool>,
    progress: InstallProgress,
}
struct InstallRun {
    command: CommandSpec,
    scope: String,
    digest: String,
    progress: InstallProgress,
    input: InstallRequest,
    start: crate::clock::ClockSample,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    scope: String,
    digest: String,
    progress: InstallProgress,
}
#[cfg(target_os = "macos")]
pub(crate) fn validate_receipts(bytes: &[u8]) -> Result<()> {
    let receipts: Vec<Receipt> =
        serde_json::from_slice(bytes).map_err(|_| EndpointError::Invalid)?;
    if receipts.len() > 256 {
        return Err(EndpointError::Invalid);
    }
    Ok(())
}
impl InstallManager {
    pub fn new(bridge: ControlBridge) -> Self {
        Self {
            bridge,
            jobs: Arc::new(Mutex::new(HashMap::new())),
            timeout: INSTALL_TIMEOUT,
            shutdown: SHUTDOWN_TIMEOUT,
            #[cfg(all(test, target_os = "macos"))]
            cleanup_hook: None,
        }
    }
    /// Smaller bounds are useful for native lifecycle fixtures; production can never exceed approved bounds.
    pub fn with_bounds(mut self, timeout: Duration, shutdown: Duration) -> Result<Self> {
        if timeout.is_zero()
            || shutdown.is_zero()
            || timeout > INSTALL_TIMEOUT
            || shutdown > SHUTDOWN_TIMEOUT
        {
            return Err(EndpointError::Invalid);
        }
        self.timeout = timeout;
        self.shutdown = shutdown;
        Ok(self)
    }
    fn lock_install(&self) -> Result<File> {
        let path = self
            .bridge
            .sync
            .cache
            .sessions
            .store
            .root()
            .join("install.lock");
        if std::fs::symlink_metadata(&path).is_ok() {
            check_private(&path, false)?;
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(path).map_err(|_| EndpointError::Storage)?;
        file.try_lock()
            .map_err(|_| EndpointError::IdentityChanged)?;
        Ok(file)
    }
    fn scope(&self, generation: &str) -> Result<String> {
        let store = &self.bridge.sync.cache.sessions.store;
        store.locked(|m| {
            if m.generation.to_string() != generation
                || !self.bridge.sync.cache.sessions.matches_selection(m)
            {
                return Err(EndpointError::IdentityChanged);
            }
            let identity = m.active.as_ref().ok_or(EndpointError::LoginRequired)?;
            Ok(hash(
                &serde_json::to_vec(&(m.installation_id, identity))
                    .map_err(|_| EndpointError::Invalid)?,
            ))
        })
    }
    fn receipts(&self) -> Result<Vec<Receipt>> {
        let Some(bytes) = self
            .bridge
            .sync
            .cache
            .sessions
            .store
            .read_component("installations.json", 262144)?
        else {
            return Ok(Vec::new());
        };
        if bytes.len() > 262144 {
            return Err(EndpointError::Storage);
        }
        let receipts: Vec<Receipt> =
            serde_json::from_slice(&bytes).map_err(|_| EndpointError::Storage)?;
        if receipts.len() > 256 {
            return Err(EndpointError::Storage);
        }
        Ok(receipts)
    }
    fn save(&self, scope: &str, digest: &str, progress: &InstallProgress) -> Result<()> {
        let store = &self.bridge.sync.cache.sessions.store;
        store.locked(|_| {
            let mut receipts = self.receipts()?;
            receipts.retain(|r| !(r.scope == scope && r.progress.mcp_id == progress.mcp_id));
            if receipts.len() >= 256 {
                receipts.remove(0);
            }
            receipts.push(Receipt {
                scope: scope.into(),
                digest: digest.into(),
                progress: progress.clone(),
            });
            let bytes = serde_json::to_vec(&receipts).map_err(|_| EndpointError::Storage)?;
            if bytes.len() > 262144 {
                return Err(EndpointError::Storage);
            }
            store.atomic_write("installations.json", &bytes)
        })
    }
    pub fn catalog(&self, generation: &str) -> BridgeResult<LocalCatalog> {
        self.bridge.check(generation)?;
        let validated = self.bridge.sync.cache.latest()?;
        if validated.generation != ControlBridge::generation(generation)? {
            return Err(EndpointError::IdentityChanged.into());
        }
        let mut catalog = crate::bridge::metadata(&validated.snapshot, generation);
        let scope = self.scope(generation)?;
        let receipts = self
            .bridge
            .sync
            .cache
            .sessions
            .store
            .locked(|_| self.receipts())?;
        for mcp in &mut catalog.mcps {
            let definition = validated
                .snapshot
                .mcps
                .iter()
                .find(|item| item.definition.id == mcp.id);
            if let Some(item) = definition {
                if let dhivon_mcp_gateway_core::Transport::Stdio { command } =
                    &item.definition.transport
                    && executable_missing(command)
                {
                    mcp.availability = "missing_executable".into();
                } else if mcp.enabled
                    && !matches!(
                        item.definition.auth,
                        dhivon_mcp_gateway_core::AuthMode::None {}
                    )
                {
                    let native = crate::upstream_credentials::NativeCredentials::new(
                        self.bridge.sync.cache.clone(),
                    );
                    let readiness = native
                        .scope(&validated, item)
                        .and_then(|scope| native.load(&scope))
                        .and_then(|head| head.access(self.bridge.sync.cache.sample()?));
                    mcp.availability = match readiness {
                        Ok(_) => "configured",
                        Err(EndpointError::UpstreamExpired) => "token_expired",
                        Err(EndpointError::UpstreamRejected) => "auth_rejected",
                        Err(EndpointError::UpstreamRequired)
                            if mcp.auth == "bearer" && mcp.secret.is_some() =>
                        {
                            "credential_sync_required"
                        }
                        _ => "auth_required",
                    }
                    .into();
                    mcp.auth_required = mcp.availability != "configured";
                }
            }
            mcp.installed = receipts.iter().any(|r| {
                r.scope == scope
                    && r.progress.mcp_id == mcp.id
                    && validated
                        .snapshot
                        .mcps
                        .iter()
                        .find(|m| m.definition.id == mcp.id)
                        .and_then(|m| m.definition.install.as_ref())
                        .and_then(|c| fingerprint(c).ok())
                        .is_some_and(|digest| digest == r.digest)
                    && matches!(
                        r.progress.state,
                        InstallState::InstalledDisabled | InstallState::Enabled
                    )
            });
        }
        self.bridge.check(generation)?;
        Ok(catalog)
    }
    pub async fn start(
        &self,
        generation: &str,
        id: &str,
        input: InstallRequest,
    ) -> BridgeResult<InstallProgress> {
        crate::bridge::segment(id)?;
        let prepared = self
            .bridge
            .prepare(generation, Some(input.expected_revision))
            .await?;
        let mcp = prepared
            .snapshot
            .mcps
            .iter()
            .find(|m| m.definition.id == id && m.definition.active)
            .ok_or_else(BridgeError::invalid)?;
        // Explicit installation starts with centrally confirmed disablement. There is no optimistic enabled state.
        if mcp.enabled {
            return Err(BridgeError::conflict(self.catalog(generation)?));
        }
        let command = mcp
            .definition
            .install
            .clone()
            .ok_or_else(BridgeError::invalid)?;
        command.validate().map_err(|_| BridgeError::invalid())?;
        let scope = self.scope(generation)?;
        let digest = fingerprint(&command)?;
        let lock = self.lock_install()?;
        self.bridge.check(generation)?;
        let previous = self.bridge.sync.cache.sessions.store.locked(|_| {
            Ok(self
                .receipts()?
                .into_iter()
                .find(|r| r.scope == scope && r.progress.mcp_id == id && r.digest == digest))
        })?;
        if let Some(mut previous) = previous
            && !input.rerun
        {
            previous.progress.generation = generation.into();
            if previous.progress.state == InstallState::Running {
                previous.progress.state = InstallState::Interrupted;
                previous.progress.message =
                    "Installation outcome is uncertain; rerun only by explicit request".into();
                self.save(&scope, &digest, &previous.progress)?;
            }
            return Ok(previous.progress);
        }
        let start = self.bridge.sync.cache.sample()?;
        let progress = InstallProgress {
            id: Uuid::new_v4().to_string(),
            mcp_id: id.into(),
            generation: generation.into(),
            state: InstallState::Running,
            started_at: epoch()?,
            completed_at: None,
            elapsed_seconds: 0,
            exit_code: None,
            message: "Installation running; output is private".into(),
        };
        self.save(&scope, &digest, &progress)?;
        let (cancel, cancelled) = watch::channel(false);
        let (done, completed) = watch::channel(false);
        {
            let mut jobs = self.jobs.lock().map_err(|_| EndpointError::Invalid)?;
            jobs.retain(|_, j| !*j.done.borrow());
            jobs.insert(
                progress.id.clone(),
                Job {
                    cancel,
                    done: completed,
                    progress: progress.clone(),
                },
            );
        }
        tracing::info!(event = "endpoint_install_started", correlation_id = %progress.id, state = "running");
        let owner = self.clone();
        let returned = progress.clone();
        tokio::spawn(async move {
            owner
                .run(
                    InstallRun {
                        command,
                        scope,
                        digest,
                        progress,
                        input,
                        start,
                    },
                    cancelled,
                )
                .await;
            drop(lock);
            let _ = done.send(true);
        });
        Ok(returned)
    }
    async fn run(&self, run: InstallRun, cancel: watch::Receiver<bool>) {
        let InstallRun {
            command,
            scope,
            digest,
            mut progress,
            input,
            start,
        } = run;
        let result = self
            .execute(&command, &digest, &progress, start, cancel.clone())
            .await;
        let failure_message = result.as_ref().err().map(ToString::to_string);
        let (state, code) = match result {
            Ok((state, code)) => (state, code),
            Err(_) => (InstallState::Failed, None),
        };
        progress.state = state;
        progress.exit_code = code;
        progress.completed_at = epoch().ok();
        progress.elapsed_seconds = self
            .bridge
            .sync
            .cache
            .sample()
            .and_then(|now| now.elapsed_since(start))
            .unwrap_or(0)
            / NANOS_PER_SECOND;
        progress.message = failure_message.unwrap_or_else(|| message(&progress.state).into());
        let fence = if progress.state == InstallState::InstalledDisabled {
            self.completion_fence(&progress, &digest, start, &cancel)
                .ok()
        } else {
            None
        };
        if progress.state == InstallState::InstalledDisabled && fence.is_none() {
            progress.message = "Installed locally; installation authorization ended, so automatic enablement was not attempted".into();
        }
        let persisted = self.save(&scope, &digest, &progress).is_ok();
        if persisted
            && progress.state == InstallState::InstalledDisabled
            && input.enable_after_install
            && fence.is_some()
        {
            if self
                .bridge
                .settings_guarded(
                    &progress.generation,
                    &progress.mcp_id,
                    SettingsUpdate {
                        expected_revision: input.expected_revision,
                        enabled: true,
                        secret_id: input.secret_id,
                    },
                    || {
                        fence.is_some_and(|fence| {
                            self.fence_permits(&progress, &digest, start, &cancel, fence)
                        })
                    },
                )
                .await
                .is_ok()
            {
                progress.state = InstallState::Enabled;
                progress.message = message(&progress.state).into();
                if self.save(&scope, &digest, &progress).is_err() {
                    progress.message =
                        "Central enablement confirmed; local receipt could not be saved".into();
                }
            } else {
                progress.message = "Installed locally; enablement unconfirmed. Review settings and retry enable explicitly".into();
            }
        } else if persisted && progress.state == InstallState::InstalledDisabled && *cancel.borrow()
        {
            progress.message = "Installed locally; automatic enablement was cancelled".into();
        } else if !persisted {
            progress.state = InstallState::Interrupted;
            progress.message =
                "Local outcome receipt unavailable; enablement was not attempted".into();
        }
        if persisted && progress.state == InstallState::InstalledDisabled {
            let _ = self.save(&scope, &digest, &progress);
        }
        tracing::info!(event = "endpoint_install_outcome", correlation_id = %progress.id, duration_seconds = progress.elapsed_seconds, state = state_name(&progress.state), exit_code = progress.exit_code);
        if let Ok(mut jobs) = self.jobs.lock()
            && let Some(job) = jobs.get_mut(&progress.id)
        {
            job.progress = progress;
        }
    }
    fn authorization(
        &self,
        progress: &InstallProgress,
        digest: &str,
    ) -> Result<crate::cache::ValidatedSnapshot> {
        self.bridge
            .check(&progress.generation)
            .map_err(|_| EndpointError::Rejected)?;
        let latest = self.bridge.sync.cache.latest()?;
        if latest.generation
            != ControlBridge::generation(&progress.generation)
                .map_err(|_| EndpointError::Invalid)?
        {
            return Err(EndpointError::Rejected);
        }
        let mcp = latest
            .snapshot
            .mcps
            .iter()
            .find(|m| m.definition.id == progress.mcp_id && m.definition.active && !m.enabled)
            .ok_or(EndpointError::Rejected)?;
        if fingerprint(
            mcp.definition
                .install
                .as_ref()
                .ok_or(EndpointError::Rejected)?,
        )? != digest
        {
            return Err(EndpointError::Rejected);
        }
        Ok(latest)
    }
    fn completion_fence(
        &self,
        progress: &InstallProgress,
        digest: &str,
        start: crate::clock::ClockSample,
        cancel: &watch::Receiver<bool>,
    ) -> Result<(crate::clock::ClockSample, u64)> {
        if *cancel.borrow() {
            return Err(EndpointError::Cancelled);
        }
        let now = self.bridge.sync.cache.sample()?;
        if now.elapsed_since(start)? >= u64::try_from(self.timeout.as_nanos()).unwrap_or(u64::MAX) {
            return Err(EndpointError::CacheExpired);
        }
        let latest = self.authorization(progress, digest)?;
        Ok((now, latest.remaining_ns))
    }
    fn fence_permits(
        &self,
        progress: &InstallProgress,
        digest: &str,
        start: crate::clock::ClockSample,
        cancel: &watch::Receiver<bool>,
        fence: (crate::clock::ClockSample, u64),
    ) -> bool {
        self.completion_fence(progress, digest, start, cancel)
            .is_ok()
            && self
                .bridge
                .sync
                .cache
                .sample()
                .and_then(|now| now.elapsed_since(fence.0))
                .is_ok_and(|elapsed| elapsed < fence.1)
    }
    #[cfg(target_os = "macos")]
    async fn execute(
        &self,
        command: &CommandSpec,
        digest: &str,
        progress: &InstallProgress,
        start: crate::clock::ClockSample,
        mut cancel: watch::Receiver<bool>,
    ) -> Result<(InstallState, Option<i32>)> {
        self.authorization(progress, digest)?;
        let child = crate::owned_process::OwnedProcess::spawn(command)?;
        let observed = loop {
            if *cancel.borrow() {
                break InstallState::Cancelled;
            }
            match self.authorization(progress, digest) {
                Ok(_) => (),
                Err(EndpointError::CacheExpired | EndpointError::ClockUncertain) => {
                    break InstallState::Expired;
                }
                Err(_) => break InstallState::Revoked,
            }
            let elapsed = match self
                .bridge
                .sync
                .cache
                .sample()
                .and_then(|now| now.elapsed_since(start))
            {
                Ok(elapsed) => elapsed,
                Err(_) => break InstallState::Expired,
            };
            if elapsed >= u64::try_from(self.timeout.as_nanos()).unwrap_or(u64::MAX) {
                break InstallState::Expired;
            }
            match child.poll_exit() {
                Ok(Some(code)) => {
                    break if code == Some(0) {
                        InstallState::InstalledDisabled
                    } else {
                        InstallState::Failed
                    };
                }
                Ok(None) => (),
                Err(_) => break InstallState::Failed,
            }
            tokio::select! { _ = tokio::time::sleep(Duration::from_millis(100)) => (), _ = cancel.changed() => () }
        };
        #[cfg(test)]
        if let Some(hook) = &self.cleanup_hook {
            hook.arrived.notify_one();
            hook.release.notified().await;
        }
        let status = child.terminate(self.shutdown).await?;
        Ok((observed, status.code()))
    }
    #[cfg(not(target_os = "macos"))]
    async fn execute(
        &self,
        _: &CommandSpec,
        _: &str,
        _: &InstallProgress,
        _: crate::clock::ClockSample,
        _: watch::Receiver<bool>,
    ) -> Result<(InstallState, Option<i32>)> {
        Err(EndpointError::Unsupported)
    }
    pub fn progress(&self, generation: &str, id: &str) -> BridgeResult<InstallProgress> {
        self.bridge.check(generation)?;
        if let Some(job) = self
            .jobs
            .lock()
            .map_err(|_| EndpointError::Invalid)?
            .get(id)
            && job.progress.generation == generation
        {
            let mut progress = job.progress.clone();
            if progress.state == InstallState::Running {
                progress.elapsed_seconds = epoch()?.saturating_sub(progress.started_at) as u64;
            }
            return Ok(progress);
        }
        let scope = self.scope(generation)?;
        let mut progress = self
            .bridge
            .sync
            .cache
            .sessions
            .store
            .locked(|_| self.receipts())?
            .into_iter()
            .find(|r| r.scope == scope && r.progress.id == id)
            .ok_or_else(BridgeError::invalid)?
            .progress;
        progress.generation = generation.into();
        if progress.state == InstallState::Running && self.lock_install().is_ok() {
            progress.state = InstallState::Interrupted;
            progress.message = message(&progress.state).into();
        }
        self.bridge.check(generation)?;
        Ok(progress)
    }
    pub fn cancel(&self, generation: &str, id: &str) -> BridgeResult<()> {
        self.bridge.check(generation)?;
        let jobs = self.jobs.lock().map_err(|_| EndpointError::Invalid)?;
        let job = jobs
            .get(id)
            .filter(|j| j.progress.generation == generation)
            .ok_or_else(BridgeError::invalid)?;
        tracing::info!(event = "endpoint_install_cancel_requested", correlation_id = %job.progress.id);
        let _ = job.cancel.send(true);
        Ok(())
    }
    pub async fn enable(
        &self,
        generation: &str,
        id: &str,
        input: SettingsUpdate,
    ) -> BridgeResult<LocalCatalog> {
        let _lock = self.lock_install()?;
        let progress = self.progress(generation, id)?;
        if !input.enabled
            || !matches!(
                progress.state,
                InstallState::InstalledDisabled | InstallState::Enabled
            )
        {
            return Err(BridgeError::invalid());
        }
        let prepared = self
            .bridge
            .prepare(generation, Some(input.expected_revision))
            .await?;
        let command = prepared
            .snapshot
            .mcps
            .iter()
            .find(|m| m.definition.id == progress.mcp_id)
            .and_then(|m| m.definition.install.as_ref())
            .ok_or_else(BridgeError::invalid)?;
        let scope = self.scope(generation)?;
        if !self.bridge.sync.cache.sessions.store.locked(|_| {
            Ok(self.receipts()?.iter().any(|r| {
                r.scope == scope
                    && r.progress.id == id
                    && r.digest == fingerprint(command).unwrap_or_default()
            }))
        })? {
            return Err(BridgeError::invalid());
        }
        let catalog = self
            .bridge
            .settings(generation, &progress.mcp_id, input)
            .await?;
        let mut progress = progress;
        progress.state = InstallState::Enabled;
        progress.message = message(&progress.state).into();
        self.save(&scope, &fingerprint(command)?, &progress)?;
        if let Some(job) = self
            .jobs
            .lock()
            .map_err(|_| EndpointError::Invalid)?
            .get_mut(id)
        {
            job.progress = progress;
        }
        Ok(catalog)
    }
    pub async fn shutdown(&self) {
        self.shutdown_matching(None).await;
    }
    pub async fn shutdown_generation(&self, generation: &str) {
        self.shutdown_matching(Some(generation)).await;
    }
    async fn shutdown_matching(&self, generation: Option<&str>) {
        let receivers = match self.jobs.lock() {
            Ok(jobs) => jobs
                .values()
                .filter(|j| generation.is_none_or(|g| j.progress.generation == g))
                .map(|j| {
                    let _ = j.cancel.send(true);
                    j.done.clone()
                })
                .collect::<Vec<_>>(),
            Err(_) => return,
        };
        let deadline = tokio::time::Instant::now() + SHUTDOWN_TIMEOUT;
        for mut receiver in receivers {
            if !*receiver.borrow() {
                let _ = tokio::time::timeout_at(deadline, receiver.wait_for(|done| *done)).await;
            }
        }
    }
}

/// A redacted filesystem diagnostic only: browsing never executes a command.
fn executable_missing(command: &CommandSpec) -> bool {
    let cwd = command
        .cwd
        .as_ref()
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok());
    if command
        .cwd
        .as_ref()
        .is_some_and(|path| !std::path::Path::new(path).is_dir())
    {
        return true;
    }
    let program = std::path::Path::new(&command.program);
    if program.is_absolute() {
        return !program.is_file();
    }
    if command.program.contains('/') {
        return cwd.is_none_or(|cwd| !cwd.join(program).is_file());
    }
    let path = command
        .env
        .get("PATH")
        .map_or("/usr/bin:/bin:/usr/sbin:/sbin", String::as_str);
    !path.split(':').take(128).any(|dir| {
        let dir = std::path::Path::new(dir);
        let candidate = if dir.is_absolute() {
            dir.join(program)
        } else if let Some(cwd) = &cwd {
            cwd.join(dir).join(program)
        } else {
            return false;
        };
        candidate.is_file()
    })
}
fn fingerprint(command: &CommandSpec) -> Result<String> {
    Ok(hash(
        &serde_json::to_vec(command).map_err(|_| EndpointError::Invalid)?,
    ))
}
fn message(state: &InstallState) -> &'static str {
    match state {
        InstallState::Running => "Installation running",
        InstallState::Failed => {
            "Installation failed; output is private; rerun only by explicit request"
        }
        InstallState::Cancelled => {
            "Installation cancelled; no automatic enablement was attempted; review current configuration"
        }
        InstallState::Expired => {
            "Installation authorization or time limit expired; no automatic enablement was attempted; review current configuration"
        }
        InstallState::Revoked => {
            "Installation authorization changed; no automatic enablement was attempted; review current configuration"
        }
        InstallState::InstalledDisabled => {
            "Installed locally; no automatic enablement was attempted; review current configuration"
        }
        InstallState::Enabled => "Installed and central enablement confirmed",
        InstallState::Interrupted => {
            "Installation outcome uncertain; rerun only by explicit request"
        }
    }
}

fn hash(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(bytes))
}

#[cfg(all(test, target_os = "macos"))]
struct CleanupHook {
    arrived: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

fn state_name(state: &InstallState) -> &'static str {
    match state {
        InstallState::Running => "running",
        InstallState::Failed => "failed",
        InstallState::Cancelled => "cancelled",
        InstallState::Expired => "expired",
        InstallState::Revoked => "revoked",
        InstallState::InstalledDisabled => "installed_disabled",
        InstallState::Enabled => "enabled",
        InstallState::Interrupted => "interrupted",
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::{
        cache::SnapshotCache,
        clock::{Clock, NativeClock},
        credentials::CredentialStore,
        session::SessionManager,
        store::{Identity, StateStore},
        sync::{ConfigTransport, FetchFuture, SyncEngine},
    };
    use dhivon_mcp_gateway_core::{
        AuthMode, ConfigSnapshot, EffectiveMcp, EndpointSession, McpDefinition, Role, ToolPolicy,
        Transport, UserInfo,
    };
    use std::{
        collections::BTreeMap,
        sync::atomic::{AtomicUsize, Ordering},
    };
    use zeroize::Zeroizing;
    #[derive(Default)]
    struct Memory(Mutex<HashMap<String, Vec<u8>>>);
    impl CredentialStore for Memory {
        fn get(&self, key: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
            Ok(self
                .0
                .lock()
                .expect("fixture credentials")
                .get(key)
                .cloned()
                .map(Zeroizing::new))
        }
        fn set(&self, key: &str, value: &[u8]) -> Result<()> {
            self.0
                .lock()
                .expect("fixture credentials")
                .insert(key.into(), value.into());
            Ok(())
        }
        fn delete(&self, key: &str) -> Result<()> {
            self.0.lock().expect("fixture credentials").remove(key);
            Ok(())
        }
    }
    struct Count(AtomicUsize);
    impl ConfigTransport for Count {
        fn fetch<'a>(&'a self, _: &'a SessionManager) -> FetchFuture<'a> {
            Box::pin(async move {
                self.0.fetch_add(1, Ordering::Relaxed);
                Err(EndpointError::Offline)
            })
        }
    }
    struct Time(Mutex<crate::clock::ClockSample>);
    impl Clock for Time {
        fn sample(&self) -> Result<crate::clock::ClockSample> {
            Ok(*self.0.lock().expect("fixture clock"))
        }
    }
    impl Time {
        fn advance(&self, seconds: u64) {
            let mut now = self.0.lock().expect("fixture clock");
            now.wall_ns += seconds * NANOS_PER_SECOND;
            now.continuous_ns += seconds * NANOS_PER_SECOND;
        }
    }
    #[tokio::test]
    async fn successful_install_losing_authorization_during_cleanup_never_attempts_enable() {
        for kind in ["cancel", "expiry", "revocation", "timeout"] {
            let root = std::env::temp_dir()
                .canonicalize()
                .expect("fixture parent")
                .join(format!("gateway-cleanup-race-owned-{}", Uuid::new_v4()));
            let store =
                StateStore::new(root.clone(), Arc::new(Memory::default())).expect("fixture store");
            let now = epoch().expect("fixture time");
            let user = UserInfo {
                name: "Fixture User".into(),
                id: Uuid::new_v4().to_string(),
                email: "fixture@example.test".into(),
                role: Role::User,
                active: true,
            };
            let credentials = EndpointSession {
                token: "private-owned-fixture-token".into(),
                user: user.clone(),
                deployment_id: Uuid::new_v4().to_string(),
                expires_at: now + 43200,
            };
            store
                .locked(|m| {
                    m.active = Some(Identity {
                        server_origin: "http://127.0.0.1:1".into(),
                        deployment_id: credentials.deployment_id.clone(),
                        user_id: user.id.clone(),
                    });
                    let account = StateStore::account(
                        m.active.as_ref().expect("identity"),
                        m.generation,
                        "session",
                    )?;
                    m.credential_accounts.push(account.clone());
                    store.write(m)?;
                    store.credentials().set(
                        &account,
                        &serde_json::to_vec(&credentials).expect("fixture credentials"),
                    )
                })
                .expect("seed fixture");
            let sessions = SessionManager::new(store.clone(), true);
            let clock = Arc::new(Time(Mutex::new(
                NativeClock::new()
                    .expect("actual native clock")
                    .sample()
                    .expect("native sample"),
            )));
            let start = clock.sample().expect("clock");
            let cache = SnapshotCache::new(sessions.clone(), clock.clone());
            let command = CommandSpec {
                program: "/usr/bin/true".into(),
                args: vec![],
                env: BTreeMap::new(),
                cwd: None,
            };
            cache
                .install(
                    &sessions.load().expect("session"),
                    ConfigSnapshot {
                        schema_version: 1,
                        deployment_id: credentials.deployment_id.clone(),
                        user,
                        revision: 1,
                        validated_at: now,
                        expires_at: now + 3600,
                        session_expires_at: credentials.expires_at,
                        mcps: vec![EffectiveMcp {
                            definition: McpDefinition {
                                id: "fixture-mcp".into(),
                                name: "Fixture".into(),
                                description: "Fixture".into(),
                                active: true,
                                transport: Transport::Stdio {
                                    command: command.clone(),
                                },
                                install: Some(command.clone()),
                                auth: AuthMode::None {},
                                tool_policy: ToolPolicy::default(),
                            },
                            enabled: false,
                            secret: None,
                        }],
                    },
                    start,
                )
                .expect("fixture snapshot");
            let attempts = Arc::new(Count(AtomicUsize::new(0)));
            let mut manager =
                InstallManager::new(ControlBridge::new(SyncEngine::new(cache, attempts.clone())));
            let hook = Arc::new(CleanupHook {
                arrived: tokio::sync::Notify::new(),
                release: tokio::sync::Notify::new(),
            });
            manager.cleanup_hook = Some(hook.clone());
            let generation = sessions
                .load()
                .expect("session")
                .metadata
                .generation
                .to_string();
            let id = Uuid::new_v4().to_string();
            let progress = InstallProgress {
                id: id.clone(),
                mcp_id: "fixture-mcp".into(),
                generation: generation.clone(),
                state: InstallState::Running,
                started_at: now,
                completed_at: None,
                elapsed_seconds: 0,
                exit_code: None,
                message: "Running".into(),
            };
            let scope = manager.scope(&generation).expect("scope");
            let digest = fingerprint(&command).expect("digest");
            let (cancel, cancelled) = watch::channel(false);
            let owned = manager.clone();
            let task = tokio::spawn(async move {
                owned
                    .run(
                        InstallRun {
                            command,
                            scope,
                            digest,
                            progress,
                            input: InstallRequest {
                                expected_revision: 1,
                                enable_after_install: true,
                                secret_id: None,
                                rerun: false,
                            },
                            start,
                        },
                        cancelled,
                    )
                    .await;
            });
            hook.arrived.notified().await; // success observed; group cleanup has not completed
            match kind {
                "cancel" => cancel.send(true).expect("cancel during successful cleanup"),
                "expiry" => clock.advance(3600),
                "timeout" => clock.advance(300),
                "revocation" => store
                    .locked(|m| store.clear_locked(m))
                    .expect("identity revoked during cleanup"),
                _ => unreachable!(),
            }
            hook.release.notify_one();
            task.await.expect("owned cleanup");
            assert_eq!(attempts.0.load(Ordering::Relaxed), 0);
            assert_eq!(
                manager
                    .receipts()
                    .expect("receipt journal")
                    .iter()
                    .find(|r| r.progress.id == id)
                    .expect("owned receipt")
                    .progress
                    .state,
                InstallState::InstalledDisabled
            );
            store
                .locked(|m| store.clear_locked(m))
                .expect("owned cleanup");
            std::fs::remove_dir_all(root).expect("owned fixture directory");
        }
    }
}
