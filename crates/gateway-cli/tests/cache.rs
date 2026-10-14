use gateway_cli::{
    EndpointError,
    cache::SnapshotCache,
    clock::{Clock, ClockSample, NANOS_PER_SECOND},
    credentials::{CredentialKind, CredentialStore},
    epoch,
    session::{NativeSession, SessionManager},
    store::{Identity, StateStore},
    sync::{ConfigTransport, FetchFuture, NativeTransport, SyncEngine},
};
#[cfg(target_os = "macos")]
use gateway_cli::{clock::NativeClock, credentials::MacKeychain};
use gateway_core::{ConfigSnapshot, EndpointSession, Role, UserInfo};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicU32, AtomicUsize, Ordering},
    },
};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Clone, Default)]
struct CapturedLogs(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for CapturedLogs {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("log fixture").extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedLogs {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}

#[derive(Default)]
struct Memory(Mutex<HashMap<String, Vec<u8>>>);
impl CredentialStore for Memory {
    fn get(&self, key: &str) -> gateway_cli::Result<Option<Zeroizing<Vec<u8>>>> {
        Ok(self
            .0
            .lock()
            .expect("fixture lock")
            .get(key)
            .cloned()
            .map(Zeroizing::new))
    }
    fn set(&self, key: &str, value: &[u8]) -> gateway_cli::Result<()> {
        self.0
            .lock()
            .expect("fixture lock")
            .insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, key: &str) -> gateway_cli::Result<()> {
        self.0.lock().expect("fixture lock").remove(key);
        Ok(())
    }
}
struct FakeClock {
    sample: Mutex<ClockSample>,
    fail: std::sync::atomic::AtomicBool,
}
impl FakeClock {
    fn new() -> Self {
        Self {
            sample: Mutex::new(ClockSample {
                boot_id: Uuid::new_v4(),
                continuous_ns: 100 * NANOS_PER_SECOND,
                wall_ns: epoch().expect("clock") as u64 * NANOS_PER_SECOND,
            }),
            fail: std::sync::atomic::AtomicBool::new(false),
        }
    }
    fn advance(&self, seconds: u64) {
        let mut sample = self.sample.lock().expect("clock fixture");
        sample.wall_ns += seconds * NANOS_PER_SECOND;
        sample.continuous_ns += seconds * NANOS_PER_SECOND;
    }
}
impl Clock for FakeClock {
    fn sample(&self) -> gateway_cli::Result<ClockSample> {
        if self.fail.load(Ordering::Relaxed) {
            return Err(EndpointError::ClockUncertain);
        }
        Ok(*self.sample.lock().expect("clock fixture"))
    }
}
struct Fixture {
    root: PathBuf,
    sessions: SessionManager,
    cache: SnapshotCache,
    clock: Arc<FakeClock>,
}
impl Fixture {
    fn new(credentials: Arc<dyn CredentialStore>) -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .expect("canonical fixture parent")
            .join(format!("gateway-cache-owned-{}", Uuid::new_v4()));
        let sessions = SessionManager::new(
            StateStore::new(root.clone(), credentials).expect("private fixture store"),
            true,
        );
        let clock = Arc::new(FakeClock::new());
        let cache = SnapshotCache::new(sessions.clone(), clock.clone());
        let f = Self {
            root,
            sessions,
            cache,
            clock,
        };
        f.seed("http://127.0.0.1:1");
        f
    }
    fn seed(&self, origin: &str) {
        self.sessions
            .store
            .locked(|m| {
                m.active = Some(Identity {
                    server_origin: origin.into(),
                    deployment_id: Uuid::new_v4().to_string(),
                    user_id: Uuid::new_v4().to_string(),
                });
                let id = m.active.as_ref().expect("fixture identity");
                let account = StateStore::account(id, m.generation, "session")?;
                let session = EndpointSession {
                    token: "private-central-cache-fixture-token".into(),
                    user: UserInfo {
                        id: id.user_id.clone(),
                        username: "cache-fixture-user".into(),
                        role: Role::User,
                        active: true,
                    },
                    deployment_id: id.deployment_id.clone(),
                    expires_at: epoch()? + 43200,
                };
                m.credential_accounts.push(account.clone());
                self.sessions.store.write(m)?;
                self.sessions.store.credentials().set(
                    &account,
                    &serde_json::to_vec(&session).expect("fixture session JSON"),
                )
            })
            .expect("fixture native session");
    }
    fn native(&self) -> NativeSession {
        self.sessions.load().expect("fixture session")
    }
    fn snapshot(&self, revision: u32) -> ConfigSnapshot {
        snapshot(
            &self.native(),
            self.clock.sample().expect("clock"),
            revision,
        )
    }
    fn install(&self, revision: u32) {
        self.cache
            .install(
                &self.native(),
                self.snapshot(revision),
                self.clock.sample().expect("clock"),
            )
            .expect("complete fixture install");
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self
            .sessions
            .store
            .locked(|m| self.sessions.store.clear_locked(m));
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn snapshot(session: &NativeSession, now: ClockSample, revision: u32) -> ConfigSnapshot {
    let validated_at = now.wall_seconds().expect("epoch");
    ConfigSnapshot {
        schema_version: 1,
        deployment_id: session.credentials.deployment_id.clone(),
        user: session.credentials.user.clone(),
        revision,
        validated_at,
        expires_at: (validated_at + 3600).min(session.credentials.expires_at),
        session_expires_at: session.credentials.expires_at,
        mcps: Vec::new(),
    }
}
struct Transport {
    clock: Arc<FakeClock>,
    revision: AtomicU32,
    mode: AtomicU8,
    calls: AtomicUsize,
}
impl Transport {
    fn new(clock: Arc<FakeClock>) -> Self {
        Self {
            clock,
            revision: AtomicU32::new(1),
            mode: AtomicU8::new(0),
            calls: AtomicUsize::new(0),
        }
    }
}
impl ConfigTransport for Transport {
    fn fetch<'a>(&'a self, sessions: &'a SessionManager) -> FetchFuture<'a> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let mode = self.mode.load(Ordering::Relaxed);
            if mode == 1 {
                return Err(EndpointError::Offline);
            }
            if mode == 2 {
                return Err(EndpointError::Response);
            }
            if mode == 5 {
                return std::future::pending().await;
            }
            let session = sessions.load()?;
            let mut config = snapshot(
                &session,
                self.clock.sample()?,
                self.revision.load(Ordering::Relaxed),
            );
            if mode == 3 {
                config.expires_at = config.validated_at + 3601;
            }
            if mode == 4 {
                config.user.active = false;
            }
            Ok((session, config))
        })
    }
}

