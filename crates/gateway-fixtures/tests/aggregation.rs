use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use gateway_cli::{
    aggregation::{AggregateError, Aggregator, alias},
    cache::SnapshotCache,
    clock::{Clock, ClockSample, NANOS_PER_SECOND},
    credentials::CredentialStore,
    epoch,
    session::SessionManager,
    store::{Identity, StateStore},
};
use gateway_core::*;
use gateway_fixtures::{FixtureState, http_router};
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicU16, AtomicU32, AtomicUsize, Ordering},
    },
    time::Duration,
};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Default)]
struct Memory(Mutex<HashMap<String, Vec<u8>>>);
impl CredentialStore for Memory {
    fn get(&self, key: &str) -> gateway_cli::Result<Option<Zeroizing<Vec<u8>>>> {
        Ok(self
            .0
            .lock()
            .expect("fixture credentials")
            .get(key)
            .cloned()
            .map(Zeroizing::new))
    }
    fn set(&self, key: &str, value: &[u8]) -> gateway_cli::Result<()> {
        self.0
            .lock()
            .expect("fixture credentials")
            .insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, key: &str) -> gateway_cli::Result<()> {
        self.0.lock().expect("fixture credentials").remove(key);
        Ok(())
    }
}
struct Time(Mutex<ClockSample>);
impl Clock for Time {
    fn sample(&self) -> gateway_cli::Result<ClockSample> {
        Ok(*self.0.lock().expect("fixture time"))
    }
}
impl Time {
    fn advance(&self, seconds: u64) {
        let mut now = self.0.lock().expect("fixture time");
        now.wall_ns += seconds * NANOS_PER_SECOND;
        now.continuous_ns += seconds * NANOS_PER_SECOND;
    }
}
struct Endpoint {
    root: PathBuf,
    cache: SnapshotCache,
    time: Arc<Time>,
    snapshot: ConfigSnapshot,
    inventory: Arc<Mutex<Vec<InventoryReport>>>,
    control: tokio::task::JoinHandle<()>,
    credential: Arc<Mutex<Option<UpstreamCredential>>>,
    credential_code: Arc<AtomicU16>,
    credential_revision: Arc<AtomicU32>,
}
impl Endpoint {
    async fn new(mcps: Vec<EffectiveMcp>) -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .expect("fixture parent")
            .join(format!("gateway-aggregation-owned-{}", Uuid::new_v4()));
        let inventory = Arc::new(Mutex::new(Vec::new()));
        let collected = inventory.clone();
        let credential = Arc::new(Mutex::new(None::<UpstreamCredential>));
        let credential_code = Arc::new(AtomicU16::new(200));
        let credential_revision = Arc::new(AtomicU32::new(1));
        let served_credential = credential.clone();
        let served_code = credential_code.clone();
        let served_revision = credential_revision.clone();
        let app = Router::new()
            .route(
                "/api/endpoint/inventory",
                post(
                    move |headers: HeaderMap, axum::Json(input): axum::Json<InventoryReport>| {
                        let collected = collected.clone();
                        async move {
                            assert_eq!(headers["authorization"], "Bearer private-central-fixture");
                            collected.lock().expect("inventory fixture").push(input);
                            StatusCode::NO_CONTENT
                        }
                    },
                ),
            )
            .route(
                "/api/endpoint/credentials/{id}",
                post(
                    move |headers: HeaderMap, axum::Json(input): axum::Json<CredentialRequest>| {
                        let credential = served_credential.clone();
                        let code = served_code.clone();
                        let revision = served_revision.clone();
                        async move {
                            assert_eq!(headers["authorization"], "Bearer private-central-fixture");
                            assert!(Uuid::parse_str(&input.installation_id).is_ok());
                            let code = StatusCode::from_u16(code.load(Ordering::Relaxed))
                                .expect("fixture code");
                            if code != StatusCode::OK {
                                return code.into_response();
                            }
                            if input.expected_revision != revision.load(Ordering::Relaxed) {
                                return StatusCode::CONFLICT.into_response();
                            }
                            match credential.lock().expect("fixture credential").as_ref() {
                                Some(credential) => axum::Json(credential.clone()).into_response(),
                                None => StatusCode::PRECONDITION_REQUIRED.into_response(),
                            }
                        }
                    },
                ),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("control listener");
        let origin = format!("http://{}", listener.local_addr().expect("control address"));
        let control = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("owned control fixture");
        });
        let store =
            StateStore::new(root.clone(), Arc::new(Memory::default())).expect("fixture state");
        let now = epoch().expect("fixture epoch");
        let user = UserInfo {
            id: Uuid::new_v4().to_string(),
            username: "fixture".into(),
            active: true,
            role: Role::User,
        };
        let session = EndpointSession {
            token: "private-central-fixture".into(),
            user: user.clone(),
            deployment_id: Uuid::new_v4().to_string(),
            expires_at: now + 43200,
        };
        store
            .locked(|m| {
                m.selected_server_origin = Some(origin.clone());
                m.active = Some(Identity {
                    server_origin: origin,
                    deployment_id: session.deployment_id.clone(),
                    user_id: user.id.clone(),
                });
                let account = StateStore::account(
                    m.active.as_ref().expect("fixture identity"),
                    m.generation,
                    "session",
                )?;
                m.credential_accounts.push(account.clone());
                store.write(m)?;
                store.credentials().set(
                    &account,
                    &serde_json::to_vec(&session).expect("fixture token encoding"),
                )
            })
            .expect("fixture session");
        let time = Arc::new(Time(Mutex::new(ClockSample {
            boot_id: Uuid::new_v4(),
            continuous_ns: 100 * NANOS_PER_SECOND,
            wall_ns: now as u64 * NANOS_PER_SECOND,
        })));
        let cache = SnapshotCache::new(SessionManager::new(store, true), time.clone());
        let snapshot = ConfigSnapshot {
            schema_version: 1,
            deployment_id: session.deployment_id,
            user,
            revision: 1,
            validated_at: now,
            expires_at: now + 3600,
            session_expires_at: session.expires_at,
            mcps,
        };
        cache
            .install(
                &cache.sessions.load().expect("fixture native session"),
                snapshot.clone(),
                time.sample().expect("fixture clock"),
            )
            .expect("fixture cache");
        Self {
            root,
            cache,
            time,
            snapshot,
            inventory,
            control,
            credential,
            credential_code,
            credential_revision,
        }
    }
    fn change(&mut self, update: impl FnOnce(&mut ConfigSnapshot)) {
        update(&mut self.snapshot);
        self.snapshot.revision += 1;
        self.credential_revision
            .store(self.snapshot.revision, Ordering::Relaxed);
        let now = self.time.sample().expect("fixture clock");
        self.snapshot.validated_at = now.wall_seconds().expect("wall time");
        self.snapshot.expires_at = self.snapshot.validated_at + 3600;
        self.cache
            .install(
                &self.cache.sessions.load().expect("fixture session"),
                self.snapshot.clone(),
                now,
            )
            .expect("new fixture policy");
    }
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        self.control.abort();
        let _ = self
            .cache
            .sessions
            .store
            .locked(|m| self.cache.sessions.store.clear_locked(m));
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn mcp(transport: Transport, policy: ToolPolicy) -> EffectiveMcp {
    EffectiveMcp {
        definition: McpDefinition {
            id: Uuid::new_v4().to_string(),
            name: "Fixture".into(),
            description: "Fixture metadata".into(),
            active: true,
            transport,
            auth: AuthMode::None {},
            install: None,
            tool_policy: policy,
        },
        enabled: true,
        secret: None,
    }
}
fn params(name: String) -> CallToolRequestParams {
    CallToolRequestParams::new(name).with_arguments(
        json!({"message":"hello"})
            .as_object()
            .cloned()
            .expect("literal arguments"),
    )
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn real_sdk_stdio_and_http_preserve_schemas_results_namespaces_policy_and_inventory() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("HTTP fixture");
    let address = listener.local_addr().expect("address");
    let upstream = tokio::spawn(async move {
        axum::serve(
            listener,
            http_router(
                FixtureState::new(format!("http://{address}"), "none".into()),
                address.to_string(),
            ),
        )
        .await
        .expect("owned HTTP MCP");
    });
    let command = CommandSpec {
        program: env!("CARGO_BIN_EXE_gateway-fixture").into(),
        args: vec!["--tag".into(), "local".into()],
        env: BTreeMap::new(),
        cwd: None,
    };
    let mut endpoint = Endpoint::new(vec![
        mcp(
            Transport::Stdio { command },
            ToolPolicy::Allowlist(vec!["echo".into()]),
        ),
        mcp(
            Transport::Http {
                url: format!("http://{address}/mcp"),
            },
            ToolPolicy::Denylist(vec!["fail".into()]),
        ),
    ])
    .await;
    let local = endpoint.snapshot.mcps[0].definition.id.clone();
    let remote = endpoint.snapshot.mcps[1].definition.id.clone();
    let log = endpoint.root.join("local-calls");
    let install_marker = endpoint.root.join("never-install");
    endpoint.change(|snapshot| {
        if let Transport::Stdio { command } = &mut snapshot.mcps[0].definition.transport {
            command.args.extend([
                "--call-log".into(),
                log.to_str().expect("fixture path").into(),
            ]);
        }
        snapshot.mcps[0].definition.install = Some(CommandSpec {
            program: "/usr/bin/touch".into(),
            args: vec![install_marker.to_str().expect("fixture marker").into()],
            env: BTreeMap::new(),
            cwd: None,
        });
    });
    let gateway = Aggregator::new(endpoint.cache.clone(), true).expect("aggregator");
    gateway.reconcile().await.expect("discovery");
    let tools = gateway.list().expect("permitted tools");
    assert_eq!(tools.len(), 2);
    assert_ne!(alias(&local, "echo"), alias(&remote, "echo"));
    for tool in &tools {
        assert!(tool.name.len() <= 128);
        assert!(tool.output_schema.is_some());
        assert_eq!(tool.input_schema["properties"]["message"]["type"], "string");
    }
    for (id, expected) in [(&local, "local:hello"), (&remote, "hello")] {
        let result = gateway
            .call(params(alias(id, "echo")))
            .await
            .expect("routed fixture call");
        assert_eq!(result.content[0].as_text().expect("text").text, expected);
        assert_eq!(result.structured_content, Some(json!({"message":expected})));
    }
    for name in [
        "echo".into(),
        alias(&local, "fail"),
        alias(&remote, "fail"),
        alias(&Uuid::new_v4().to_string(), "echo"),
    ] {
        assert!(matches!(
            gateway.call(params(name)).await,
            Err(AggregateError::Denied)
        ));
    }
    assert_eq!(
        std::fs::read_to_string(&log).expect("local call witness"),
        "echo\n"
    );
    assert!(!install_marker.exists());
    for _ in 0..100 {
        if endpoint.inventory.lock().expect("inventory").len() >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let reports = endpoint.inventory.lock().expect("inventory").clone();
    assert_eq!(reports.len(), 2);
    for report in reports {
        assert_eq!(report.names, ["echo", "fail"]);
        let public = serde_json::to_value(report).expect("inventory JSON");
        assert_eq!(public.as_object().expect("metadata").len(), 2);
        assert!(!public.to_string().contains("hello"));
    }
    let local_alias = alias(&local, "echo");
    endpoint.change(|snapshot| {
        snapshot.mcps[0].definition.tool_policy = ToolPolicy::Denylist(vec![]);
        snapshot.mcps[1].enabled = false;
    });
    assert!(matches!(
        gateway.call(params(alias(&remote, "echo"))).await,
        Err(AggregateError::Denied)
    ));
    assert_eq!(gateway.list().expect("latest policy").len(), 2);
    gateway.reconcile().await.expect("policy rediscovery");
    assert!(
        gateway
            .list()
            .expect("stable route")
            .iter()
            .any(|tool| tool.name == local_alias)
    );
    let failure = gateway
        .call(CallToolRequestParams::new(alias(&local, "fail")))
        .await
        .expect("tool error result");
    assert_eq!(failure.is_error, Some(true));
    assert_eq!(
        failure.content[0].as_text().expect("failure text").text,
        "fixture failure"
    );
    endpoint.time.advance(3600);
    assert!(gateway.list().is_err());
    assert!(matches!(
        gateway.call(params(local_alias)).await,
        Err(AggregateError::Denied)
    ));
    gateway.close().await;
    upstream.abort();
}

#[derive(Clone)]
struct WireState {
    mode: Arc<AtomicU8>,
    calls: Arc<AtomicUsize>,
    capabilities: Arc<Mutex<Option<Value>>>,
    entered: Arc<tokio::sync::Notify>,
    cancellations: Arc<AtomicUsize>,
}
async fn wire(
    State(state): State<WireState>,
    headers: HeaderMap,
    axum::Json(request): axum::Json<Value>,
) -> Response {
    assert!(
        headers.get("authorization").is_none(),
        "central token must never reach upstream"
    );
    let method = request["method"].as_str().unwrap_or("");
    let result = match method {
        "initialize" => {
            *state.capabilities.lock().expect("capabilities") =
                Some(request["params"]["capabilities"].clone());
            json!({"protocolVersion":"2025-11-25","capabilities":{"tools":{"listChanged":true}},"serverInfo":{"name":"controlled-wire-fixture","version":"1"}})
        }
        "notifications/cancelled" => {
            state.cancellations.fetch_add(1, Ordering::Relaxed);
            return StatusCode::ACCEPTED.into_response();
        }
        "notifications/initialized" => {
            return StatusCode::ACCEPTED.into_response();
        }
        "tools/list" => {
            let mode = state.mode.load(Ordering::Relaxed);
            if mode == 6 {
                state.entered.notify_one();
                return std::future::pending().await;
            }
            if mode == 2 {
                return (
                    [("content-type", "application/json")],
                    "private-malformed-fixture{",
                )
                    .into_response();
            }
            if mode == 4 {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "private-upstream-diagnostic",
                )
                    .into_response();
            }
            let mut tool = json!({"name":"echo","description":"controlled fixture","inputSchema":{"type":"object","properties":{"message":{"type":"string"}}},"outputSchema":{"type":"object","properties":{"message":{"type":"string"}}}});
            if mode == 5 {
                tool["inputSchema"]["properties"]["extra"] = json!({"type":"boolean"});
            }
            if mode == 1 {
                tool["name"] = if request["params"]["cursor"].is_string() {
                    json!("second")
                } else {
                    json!("first")
                };
                json!({"tools":[tool],"nextCursor":"loop"})
            } else if mode == 7 {
                json!({"tools":[tool.clone(),tool]})
            } else if mode == 3 {
                let mut required = tool.clone();
                required["name"] = json!("task-only");
                required["execution"] = json!({"taskSupport":"required"});
                json!({"tools":[tool,required]})
            } else {
                json!({"tools":[tool]})
            }
        }
        "tools/call" => {
            state.calls.fetch_add(1, Ordering::Relaxed);
            if state.mode.load(Ordering::Relaxed) == 8 {
                state.entered.notify_one();
                return std::future::pending().await;
            }
            json!({"content":[{"type":"text","text":"wire:hello"}],"structuredContent":{"message":"wire:hello"}})
        }
        _ => json!({}),
    };
    axum::Json(json!({"jsonrpc":"2.0","id":request["id"],"result":result})).into_response()
}
#[tokio::test]
async fn required_tasks_looping_pages_invalid_responses_and_schema_changes_are_isolated() {
    let healthy_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("healthy fixture");
    let healthy_address = healthy_listener.local_addr().expect("healthy address");
    let healthy = tokio::spawn(async move {
        axum::serve(
            healthy_listener,
            http_router(
                FixtureState::new(format!("http://{healthy_address}"), "none".into()),
                healthy_address.to_string(),
            ),
        )
        .await
        .expect("healthy MCP");
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("wire fixture");
    let address = listener.local_addr().expect("address");
    let state = WireState {
        mode: Arc::new(AtomicU8::new(3)),
        calls: Arc::new(AtomicUsize::new(0)),
        capabilities: Arc::new(Mutex::new(None)),
        entered: Arc::new(tokio::sync::Notify::new()),
        cancellations: Arc::new(AtomicUsize::new(0)),
    };
    let app = Router::new()
        .route("/mcp", post(wire))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("owned wire fixture");
    });
    let endpoint = Endpoint::new(vec![
        mcp(
            Transport::Http {
                url: format!("http://{address}/mcp"),
            },
            ToolPolicy::Denylist(vec![]),
        ),
        mcp(
            Transport::Http {
                url: format!("http://{healthy_address}/mcp"),
            },
            ToolPolicy::Allowlist(vec!["echo".into()]),
        ),
    ])
    .await;
    let id = endpoint.snapshot.mcps[0].definition.id.clone();
    let healthy_id = endpoint.snapshot.mcps[1].definition.id.clone();
    let gateway = Aggregator::new(endpoint.cache.clone(), true)
        .expect("aggregator")
        .with_startup_timeout(Duration::from_secs(5))
        .expect("bounded fixture");
    gateway.reconcile().await.expect("task profile discovery");
    assert_eq!(
        gateway.list().expect("ordinary tools").len(),
        2,
        "statuses: {:?}",
        gateway.statuses().expect("diagnostics")
    );
    assert!(matches!(
        gateway.call(params(alias(&id, "task-only"))).await,
        Err(AggregateError::Denied)
    ));
    assert_eq!(state.calls.load(Ordering::Relaxed), 0);
    assert_eq!(
        state
            .capabilities
            .lock()
            .expect("client capabilities")
            .as_ref(),
        Some(&json!({}))
    );
    let original = gateway
        .list()
        .expect("old schema")
        .into_iter()
        .find(|tool| tool.name == alias(&id, "echo"))
        .expect("wire route");
    state.mode.store(5, Ordering::Relaxed);
    gateway.reconcile().await.expect("changed schema");
    let changed = gateway
        .list()
        .expect("changed tools")
        .into_iter()
        .find(|tool| tool.name == original.name)
        .expect("wire route");
    assert_eq!(changed.name, original.name);
    assert_eq!(
        changed.input_schema["properties"]["extra"]["type"],
        "boolean"
    );
    let result = gateway
        .call(params(changed.name.into_owned()))
        .await
        .expect("wire tool call");
    assert_eq!(
        result.structured_content,
        Some(json!({"message":"wire:hello"}))
    );
    for mode in [1, 2, 4, 7] {
        state.mode.store(mode, Ordering::Relaxed);
        gateway.reconcile().await.expect("isolated failure");
        assert_eq!(gateway.list().expect("failed upstream hidden").len(), 1);
        assert!(gateway.call(params(alias(&id, "echo"))).await.is_err());
        assert_eq!(
            gateway
                .call(params(alias(&healthy_id, "echo")))
                .await
                .expect("healthy upstream remains usable")
                .structured_content,
            Some(json!({"message":"hello"}))
        );
    }
    assert_eq!(
        state.calls.load(Ordering::Relaxed),
        1,
        "failed or hidden calls never reach the fixture"
    );
    gateway.close().await;
    server.abort();
    healthy.abort();
}

