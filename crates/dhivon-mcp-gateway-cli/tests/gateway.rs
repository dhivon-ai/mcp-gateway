#![cfg(target_os = "macos")]
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use dhivon_mcp_gateway_cli::{epoch, owned_process::OwnedProcess};
use dhivon_mcp_gateway_core::*;
use rmcp::{
    ServerHandler, ServiceExt,
    model::*,
    transport::{
        async_rw::AsyncRwTransport,
        streamable_http_server::{
            StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
        },
    },
};
use serde_json::{Value, json};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU16, AtomicUsize, Ordering},
    },
    time::Duration,
};
use uuid::Uuid;

// The release rehearsal executes the extracted archive through this same Mac journey.
fn cli_binary() -> &'static str {
    static BINARY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    BINARY.get_or_init(|| {
        let binary = std::env::var("DHIVON_MCP_GATEWAY_TEST_CLI")
            .unwrap_or_else(|_| env!("CARGO_BIN_EXE_dhivon-mcp-gateway").into());
        assert!(
            Path::new(&binary).is_absolute() && Path::new(&binary).is_file(),
            "Use an absolute existing test CLI"
        );
        binary
    })
}
#[derive(Clone)]
struct Remote(Arc<AtomicUsize>);
impl ServerHandler for Remote {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2025_11_25)
            .with_server_info(Implementation::new("actual-cli-remote-fixture", "1"))
    }
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Owned(vec![ProtocolVersion::V_2025_11_25])
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: vec![Tool::new(
                "echo",
                "Remote fixture",
                json!({"type":"object"})
                    .as_object()
                    .cloned()
                    .expect("schema"),
            )],
            ..Default::default()
        })
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        self.0.fetch_add(1, Ordering::Relaxed);
        let text = if request
            .arguments
            .as_ref()
            .and_then(|args| args.get("large"))
            .is_some_and(|value| value == true)
        {
            "private-backpressure-fixture".repeat(60_000)
        } else {
            "remote:hello".into()
        };
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]).into())
    }
}
#[derive(Clone)]
struct Control {
    snapshot: Arc<Mutex<ConfigSnapshot>>,
    code: Arc<AtomicU16>,
    polls: Arc<AtomicUsize>,
}
async fn config(State(control): State<Control>) -> (StatusCode, axum::Json<ConfigSnapshot>) {
    control.polls.fetch_add(1, Ordering::Relaxed);
    let mut snapshot = control.snapshot.lock().expect("owned config");
    snapshot.validated_at = epoch().expect("epoch");
    snapshot.expires_at = snapshot.validated_at + 3600;
    (
        StatusCode::from_u16(control.code.load(Ordering::Relaxed)).expect("fixture status"),
        axum::Json(snapshot.clone()),
    )
}
async fn settings(
    State(control): State<Control>,
    axum::Json(input): axum::Json<SettingsUpdate>,
) -> (StatusCode, axum::Json<ConfigSnapshot>) {
    let mut snapshot = control.snapshot.lock().expect("owned config");
    if input.expected_revision != snapshot.revision {
        return (StatusCode::CONFLICT, axum::Json(snapshot.clone()));
    }
    snapshot.mcps[0].enabled = input.enabled;
    snapshot.revision += 1;
    snapshot.validated_at = epoch().expect("epoch");
    snapshot.expires_at = snapshot.validated_at + 3600;
    (StatusCode::OK, axum::Json(snapshot.clone()))
}
struct Files(PathBuf);
/// A failed assertion must signal graceful native cleanup before the ownership guard can kill the CLI.
struct Process(Option<OwnedProcess>);
impl Process {
    fn spawn(spec: &CommandSpec) -> dhivon_mcp_gateway_cli::Result<Self> {
        OwnedProcess::spawn(spec).map(|process| Self(Some(process)))
    }
    fn spawn_protocol(spec: &CommandSpec) -> dhivon_mcp_gateway_cli::Result<Self> {
        OwnedProcess::spawn_protocol(spec).map(|process| Self(Some(process)))
    }
    async fn terminate(
        mut self,
        duration: Duration,
    ) -> dhivon_mcp_gateway_cli::Result<std::process::ExitStatus> {
        self.0
            .take()
            .expect("owned test process")
            .terminate(duration)
            .await
    }
}
impl std::ops::Deref for Process {
    type Target = OwnedProcess;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("owned test process")
    }
}
impl std::ops::DerefMut for Process {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("owned test process")
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let Some(process) = self.0.as_ref() else {
            return;
        };
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(process.id() as i32),
            nix::sys::signal::Signal::SIGTERM,
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(6);
        while std::time::Instant::now() < deadline
            && process.poll_exit().is_ok_and(|status| status.is_none())
        {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
impl Files {
    fn new() -> Self {
        use std::os::unix::fs::DirBuilderExt;
        let root = std::env::temp_dir()
            .canonicalize()
            .expect("fixture parent")
            .join(format!("gateway-agent-cli-owned-{}", Uuid::new_v4()));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .expect("private root");
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(root.join("native-state"))
            .expect("private native state");
        Self(root)
    }
    fn state(&self) -> PathBuf {
        self.0.join("native-state")
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let journal = self.state().join("identity.json");
        if (journal.exists() || journal.with_file_name("endpoint-v2").exists())
            && std::fs::read(&journal)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .is_none_or(|state| state["fixtureRetired"] != true)
        {
            eprintln!(
                "Owned gateway fixture cleanup journal retained: {}",
                self.0.display()
            );
            return;
        }
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn command(program: &str, args: Vec<String>) -> CommandSpec {
    CommandSpec {
        program: program.into(),
        args,
        env: BTreeMap::new(),
        cwd: None,
    }
}
fn local(path: &Path) -> CommandSpec {
    command("/usr/bin/python3", vec!["-u".into(), "-c".into(), r#"import os,sys,json,signal,subprocess
signal.signal(signal.SIGTERM,signal.SIG_IGN)
child=subprocess.Popen(['/usr/bin/python3','-c','import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); time.sleep(120)'])
with open(sys.argv[1],'w') as f: json.dump([os.getpid(),child.pid],f)
print('private-upstream-stderr-fixture',file=sys.stderr,flush=True)
for line in sys.stdin:
    request=json.loads(line)
    method=request.get('method')
    if 'id' not in request: continue
    if method=='initialize': result={'protocolVersion':'2025-11-25','capabilities':{'tools':{}},'serverInfo':{'name':'actual-cli-local-fixture','version':'1'}}
    elif method=='tools/list': result={'tools':[{'name':'echo','inputSchema':{'type':'object'},'description':'Local fixture'}]}
    elif method=='tools/call': result={'content':[{'type':'text','text':'local:hello'}]}
    else: result={}
    print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':result}),flush=True)
"#.into(), path.to_str().expect("owned PID path").into()])
}
fn definition(id: &str, transport: Transport, install: Option<CommandSpec>) -> EffectiveMcp {
    EffectiveMcp {
        definition: McpDefinition {
            id: id.into(),
            name: "Fixture".into(),
            description: "Metadata".into(),
            active: true,
            transport,
            install,
            auth: AuthMode::None {},
            tool_policy: ToolPolicy::Allowlist(vec!["echo".into()]),
        },
        enabled: true,
        secret: None,
    }
}
async fn pids(path: &Path) -> Vec<i32> {
    for _ in 0..200 {
        if let Ok(bytes) = std::fs::read(path)
            && let Ok(pids) = serde_json::from_slice(&bytes)
        {
            return pids;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("owned local process did not start")
}
async fn gone(pids: &[i32]) {
    for _ in 0..200 {
        if pids.iter().all(|pid| {
            nix::sys::signal::kill(nix::unistd::Pid::from_raw(*pid), None)
                == Err(nix::errno::Errno::ESRCH)
        }) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("owned native upstream remains")
}
async fn local_login(base: &str, client: &reqwest::Client) -> (String, String, String) {
    let mut page = None;
    for _ in 0..200 {
        if let Ok(response) = client.get(format!("{base}/")).send().await {
            page = Some(response);
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let page = page.expect("actual local UI");
    let cookie = page.headers()["set-cookie"]
        .to_str()
        .expect("cookie")
        .split(';')
        .next()
        .expect("cookie pair")
        .to_owned();
    let html = page.text().await.expect("compiled local page");
    if std::env::var_os("DHIVON_MCP_GATEWAY_TEST_CLI").is_some() {
        assert!(
            html.contains("data-gateway-mode=\"local\""),
            "packaged local app mode"
        );
        let script = html
            .split("src=\"")
            .nth(1)
            .expect("embedded module")
            .split('"')
            .next()
            .expect("module URL");
        assert!(script.starts_with("/assets/"));
        let response = client
            .get(format!("{base}{script}"))
            .header("cookie", &cookie)
            .send()
            .await
            .expect("embedded script");
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.bytes().await.expect("embedded script bytes").len() > 1000);
    }
    let context: Value = client
        .get(format!("{base}/api/context"))
        .header("cookie", &cookie)
        .send()
        .await
        .expect("context")
        .json()
        .await
        .expect("context JSON");
    let csrf = context["csrfToken"].as_str().expect("CSRF").to_owned();
    let response = client
        .post(format!("{base}/api/auth/login"))
        .header("cookie", &cookie)
        .header("origin", base)
        .header("x-csrf-token", &csrf)
        .json(&LoginRequest {
            email: "owned-fixture@example.test".into(),
            password: "private-fixture-password".into(),
        })
        .send()
        .await
        .expect("actual Keychain login");
    assert_eq!(response.status(), StatusCode::OK);
    let logged: Value = response.json().await.expect("redacted login");
    let generation = logged["identity"]["generation"]
        .as_str()
        .expect("generation")
        .to_owned();
    (cookie, csrf, generation)
}
fn assert_fixture_slots(files: &Files, expected: usize) {
    use base64::Engine;
    use security_framework::item::{ItemClass, ItemSearchOptions, Limit};
    use sha2::{Digest, Sha256};
    let root = files.state().canonicalize().expect("fixture path");
    let digest = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(root.to_str().expect("fixture").as_bytes()));
    let legacy = format!("com.dhivon.mcp-gateway.fixture.{digest}");
    for (service, count) in [(&legacy, 0), (&format!("{legacy}.master.v2"), expected)] {
        let n = match ItemSearchOptions::new()
            .class(ItemClass::generic_password())
            .service(service)
            .load_attributes(true)
            .limit(Limit::Max(2))
            .search()
        {
            Ok(items) => items.len(),
            Err(e) if e.code() == -25300 => 0,
            Err(_) => panic!("fixture keychain attribute lookup failed"),
        };
        assert_eq!(n, count, "exact owned namespace, no secret read");
    }
}
async fn cleanup(files: &Files) {
    let status = tokio::process::Command::new(cli_binary())
        .args(["logout", "--allow-fixture-http", "--fixture-state-path"])
        .arg(files.state())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .expect("creating CLI credential cleanup");
    assert!(
        status.success(),
        "Keychain cleanup failed; retained owned journal"
    );
    assert!(
        std::process::Command::new(cli_binary())
            .args(["fixture-retire", "--fixture-state-path"])
            .arg(files.state())
            .status()
            .expect("verify and retire fixture master")
            .success()
    );
    assert_fixture_slots(files, 0);
}

#[tokio::test]
async fn actual_cli_one_stdio_entry_supports_local_remote_shared_changes_offline_restart_and_owned_exit()
 {
    let files = Files::new();
    if let Some(creator) = std::env::var_os("DHIVON_MCP_GATEWAY_TEST_V1_CLI") {
        let path = Path::new(&creator);
        assert!(path.is_absolute() && path.is_file());
        assert!(
            std::process::Command::new(path)
                .args(["status", "--allow-fixture-http", "--fixture-state-path"])
                .arg(files.state())
                .output()
                .expect("original CLI")
                .status
                .success()
        );
        let old: Value = serde_json::from_slice(
            &std::fs::read(files.state().join("identity.json")).expect("legacy format"),
        )
        .expect("legacy metadata");
        assert_eq!(old["schemaVersion"], 1);
    }
    let local_pids = files.0.join("upstream-pids");
    let install_marker = files.0.join("never-installed");
    let remote_calls = Arc::new(AtomicUsize::new(0));
    let remote = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("remote listener");
    let remote_address = remote.local_addr().expect("remote address");
    let calls = remote_calls.clone();
    let service = StreamableHttpService::new(
        move || Ok(Remote(calls.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_allowed_hosts(vec![remote_address.to_string()]),
    );
    let secure_calls = remote_calls.clone();
    let secured = StreamableHttpService::new(
        move || Ok(Remote(secure_calls.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_allowed_hosts(vec![remote_address.to_string()]),
    );
    let secured = Router::new()
        .nest_service("/secure", secured)
        .layer(axum::middleware::from_fn(
            |request: axum::extract::Request, next: axum::middleware::Next| async move {
                if request
                    .headers()
                    .get("authorization")
                    .and_then(|value| value.to_str().ok())
                    != Some("Bearer private-cli-bearer-fixture")
                {
                    return StatusCode::UNAUTHORIZED.into_response();
                }
                next.run(request).await
            },
        ));
    let oauth_calls = remote_calls.clone();
    let oauth_service = StreamableHttpService::new(
        move || Ok(Remote(oauth_calls.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_allowed_hosts(vec![remote_address.to_string()]),
    );
    let oauth_service =
        Router::new()
            .nest_service("/oauth", oauth_service)
            .layer(axum::middleware::from_fn(
                |request: axum::extract::Request, next: axum::middleware::Next| async move {
                    if request
                        .headers()
                        .get("authorization")
                        .and_then(|value| value.to_str().ok())
                        != Some("Bearer private-cli-oauth-fixture")
                    {
                        return StatusCode::UNAUTHORIZED.into_response();
                    }
                    next.run(request).await
                },
            ));
    let remote_task = tokio::spawn(async move {
        axum::serve(
            remote,
            Router::new()
                .nest_service("/mcp", service)
                .merge(secured)
                .merge(oauth_service),
        )
        .await
        .expect("owned real SDK remote");
    });
    let now = epoch().expect("epoch");
    let control = Control {
        code: Arc::new(AtomicU16::new(200)),
        polls: Arc::new(AtomicUsize::new(0)),
        snapshot: Arc::new(Mutex::new(ConfigSnapshot {
            schema_version: 1,
            deployment_id: Uuid::new_v4().to_string(),
            user: UserInfo {
                name: "Fixture User".into(),
                id: Uuid::new_v4().to_string(),
                email: "owned-fixture@example.test".into(),
                role: dhivon_mcp_gateway_core::Role::User,
                active: true,
            },
            revision: 1,
            validated_at: now,
            expires_at: now + 3600,
            session_expires_at: now + 43200,
            mcps: vec![
                definition(
                    "local-fixture",
                    Transport::Stdio {
                        command: local(&local_pids),
                    },
                    Some(command(
                        "/usr/bin/touch",
                        vec![install_marker.to_str().expect("marker").into()],
                    )),
                ),
                definition(
                    "remote-fixture",
                    Transport::Http {
                        url: format!("http://{remote_address}/mcp"),
                    },
                    None,
                ),
            ],
        })),
    };
    let secret_id = Uuid::new_v4().to_string();
    {
        let mut snapshot = control.snapshot.lock().expect("bearer fixture");
        let mut bearer = definition(
            "bearer-fixture",
            Transport::Http {
                url: format!("http://{remote_address}/secure"),
            },
            None,
        );
        bearer.definition.auth = AuthMode::Bearer {};
        bearer.secret = Some(SecretRef {
            id: secret_id.clone(),
            key: "CLI_KEY".into(),
            version: 1,
        });
        snapshot.mcps.push(bearer);
        let mut oauth = definition(
            "oauth-fixture",
            Transport::Http {
                url: format!("http://{remote_address}/oauth"),
            },
            None,
        );
        oauth.definition.auth = AuthMode::OAuth {
            client_id: "public-cli-fixture".into(),
            scopes: vec!["tools".into()],
        };
        snapshot.mcps.push(oauth);
    }
    let bearer_status = Arc::new(AtomicU16::new(200));
    let response_status = bearer_status.clone();
    let refused = Arc::new(AtomicUsize::new(0));
    let refused_count = refused.clone();
    let bearer = UpstreamCredential::Bearer {
        token: "private-cli-bearer-fixture".into(),
        secret_id,
        version: 1,
    };
    // The guard has the same sealed envelope but serializes its credential as null.
    let bearer_size_delta = serde_json::to_vec(&bearer)
        .expect("serialize synthetic bearer")
        .len()
        - b"null".len();
    let app = Router::new()
        .route("/api/config", get(config))
        .route(
            "/api/endpoint/login",
            post(|State(control): State<Control>| async move {
                let snapshot = control.snapshot.lock().expect("fixture config");
                axum::Json(EndpointSession {
                    token: "private-native-central-fixture".into(),
                    user: snapshot.user.clone(),
                    deployment_id: snapshot.deployment_id.clone(),
                    expires_at: snapshot.session_expires_at,
                })
            }),
        )
        .route(
            "/api/auth/logout",
            post(|| async { StatusCode::NO_CONTENT }),
        )
        .route("/api/settings/local-fixture", put(settings))
        .route(
            "/api/endpoint/credentials/bearer-fixture",
            post(
                move |headers: axum::http::HeaderMap,
                      axum::Json(input): axum::Json<CredentialRequest>| {
                    let bearer = bearer.clone();
                    let code = response_status.load(Ordering::SeqCst);
                    if code == 403 {
                        refused_count.fetch_add(1, Ordering::SeqCst);
                    }
                    async move {
                        assert_eq!(
                            headers["authorization"],
                            "Bearer private-native-central-fixture"
                        );
                        assert!(Uuid::parse_str(&input.installation_id).is_ok());
                        (
                            StatusCode::from_u16(code).expect("fixture HTTP status"),
                            axum::Json(bearer),
                        )
                    }
                },
            ),
        )
        .route(
            "/api/endpoint/inventory",
            post(|| async { StatusCode::NO_CONTENT }),
        )
        .route(
            "/api/endpoint/credentials/oauth-fixture",
            post(
                move |headers: axum::http::HeaderMap,
                      axum::Json(input): axum::Json<CredentialRequest>| async move {
                    assert_eq!(
                        headers["authorization"],
                        "Bearer private-native-central-fixture"
                    );
                    axum::Json(UpstreamCredential::OAuth {
                        credentials: OAuthCredentials {
                            access_token: "private-cli-oauth-fixture".into(),
                            refresh_token: Some("private-cli-refresh-fixture".into()),
                            expires_at: now + 3600,
                            issuer: format!("http://{remote_address}"),
                            resource: format!("http://{remote_address}/oauth"),
                            client_id: "public-cli-fixture".into(),
                            scopes: vec!["tools".into()],
                        },
                        version: 1,
                        installation_id: input.installation_id,
                    })
                },
            ),
        )
        .with_state(control.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("control listener");
    let origin = format!("http://{}", listener.local_addr().expect("control address"));
    let central_task = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("owned control");
    });
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("local port")
        .local_addr()
        .expect("port")
        .port();
    let ui = Process::spawn(&command(
        cli_binary(),
        vec![
            "ui".into(),
            "--server".into(),
            origin.clone(),
            "--allow-fixture-http".into(),
            "--fixture-state-path".into(),
            files.state().to_str().expect("state").into(),
            "--port".into(),
            port.to_string(),
            "--no-open".into(),
        ],
    ))
    .expect("actual UI CLI");
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("local HTTP");
    let base = format!("http://127.0.0.1:{port}");
    let (cookie, csrf, generation) = local_login(&base, &client).await;
    assert_fixture_slots(&files, 1);
    // Damage only the synthetic snapshot, exercise denial with the same live UI,
    // then repair through its authenticated refresh route before starting the agent.
    control.code.store(503, Ordering::Relaxed);
    std::fs::write(files.state().join("snapshot.enc"), b"truncated")
        .expect("damage fixture snapshot");
    let status: Value = client
        .get(format!("{base}/api/status"))
        .header("cookie", &cookie)
        .send()
        .await
        .expect("redacted status")
        .json()
        .await
        .expect("status");
    assert_eq!(status["configuration"]["state"], "invalid");
    control.code.store(200, Ordering::Relaxed);
    let repaired = client
        .post(format!("{base}/api/refresh"))
        .header("cookie", &cookie)
        .header("origin", &base)
        .header("x-csrf-token", &csrf)
        .json(&LocalAction {
            expected_generation: generation.clone(),
            input: (),
        })
        .send()
        .await
        .expect("recover in running UI");
    assert_eq!(repaired.status(), StatusCode::OK);
    let gateway_spec = command(
        cli_binary(),
        vec![
            "mcp".into(),
            "--allow-fixture-http".into(),
            "--fixture-state-path".into(),
            files.state().to_str().expect("state").into(),
        ],
    );
    let mut gateway = Process::spawn_protocol(&gateway_spec).expect("actual gateway command");
    let (read, write) = gateway.take_protocol().expect("one STDIO pipe");
    let agent = ClientConfig::new(
        ClientCapabilities::default(),
        Implementation::new("standard-one-entry-client", "1"),
    )
    .with_protocol_version(ProtocolVersion::V_2025_11_25)
    .serve(AsyncRwTransport::new_client(read, write))
    .await
    .expect("CLI MCP negotiation");
    let tools = agent
        .list_all_tools()
        .await
        .expect("CLI aggregate discovery");
    assert_eq!(tools.len(), 4);
    let owned = pids(&local_pids).await;
    for (id, expected) in [
        ("local-fixture", "local:hello"),
        ("remote-fixture", "remote:hello"),
        ("bearer-fixture", "remote:hello"),
        ("oauth-fixture", "remote:hello"),
    ] {
        let result = agent
            .call_tool(CallToolRequestParams::new(
                dhivon_mcp_gateway_cli::aggregation::alias(id, "echo"),
            ))
            .await
            .expect("single-entry routed call");
        assert_eq!(
            result.content[0].as_text().expect("result text").text,
            expected
        );
    }
    assert!(!install_marker.exists());
    assert!(
        agent
            .call_tool(CallToolRequestParams::new("echo"))
            .await
            .is_err()
    );
    assert_eq!(remote_calls.load(Ordering::Relaxed), 3);
    let fence = std::fs::read(files.state().join("identity.json")).expect("inert legacy fence");
    assert_eq!(fence, br#"{"schemaVersion":2}"#);
    let mut files_to_scan = vec![files.state()];
    let mut records = 0;
    while let Some(path) = files_to_scan.pop() {
        if path.is_dir() {
            files_to_scan.extend(
                std::fs::read_dir(path)
                    .expect("fixture directory")
                    .map(|e| e.expect("fixture entry").path()),
            );
            continue;
        }
        if path
            .file_name()
            .expect("fixture file name")
            .to_string_lossy()
            .starts_with("record-")
        {
            records += 1;
        }
        let bytes = std::fs::read(path).expect("fixture encrypted file");
        for text in [
            "private-cli-bearer-fixture",
            "private-cli-oauth-fixture",
            "private-cli-refresh-fixture",
            "bearer-fixture",
            &origin,
        ] {
            assert!(!String::from_utf8_lossy(&bytes).contains(text));
        }
    }
    assert!(records >= 3);
    let revision = control.snapshot.lock().expect("revision").revision;
    let response = client
        .put(format!("{base}/api/settings/local-fixture"))
        .header("cookie", &cookie)
        .header("origin", &base)
        .header("x-csrf-token", &csrf)
        .json(&LocalAction {
            expected_generation: generation,
            input: SettingsUpdate {
                expected_revision: revision,
                enabled: false,
                secret_id: None,
            },
        })
        .send()
        .await
        .expect("same-Mac disable");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        agent
            .call_tool(CallToolRequestParams::new(
                dhivon_mcp_gateway_cli::aggregation::alias("local-fixture", "echo")
            ))
            .await
            .is_err()
    );
    let bearer_call = || {
        CallToolRequestParams::new(dhivon_mcp_gateway_cli::aggregation::alias(
            "bearer-fixture",
            "echo",
        ))
    };
    for _ in 0..100 {
        if agent
            .list_all_tools()
            .await
            .expect("live catalog")
            .iter()
            .any(|tool| {
                tool.name == dhivon_mcp_gateway_cli::aggregation::alias("bearer-fixture", "echo")
            })
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    agent
        .call_tool(bearer_call())
        .await
        .expect("current bearer available before refusal");
    let began = std::time::Instant::now();
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(gateway.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
    )
    .expect("only owned gateway signal");
    for _ in 0..600 {
        if gateway.poll_exit().expect("gateway exit").is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let observed = gateway.poll_exit().expect("gateway exit state");
    let status = gateway
        .terminate(Duration::from_secs(1))
        .await
        .expect("owned final reap");
    let elapsed = began.elapsed();
    let _ = agent.cancel().await;
    assert_eq!(observed, Some(Some(0)));
    assert!(status.success());
    assert!(elapsed < Duration::from_secs(6));
    gone(&owned).await;
    // This checkout's package owns its fixture Keychain entry. Select the two
    // synthetic bearer files by their known framing sizes, without accessing its key.
    let sealed: Vec<_> = std::fs::read_dir(files.state().join("endpoint-v2"))
        .expect("sealed fixture directory")
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("record-"))
        .filter_map(|e| std::fs::read(e.path()).ok().map(|bytes| (e.path(), bytes)))
        .collect();
    let pairs: Vec<_> = sealed
        .iter()
        .flat_map(|head| {
            sealed.iter().filter_map(move |guard| {
                (head.1.len() == guard.1.len() + bearer_size_delta).then_some([head, guard])
            })
        })
        .collect();
    assert_eq!(pairs.len(), 1, "unique synthetic bearer head and guard");
    let records = pairs[0];
    for (path, _) in records {
        std::fs::write(path, b"corrupt").expect("damage bearer fixture");
    }
    bearer_status.store(403, Ordering::SeqCst);
    control.code.store(503, Ordering::SeqCst);
    // Shutdown must also complete when an agent stops reading a response larger than its pipe buffer.
    {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        let blocked = Process::spawn_protocol(&gateway_spec).expect("backpressure gateway");
        let mut blocked = blocked;
        let (read, mut write) = blocked.take_protocol().expect("backpressure pipes");
        let mut read = tokio::io::BufReader::new(read);
        for request in [
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"blocked-output-agent","version":"1"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        ] {
            write
                .write_all((request.to_string() + "\n").as_bytes())
                .await
                .expect("raw standard request");
        }
        loop {
            let mut line = String::new();
            tokio::time::timeout(Duration::from_secs(35), read.read_line(&mut line))
                .await
                .expect("raw discovery bound")
                .expect("raw discovery");
            let frame: Value = serde_json::from_str(&line).expect("stdout protocol only");
            if frame["id"] == 2 {
                assert_eq!(frame["result"]["tools"].as_array().expect("tools").len(), 2);
                break;
            }
        }
        let before = remote_calls.load(Ordering::Relaxed);
        let call = json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":dhivon_mcp_gateway_cli::aggregation::alias("remote-fixture","echo"),"arguments":{"large":true}}});
        write
            .write_all((call.to_string() + "\n").as_bytes())
            .await
            .expect("large-result request");
        for _ in 0..200 {
            if remote_calls.load(Ordering::Relaxed) > before {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(remote_calls.load(Ordering::Relaxed), before + 1);
        tokio::time::sleep(Duration::from_millis(100)).await;
        let began = std::time::Instant::now();
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(blocked.id() as i32),
            nix::sys::signal::Signal::SIGTERM,
        )
        .expect("owned blocked gateway signal");
        for _ in 0..600 {
            if blocked
                .poll_exit()
                .expect("blocked gateway state")
                .is_some()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let observed = blocked.poll_exit().expect("blocked gateway exit");
        let status = blocked
            .terminate(Duration::from_secs(1))
            .await
            .expect("blocked gateway final reap");
        assert_eq!(observed, Some(Some(0)));
        assert!(status.success());
        assert!(began.elapsed() < Duration::from_secs(6));
        drop(read);
        drop(write);
    }
    assert!(
        refused.load(Ordering::SeqCst) > 0,
        "the server refused this bearer"
    );
    for (path, bytes) in records {
        std::fs::write(path, bytes).expect("restore saved ciphertext");
    }
    bearer_status.store(503, Ordering::SeqCst);
    let _ = ui
        .terminate(Duration::from_secs(5))
        .await
        .expect("owned UI stop");
    // A disconnected cold restart can use only the previously committed valid cache.
    control.code.store(503, Ordering::Relaxed);
    let mut offline = Process::spawn_protocol(&gateway_spec).expect("offline gateway");
    let (read, write) = offline.take_protocol().expect("offline STDIO");
    let agent = ClientConfig::new(
        ClientCapabilities::default(),
        Implementation::new("offline-standard-client", "1"),
    )
    .with_protocol_version(ProtocolVersion::V_2025_11_25)
    .serve(AsyncRwTransport::new_client(read, write))
    .await
    .expect("valid offline startup");
    assert_eq!(
        agent
            .list_all_tools()
            .await
            .expect("offline cached tools")
            .len(),
        2
    );
    assert!(
        agent
            .call_tool(CallToolRequestParams::new(
                dhivon_mcp_gateway_cli::aggregation::alias("remote-fixture", "echo")
            ))
            .await
            .is_ok()
    );
    assert!(
        agent.call_tool(bearer_call()).await.is_err(),
        "restored bearer denied after restart"
    );
    bearer_status.store(200, Ordering::SeqCst);
    let polls = control.polls.load(Ordering::Relaxed);
    {
        let mut snapshot = control.snapshot.lock().expect("server policy");
        snapshot.mcps[1].enabled = false;
        snapshot.mcps[2].enabled = false;
        snapshot.mcps[3].enabled = false;
        snapshot.revision += 1;
    }
    control.code.store(200, Ordering::Relaxed);
    for _ in 0..200 {
        if control.polls.load(Ordering::Relaxed) > polls
            && agent
                .list_tools(None)
                .await
                .expect("reconnected catalog")
                .tools
                .is_empty()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        agent
            .list_tools(None)
            .await
            .expect("reconnected policy")
            .tools
            .is_empty()
    );
    assert!(
        agent
            .call_tool(CallToolRequestParams::new(
                dhivon_mcp_gateway_cli::aggregation::alias("remote-fixture", "echo")
            ))
            .await
            .is_err()
    );
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(offline.id() as i32),
        nix::sys::signal::Signal::SIGINT,
    )
    .expect("only owned offline gateway signal");
    for _ in 0..300 {
        if offline.poll_exit().expect("offline gateway exit").is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        offline
            .terminate(Duration::from_secs(1))
            .await
            .expect("offline final reap")
            .success()
    );
    let _ = agent.cancel().await;
    // Interrupted upstream initialization cannot detach its task or strand its process group.
    {
        let mut pending = local(&local_pids);
        pending.args[2] = pending.args[2].replace(
            "for line in sys.stdin:",
            "import time; time.sleep(120)\nfor line in sys.stdin:",
        );
        {
            let mut snapshot = control.snapshot.lock().expect("pending startup policy");
            snapshot.mcps[0].definition.transport = Transport::Stdio { command: pending };
            snapshot.mcps[0].enabled = true;
            snapshot.revision += 1;
        }
        std::fs::remove_file(&local_pids).expect("clear verified exited PID witness");
        let ui = Process::spawn(&command(
            cli_binary(),
            vec![
                "ui".into(),
                "--server".into(),
                origin,
                "--allow-fixture-http".into(),
                "--fixture-state-path".into(),
                files.state().to_str().expect("state").into(),
                "--port".into(),
                port.to_string(),
                "--no-open".into(),
            ],
        ))
        .expect("pending startup UI");
        let _ = local_login(&base, &client).await;
        let mut pending_gateway =
            Process::spawn_protocol(&gateway_spec).expect("pending startup gateway");
        let (read, write) = pending_gateway
            .take_protocol()
            .expect("pending gateway STDIO");
        let agent = ClientConfig::new(
            ClientCapabilities::default(),
            Implementation::new("pending-startup-agent", "1"),
        )
        .with_protocol_version(ProtocolVersion::V_2025_11_25)
        .serve(AsyncRwTransport::new_client(read, write))
        .await
        .expect("agent handshake precedes upstream startup");
        let owned = pids(&local_pids).await;
        let began = std::time::Instant::now();
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pending_gateway.id() as i32),
            nix::sys::signal::Signal::SIGTERM,
        )
        .expect("owned pending gateway signal");
        for _ in 0..600 {
            if pending_gateway
                .poll_exit()
                .expect("pending gateway state")
                .is_some()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let observed = pending_gateway.poll_exit().expect("pending gateway exit");
        let status = pending_gateway
            .terminate(Duration::from_secs(1))
            .await
            .expect("pending final reap");
        assert_eq!(observed, Some(Some(0)));
        assert!(status.success());
        assert!(began.elapsed() < Duration::from_secs(6));
        gone(&owned).await;
        let _ = agent.cancel().await;
        let _ = ui
            .terminate(Duration::from_secs(5))
            .await
            .expect("pending UI stop");
    }
    cleanup(&files).await;
    central_task.abort();
    remote_task.abort();
}