#[test]
fn exact_expiry_restart_rewrites_and_upstream_refresh_do_not_extend() {
    for elapsed in [3599, 3600, 3601] {
        let f = Fixture::new(Arc::new(Memory::default()));
        f.install(1);
        let cipher = std::fs::read(f.root.join("snapshot.enc")).expect("ciphertext");
        assert!(!String::from_utf8_lossy(&cipher).contains("cache-fixture-user"));
        assert!(!String::from_utf8_lossy(&cipher).contains("private-central-cache-fixture-token"));
        f.clock.advance(elapsed);
        // Byte rewrite and unrelated credential replacement cannot alter the immutable lease.
        f.sessions
            .store
            .locked(|_| f.sessions.store.atomic_write("snapshot.enc", &cipher))
            .expect("rewrite same ciphertext");
        f.sessions
            .save_credential(
                f.native().metadata.generation,
                CredentialKind::Upstream {
                    mcp_id: Uuid::new_v4(),
                    version: 2,
                },
                b"private-refreshed-upstream-token",
            )
            .expect("credential refresh");
        let restarted = SnapshotCache::new(f.sessions.clone(), f.clock.clone());
        let result = restarted.latest();
        if elapsed == 3599 {
            assert_eq!(result.expect("still valid").remaining_ns, NANOS_PER_SECOND);
        } else {
            assert!(matches!(result, Err(EndpointError::CacheExpired)));
        }
    }
}

#[test]
fn session_deadline_request_time_and_timestamp_validation() {
    let f = Fixture::new(Arc::new(Memory::default()));
    let mut native = f.native();
    native.credentials.expires_at = f
        .clock
        .sample()
        .expect("clock")
        .wall_seconds()
        .expect("wall")
        + 30;
    let account = StateStore::account(
        native.metadata.active.as_ref().expect("identity"),
        native.metadata.generation,
        "session",
    )
    .expect("account");
    f.sessions
        .store
        .credentials()
        .set(
            &account,
            &serde_json::to_vec(&native.credentials).expect("session"),
        )
        .expect("short fixture session");
    let start = f.clock.sample().expect("clock");
    let config = snapshot(&native, start, 1);
    f.clock.advance(10);
    f.cache
        .install(&native, config, start)
        .expect("charged request time");
    assert_eq!(
        f.cache.latest().expect("valid").remaining_ns,
        20 * NANOS_PER_SECOND
    );
    f.clock.advance(20);
    assert!(matches!(f.cache.latest(), Err(EndpointError::CacheExpired)));
    let f = Fixture::new(Arc::new(Memory::default()));
    for bad in 0..4 {
        let mut snapshot = f.snapshot(1);
        match bad {
            0 => snapshot.expires_at = snapshot.validated_at + 3601,
            1 => snapshot.validated_at += 1,
            2 => snapshot.session_expires_at += 1,
            _ => snapshot.schema_version = 999,
        };
        assert!(
            f.cache
                .install(&f.native(), snapshot, f.clock.sample().expect("clock"))
                .is_err()
        );
        assert!(f.cache.latest().is_err());
    }
}