#[tokio::test]
async fn startup_uses_sleep_aware_native_time_without_extending_the_policy_lease() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("pending fixture");
    let address = listener.local_addr().expect("address");
    let state = WireState {
        mode: Arc::new(AtomicU8::new(6)),
        calls: Arc::new(AtomicUsize::new(0)),
        capabilities: Arc::new(Mutex::new(None)),
        entered: Arc::new(tokio::sync::Notify::new()),
        cancellations: Arc::new(AtomicUsize::new(0)),
    };
    let app = Router::new()
        .route("/mcp", post(wire))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("pending MCP fixture");
    });
    let endpoint = Endpoint::new(vec![mcp(
        Transport::Http {
            url: format!("http://{address}/mcp"),
        },
        ToolPolicy::Denylist(vec![]),
    )])
    .await;
    let id = endpoint.snapshot.mcps[0].definition.id.clone();
    let gateway = Aggregator::new(endpoint.cache.clone(), true).expect("aggregator");
    let pending = gateway.reconcile();
    tokio::pin!(pending);
    tokio::select! { _ = state.entered.notified() => (), _ = &mut pending => panic!("startup completed before pending discovery") }
    endpoint.time.advance(30);
    tokio::time::timeout(Duration::from_secs(1), &mut pending)
        .await
        .expect("native startup deadline observed")
        .expect("isolated startup failure");
    assert_eq!(gateway.statuses().expect("metadata")[&id], "unavailable");
    assert!(gateway.list().expect("no tools").is_empty());
    assert!(
        endpoint.cache.latest().is_ok(),
        "startup timeout cannot renew or invalidate a still-valid policy lease"
    );
    assert_eq!(state.calls.load(Ordering::Relaxed), 0);
    gateway.close().await;
    server.abort();
}

