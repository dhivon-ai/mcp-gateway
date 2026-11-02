use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
};
use dhivon_mcp_gateway_cli::{
    EndpointError,
    bridge::ControlBridge,
    cache::SnapshotCache,
    central::CentralClient,
    clock::{Clock, ClockSample, NANOS_PER_SECOND},
    credentials::CredentialStore,
    epoch,
    install::InstallManager,
    management::{ManagementState, router},
    session::SessionManager,
    store::StateStore,
    sync::{NativeTransport, SyncEngine},
};
use dhivon_mcp_gateway_core::*;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tower::ServiceExt;
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Default)]
struct Memory {
    entries: Mutex<HashMap<String, Vec<u8>>>,
    fail: AtomicBool,
}
impl CredentialStore for Memory {
    fn get(&self, key: &str) -> dhivon_mcp_gateway_cli::Result<Option<Zeroizing<Vec<u8>>>> {
        Ok(self
            .entries
            .lock()
            .expect("owned management fixture")
            .get(key)
            .cloned()
            .map(Zeroizing::new))
    }
    fn set(&self, key: &str, value: &[u8]) -> dhivon_mcp_gateway_cli::Result<()> {
        if self.fail.load(Ordering::Relaxed) {
            return Err(EndpointError::Keychain);
        }
        self.entries
            .lock()
            .expect("owned management fixture")
            .insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, key: &str) -> dhivon_mcp_gateway_cli::Result<()> {
        self.entries
            .lock()
            .expect("owned management fixture")
            .remove(key);
        Ok(())
    }
}
struct Time {
    now: Mutex<ClockSample>,
    fail: AtomicBool,
}
impl Clock for Time {
    fn sample(&self) -> dhivon_mcp_gateway_cli::Result<ClockSample> {
        if self.fail.load(Ordering::Relaxed) {
            return Err(EndpointError::ClockUncertain);
        }
        Ok(*self.now.lock().expect("owned management fixture"))
    }
}
impl Time {
    #[cfg(target_os = "macos")]
    fn advance(&self, seconds: u64) {
        let mut now = self.now.lock().expect("owned management fixture");
        now.wall_ns += seconds * NANOS_PER_SECOND;
        now.continuous_ns += seconds * NANOS_PER_SECOND;
    }
}
#[derive(Clone)]
struct Central {
    snapshot: Arc<Mutex<ConfigSnapshot>>,
    config_code: Arc<AtomicU16>,
    mutation_code: Arc<AtomicU16>,
    mutations: Arc<AtomicUsize>,
    owners: Arc<Mutex<Vec<String>>>,
    break_publication: Arc<AtomicBool>,
    memory: Arc<Memory>,
    delay: Arc<AtomicBool>,
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}
struct Fixture {
    root: PathBuf,
    sessions: SessionManager,
    sync: SyncEngine,
    installs: InstallManager,
    central: Central,
    client: CentralClient,
    task: tokio::task::JoinHandle<()>,
    time: Arc<Time>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
        let _ = self
            .sessions
            .store
            .locked(|m| self.sessions.store.clear_locked(m));
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    async fn new(command: CommandSpec) -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .expect("owned management fixture")
            .join(format!("gateway-management-owned-{}", Uuid::new_v4()));
        let memory = Arc::new(Memory::default());
        let now = epoch().expect("owned management fixture");
        let user = UserInfo {
            name: "Fixture User".into(),
            id: Uuid::new_v4().to_string(),
            email: "alice@example.test".into(),
            role: Role::User,
            active: true,
        };
        let snapshot = ConfigSnapshot {
            schema_version: 1,
            deployment_id: Uuid::new_v4().to_string(),
            user,
            revision: 1,
            validated_at: now,
            expires_at: now + 3600,
            session_expires_at: now + 43200,
            mcps: vec![EffectiveMcp {
                definition: McpDefinition {
                    id: "fixture-mcp".into(),
                    name: "Fixture MCP".into(),
                    description: "Metadata only".into(),
                    active: true,
                    transport: Transport::Stdio {
                        command: command.clone(),
                    },
                    install: Some(command),
                    auth: AuthMode::None {},
                    tool_policy: ToolPolicy::Allowlist(vec!["read".into()]),
                },
                enabled: false,
                secret: None,
            }],
        };
        let central = Central {
            snapshot: Arc::new(Mutex::new(snapshot)),
            config_code: Arc::new(AtomicU16::new(200)),
            mutation_code: Arc::new(AtomicU16::new(200)),
            mutations: Arc::new(AtomicUsize::new(0)),
            owners: Arc::new(Mutex::new(Vec::new())),
            break_publication: Arc::new(AtomicBool::new(false)),
            memory: memory.clone(),
            delay: Arc::new(AtomicBool::new(false)),
            entered: Arc::new(tokio::sync::Notify::new()),
            release: Arc::new(tokio::sync::Notify::new()),
        };
        let app = Router::new()
            .route("/api/endpoint/login", post(|State(s): State<Central>, axum::Json(input): axum::Json<LoginRequest>| async move { let mut snapshot = s.snapshot.lock().expect("owned management fixture"); if snapshot.user.email != input.email { snapshot.user.id = Uuid::new_v4().to_string(); snapshot.user.email = input.email.clone(); } axum::Json(EndpointSession { token: format!("private-central-{}", input.email.split('@').next().expect("fixture local part")), user: snapshot.user.clone(), deployment_id: snapshot.deployment_id.clone(), expires_at: snapshot.session_expires_at }) }))
            .route("/api/config", get(|State(s): State<Central>| async move { let code = StatusCode::from_u16(s.config_code.load(Ordering::Relaxed)).expect("owned management fixture"); (code, axum::Json(s.snapshot.lock().expect("owned management fixture").clone())) }))
            .route("/api/auth/logout", post(|| async { StatusCode::NO_CONTENT }))
            .route("/api/settings/{id}", put(|State(s): State<Central>, headers: HeaderMap, axum::Json(input): axum::Json<SettingsUpdate>| async move {
                s.mutations.fetch_add(1, Ordering::Relaxed); s.owners.lock().expect("owned management fixture").push(headers.get("authorization").expect("owned management fixture").to_str().expect("owned management fixture").into());
                if s.delay.load(Ordering::Relaxed) { s.entered.notify_one(); s.release.notified().await; }
                let code = StatusCode::from_u16(s.mutation_code.load(Ordering::Relaxed)).expect("owned management fixture");
                let mut snapshot = s.snapshot.lock().expect("owned management fixture");
                if code != StatusCode::OK || input.expected_revision != snapshot.revision { return (if code == StatusCode::OK { StatusCode::CONFLICT } else { code }, axum::Json(serde_json::to_value(ApiError { code: if code == StatusCode::FORBIDDEN { ErrorCode::Forbidden } else { ErrorCode::Conflict }, message: "private-server-diagnostic".into(), revision: Some(snapshot.revision), snapshot: if code == StatusCode::CONFLICT { Some(snapshot.clone()) } else { None } }).expect("owned management fixture"))).into_response(); }
                snapshot.revision += 1; snapshot.mcps[0].enabled = input.enabled;
                if s.break_publication.load(Ordering::Relaxed) { s.memory.fail.store(true, Ordering::Relaxed); }
                axum::Json(snapshot.clone()).into_response()
            }))
            .route("/api/secrets", get(|State(s): State<Central>| async move { let code = StatusCode::from_u16(s.mutation_code.load(Ordering::Relaxed)).expect("owned management fixture"); if code == StatusCode::OK { axum::Json(serde_json::json!([secret_meta()])).into_response() } else { secret_conflict(code).into_response() } }).post(secret_write))
            .route("/api/secrets/{id}", put(secret_write).delete(|State(s): State<Central>, Path(_): Path<String>, axum::Json(_): axum::Json<VersionInput>| async move { s.mutations.fetch_add(1, Ordering::Relaxed); StatusCode::NO_CONTENT }))
            .route("/api/secrets/{id}/key", put(|State(s): State<Central>, axum::Json(_): axum::Json<SecretRename>| async move { s.mutations.fetch_add(1, Ordering::Relaxed); axum::Json(secret_meta()) }))
            .with_state(central.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("owned management fixture");
        let client = CentralClient::new(
            &format!(
                "http://{}",
                listener.local_addr().expect("owned management fixture")
            ),
            true,
        )
        .expect("owned management fixture");
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("owned management fixture");
        });
        let sessions = SessionManager::new(
            StateStore::new(root.clone(), memory).expect("owned management fixture"),
            true,
        )
        .select_server(&client)
        .await
        .expect("owned management fixture");
        sessions
            .login(
                &client,
                LoginRequest {
                    email: "alice@example.test".into(),
                    password: "private-password".into(),
                },
            )
            .await
            .expect("owned management fixture");
        let time = Arc::new(Time {
            now: Mutex::new(ClockSample {
                boot_id: Uuid::new_v4(),
                wall_ns: now as u64 * NANOS_PER_SECOND,
                continuous_ns: 100 * NANOS_PER_SECOND,
            }),
            fail: AtomicBool::new(false),
        });
        let sync = SyncEngine::new(
            SnapshotCache::new(sessions.clone(), time.clone()),
            Arc::new(NativeTransport),
        );
        sync.refresh().await.expect("owned management fixture");
        let installs = InstallManager::new(ControlBridge::new(sync.clone()))
            .with_bounds(Duration::from_secs(2), Duration::from_millis(200))
            .expect("owned management fixture");
        Self {
            root,
            sessions,
            sync,
            installs,
            central,
            client,
            task,
            time,
        }
    }
    fn generation(&self) -> String {
        self.sessions
            .load()
            .expect("owned management fixture")
            .metadata
            .generation
            .to_string()
    }
    fn settings(&self, enabled: bool) -> SettingsUpdate {
        SettingsUpdate {
            expected_revision: self
                .central
                .snapshot
                .lock()
                .expect("owned management fixture")
                .revision,
            enabled,
            secret_id: None,
        }
    }
    async fn finish(&self, generation: &str, id: &str) -> InstallProgress {
        for _ in 0..150 {
            let progress = self
                .installs
                .progress(generation, id)
                .unwrap_or_else(|_| panic!("fixture progress"));
            if progress.state != InstallState::Running {
                return progress;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("owned install did not finish")
    }
}
fn secret_meta() -> SecretMetadata {
    SecretMetadata {
        id: "fixture-secret".into(),
        key: "API_KEY".into(),
        version: 2,
        updated_at: epoch().expect("owned management fixture"),
    }
}
fn secret_conflict(code: StatusCode) -> (StatusCode, axum::Json<ApiError>) {
    (
        code,
        axum::Json(ApiError {
            code: ErrorCode::Conflict,
            message: "private-server-diagnostic".into(),
            revision: Some(1),
            snapshot: None,
        }),
    )
}
async fn secret_write(
    State(s): State<Central>,
    axum::Json(input): axum::Json<SecretWrite>,
) -> axum::response::Response {
    s.mutations.fetch_add(1, Ordering::Relaxed);
    if s.mutation_code.load(Ordering::Relaxed) != 200 || input.expected_version == Some(1) {
        return secret_conflict(StatusCode::CONFLICT).into_response();
    }
    axum::Json(secret_meta()).into_response()
}
fn command(
    source: &str,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    cwd: Option<String>,
) -> CommandSpec {
    CommandSpec {
        program: "/usr/bin/python3".into(),
        args: vec!["-c".into(), source.into()]
            .into_iter()
            .chain(args)
            .collect(),
        env,
        cwd,
    }
}
fn success_command() -> CommandSpec {
    command("pass", vec![], BTreeMap::new(), None)
}
fn install_input(enable: bool) -> InstallRequest {
    InstallRequest {
        expected_revision: 1,
        enable_after_install: enable,
        secret_id: None,
        rerun: false,
    }
}

#[tokio::test]
async fn bridge_online_conflicts_durable_ack_and_redaction() {
    let f = Fixture::new(success_command()).await;
    let generation = f.generation();
    let bridge = &f.installs.bridge;
    f.central.config_code.store(503, Ordering::Relaxed);
    assert!(
        bridge
            .settings(&generation, "fixture-mcp", f.settings(true))
            .await
            .is_err()
    );
    assert!(bridge.secrets(&generation).await.is_err());
    assert!(
        bridge
            .secret(
                &generation,
                reqwest::Method::POST,
                None,
                false,
                SecretWrite {
                    key: "API_KEY".into(),
                    value: "private-vault-value".into(),
                    expected_version: None
                }
            )
            .await
            .is_err()
    );
    assert!(
        f.installs
            .start(&generation, "fixture-mcp", install_input(true))
            .await
            .is_err()
    );
    assert_eq!(f.central.mutations.load(Ordering::Relaxed), 0);
    assert!(
        !f.sync
            .cache
            .latest()
            .expect("owned management fixture")
            .snapshot
            .mcps[0]
            .enabled
    );
    f.central.config_code.store(200, Ordering::Relaxed);
    f.central.mutation_code.store(409, Ordering::Relaxed);
    let error = bridge
        .settings(&generation, "fixture-mcp", f.settings(true))
        .await
        .expect_err("central refusal");
    assert_eq!(error.status, StatusCode::CONFLICT);
    assert_eq!(error.body.revision, Some(1));
    assert!(
        !serde_json::to_string(&error.body)
            .expect("owned management fixture")
            .contains("private-")
    );
    assert_eq!(f.central.mutations.load(Ordering::Relaxed), 1);
    let error = bridge
        .secret(
            &generation,
            reqwest::Method::PUT,
            Some("fixture-secret"),
            false,
            SecretWrite {
                key: "API_KEY".into(),
                value: "private-vault-value".into(),
                expected_version: Some(1),
            },
        )
        .await
        .expect_err("central refusal");
    assert_eq!(error.status, StatusCode::CONFLICT);
    f.central.mutation_code.store(403, Ordering::Relaxed);
    assert_eq!(
        bridge
            .settings(&generation, "fixture-mcp", f.settings(true))
            .await
            .expect_err("central refusal")
            .status,
        StatusCode::FORBIDDEN
    );
    f.central.mutation_code.store(200, Ordering::Relaxed);
    let catalog = bridge
        .settings(&generation, "fixture-mcp", f.settings(true))
        .await
        .expect("owned management fixture");
    assert!(catalog.mcps[0].enabled);
    assert_eq!(
        f.sync
            .cache
            .latest()
            .expect("owned management fixture")
            .snapshot
            .revision,
        catalog.revision
    );
    assert!(
        !serde_json::to_string(&catalog)
            .expect("owned management fixture")
            .contains("python")
    );
    f.central.break_publication.store(true, Ordering::Relaxed);
    assert!(
        bridge
            .settings(&generation, "fixture-mcp", f.settings(false))
            .await
            .is_err()
    );
    assert!(f.sync.cache.latest().is_err());
}

#[tokio::test]
async fn pinned_owner_and_atomic_logout_preserve_concurrent_login() {
    let f = Fixture::new(success_command()).await;
    let generation = f.generation();
    f.central.delay.store(true, Ordering::Relaxed);
    let bridge = f.installs.bridge.clone();
    let old_generation = generation.clone();
    let input = f.settings(true);
    let write =
        tokio::spawn(async move { bridge.settings(&old_generation, "fixture-mcp", input).await });
    f.central.entered.notified().await;
    f.sessions
        .login(
            &f.client,
            LoginRequest {
                email: "bob@example.test".into(),
                password: "private-other-password".into(),
            },
        )
        .await
        .expect("owned management fixture");
    f.central.release.notify_one();
    assert!(write.await.expect("owned management fixture").is_err());
    assert_eq!(
        f.central
            .owners
            .lock()
            .expect("owned management fixture")
            .as_slice(),
        &["Bearer private-central-alice"]
    );
    assert!(matches!(
        f.sessions
            .logout_generation(Uuid::parse_str(&generation).expect("owned management fixture"))
            .await,
        Err(EndpointError::IdentityChanged)
    ));
    assert_eq!(
        f.sessions
            .load()
            .expect("owned management fixture")
            .credentials
            .user
            .email,
        "bob@example.test"
    );
    assert!(
        f.installs
            .bridge
            .settings(&generation, "fixture-mcp", f.settings(false))
            .await
            .is_err()
    );
    assert_eq!(f.central.mutations.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn vault_write_only_metadata_and_rejection() {
    let f = Fixture::new(success_command()).await;
    let generation = f.generation();
    let bridge = &f.installs.bridge;
    let metadata = bridge
        .secrets(&generation)
        .await
        .expect("owned management fixture");
    assert_eq!(metadata[0].id, "fixture-secret");
    let outcome = bridge
        .secret(
            &generation,
            reqwest::Method::POST,
            None,
            false,
            SecretWrite {
                key: "API_KEY".into(),
                value: "private-vault-value".repeat(800),
                expected_version: None,
            },
        )
        .await
        .expect("owned management fixture");
    assert!(outcome.confirmed && outcome.synchronized);
    assert!(
        !serde_json::to_string(&outcome)
            .expect("owned management fixture")
            .contains("private-")
    );
    assert!(
        bridge
            .secret(
                &generation,
                reqwest::Method::PUT,
                Some("fixture-secret"),
                true,
                SecretRename {
                    key: "RENAMED".into(),
                    expected_version: 2
                }
            )
            .await
            .expect("owned management fixture")
            .confirmed
    );
    assert!(
        bridge
            .secret(
                &generation,
                reqwest::Method::DELETE,
                Some("fixture-secret"),
                false,
                VersionInput {
                    expected_version: 2
                }
            )
            .await
            .expect("owned management fixture")
            .metadata
            .is_none()
    );
    f.central.config_code.store(401, Ordering::Relaxed);
    assert!(
        bridge
            .settings(&generation, "fixture-mcp", f.settings(true))
            .await
            .is_err()
    );
    assert!(matches!(
        f.sessions.load(),
        Err(EndpointError::LoginRequired)
    ));
    assert!(f.sync.cache.latest().is_err());
}

#[tokio::test]
async fn protected_routes_bind_generation_host_origin_csrf_and_bound_values() {
    let f = Fixture::new(success_command()).await;
    let generation = f.generation();
    let app = router(
        ManagementState::new(
            f.sessions.clone(),
            f.client.clone(),
            "127.0.0.1:18239".parse().expect("owned management fixture"),
            None,
        )
        .expect("owned management fixture")
        .with_sync(f.sync.clone())
        .expect("owned management fixture"),
    );
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/")
                .header("host", "127.0.0.1:18239")
                .body(axum::body::Body::empty())
                .expect("owned management fixture"),
        )
        .await
        .expect("owned management fixture");
    let cookie = response
        .headers()
        .get("set-cookie")
        .expect("owned management fixture")
        .to_str()
        .expect("owned management fixture")
        .split(';')
        .next()
        .expect("owned management fixture")
        .to_string();
    let page = axum::body::to_bytes(response.into_body(), 8192)
        .await
        .expect("page");
    assert!(String::from_utf8_lossy(&page).contains("data-gateway-mode=\"local\""));
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/context")
                .header("host", "127.0.0.1:18239")
                .header("cookie", &cookie)
                .body(axum::body::Body::empty())
                .expect("owned management fixture"),
        )
        .await
        .expect("owned management fixture");
    let context: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 8192)
            .await
            .expect("owned management fixture"),
    )
    .expect("owned management fixture");
    assert_eq!(context["status"]["identity"]["generation"], generation);
    assert_eq!(context["mode"], "local");
    assert_eq!(context["status"]["identity"]["email"], "alice@example.test");
    let csrf = context["csrfToken"]
        .as_str()
        .expect("owned management fixture");
    for (expected_generation, expected_status) in
        [(Uuid::new_v4().to_string(), 409), (generation.clone(), 200)]
    {
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/refresh")
                    .header("host", "127.0.0.1:18239")
                    .header("origin", "http://127.0.0.1:18239")
                    .header("cookie", &cookie)
                    .header("x-csrf-token", csrf)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({"expectedGeneration":expected_generation,"input":null})
                            .to_string(),
                    ))
                    .expect("refresh request"),
            )
            .await
            .expect("refresh response");
        assert_eq!(response.status().as_u16(), expected_status);
    }
    let body = serde_json::to_string(&LocalAction {
        expected_generation: generation.clone(),
        input: SecretWrite {
            key: "API_KEY".into(),
            value: "private-vault-value".repeat(800),
            expected_version: None,
        },
    })
    .expect("owned management fixture");
    for (host, origin, token, site, expected) in [
        (
            "evil:18239",
            "http://127.0.0.1:18239",
            csrf,
            "same-origin",
            403,
        ),
        (
            "127.0.0.1:18239",
            "https://evil.example",
            csrf,
            "cross-site",
            403,
        ),
        (
            "127.0.0.1:18239",
            "http://127.0.0.1:18239",
            "wrong",
            "same-origin",
            403,
        ),
        (
            "127.0.0.1:18239",
            "http://127.0.0.1:18239",
            csrf,
            "same-origin",
            200,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/secrets")
                    .header("host", host)
                    .header("origin", origin)
                    .header("x-csrf-token", token)
                    .header("sec-fetch-site", site)
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(body.clone()))
                    .expect("owned management fixture"),
            )
            .await
            .expect("owned management fixture");
        assert_eq!(response.status().as_u16(), expected);
        let text = String::from_utf8(
            axum::body::to_bytes(response.into_body(), 8192)
                .await
                .expect("owned management fixture")
                .to_vec(),
        )
        .expect("owned management fixture");
        assert!(!text.contains("private-"));
    }
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/catalog")
                .header("host", "127.0.0.1:18239")
                .header("cookie", cookie)
                .header("x-endpoint-generation", Uuid::new_v4().to_string())
                .body(axum::body::Body::empty())
                .expect("owned management fixture"),
        )
        .await
        .expect("owned management fixture");
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[cfg(target_os = "macos")]
struct OwnedFiles(PathBuf);
#[cfg(target_os = "macos")]
#[tokio::test]
async fn catalog_diagnostics_never_execute_commands_or_claim_enablement() {
    let files = OwnedFiles::new();
    let marker = files.path("catalog-must-not-execute");
    let command = CommandSpec {
        program: "/usr/bin/touch".into(),
        args: vec![marker.clone()],
        env: BTreeMap::new(),
        cwd: None,
    };
    let f = Fixture::new(command).await;
    let catalog = f.installs.catalog(&f.generation()).expect("catalog");
    assert!(!std::path::Path::new(&marker).exists());
    assert!(!catalog.mcps[0].enabled && !catalog.mcps[0].installed);
    {
        let mut snapshot = f.central.snapshot.lock().expect("fixture config");
        snapshot.revision += 1;
        snapshot.mcps[0].definition.transport = Transport::Stdio {
            command: CommandSpec {
                program: files.path("missing-executable"),
                args: vec![],
                env: BTreeMap::new(),
                cwd: None,
            },
        };
    }
    f.sync.refresh().await.expect("new metadata");
    let catalog = f
        .installs
        .catalog(&f.generation())
        .expect("redacted diagnostic");
    assert_eq!(catalog.mcps[0].availability, "missing_executable");
    assert!(!catalog.mcps[0].enabled);
    assert!(
        !serde_json::to_string(&catalog)
            .expect("public metadata")
            .contains("missing-executable")
    );
    assert_eq!(f.central.mutations.load(Ordering::Relaxed), 0);
}
#[cfg(target_os = "macos")]
impl OwnedFiles {
    fn new() -> Self {
        use std::os::unix::fs::DirBuilderExt;
        let path = std::env::temp_dir()
            .canonicalize()
            .expect("owned management fixture")
            .join(format!("gateway-process-owned-{}", Uuid::new_v4()));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .expect("owned management fixture");
        Self(path)
    }
    fn path(&self, name: &str) -> String {
        self.0
            .join(name)
            .to_str()
            .expect("owned management fixture")
            .into()
    }
}
#[cfg(target_os = "macos")]
impl Drop for OwnedFiles {
    fn drop(&mut self) {
        let journal = self.0.join("native-state/identity.json");
        if journal.exists()
            && std::fs::read(&journal)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                .is_none_or(|state| state["fixtureRetired"] != true)
        {
            eprintln!(
                "Owned native fixture cleanup journal retained: {}",
                self.0.display()
            );
            return;
        }
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[cfg(target_os = "macos")]
async fn read_pids(path: &str) -> Vec<i32> {
    for _ in 0..100 {
        if let Ok(bytes) = std::fs::read(path)
            && let Ok(pids) = serde_json::from_slice(&bytes)
        {
            return pids;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("owned pid fixture did not start")
}
#[cfg(target_os = "macos")]
async fn assert_gone(pids: &[i32]) {
    for _ in 0..100 {
        if pids.iter().all(|pid| {
            nix::sys::signal::kill(nix::unistd::Pid::from_raw(*pid), None)
                == Err(nix::errno::Errno::ESRCH)
        }) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("owned fixture process still exists")
}
#[cfg(target_os = "macos")]
fn tree_command(path: String, exit: Option<i32>) -> CommandSpec {
    let end = exit.map_or("time.sleep(60)".into(), |code| format!("sys.exit({code})"));
    command(
        &format!(
            "import os,sys,time,signal,subprocess,json\nsignal.signal(signal.SIGTERM,signal.SIG_IGN)\nchild=subprocess.Popen(['/usr/bin/python3','-c','import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); time.sleep(60)'])\nwith open(sys.argv[1],'w') as f: json.dump([os.getpid(),child.pid],f)\nprint('private-child-stdout')\nprint('private-child-stderr',file=sys.stderr)\n{end}"
        ),
        vec![path],
        BTreeMap::new(),
        None,
    )
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn actual_mac_unreaped_group_success_failure_drop_and_abort_cleanup() {
    use dhivon_mcp_gateway_cli::owned_process::OwnedProcess;
    let files = OwnedFiles::new();
    for (name, code) in [("success", 0), ("failure", 7)] {
        let path = files.path(name);
        let process = OwnedProcess::spawn(&tree_command(path.clone(), Some(code)))
            .expect("owned management fixture");
        let id = process.id();
        let pids = read_pids(&path).await;
        assert_eq!(id, pids[0] as u32);
        for _ in 0..100 {
            if process
                .poll_exit()
                .expect("owned management fixture")
                .is_some()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert_eq!(
            process.poll_exit().expect("owned management fixture"),
            Some(Some(code))
        );
        assert!(
            nix::sys::signal::kill(nix::unistd::Pid::from_raw(pids[0]), None).is_ok(),
            "leader remains reserved until cleanup"
        );
        assert_eq!(
            process
                .terminate(Duration::from_millis(100))
                .await
                .expect("owned management fixture")
                .code(),
            Some(code)
        );
        assert_gone(&pids).await;
    }
    let path = files.path("drop");
    let process =
        OwnedProcess::spawn(&tree_command(path.clone(), None)).expect("owned management fixture");
    let pids = read_pids(&path).await;
    drop(process);
    assert_gone(&pids).await;
    let path = files.path("abort");
    let owned_path = path.clone();
    let task = tokio::spawn(async move {
        let _process =
            OwnedProcess::spawn(&tree_command(owned_path, None)).expect("owned management fixture");
        std::future::pending::<()>().await;
    });
    let pids = read_pids(&path).await;
    task.abort();
    let _ = task.await;
    assert_gone(&pids).await;
    let missing = CommandSpec {
        program: files.path("nonexistent-private-program"),
        args: vec![],
        env: BTreeMap::new(),
        cwd: None,
    };
    assert!(matches!(
        OwnedProcess::spawn(&missing),
        Err(EndpointError::MissingExecutable)
    ));
    let denied = files.path("not-executable");
    std::fs::write(&denied, "private-script").expect("owned management fixture");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o600))
        .expect("owned management fixture");
    assert!(matches!(
        OwnedProcess::spawn(&CommandSpec {
            program: denied,
            ..missing
        }),
        Err(EndpointError::ExecutionDenied)
    ));
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn actual_mac_literal_arguments_env_cwd_installed_disabled_and_enable_retry() {
    let files = OwnedFiles::new();
    let marker = files.path("marker");
    let command = command(
        "import sys,os,json\nwith open(sys.argv[1],'a') as f: f.write(json.dumps([sys.argv[2:],os.environ['OWNED_PRIVATE_ENV'],os.getcwd()])+'\\n')\nprint('private-install-output')",
        vec![
            marker.clone(),
            "$(touch NEVER_EXECUTE)".into(),
            "argument with spaces".into(),
        ],
        BTreeMap::from([("OWNED_PRIVATE_ENV".into(), "private-env-value".into())]),
        Some(files.0.to_str().expect("owned management fixture").into()),
    );
    let f = Fixture::new(command).await;
    let generation = f.generation();
    f.central.mutation_code.store(503, Ordering::Relaxed);
    let start = f
        .installs
        .start(&generation, "fixture-mcp", install_input(true))
        .await
        .expect("owned management fixture");
    let finished = f.finish(&generation, &start.id).await;
    assert_eq!(
        finished.state,
        InstallState::InstalledDisabled,
        "{finished:?}"
    );
    assert!(
        !f.sync
            .cache
            .latest()
            .expect("owned management fixture")
            .snapshot
            .mcps[0]
            .enabled
    );
    let records = std::fs::read_to_string(&marker).expect("owned management fixture");
    assert_eq!(records.lines().count(), 1);
    assert!(
        records.contains("$(touch NEVER_EXECUTE)")
            && records.contains("argument with spaces")
            && records.contains("private-env-value")
    );
    assert!(!files.0.join("NEVER_EXECUTE").exists());
    let restarted = InstallManager::new(f.installs.bridge.clone());
    let retry = restarted
        .start(&generation, "fixture-mcp", install_input(true))
        .await
        .expect("owned management fixture");
    assert_eq!(retry.id, start.id);
    assert_eq!(
        std::fs::read_to_string(&marker)
            .expect("owned management fixture")
            .lines()
            .count(),
        1
    );
    f.central.mutation_code.store(200, Ordering::Relaxed);
    let catalog = restarted
        .enable(&generation, &start.id, f.settings(true))
        .await
        .expect("owned management fixture");
    assert!(catalog.mcps[0].enabled);
    assert_eq!(
        std::fs::read_to_string(&marker)
            .expect("owned management fixture")
            .lines()
            .count(),
        1
    );
    assert_eq!(
        restarted
            .progress(&generation, &start.id)
            .expect("owned management fixture")
            .state,
        InstallState::Enabled
    );
    let disk = std::fs::read_to_string(f.root.join("installations.json"))
        .expect("owned management fixture");
    let public = serde_json::to_string(&(
        finished,
        f.installs
            .catalog(&generation)
            .expect("owned management fixture"),
        f.sync.status().expect("owned management fixture"),
    ))
    .expect("owned management fixture");
    for secret in [
        "private-env-value",
        "private-install-output",
        "$(touch",
        "/usr/bin/python3",
        "private-central",
        "private-password",
    ] {
        assert!(!disk.contains(secret));
        assert!(!public.contains(secret));
    }
    {
        let mut snapshot = f.central.snapshot.lock().expect("owned management fixture");
        snapshot.revision += 1;
        snapshot.mcps[0]
            .definition
            .install
            .as_mut()
            .expect("owned management fixture")
            .args
            .push("changed".into());
    }
    f.sync.refresh().await.expect("owned management fixture");
    assert!(
        !f.installs
            .catalog(&generation)
            .expect("owned management fixture")
            .mcps[0]
            .installed
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn actual_mac_cancel_expiry_revocation_clock_failure_and_timeout_remain_disabled() {
    for kind in [
        "cancel",
        "expiry",
        "revocation",
        "clock",
        "timeout",
        "shutdown",
    ] {
        let files = OwnedFiles::new();
        let path = files.path(kind);
        let f = Fixture::new(tree_command(path.clone(), None)).await;
        let generation = f.generation();
        let start = f
            .installs
            .start(&generation, "fixture-mcp", install_input(true))
            .await
            .expect("owned management fixture");
        let pids = read_pids(&path).await;
        match kind {
            "cancel" => f
                .installs
                .cancel(&generation, &start.id)
                .expect("owned management fixture"),
            "expiry" => f.time.advance(3600),
            "revocation" => {
                {
                    let mut snapshot = f.central.snapshot.lock().expect("owned management fixture");
                    snapshot.revision += 1;
                    snapshot.mcps.clear();
                }
                f.sync.refresh().await.expect("owned management fixture");
            }
            "clock" => f.time.fail.store(true, Ordering::Relaxed),
            "shutdown" => f.installs.shutdown().await,
            "timeout" => f.time.advance(2),
            _ => (),
        }
        let progress = f.finish(&generation, &start.id).await;
        assert_gone(&pids).await;
        assert_eq!(f.central.mutations.load(Ordering::Relaxed), 0);
        assert_eq!(
            progress.state,
            match kind {
                "cancel" | "shutdown" => InstallState::Cancelled,
                "revocation" => InstallState::Revoked,
                _ => InstallState::Expired,
            }
        );
        assert!(
            !f.central
                .snapshot
                .lock()
                .expect("owned management fixture")
                .mcps
                .iter()
                .any(|m| m.enabled)
        );
    }
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn concurrent_client_enablement_is_not_overruled_by_install_receipt() {
    let files = OwnedFiles::new();
    let marker = files.path("concurrent-client");
    let f = Fixture::new(tree_command(marker.clone(), None)).await;
    let generation = f.generation();
    let started = f
        .installs
        .start(&generation, "fixture-mcp", install_input(true))
        .await
        .expect("explicit install");
    let pids = read_pids(&marker).await;
    // A second management client confirms enablement while the installer is running.
    let another_client = ControlBridge::new(f.sync.clone());
    let confirmed = another_client
        .settings(&generation, "fixture-mcp", f.settings(true))
        .await
        .expect("concurrent central enable");
    assert!(confirmed.mcps[0].enabled);
    let progress = f.finish(&generation, &started.id).await;
    assert_gone(&pids).await;
    assert_eq!(progress.state, InstallState::Revoked);
    assert!(
        progress
            .message
            .contains("no automatic enablement was attempted")
    );
    assert!(!progress.message.contains("remains disabled"));
    assert_eq!(f.central.mutations.load(Ordering::Relaxed), 1);
    assert!(
        f.installs
            .catalog(&generation)
            .expect("current catalog")
            .mcps[0]
            .enabled
    );
    let restarted = InstallManager::new(f.installs.bridge.clone());
    assert_eq!(
        restarted
            .progress(&generation, &started.id)
            .expect("durable receipt")
            .message,
        progress.message
    );
    assert!(
        restarted
            .catalog(&generation)
            .expect("current catalog")
            .mcps[0]
            .enabled
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn actual_mac_failed_install_and_missing_program_have_fixed_actionable_progress() {
    for (command, expected_message) in [
        (
            command("import sys; sys.exit(9)", vec![], BTreeMap::new(), None),
            "Installation failed",
        ),
        (
            CommandSpec {
                program: "/nonexistent-private-program".into(),
                args: vec![],
                env: BTreeMap::new(),
                cwd: None,
            },
            "executable or working directory is missing",
        ),
    ] {
        let f = Fixture::new(command).await;
        let generation = f.generation();
        let started = f
            .installs
            .start(&generation, "fixture-mcp", install_input(true))
            .await
            .expect("owned management fixture");
        let progress = f.finish(&generation, &started.id).await;
        assert_eq!(progress.state, InstallState::Failed);
        assert!(progress.message.contains(expected_message));
        assert!(!progress.message.contains("private-program"));
        assert_eq!(f.central.mutations.load(Ordering::Relaxed), 0);
        let previous = f
            .installs
            .start(&generation, "fixture-mcp", install_input(true))
            .await
            .expect("owned management fixture");
        assert_eq!(previous.id, started.id);
        let rerun = f
            .installs
            .start(
                &generation,
                "fixture-mcp",
                InstallRequest {
                    rerun: true,
                    ..install_input(false)
                },
            )
            .await
            .expect("owned management fixture");
        assert_ne!(rerun.id, started.id);
        f.finish(&generation, &rerun.id).await;
    }
}

#[tokio::test]
async fn every_central_mutation_refuses_offline_without_queue_or_side_effects() {
    let f = Fixture::new(success_command()).await;
    let generation = f.generation();
    let bridge = &f.installs.bridge;
    f.central.config_code.store(503, Ordering::Relaxed);
    for enabled in [false, true] {
        assert!(
            bridge
                .settings(&generation, "fixture-mcp", f.settings(enabled))
                .await
                .is_err()
        );
    }
    for (method, id) in [
        (reqwest::Method::POST, None),
        (reqwest::Method::PUT, Some("fixture-secret")),
    ] {
        assert!(
            bridge
                .secret(
                    &generation,
                    method,
                    id,
                    false,
                    SecretWrite {
                        key: "API_KEY".into(),
                        value: "private-vault-value".into(),
                        expected_version: Some(2)
                    }
                )
                .await
                .is_err()
        );
    }
    assert!(
        bridge
            .secret(
                &generation,
                reqwest::Method::PUT,
                Some("fixture-secret"),
                true,
                SecretRename {
                    key: "RENAMED".into(),
                    expected_version: 2
                }
            )
            .await
            .is_err()
    );
    assert!(
        bridge
            .secret(
                &generation,
                reqwest::Method::DELETE,
                Some("fixture-secret"),
                false,
                VersionInput {
                    expected_version: 2
                }
            )
            .await
            .is_err()
    );
    assert!(
        f.installs
            .start(&generation, "fixture-mcp", install_input(false))
            .await
            .is_err()
    );
    assert_eq!(f.central.mutations.load(Ordering::Relaxed), 0);
    f.central.config_code.store(200, Ordering::Relaxed);
    f.sync.refresh().await.expect("owned management fixture");
    assert_eq!(f.central.mutations.load(Ordering::Relaxed), 0);
    assert!(!f.root.join("installations.json").exists());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn stale_generation_shutdown_cannot_cancel_new_owner_install_and_process_lock_prevents_queue()
{
    let files = OwnedFiles::new();
    let path = files.path("generation");
    let f = Fixture::new(tree_command(path.clone(), None)).await;
    let stale_generation = f.generation();
    f.sessions
        .login(
            &f.client,
            LoginRequest {
                email: "bob@example.test".into(),
                password: "private-other-password".into(),
            },
        )
        .await
        .expect("owned management fixture");
    let generation = f.generation();
    let started = f
        .installs
        .start(&generation, "fixture-mcp", install_input(false))
        .await
        .expect("owned management fixture");
    let pids = read_pids(&path).await;
    f.installs.shutdown_generation(&stale_generation).await;
    tokio::time::sleep(Duration::from_millis(125)).await;
    assert_eq!(
        f.installs
            .progress(&generation, &started.id)
            .expect("owned management fixture")
            .state,
        InstallState::Running
    );
    let another_ui = InstallManager::new(f.installs.bridge.clone());
    assert!(
        another_ui
            .start(&generation, "fixture-mcp", install_input(false))
            .await
            .is_err()
    );
    assert_eq!(
        another_ui
            .progress(&generation, &started.id)
            .expect("owned management fixture")
            .state,
        InstallState::Running
    );
    f.installs.shutdown_generation(&generation).await;
    assert_gone(&pids).await;
    assert_eq!(
        f.finish(&generation, &started.id).await.state,
        InstallState::Cancelled
    );
}

#[derive(Clone, Default)]
struct Logs(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for Logs {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("captured log fixture")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Logs {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}
#[cfg(target_os = "macos")]
#[tokio::test]
async fn actual_management_and_install_events_are_correlated_and_redacted() {
    let logs = Logs::default();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(logs.clone())
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("one integration log capture");
    let f = Fixture::new(command(
        "print('private-process-output')",
        vec![],
        BTreeMap::from([("PRIVATE_ENV".into(), "private-process-env".into())]),
        None,
    ))
    .await;
    let generation = f.generation();
    f.installs
        .bridge
        .secret(
            &generation,
            reqwest::Method::POST,
            None,
            false,
            SecretWrite {
                key: "API_KEY".into(),
                value: "private-vault-value".into(),
                expected_version: None,
            },
        )
        .await
        .expect("vault write");
    let started = f
        .installs
        .start(&generation, "fixture-mcp", install_input(true))
        .await
        .expect("install start");
    f.finish(&generation, &started.id).await;
    let text =
        String::from_utf8(logs.0.lock().expect("captured logs").clone()).expect("UTF8 events");
    for field in [
        "endpoint_management_outcome",
        "endpoint_install_started",
        "endpoint_install_outcome",
        "correlation_id",
        "duration_ms",
        "duration_seconds",
    ] {
        assert!(text.contains(field), "missing {field}");
    }
    for private in [
        "private-process-output",
        "private-process-env",
        "private-vault-value",
        "private-central-alice",
        "private-password",
        "/usr/bin/python3",
        "private-server-diagnostic",
    ] {
        assert!(!text.contains(private), "sensitive event content");
    }
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn actual_cli_sigterm_and_ctrl_c_clean_owned_installer_without_production_state() {
    use dhivon_mcp_gateway_cli::owned_process::OwnedProcess;
    for signal in [
        nix::sys::signal::Signal::SIGTERM,
        nix::sys::signal::Signal::SIGINT,
    ] {
        let files = OwnedFiles::new();
        let path = files.path("cli-pids");
        let f = Fixture::new(tree_command(path.clone(), None)).await;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("owned local port");
        let port = listener.local_addr().expect("port").port();
        drop(listener);
        let state_path = files.0.join("native-state");
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&state_path)
            .expect("private fixture state directory");
        let spec = CommandSpec {
            program: env!("CARGO_BIN_EXE_dhivon-mcp-gateway").into(),
            args: vec![
                "ui".into(),
                "--server".into(),
                f.client.origin(),
                "--allow-fixture-http".into(),
                "--no-open".into(),
                "--port".into(),
                port.to_string(),
                "--fixture-state-path".into(),
                state_path.to_str().expect("state path").into(),
            ],
            env: BTreeMap::new(),
            cwd: None,
        };
        let process = OwnedProcess::spawn(&spec).expect("actual isolated UI CLI");
        let base = format!("http://127.0.0.1:{port}");
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("local HTTP");
        let mut page = None;
        for _ in 0..150 {
            if let Ok(response) = client.get(format!("{base}/")).send().await {
                page = Some(response);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let page = page.expect("actual UI startup");
        let cookie = page.headers()["set-cookie"]
            .to_str()
            .expect("cookie")
            .split(';')
            .next()
            .expect("cookie pair")
            .to_owned();
        let context: serde_json::Value = client
            .get(format!("{base}/api/context"))
            .header("cookie", &cookie)
            .send()
            .await
            .expect("context")
            .json()
            .await
            .expect("context JSON");
        let csrf = context["csrfToken"].as_str().expect("CSRF");
        let logged: serde_json::Value = client
            .post(format!("{base}/api/auth/login"))
            .header("cookie", &cookie)
            .header("origin", &base)
            .header("x-csrf-token", csrf)
            .json(&LoginRequest {
                email: "alice@example.test".into(),
                password: "private-cli-password".into(),
            })
            .send()
            .await
            .expect("native actual Keychain login")
            .json()
            .await
            .expect("login status");
        let generation = logged["identity"]["generation"]
            .as_str()
            .expect("native generation");
        let response = client
            .post(format!("{base}/api/mcps/fixture-mcp/install"))
            .header("cookie", &cookie)
            .header("origin", &base)
            .header("x-csrf-token", csrf)
            .json(&LocalAction {
                expected_generation: generation.to_owned(),
                input: install_input(true),
            })
            .send()
            .await
            .expect("actual install route");
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let pids = read_pids(&path).await;
        let began = std::time::Instant::now();
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(process.id() as i32), signal)
            .expect("signal only owned CLI leader");
        for _ in 0..350 {
            if process.poll_exit().expect("owned UI exit state").is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let observed = process.poll_exit().expect("owned UI exited");
        let status = process
            .terminate(Duration::from_secs(1))
            .await
            .expect("owned CLI final reap");
        let shutdown_elapsed = began.elapsed();
        let cleanup = std::process::Command::new(env!("CARGO_BIN_EXE_dhivon-mcp-gateway"))
            .args(["logout", "--allow-fixture-http", "--fixture-state-path"])
            .arg(&state_path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("creating binary owned Keychain cleanup");
        assert!(
            cleanup.success(),
            "creating binary cleanup failed; journal retained"
        );
        assert!(
            std::process::Command::new(&spec.program)
                .args(["fixture-retire", "--fixture-state-path"])
                .arg(&state_path)
                .status()
                .expect("verify and retire fixture master")
                .success()
        );
        assert_eq!(observed, Some(Some(0)));
        assert!(status.success());
        assert!(
            shutdown_elapsed < Duration::from_secs(6),
            "owned CLI shutdown duration ms={}",
            shutdown_elapsed.as_millis()
        );
        assert_gone(&pids).await;
        assert_eq!(f.central.mutations.load(Ordering::Relaxed), 0);
    }
}