#[test]
fn wall_rollback_boot_change_native_regression_and_missing_clock_fail_closed() {
    for mode in 0..4 {
        let f = Fixture::new(Arc::new(Memory::default()));
        f.install(1);
        f.clock.advance(100);
        f.cache.latest().expect("high water mark");
        {
            let mut sample = f.clock.sample.lock().expect("clock fixture");
            match mode {
                0 => sample.wall_ns -= NANOS_PER_SECOND,
                1 => sample.boot_id = Uuid::new_v4(),
                2 => sample.continuous_ns -= NANOS_PER_SECOND,
                _ => {
                    f.clock.fail.store(true, Ordering::Relaxed);
                }
            }
        }
        let restarted = SnapshotCache::new(f.sessions.clone(), f.clock.clone());
        assert!(matches!(
            restarted.latest(),
            Err(EndpointError::ClockUncertain)
        ));
        f.clock.fail.store(false, Ordering::Relaxed);
        assert!(restarted.latest().is_err());
    }
}

#[test]
fn simulated_sleep_advances_native_deadline_even_when_wall_does_not() {
    let f = Fixture::new(Arc::new(Memory::default()));
    f.install(1);
    f.clock.sample.lock().expect("clock fixture").continuous_ns += 3600 * NANOS_PER_SECOND;
    assert!(matches!(f.cache.latest(), Err(EndpointError::CacheExpired)));
}

#[test]
fn tamper_missing_key_copy_and_prior_receipt_replay_require_online_recovery() {
    let credentials = Arc::new(Memory::default());
    let f = Fixture::new(credentials.clone());
    f.install(1);
    let old = std::fs::read(f.root.join("snapshot.enc")).expect("ciphertext");
    f.clock.advance(60);
    f.install(2);
    let current = std::fs::read(f.root.join("snapshot.enc")).expect("new ciphertext");
    assert_ne!(old, current);
    f.sessions
        .store
        .locked(|_| f.sessions.store.atomic_write("snapshot.enc", &old))
        .expect("replay old receipt");
    assert!(matches!(f.cache.latest(), Err(EndpointError::InvalidCache)));
    f.sessions
        .store
        .locked(|_| f.sessions.store.atomic_write("snapshot.enc", &current))
        .expect("restore bytes");
    assert!(f.cache.latest().is_err());
    f.install(2);
    f.cache.latest().expect("online recovery");
    let mut corrupt = std::fs::read(f.root.join("snapshot.enc")).expect("ciphertext");
    let last = corrupt.len() - 1;
    corrupt[last] ^= 1;
    f.sessions
        .store
        .locked(|_| f.sessions.store.atomic_write("snapshot.enc", &corrupt))
        .expect("tamper");
    assert!(matches!(f.cache.latest(), Err(EndpointError::InvalidCache)));
    f.install(2);
    let other = Fixture::new(credentials.clone());
    let copied = std::fs::read(f.root.join("snapshot.enc")).expect("ciphertext");
    other
        .sessions
        .store
        .locked(|_| other.sessions.store.atomic_write("snapshot.enc", &copied))
        .expect("copy to other principal");
    assert!(other.cache.latest().is_err());
    // Even supplying the source key/checkpoint cannot authenticate copied ciphertext for the other identity.
    other.install(2);
    let source_metadata = f.sessions.store.read().expect("source metadata");
    let target_metadata = other.sessions.store.read().expect("target metadata");
    let source_key = StateStore::account(
        source_metadata.active.as_ref().expect("identity"),
        source_metadata.generation,
        "cache-key",
    )
    .expect("source key account");
    let target_key = StateStore::account(
        target_metadata.active.as_ref().expect("identity"),
        target_metadata.generation,
        "cache-key",
    )
    .expect("target key account");
    let source_head = source_metadata
        .cache_head_account
        .as_ref()
        .expect("source checkpoint");
    let target_head = target_metadata
        .cache_head_account
        .as_ref()
        .expect("target checkpoint");
    credentials
        .set(
            &target_key,
            &credentials
                .get(&source_key)
                .expect("source key")
                .expect("key"),
        )
        .expect("owned key-copy fixture");
    credentials
        .set(
            target_head,
            &credentials
                .get(source_head)
                .expect("source checkpoint")
                .expect("checkpoint"),
        )
        .expect("owned checkpoint-copy fixture");
    other
        .sessions
        .store
        .locked(|_| other.sessions.store.atomic_write("snapshot.enc", &copied))
        .expect("owned ciphertext-copy fixture");
    assert!(matches!(
        other.cache.latest(),
        Err(EndpointError::InvalidCache)
    ));
    let account = StateStore::account(
        f.native().metadata.active.as_ref().expect("identity"),
        f.native().metadata.generation,
        "cache-key",
    )
    .expect("cache account");
    credentials.delete(&account).expect("missing key");
    assert!(f.cache.latest().is_err());
    f.install(3);
    f.cache.latest().expect("online missing-key recovery");
}