#[tokio::test]
async fn disable_during_pending_discovery_cancels_startup_before_publication() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("pending fixture");
    let address = listener.local_addr().expect("address");
    let state = WireState {
        mode: Arc::new(AtomicU8::new(6)),
        calls: Arc::new(AtomicUsize::new(0)),
        capabilities: Arc::new(Mutex::new(None)),
        entered: Arc::new(tokio::sync::Notify::new()),
        cancellations: Arc::new(AtomicUsize::new(0)),
    };
    let app = Router::new()
        .route("/mcp", post(wire))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("owned pending MCP");
    });
    let mut endpoint = Endpoint::new(vec![mcp(
        Transport::Http {
            url: format!("http://{address}/mcp"),
        },
        ToolPolicy::Denylist(vec![]),
    )])
    .await;
    let id = endpoint.snapshot.mcps[0].definition.id.clone();
    let gateway = Aggregator::new(endpoint.cache.clone(), true).expect("aggregator");
    let pending = gateway.reconcile();
    tokio::pin!(pending);
    tokio::select! { _ = state.entered.notified() => (), _ = &mut pending => panic!("expected pending discovery") }
    endpoint.change(|snapshot| snapshot.mcps[0].enabled = false);
    tokio::time::timeout(Duration::from_secs(1), &mut pending)
        .await
        .expect("learned disable cancels startup")
        .expect("isolated disabled upstream");
    assert!(gateway.list().expect("current policy").is_empty());
    assert!(gateway.call(params(alias(&id, "echo"))).await.is_err());
    assert_eq!(state.calls.load(Ordering::Relaxed), 0);
    gateway.close().await;
    server.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn disabled_missing_executable_and_empty_allowlist_never_install_or_dispatch() {
    let fixture_command = CommandSpec {
        program: env!("CARGO_BIN_EXE_gateway-fixture").into(),
        args: vec![],
        env: BTreeMap::new(),
        cwd: None,
    };
    let mut endpoint = Endpoint::new(vec![mcp(
        Transport::Stdio {
            command: fixture_command,
        },
        ToolPolicy::Allowlist(vec![]),
    )])
    .await;
    let permitted_id = endpoint.snapshot.mcps[0].definition.id.clone();
    let log = endpoint.root.join("no-calls");
    let marker = endpoint.root.join("no-spawn");
    let mut disabled = mcp(
        Transport::Stdio {
            command: CommandSpec {
                program: "/usr/bin/touch".into(),
                args: vec![marker.to_str().expect("marker").into()],
                env: BTreeMap::new(),
                cwd: None,
            },
        },
        ToolPolicy::Denylist(vec![]),
    );
    disabled.enabled = false;
    let missing = mcp(
        Transport::Stdio {
            command: CommandSpec {
                program: endpoint
                    .root
                    .join("private-missing-program")
                    .to_str()
                    .expect("missing program")
                    .into(),
                args: vec![],
                env: BTreeMap::new(),
                cwd: None,
            },
        },
        ToolPolicy::Denylist(vec![]),
    );
    let missing_id = missing.definition.id.clone();
    let mut bearer = mcp(
        Transport::Http {
            url: "http://127.0.0.1:1/mcp".into(),
        },
        ToolPolicy::Denylist(vec![]),
    );
    bearer.definition.auth = AuthMode::Bearer {};
    bearer.secret = Some(SecretRef {
        id: Uuid::new_v4().to_string(),
        key: "OWNED_KEY".into(),
        version: 1,
    });
    let bearer_id = bearer.definition.id.clone();
    endpoint.change(|snapshot| {
        if let Transport::Stdio { command } = &mut snapshot.mcps[0].definition.transport {
            command
                .args
                .extend(["--call-log".into(), log.to_str().expect("log").into()]);
        }
        snapshot.mcps.extend([disabled, missing, bearer]);
    });
    let gateway = Aggregator::new(endpoint.cache.clone(), true).expect("aggregator");
    gateway.reconcile().await.expect("isolated discovery");
    assert!(gateway.list().expect("empty allowlist").is_empty());
    for mcp in &endpoint.snapshot.mcps {
        assert!(matches!(
            gateway
                .call(params(alias(&mcp.definition.id, "echo")))
                .await,
            Err(AggregateError::Denied)
        ));
    }
    assert!(!log.exists() && !marker.exists());
    assert_eq!(
        gateway.statuses().expect("metadata")[&missing_id],
        "missing_executable"
    );
    assert_eq!(
        gateway.statuses().expect("metadata")[&bearer_id],
        "auth_required"
    );
    assert_eq!(
        gateway.statuses().expect("metadata")[&permitted_id],
        "available",
        "discovery never grants a tool"
    );
    gateway.close().await;
}