#[test]
fn stale_revision_equal_revision_policy_changes_and_login_generation_are_rejected() {
    let f = Fixture::new(Arc::new(Memory::default()));
    f.install(4);
    let original = std::fs::read(f.root.join("snapshot.enc")).expect("ciphertext");
    f.clock.advance(1);
    assert!(matches!(
        f.cache
            .install(&f.native(), f.snapshot(3), f.clock.sample().expect("clock")),
        Err(EndpointError::RevisionRegression)
    ));
    let mut conflicting = f.snapshot(4);
    conflicting.user.username = "different-policy-same-revision".into();
    assert!(matches!(
        f.cache
            .install(&f.native(), conflicting, f.clock.sample().expect("clock")),
        Err(EndpointError::RevisionRegression)
    ));
    assert_eq!(
        std::fs::read(f.root.join("snapshot.enc")).expect("unchanged ciphertext"),
        original
    );
    let native = f.native();
    let snapshot = f.snapshot(5);
    f.sessions
        .invalidate(native.metadata.generation)
        .expect("logout identity");
    assert!(matches!(
        f.cache
            .install(&native, snapshot, f.clock.sample().expect("clock")),
        Err(EndpointError::IdentityChanged)
    ));
    assert!(!f.root.join("snapshot.enc").exists());
}

#[tokio::test]
async fn complete_unchanged_refresh_startup_polling_failures_and_reconnect() {
    let f = Fixture::new(Arc::new(Memory::default()));
    let transport = Arc::new(Transport::new(f.clock.clone()));
    let engine = SyncEngine::new(f.cache.clone(), transport.clone());
    assert!(engine.poll_if_due().await.expect("startup refresh"));
    assert_eq!(transport.calls.load(Ordering::Relaxed), 1);
    let expiry = engine.status().expect("status").configuration.expires_at;
    f.clock.advance(59);
    assert!(!engine.poll_if_due().await.expect("not due"));
    f.clock.advance(1);
    assert!(
        engine
            .poll_if_due()
            .await
            .expect("60 second unchanged refresh")
    );
    assert!(engine.status().expect("status").configuration.expires_at > expiry);
    let durable = std::fs::read(f.root.join("snapshot.enc")).expect("durable snapshot");
    for mode in [1, 2, 3] {
        transport.mode.store(mode, Ordering::Relaxed);
        f.clock.advance(60);
        assert!(engine.refresh().await.is_err());
        assert_eq!(
            std::fs::read(f.root.join("snapshot.enc")).expect("no failed renewal"),
            durable
        );
        assert!(!engine.status().expect("status").online);
    }
    transport.mode.store(0, Ordering::Relaxed);
    f.clock.advance(1);
    assert!(
        engine
            .poll_if_due()
            .await
            .expect("immediate responsive reconnect retry")
    );
    assert!(engine.status().expect("status").online);
    transport.mode.store(4, Ordering::Relaxed);
    f.clock.advance(1);
    assert!(matches!(
        engine.refresh().await,
        Err(EndpointError::Rejected)
    ));
    assert_eq!(
        engine.status().expect("status").configuration.state,
        "login_required"
    );
}

struct FailCommittedHead {
    memory: Memory,
    fail: std::sync::atomic::AtomicBool,
}
impl CredentialStore for FailCommittedHead {
    fn get(&self, key: &str) -> gateway_cli::Result<Option<Zeroizing<Vec<u8>>>> {
        self.memory.get(key)
    }
    fn set(&self, key: &str, value: &[u8]) -> gateway_cli::Result<()> {
        if self.fail.load(Ordering::Relaxed)
            && key.contains(":cache-head:")
            && serde_json::from_slice::<serde_json::Value>(value)
                .ok()
                .is_some_and(|v| v["committed"] == true)
        {
            return Err(EndpointError::Keychain);
        }
        self.memory.set(key, value)
    }
    fn delete(&self, key: &str) -> gateway_cli::Result<()> {
        self.memory.delete(key)
    }
}
#[test]
fn partial_persistence_cannot_publish_or_fall_back_to_previous_snapshot() {
    let credentials = Arc::new(FailCommittedHead {
        memory: Memory::default(),
        fail: std::sync::atomic::AtomicBool::new(false),
    });
    let f = Fixture::new(credentials.clone());
    f.install(1);
    f.clock.advance(60);
    credentials.fail.store(true, Ordering::Relaxed);
    assert!(matches!(
        f.cache
            .install(&f.native(), f.snapshot(2), f.clock.sample().expect("clock")),
        Err(EndpointError::Keychain)
    ));
    assert!(matches!(f.cache.latest(), Err(EndpointError::InvalidCache)));
    credentials.fail.store(false, Ordering::Relaxed);
    f.install(2);
    f.cache.latest().expect("complete recovery");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            f.root.join("snapshot.enc"),
            std::fs::Permissions::from_mode(0o644),
        )
        .expect("fixture publication failure");
        f.clock.advance(60);
        assert!(matches!(
            f.cache
                .install(&f.native(), f.snapshot(3), f.clock.sample().expect("clock")),
            Err(EndpointError::Storage)
        ));
        assert!(f.cache.latest().is_err());
        std::fs::set_permissions(
            f.root.join("snapshot.enc"),
            std::fs::Permissions::from_mode(0o600),
        )
        .expect("repair fixture permissions");
        assert!(f.cache.latest().is_err());
        f.install(3);
        f.cache.latest().expect("new complete recovery only");
    }
}