#[derive(Clone)]
struct AgentObserver(Arc<AtomicUsize>);

#[tokio::test]
async fn bearer_resolution_replacement_rejection_and_offline_lease_are_scoped() {
    use axum::middleware;
    use gateway_cli::{
        remote_credentials::RemoteCredentials,
        sync::{NativeTransport, SyncEngine},
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("protected fixture");
    let address = listener.local_addr().expect("address");
    let state = WireState {
        mode: Arc::new(AtomicU8::new(0)),
        calls: Arc::new(AtomicUsize::new(0)),
        capabilities: Arc::new(Mutex::new(None)),
        entered: Arc::new(tokio::sync::Notify::new()),
        cancellations: Arc::new(AtomicUsize::new(0)),
    };
    let expected = Arc::new(Mutex::new("private-upstream-key-1".to_owned()));
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let check = expected.clone();
    let recorded = seen.clone();
    let app = Router::new()
        .route("/mcp", post(wire))
        .with_state(state.clone())
        .layer(middleware::from_fn(
            move |mut request: axum::extract::Request, next: middleware::Next| {
                let expected = check.clone();
                let seen = recorded.clone();
                async move {
                    let header = request
                        .headers()
                        .get("authorization")
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("")
                        .to_owned();
                    assert!(!header.contains("private-central"));
                    seen.lock().expect("header witness").push(header.clone());
                    if header
                        != format!(
                            "Bearer {}",
                            expected.lock().expect("expected credential").as_str()
                        )
                    {
                        return StatusCode::UNAUTHORIZED.into_response();
                    }
                    request.headers_mut().remove("authorization");
                    next.run(request).await
                }
            },
        ));
    let protected = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("owned protected MCP");
    });
    let healthy_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("healthy fixture");
    let healthy_address = healthy_listener.local_addr().expect("healthy address");
    let healthy = tokio::spawn(async move {
        axum::serve(
            healthy_listener,
            http_router(
                FixtureState::new(format!("http://{healthy_address}"), "none".into()),
                healthy_address.to_string(),
            ),
        )
        .await
        .expect("healthy MCP");
    });
    let secret = SecretRef {
        id: Uuid::new_v4().to_string(),
        key: "OWNED_KEY".into(),
        version: 1,
    };
    let mut secured = mcp(
        Transport::Http {
            url: format!("http://{address}/mcp"),
        },
        ToolPolicy::Allowlist(vec!["echo".into()]),
    );
    secured.definition.auth = AuthMode::Bearer {};
    secured.secret = Some(secret.clone());
    let mut endpoint = Endpoint::new(vec![
        secured,
        mcp(
            Transport::Http {
                url: format!("http://{healthy_address}/mcp"),
            },
            ToolPolicy::Allowlist(vec!["echo".into()]),
        ),
    ])
    .await;
    let id = endpoint.snapshot.mcps[0].definition.id.clone();
    let healthy_id = endpoint.snapshot.mcps[1].definition.id.clone();
    *endpoint.credential.lock().expect("central credential") = Some(UpstreamCredential::Bearer {
        token: "private-upstream-key-1".into(),
        secret_id: secret.id.clone(),
        version: 1,
    });
    let resolver = Arc::new(RemoteCredentials::new(SyncEngine::new(
        endpoint.cache.clone(),
        Arc::new(NativeTransport),
    )));
    let gateway = Aggregator::new(endpoint.cache.clone(), true)
        .expect("gateway")
        .with_credentials(resolver.clone());
    gateway.reconcile().await.expect("authenticated discovery");
    assert_eq!(gateway.list().expect("tools").len(), 2);
    gateway
        .call(params(alias(&id, "echo")))
        .await
        .expect("protected call");
    assert_eq!(state.calls.load(Ordering::Relaxed), 1);
    // A newly confirmed secret version immediately blocks dispatch through the old connection.
    endpoint.change(|snapshot| snapshot.mcps[0].secret.as_mut().expect("key").version = 2);
    *endpoint.credential.lock().expect("central credential") = Some(UpstreamCredential::Bearer {
        token: "private-upstream-key-2".into(),
        secret_id: secret.id.clone(),
        version: 2,
    });
    *expected.lock().expect("expected key") = "private-upstream-key-2".into();
    assert!(gateway.call(params(alias(&id, "echo"))).await.is_err());
    assert_eq!(state.calls.load(Ordering::Relaxed), 1);
    gateway.reconcile().await.expect("replacement discovery");
    gateway
        .call(params(alias(&id, "echo")))
        .await
        .expect("replacement token call");
    // A remote401 is recorded for exactly this credential; healthy tools remain available.
    *expected.lock().expect("remote revocation") = "revoked-fixture".into();
    assert!(gateway.call(params(alias(&id, "echo"))).await.is_err());
    assert!(
        gateway
            .list()
            .expect("scoped rejection")
            .iter()
            .all(|tool| tool.name != alias(&id, "echo"))
    );
    gateway
        .call(params(alias(&healthy_id, "echo")))
        .await
        .expect("healthy tool after401");
    gateway
        .reconcile()
        .await
        .expect("rejected credential isolated");
    assert_eq!(gateway.statuses().expect("status")[&id], "auth_rejected");
    // A changed owned value can recover, while an unrelated key returned by the API cannot.
    endpoint.change(|snapshot| snapshot.mcps[0].secret.as_mut().expect("key").version = 3);
    *endpoint.credential.lock().expect("foreign response") = Some(UpstreamCredential::Bearer {
        token: "private-foreign-fixture".into(),
        secret_id: Uuid::new_v4().to_string(),
        version: 3,
    });
    gateway
        .reconcile()
        .await
        .expect("foreign credential isolated");
    assert!(gateway.call(params(alias(&id, "echo"))).await.is_err());
    *endpoint.credential.lock().expect("correct key") = Some(UpstreamCredential::Bearer {
        token: "private-upstream-key-3".into(),
        secret_id: secret.id,
        version: 3,
    });
    *expected.lock().expect("remote key") = "private-upstream-key-3".into();
    gateway
        .reconcile()
        .await
        .expect("correct credential recovery");
    gateway
        .call(params(alias(&id, "echo")))
        .await
        .expect("recovered call");
    let expiry = endpoint
        .cache
        .latest()
        .expect("existing lease")
        .snapshot
        .expires_at;
    endpoint.credential_code.store(503, Ordering::Relaxed);
    gateway
        .reconcile()
        .await
        .expect("offline cached credentials");
    assert_eq!(
        endpoint
            .cache
            .latest()
            .expect("unchanged lease")
            .snapshot
            .expires_at,
        expiry
    );
    endpoint.time.advance(3599);
    gateway
        .call(params(alias(&id, "echo")))
        .await
        .expect("valid cached bearer before expiry");
    endpoint.time.advance(1);
    let calls = state.calls.load(Ordering::Relaxed);
    assert!(gateway.list().is_err());
    assert!(gateway.call(params(alias(&id, "echo"))).await.is_err());
    assert_eq!(state.calls.load(Ordering::Relaxed), calls);
    {
        let headers = seen.lock().expect("scoped headers");
        assert!(
            headers
                .iter()
                .all(|value| value.starts_with("Bearer private-upstream-key-"))
        );
    }
    gateway.close().await;
    protected.abort();
    healthy.abort();
}
impl rmcp::ClientHandler for AgentObserver {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        rmcp::model::ClientConfig::new(
            rmcp::model::ClientCapabilities::default(),
            rmcp::model::Implementation::new("standard-agent-fixture", "1"),
        )
        .with_protocol_version(rmcp::model::ProtocolVersion::V_2025_11_25)
    }
    async fn on_tool_list_changed(&self, _: rmcp::service::NotificationContext<rmcp::RoleClient>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[tokio::test]
async fn standard_client_runtime_enforces_cancellation_concurrency_expiry_and_notifications() {
    use gateway_cli::{
        runtime::{GatewayService, coordinate},
        sync::{NativeTransport, SyncEngine},
    };
    use rmcp::{ServiceExt, model::*, transport::async_rw::AsyncRwTransport};
    use tokio_util::sync::CancellationToken;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("runtime fixture");
    let address = listener.local_addr().expect("address");
    let state = WireState {
        mode: Arc::new(AtomicU8::new(0)),
        calls: Arc::new(AtomicUsize::new(0)),
        capabilities: Arc::new(Mutex::new(None)),
        entered: Arc::new(tokio::sync::Notify::new()),
        cancellations: Arc::new(AtomicUsize::new(0)),
    };
    let app = Router::new()
        .route("/mcp", post(wire))
        .with_state(state.clone());
    let upstream = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("owned runtime MCP");
    });
    let mut endpoint = Endpoint::new(vec![mcp(
        Transport::Http {
            url: format!("http://{address}/mcp"),
        },
        ToolPolicy::Denylist(vec![]),
    )])
    .await;
    let id = endpoint.snapshot.mcps[0].definition.id.clone();
    let gateway = Arc::new(Aggregator::new(endpoint.cache.clone(), true).expect("gateway"));
    gateway.reconcile().await.expect("discovery");
    let stop = CancellationToken::new();
    let (ready, initialized) = tokio::sync::watch::channel(true);
    let handler = GatewayService::new(
        gateway.clone(),
        stop.clone(),
        initialized,
        Duration::from_secs(60),
        16,
    )
    .expect("runtime limits");
    let (server_wire, client_wire) = tokio::io::duplex(64 * 1024);
    let (server_read, server_write) = tokio::io::split(server_wire);
    let (client_read, client_write) = tokio::io::split(client_wire);
    let serving =
        tokio::spawn(handler.serve(AsyncRwTransport::new_server(server_read, server_write)));
    let changes = Arc::new(AtomicUsize::new(0));
    let agent = AgentObserver(changes.clone())
        .serve(AsyncRwTransport::new_client(client_read, client_write))
        .await
        .expect("standard MCP client");
    let server = serving
        .await
        .expect("server task")
        .expect("agent handshake");
    let info = serde_json::to_value(agent.peer_info().expect("gateway capabilities"))
        .expect("public info");
    assert_eq!(info["protocolVersion"], "2025-11-25");
    assert_eq!(info["capabilities"], json!({"tools":{"listChanged":true}}));
    let coordinator = tokio::spawn(coordinate(
        gateway.clone(),
        SyncEngine::new(endpoint.cache.clone(), Arc::new(NativeTransport)),
        server.peer().clone(),
        stop.clone(),
        ready,
    ));
    assert_eq!(
        agent
            .list_tools(None)
            .await
            .expect("agent tools")
            .tools
            .len(),
        1
    );
    assert_eq!(
        agent
            .call_tool(params(alias(&id, "echo")))
            .await
            .expect("agent routed call")
            .structured_content,
        Some(json!({"message":"wire:hello"}))
    );
    assert!(agent.call_tool(params("echo".into())).await.is_err());
    assert_eq!(state.calls.load(Ordering::Relaxed), 1);
    // A sleep-aware call deadline does not depend on a Tokio sleep advancing.
    state.mode.store(8, Ordering::Relaxed);
    {
        let call = agent.call_tool(params(alias(&id, "echo")));
        tokio::pin!(call);
        tokio::select! { _ = state.entered.notified() => (), _ = &mut call => panic!("expected pending tool") }
        endpoint.time.advance(60);
        let result = tokio::time::timeout(Duration::from_secs(1), &mut call)
            .await
            .expect("native call deadline")
            .expect("actionable tool error");
        assert_eq!(result.is_error, Some(true));
    }
    // The standard client's notification maps to the exact upstream request.
    let request = ClientRequest::CallToolRequest(CallToolRequest::new(params(alias(&id, "echo"))));
    let handle = agent
        .send_cancellable_request(request, rmcp::service::PeerRequestOptions::no_options())
        .await
        .expect("cancellable call");
    state.entered.notified().await;
    agent
        .notify_cancelled(CancelledNotificationParam::new(
            Some(handle.id.clone()),
            Some("private-agent-cancellation-reason".into()),
        ))
        .await
        .expect("agent cancellation");
    drop(handle);
    for _ in 0..100 {
        if state.cancellations.load(Ordering::Relaxed) >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        state.cancellations.load(Ordering::Relaxed) >= 2,
        "deadline and agent cancellation reached upstream"
    );
    let before = state.calls.load(Ordering::Relaxed);
    let mut pending = Vec::new();
    for _ in 0..16 {
        pending.push(
            agent
                .send_cancellable_request(
                    ClientRequest::CallToolRequest(CallToolRequest::new(params(alias(
                        &id, "echo",
                    )))),
                    rmcp::service::PeerRequestOptions::no_options(),
                )
                .await
                .expect("parallel call"),
        );
    }
    for _ in 0..200 {
        if state.calls.load(Ordering::Relaxed) == before + 16 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(state.calls.load(Ordering::Relaxed), before + 16);
    assert!(
        agent.call_tool(params(alias(&id, "echo"))).await.is_err(),
        "seventeenth call is refused without a queue"
    );
    assert_eq!(state.calls.load(Ordering::Relaxed), before + 16);
    let observed = changes.load(Ordering::Relaxed);
    endpoint.change(|snapshot| snapshot.mcps[0].enabled = false);
    assert!(
        agent.call_tool(params(alias(&id, "echo"))).await.is_err(),
        "same-Mac publication gates the next dispatch"
    );
    for _ in 0..150 {
        if changes.load(Ordering::Relaxed) > observed
            && gateway.list().expect("current listing").is_empty()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(changes.load(Ordering::Relaxed) > observed);
    assert!(
        agent
            .list_tools(None)
            .await
            .expect("disabled tools")
            .tools
            .is_empty()
    );
    drop(pending);
    endpoint.time.advance(3600);
    assert!(
        agent
            .list_tools(None)
            .await
            .expect("expired listing")
            .tools
            .is_empty()
    );
    assert!(agent.call_tool(params(alias(&id, "echo"))).await.is_err());
    stop.cancel();
    let _ = coordinator.await;
    gateway.close().await;
    let _ = agent.cancel().await;
    let _ = server.cancel().await;
    upstream.abort();
}