#[tokio::test]
async fn native_http_partial_error_unchanged_and_401_refresh_behavior() {
    let logs = CapturedLogs::default();
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(logs.clone())
            .finish(),
    )
    .expect("single captured-log fixture");
    use axum::{Router, extract::State, http::StatusCode, routing::get};
    #[derive(Clone)]
    struct HttpFixture {
        sessions: SessionManager,
        clock: Arc<FakeClock>,
        mode: Arc<AtomicU8>,
    }
    let f = Fixture::new(Arc::new(Memory::default()));
    let mode = Arc::new(AtomicU8::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("fixture listener");
    let origin = format!("http://{}", listener.local_addr().expect("fixture address"));
    f.sessions
        .store
        .locked(|m| {
            let old = m.active.clone().expect("fixture identity");
            let old_account = StateStore::account(&old, m.generation, "session")?;
            let bytes = f
                .sessions
                .store
                .credentials()
                .get(&old_account)?
                .expect("fixture session");
            m.active.as_mut().expect("identity").server_origin = origin.clone();
            let account = StateStore::account(
                m.active.as_ref().expect("identity"),
                m.generation,
                "session",
            )?;
            m.credential_accounts = vec![account.clone()];
            f.sessions.store.write(m)?;
            f.sessions.store.credentials().set(&account, &bytes)?;
            f.sessions.store.credentials().delete(&old_account)
        })
        .expect("fixture origin selection");
    let app = Router::new()
        .route(
            "/api/config",
            get(|State(h): State<HttpFixture>| async move {
                match h.mode.load(Ordering::Relaxed) {
                    1 => (
                        StatusCode::OK,
                        "{\"schemaVersion\":1,\"private-input\":\"never-log-parser-body".into(),
                    ),
                    2 => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "private-error-body".into(),
                    ),
                    3 => (StatusCode::UNAUTHORIZED, "private-rejection-body".into()),
                    _ => (
                        StatusCode::OK,
                        serde_json::to_string(&snapshot(
                            &h.sessions.load().expect("session"),
                            h.clock.sample().expect("clock"),
                            1,
                        ))
                        .expect("snapshot JSON"),
                    ),
                }
            }),
        )
        .with_state(HttpFixture {
            sessions: f.sessions.clone(),
            clock: f.clock.clone(),
            mode: mode.clone(),
        });
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("fixture server");
    });
    let engine = SyncEngine::new(f.cache.clone(), Arc::new(NativeTransport));
    engine.refresh().await.expect("real native HTTP sync");
    let first_expiry = engine.status().expect("status").configuration.expires_at;
    for failure in [1, 2] {
        mode.store(failure, Ordering::Relaxed);
        f.clock.advance(1);
        assert!(engine.refresh().await.is_err());
        assert_eq!(
            engine.status().expect("status").configuration.expires_at,
            first_expiry
        );
    }
    mode.store(0, Ordering::Relaxed);
    f.clock.advance(60);
    engine
        .refresh()
        .await
        .expect("unchanged revision complete renewal");
    assert!(engine.status().expect("status").configuration.expires_at > first_expiry);
    mode.store(3, Ordering::Relaxed);
    assert!(matches!(
        engine.refresh().await,
        Err(EndpointError::Rejected)
    ));
    assert!(!f.root.join("snapshot.enc").exists());
    let status = serde_json::to_string(&engine.status().expect("redacted status")).expect("JSON");
    for secret in [
        "private-central-cache-fixture-token",
        "private-rejection-body",
        "private-error-body",
        "never-log-parser-body",
    ] {
        assert!(!status.contains(secret));
    }
    let captured = String::from_utf8(logs.0.lock().expect("logs").clone()).expect("UTF8 logs");
    assert!(captured.contains("endpoint_config_committed"));
    assert!(captured.contains("endpoint_sync_failed"));
    for secret in [
        "private-central-cache-fixture-token",
        "private-rejection-body",
        "private-error-body",
        "never-log-parser-body",
    ] {
        assert!(!captured.contains(secret));
    }
    server.abort();
}

#[tokio::test]
async fn refresh_whole_operation_has_ten_second_deadline() {
    let f = Fixture::new(Arc::new(Memory::default()));
    f.install(1);
    let transport = Arc::new(Transport::new(f.clock.clone()));
    transport.mode.store(5, Ordering::Relaxed);
    let engine = SyncEngine::new(f.cache.clone(), transport);
    let start = std::time::Instant::now();
    assert!(matches!(
        engine.refresh().await,
        Err(EndpointError::Offline)
    ));
    assert!(start.elapsed() >= std::time::Duration::from_secs(10));
    assert!(start.elapsed() < std::time::Duration::from_secs(12));
    assert!(f.cache.latest().is_ok());
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "owned subprocess helper invoked by actual_mac_clock_cache_and_process_restart"]
fn native_cache_child() {
    let root = PathBuf::from(
        std::env::var_os("MCP_GATEWAY_OWNED_CACHE_CHILD_ROOT").expect("owned child root"),
    );
    let service =
        std::env::var("MCP_GATEWAY_OWNED_CACHE_CHILD_SERVICE").expect("owned fixture service");
    let store = StateStore::new(root, Arc::new(MacKeychain::new(service)))
        .expect("existing protected store");
    let cache = SnapshotCache::new(
        SessionManager::new(store, true),
        Arc::new(NativeClock::new().expect("actual native clock")),
    );
    let current = cache.latest().expect("actual separate-process cache read");
    assert_eq!(current.snapshot.revision, 42);
    assert!(current.remaining_ns > 0);
    assert!(current.remaining_ns <= 3600 * NANOS_PER_SECOND);
}

#[cfg(target_os = "macos")]
#[test]
fn actual_mac_clock_cache_and_process_restart() {
    let service = format!("com.cadre.mcp-gateway.owned-cache-test.{}", Uuid::new_v4());
    let keychain = Arc::new(MacKeychain::new(service.clone()));
    let f = Fixture::new(keychain.clone());
    let native_clock = Arc::new(NativeClock::new().expect("actual Darwin continuous clock"));
    let first = native_clock.sample().expect("actual first sample");
    std::thread::sleep(std::time::Duration::from_millis(20));
    let independent_clock = NativeClock::new().expect("independent native constructor");
    let second = independent_clock.sample().expect("actual second sample");
    assert_eq!(first.boot_id, second.boot_id);
    assert!(second.continuous_ns > first.continuous_ns);
    assert!(second.wall_ns >= first.wall_ns);
    let cache = SnapshotCache::new(f.sessions.clone(), native_clock);
    cache
        .install(&f.native(), snapshot(&f.native(), second, 42), second)
        .expect("actual encrypted durable cache");
    let initial = cache.latest().expect("actual cache");
    let ciphertext = std::fs::read(f.root.join("snapshot.enc")).expect("actual ciphertext");
    let account = StateStore::account(
        f.native().metadata.active.as_ref().expect("identity"),
        f.native().metadata.generation,
        "cache-key",
    )
    .expect("cache account");
    assert!(
        keychain
            .get(&account)
            .expect("actual Keychain cache key")
            .is_some()
    );
    let child = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", "native_cache_child", "--ignored", "--nocapture"])
        .env("MCP_GATEWAY_OWNED_CACHE_CHILD_ROOT", &f.root)
        .env("MCP_GATEWAY_OWNED_CACHE_CHILD_SERVICE", &service)
        .output()
        .expect("owned cache reader process");
    assert!(
        child.status.success(),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&child.stdout).contains("private-central-cache-fixture-token")
    );
    let restarted = SnapshotCache::new(f.sessions.clone(), Arc::new(independent_clock));
    let after = restarted.latest().expect("restart preserves deadline");
    assert!(after.remaining_ns < initial.remaining_ns);
    assert_eq!(
        std::fs::read(f.root.join("snapshot.enc")).expect("unchanged immutable snapshot"),
        ciphertext
    );
    f.sessions
        .store
        .locked(|m| f.sessions.store.clear_locked(m))
        .expect("actual cache logout cleanup");
    assert!(
        keychain
            .get(&account)
            .expect("actual key deleted")
            .is_none()
    );
    assert!(!f.root.join("snapshot.enc").exists());
}

struct FailCheckpointRetirement {
    memory: Memory,
    fail: std::sync::atomic::AtomicBool,
}
impl CredentialStore for FailCheckpointRetirement {
    fn get(&self, key: &str) -> gateway_cli::Result<Option<Zeroizing<Vec<u8>>>> {
        self.memory.get(key)
    }
    fn set(&self, key: &str, value: &[u8]) -> gateway_cli::Result<()> {
        self.memory.set(key, value)
    }
    fn delete(&self, key: &str) -> gateway_cli::Result<()> {
        if self.fail.load(Ordering::Relaxed) && key.contains(":cache-head:") {
            return Err(EndpointError::Keychain);
        }
        self.memory.delete(key)
    }
}
#[test]
fn checkpoint_pointer_replay_retirement_failure_and_recovery_are_guarded() {
    let credentials = Arc::new(FailCheckpointRetirement {
        memory: Memory::default(),
        fail: std::sync::atomic::AtomicBool::new(false),
    });
    let f = Fixture::new(credentials.clone());
    f.install(1);
    let original_head = f
        .sessions
        .store
        .read()
        .expect("metadata")
        .cache_head_account
        .expect("protected head pointer");
    f.cache.latest().expect("watermark checkpoint publication");
    let current = f.sessions.store.read().expect("metadata");
    assert_eq!(current.credential_accounts.len(), 3);
    assert!(!current.credential_accounts.contains(&original_head));
    assert!(
        credentials
            .get(&original_head)
            .expect("retired checkpoint")
            .is_none()
    );
    f.sessions
        .store
        .locked(|m| {
            m.cache_head_account = Some(original_head.clone());
            f.sessions.store.write(m)
        })
        .expect("owned old-pointer replay fixture");
    assert!(f.cache.latest().is_err());
    // Restore only the ordinary pointer so the independent failure scenario can proceed.
    std::fs::write(
        f.root.join("identity.json"),
        serde_json::to_vec(&current).expect("metadata fixture JSON"),
    )
    .expect("restore fixture pointer");
    credentials.fail.store(true, Ordering::Relaxed);
    assert!(matches!(f.cache.latest(), Err(EndpointError::Keychain)));
    let incomplete = f
        .sessions
        .store
        .read()
        .expect("pending publication metadata");
    assert!(incomplete.cache_publication_pending);
    assert!(incomplete.credential_accounts.len() > 3);
    assert!(f.cache.latest().is_err());
    // Repointing to the old still-owned receipt does not bypass the persisted publication barrier.
    f.sessions
        .store
        .locked(|m| {
            m.cache_head_account = current.cache_head_account.clone();
            f.sessions.store.write(m)
        })
        .expect("pending pointer replay fixture");
    assert!(f.cache.latest().is_err());
    credentials.fail.store(false, Ordering::Relaxed);
    f.clock.advance(1);
    f.install(1);
    f.cache.latest().expect("new complete online recovery");
    let recovered = f.sessions.store.read().expect("bounded recovered metadata");
    assert!(!recovered.cache_publication_pending);
    assert_eq!(recovered.credential_accounts.len(), 3);
}

struct AdvanceDuringPublication {
    memory: Memory,
    clock: Mutex<Option<Arc<FakeClock>>>,
    action: AtomicU8,
}
impl CredentialStore for AdvanceDuringPublication {
    fn get(&self, key: &str) -> gateway_cli::Result<Option<Zeroizing<Vec<u8>>>> {
        self.memory.get(key)
    }
    fn set(&self, key: &str, value: &[u8]) -> gateway_cli::Result<()> {
        if key.contains(":cache-head:")
            && serde_json::from_slice::<serde_json::Value>(value)
                .ok()
                .is_some_and(|v| v["committed"] == true)
        {
            let action = self.action.swap(0, Ordering::Relaxed);
            let clock = self.clock.lock().expect("fixture clock");
            if action == 1 {
                clock.as_ref().expect("configured clock").advance(3600);
            }
            if action == 2 {
                clock
                    .as_ref()
                    .expect("configured clock")
                    .fail
                    .store(true, Ordering::Relaxed);
            }
        }
        self.memory.set(key, value)
    }
    fn delete(&self, key: &str) -> gateway_cli::Result<()> {
        self.memory.delete(key)
    }
}
#[test]
fn expiry_during_keychain_publication_never_returns_usable_authorization() {
    for during_install in [false, true] {
        let credentials = Arc::new(AdvanceDuringPublication {
            memory: Memory::default(),
            clock: Mutex::new(None),
            action: AtomicU8::new(0),
        });
        let f = Fixture::new(credentials.clone());
        *credentials.clock.lock().expect("clock fixture") = Some(f.clock.clone());
        if !during_install {
            f.install(1);
        }
        credentials.action.store(1, Ordering::Relaxed);
        if during_install {
            assert!(matches!(
                f.cache
                    .install(&f.native(), f.snapshot(1), f.clock.sample().expect("clock")),
                Err(EndpointError::CacheExpired)
            ));
        } else {
            assert!(matches!(f.cache.latest(), Err(EndpointError::CacheExpired)));
        }
        assert!(f.cache.latest().is_err());
    }
}

struct DelayedTransport {
    clock: Arc<FakeClock>,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
impl ConfigTransport for DelayedTransport {
    fn fetch<'a>(&'a self, sessions: &'a SessionManager) -> FetchFuture<'a> {
        Box::pin(async move {
            let native = sessions.load()?;
            let snapshot = snapshot(&native, self.clock.sample()?, 2);
            self.entered.notify_one();
            self.release.notified().await;
            Ok((native, snapshot))
        })
    }
}
#[tokio::test]
async fn delayed_refresh_cannot_overwrite_newer_shared_revision_or_logout() {
    for logout in [false, true] {
        let f = Fixture::new(Arc::new(Memory::default()));
        f.install(1);
        let transport = Arc::new(DelayedTransport {
            clock: f.clock.clone(),
            entered: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
        });
        let engine = SyncEngine::new(f.cache.clone(), transport.clone());
        let pending = tokio::spawn(async move { engine.refresh().await });
        transport.entered.notified().await;
        f.clock.advance(1);
        if logout {
            f.sessions
                .invalidate(f.native().metadata.generation)
                .expect("logout during refresh");
        } else {
            f.install(3);
        }
        transport.release.notify_one();
        let result = pending.await.expect("refresh task");
        if logout {
            assert!(matches!(result, Err(EndpointError::IdentityChanged)));
            assert!(f.cache.latest().is_err());
        } else {
            assert!(matches!(result, Err(EndpointError::RevisionRegression)));
            assert_eq!(
                f.cache
                    .latest()
                    .expect("latest shared committed revision")
                    .snapshot
                    .revision,
                3
            );
        }
    }
}

#[tokio::test]
async fn running_sync_startup_notification_and_owned_shutdown() {
    let f = Fixture::new(Arc::new(Memory::default()));
    let transport = Arc::new(Transport::new(f.clock.clone()));
    let engine = SyncEngine::new(f.cache.clone(), transport.clone());
    let mut changes = engine.authorization_changes();
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let running = engine.clone();
    let task = tokio::spawn(async move { running.run(stopped).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), changes.changed())
        .await
        .expect("startup deadline")
        .expect("startup notice");
    assert_eq!(changes.borrow().state, "valid");
    f.clock.advance(1);
    transport.revision.store(2, Ordering::Relaxed);
    engine.notify_reconnected();
    tokio::time::timeout(std::time::Duration::from_secs(2), changes.changed())
        .await
        .expect("notification deadline")
        .expect("reconnect notice");
    assert_eq!(changes.borrow().revision, Some(2));
    assert_eq!(transport.calls.load(Ordering::Relaxed), 2);
    stop.send(true).expect("owned stop");
    tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .expect("bounded sync shutdown")
        .expect("owned task exit");
}

#[test]
fn clock_failure_after_final_publication_requires_new_online_recovery() {
    let credentials = Arc::new(AdvanceDuringPublication {
        memory: Memory::default(),
        clock: Mutex::new(None),
        action: AtomicU8::new(0),
    });
    let f = Fixture::new(credentials.clone());
    *credentials.clock.lock().expect("clock fixture") = Some(f.clock.clone());
    credentials.action.store(2, Ordering::Relaxed);
    assert!(matches!(
        f.cache
            .install(&f.native(), f.snapshot(1), f.clock.sample().expect("clock")),
        Err(EndpointError::ClockUncertain)
    ));
    f.clock.fail.store(false, Ordering::Relaxed);
    assert!(
        f.sessions
            .store
            .read()
            .expect("metadata")
            .cache_publication_pending
    );
    assert!(f.cache.latest().is_err());
    f.install(1);
    f.cache.latest().expect("new complete online recovery");
}

#[tokio::test]
async fn same_server_cache_preservation_and_delayed_other_origin_sync_isolation() {
    use gateway_cli::{central::CentralClient, management::ManagementState};
    let f = Fixture::new(Arc::new(Memory::default()));
    f.install(1);
    let old = CentralClient::new("http://127.0.0.1:1", true).expect("old fixture origin");
    let new = CentralClient::new("http://127.0.0.1:2", true).expect("new fixture origin");
    let original = f.native().metadata.generation;
    let ciphertext = std::fs::read(f.root.join("snapshot.enc")).expect("snapshot");
    let old_sessions = f
        .sessions
        .select_server(&old)
        .await
        .expect("same server selection");
    assert_eq!(
        old_sessions
            .load()
            .expect("same identity")
            .metadata
            .generation,
        original
    );
    assert_eq!(
        std::fs::read(f.root.join("snapshot.enc")).expect("preserved cache"),
        ciphertext
    );
    let old_cache = SnapshotCache::new(old_sessions.clone(), f.clock.clone());
    assert_eq!(
        old_cache
            .latest()
            .expect("same origin valid cache")
            .snapshot
            .revision,
        1
    );
    let transport = Arc::new(DelayedTransport {
        clock: f.clock.clone(),
        entered: tokio::sync::Notify::new(),
        release: tokio::sync::Notify::new(),
    });
    let engine = SyncEngine::new(old_cache, transport.clone());
    assert!(
        ManagementState::new(
            old_sessions.clone(),
            old.clone(),
            "127.0.0.1:18126".parse().expect("address"),
            None
        )
        .expect("old UI")
        .with_sync(engine.clone())
        .is_ok()
    );
    assert!(
        ManagementState::new(
            old_sessions.clone(),
            new.clone(),
            "127.0.0.1:18127".parse().expect("address"),
            None
        )
        .expect("new UI")
        .with_sync(engine.clone())
        .is_err()
    );
    let pending_engine = engine.clone();
    let pending = tokio::spawn(async move { pending_engine.refresh().await });
    transport.entered.notified().await;
    let new_sessions = f
        .sessions
        .select_server(&new)
        .await
        .expect("different origin selection");
    assert!(!f.root.join("snapshot.enc").exists());
    f.seed(&new.origin());
    let new_cache = SnapshotCache::new(new_sessions.clone(), f.clock.clone());
    let native = new_sessions.load().expect("new fixture identity");
    new_cache
        .install(
            &native,
            snapshot(&native, f.clock.sample().expect("clock"), 3),
            f.clock.sample().expect("clock"),
        )
        .expect("new origin cache");
    let current = new_sessions
        .load()
        .expect("new session")
        .metadata
        .generation;
    transport.release.notify_one();
    assert!(matches!(
        pending.await.expect("old refresh task"),
        Err(EndpointError::ServerMismatch)
    ));
    assert!(
        !engine
            .due()
            .expect("older UI does not poll other selection")
    );
    let status = engine.status().expect("fixed old status");
    assert_eq!(status.identity.state, "server_mismatch");
    assert_eq!(status.configuration.state, "server_mismatch");
    assert!(!status.online);
    assert_eq!(
        new_cache
            .latest()
            .expect("new shared cache remains valid")
            .snapshot
            .revision,
        3
    );
    assert_eq!(
        new_sessions
            .load()
            .expect("new generation unaffected")
            .metadata
            .generation,
        current
    );
}
