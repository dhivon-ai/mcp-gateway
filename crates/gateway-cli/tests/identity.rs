use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
#[cfg(target_os = "macos")]
use gateway_cli::credentials::MacKeychain;
use gateway_cli::{
    EndpointError,
    central::CentralClient,
    credentials::{CredentialKind, CredentialStore},
    epoch,
    session::SessionManager,
    store::{StateStore, check_private},
};
use gateway_core::{EndpointSession, LoginRequest, Role, UserInfo};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU16, Ordering},
    },
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
struct Fixture {
    root: PathBuf,
    manager: SessionManager,
}
impl Fixture {
    fn new(credentials: Arc<dyn CredentialStore>) -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .expect("canonical fixture parent")
            .join(format!("gateway-endpoint-owned-{}", Uuid::new_v4()));
        let store = StateStore::new(root.clone(), credentials).expect("private store");
        Self {
            root,
            manager: SessionManager::new(store, true),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self
            .manager
            .store
            .locked(|m| self.manager.store.clear_locked(m));
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
#[derive(Clone)]
struct Central {
    deployment: String,
    alice: String,
    bob: String,
    response: Arc<AtomicU16>,
    revoked: Arc<AtomicU16>,
}
struct Server {
    client: CentralClient,
    state: Central,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn server() -> Server {
    let state = Central {
        deployment: Uuid::new_v4().to_string(),
        alice: Uuid::new_v4().to_string(),
        bob: Uuid::new_v4().to_string(),
        response: Arc::new(AtomicU16::new(200)),
        revoked: Arc::new(AtomicU16::new(0)),
    };
    let app = Router::new()
        .route(
            "/api/endpoint/login",
            post(
                |State(s): State<Central>, axum::Json(input): axum::Json<LoginRequest>| async move {
                    axum::Json(EndpointSession {
                        token: format!(
                            "private-central-token-{}",
                            input.email.split('@').next().expect("fixture local part")
                        ),
                        user: UserInfo {
                            name: "Fixture User".into(),
                            id: if input.email == "alice@example.test" {
                                s.alice
                            } else {
                                s.bob
                            },
                            email: input.email,
                            role: Role::User,
                            active: true,
                        },
                        deployment_id: s.deployment,
                        expires_at: epoch().expect("clock") + 43200,
                    })
                },
            ),
        )
        .route(
            "/api/config",
            get(|State(s): State<Central>| async move {
                (
                    StatusCode::from_u16(s.response.load(Ordering::Relaxed))
                        .expect("fixture status"),
                    "{}",
                )
            }),
        )
        .route(
            "/api/auth/logout",
            post(|State(s): State<Central>| async move {
                s.revoked.fetch_add(1, Ordering::Relaxed);
                StatusCode::NO_CONTENT
            }),
        )
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let client = CentralClient::new(
        &format!("http://{}", listener.local_addr().expect("address")),
        true,
    )
    .expect("fixture client");
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("fixture server");
    });
    Server {
        client,
        state,
        task,
    }
}
fn login(name: &str) -> LoginRequest {
    LoginRequest {
        email: format!("{name}@example.test"),
        password: "never-persist-this-password".into(),
    }
}

#[tokio::test]
async fn identity_generation_cleanup_and_redacted_status() {
    let credentials = Arc::new(Memory::default());
    let f = Fixture::new(credentials.clone());
    let s = server().await;
    f.manager
        .login(&s.client, login("alice"))
        .await
        .expect("alice login");
    let first = f.manager.load().expect("session");
    let account = f
        .manager
        .save_credential(
            first.metadata.generation,
            CredentialKind::Upstream {
                mcp_id: Uuid::nil(),
                version: 1,
            },
            b"private-upstream-token",
        )
        .expect("credential");
    f.manager
        .save_credential(
            first.metadata.generation,
            CredentialKind::CacheKey,
            &[7; 32],
        )
        .expect("key");
    f.manager
        .store
        .locked(|_| {
            f.manager
                .store
                .atomic_write("snapshot.enc", b"ciphertext-fixture")
        })
        .expect("cache fixture");
    let metadata = std::fs::read_to_string(f.root.join("identity.json")).expect("metadata");
    let status = serde_json::to_string(&f.manager.status().expect("status")).expect("json");
    for value in [
        "never-persist-this-password",
        "private-central-token",
        "private-upstream-token",
    ] {
        assert!(!metadata.contains(value));
        assert!(!status.contains(value));
    }
    check_private(&f.root, true).expect("directory permissions");
    check_private(&f.root.join("identity.json"), false).expect("file permissions");
    f.manager
        .login(&s.client, login("bob"))
        .await
        .expect("bob login");
    let second = f.manager.load().expect("session");
    assert_ne!(first.metadata.generation, second.metadata.generation);
    assert_eq!(
        first.metadata.installation_id,
        second.metadata.installation_id
    );
    assert_ne!(first.metadata.active, second.metadata.active);
    assert!(credentials.get(&account).expect("keychain").is_none());
    assert!(!f.root.join("snapshot.enc").exists());
    assert!(matches!(
        f.manager.save_credential(
            first.metadata.generation,
            CredentialKind::CacheKey,
            &[8; 32]
        ),
        Err(EndpointError::IdentityChanged)
    ));
    f.manager
        .invalidate(first.metadata.generation)
        .expect("stale rejection cannot clear new user");
    assert_eq!(
        f.manager
            .load()
            .expect("bob remains")
            .credentials
            .user
            .email,
        "bob@example.test"
    );
    f.manager.logout().await.expect("logout");
    assert!(credentials.0.lock().expect("fixture lock").is_empty());
    assert_eq!(f.manager.status().expect("status").state, "login_required");
    assert_eq!(s.state.revoked.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn rejection_clears_but_outage_server_failure_and_forbidden_do_not() {
    let f = Fixture::new(Arc::new(Memory::default()));
    let s = server().await;
    f.manager
        .login(&s.client, login("alice"))
        .await
        .expect("login");
    for status in [500, 403, 302] {
        s.state.response.store(status, Ordering::Relaxed);
        assert!(matches!(
            f.manager
                .request(reqwest::Method::GET, "/api/config", None)
                .await,
            Err(EndpointError::Response)
        ));
        assert!(f.manager.load().is_ok());
    }
    s.task.abort();
    assert!(matches!(
        f.manager
            .request(reqwest::Method::GET, "/api/config", None)
            .await,
        Err(EndpointError::Offline)
    ));
    assert!(f.manager.load().is_ok());
    let s = server().await;
    f.manager
        .login(&s.client, login("alice"))
        .await
        .expect("login");
    s.state.response.store(401, Ordering::Relaxed);
    assert!(matches!(
        f.manager
            .request(reqwest::Method::GET, "/api/config", None)
            .await,
        Err(EndpointError::Rejected)
    ));
    assert_eq!(f.manager.status().expect("status").state, "login_required");
}

#[test]
fn url_and_storage_boundaries() {
    for url in [
        "http://example.com",
        "https://name:secret@example.com",
        "https://example.com/path",
        "https://example.com/?token=x",
        "https://example.com/#token",
        "http://127.0.0.1:1",
    ] {
        assert!(CentralClient::new(url, false).is_err());
    }
    assert!(CentralClient::new("http://127.0.0.1:1", true).is_ok());
    assert!(CentralClient::new("http://example.com", true).is_err());
    let f = Fixture::new(Arc::new(Memory::default()));
    let metadata_path = f.root.join("identity.json");
    let valid_metadata = std::fs::read(&metadata_path).expect("valid fixture metadata");
    let mut oversized_metadata = valid_metadata.clone();
    // Valid JSON plus whitespace crosses the boundary, so parser failure cannot stand in for the size check.
    oversized_metadata.resize(65537, b' ');
    std::fs::write(&metadata_path, &oversized_metadata).expect("oversized owned fixture");
    assert!(matches!(
        f.manager.store.read(),
        Err(EndpointError::Invalid)
    ));
    std::fs::write(&metadata_path, &valid_metadata).expect("restore owned fixture");
    assert!(f.manager.store.read().is_ok());
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        std::fs::set_permissions(
            f.root.join("identity.json"),
            std::fs::Permissions::from_mode(0o644),
        )
        .expect("fixture permissions");
        assert!(f.manager.store.read().is_err());
        std::fs::remove_file(f.root.join("identity.json")).expect("remove owned fixture");
        symlink("/etc/passwd", f.root.join("identity.json")).expect("symlink fixture");
        assert!(f.manager.store.read().is_err());
    }
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn actual_mac_keychain_replacement_logout_and_identity_isolation() {
    let keychain = Arc::new(MacKeychain::new(format!(
        "com.cadre.mcp-gateway.owned-test.{}",
        Uuid::new_v4()
    )));
    let f = Fixture::new(keychain.clone());
    let s = server().await;
    keychain
        .set("roundtrip", b"first-value")
        .expect("actual Keychain insert");
    assert_eq!(
        keychain
            .get("roundtrip")
            .expect("read")
            .expect("entry")
            .as_slice(),
        b"first-value"
    );
    keychain
        .set("roundtrip", b"second-value")
        .expect("actual Keychain replace");
    assert_eq!(
        keychain
            .get("roundtrip")
            .expect("read")
            .expect("entry")
            .as_slice(),
        b"second-value"
    );
    keychain
        .delete("roundtrip")
        .expect("actual Keychain delete");
    assert!(keychain.get("roundtrip").expect("read deleted").is_none());
    f.manager
        .login(&s.client, login("alice"))
        .await
        .expect("actual Keychain login");
    let first = f.manager.load().expect("actual Keychain session");
    let owned = f
        .manager
        .save_credential(
            first.metadata.generation,
            CredentialKind::Upstream {
                mcp_id: Uuid::nil(),
                version: 1,
            },
            b"fixture-upstream-token",
        )
        .expect("actual upstream entry");
    let other = Fixture::new(keychain.clone());
    let other_server = server().await;
    other
        .manager
        .login(&other_server.client, login("alice"))
        .await
        .expect("separate deployment login");
    let other_session = other.manager.load().expect("other session");
    let other_key = other
        .manager
        .save_credential(
            other_session.metadata.generation,
            CredentialKind::CacheKey,
            &[9; 32],
        )
        .expect("other key");
    assert_ne!(first.metadata.active, other_session.metadata.active);
    f.manager
        .login(&s.client, login("bob"))
        .await
        .expect("identity switch");
    assert!(
        keychain
            .get(&owned)
            .expect("deleted old identity")
            .is_none()
    );
    assert!(
        keychain
            .get(&other_key)
            .expect("separate identity survives")
            .is_some()
    );
    let new = f.manager.load().expect("new session");
    let new_key = f
        .manager
        .save_credential(new.metadata.generation, CredentialKind::CacheKey, &[7; 32])
        .expect("new key");
    f.manager.logout().await.expect("actual logout");
    assert!(keychain.get(&new_key).expect("logout deleted").is_none());
    other.manager.logout().await.expect("other logout");
    assert!(keychain.get(&other_key).expect("other deleted").is_none());
}

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
async fn local_request(
    app: &Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (StatusCode, axum::http::HeaderMap, String) {
    use tower::ServiceExt;
    let mut request = axum::http::Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(axum::body::Body::from(body.to_owned()))
                .expect("request"),
        )
        .await
        .expect("router");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), 65536)
        .await
        .expect("bounded body");
    (
        status,
        headers,
        String::from_utf8(bytes.to_vec()).expect("UTF8 fixture"),
    )
}
#[tokio::test]
async fn management_host_origin_local_cookie_csrf_callback_and_redaction() {
    use gateway_cli::management::{ManagementState, router};
    let logs = CapturedLogs::default();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(logs.clone())
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("single captured-log subscriber");
    let f = Fixture::new(Arc::new(Memory::default()));
    let s = server().await;
    let app = router(
        ManagementState::new(
            f.manager.clone(),
            s.client.clone(),
            "127.0.0.1:18123".parse().expect("address"),
            None,
        )
        .expect("management state"),
    );
    for headers in [
        vec![],
        vec![("host", "localhost:18123")],
        vec![
            ("host", "127.0.0.1:18123"),
            ("origin", "https://evil.example"),
        ],
        vec![
            ("host", "127.0.0.1:18123"),
            ("sec-fetch-site", "cross-site"),
        ],
        vec![("host", "127.0.0.1:18123"), ("sec-fetch-site", "same-site")],
    ] {
        assert_eq!(
            local_request(&app, "GET", "/", &headers, "").await.0,
            StatusCode::FORBIDDEN
        );
    }
    let (_, headers, page) = local_request(
        &app,
        "GET",
        "/",
        &[("host", "127.0.0.1:18123"), ("sec-fetch-site", "none")],
        "",
    )
    .await;
    assert!(!page.contains("private-central-token"));
    let raw = headers
        .get("set-cookie")
        .expect("cookie")
        .to_str()
        .expect("UTF8 cookie");
    assert!(raw.contains("HttpOnly; SameSite=Strict"));
    assert_eq!(headers["cache-control"], "no-store");
    assert!(
        headers["content-security-policy"]
            .to_str()
            .expect("CSP")
            .contains("frame-ancestors 'none'")
    );
    let cookie = raw.split(';').next().expect("cookie pair");
    assert_eq!(
        local_request(
            &app,
            "GET",
            "/api/context",
            &[("host", "127.0.0.1:18123")],
            ""
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (_, _, context) = local_request(
        &app,
        "GET",
        "/api/context",
        &[
            ("host", "127.0.0.1:18123"),
            ("cookie", cookie),
            ("sec-fetch-site", "same-origin"),
        ],
        "",
    )
    .await;
    let json: serde_json::Value = serde_json::from_str(&context).expect("context JSON");
    let csrf = json["csrfToken"].as_str().expect("CSRF");
    let body = serde_json::to_string(&login("alice")).expect("login JSON");
    for extra in [
        vec![],
        vec![("origin", "http://127.0.0.1:18123")],
        vec![
            ("origin", "http://127.0.0.1:18123"),
            ("x-csrf-token", "wrong"),
        ],
        vec![("x-csrf-token", csrf)],
        vec![("origin", "https://evil.example"), ("x-csrf-token", csrf)],
        vec![
            ("origin", "http://127.0.0.1:18123"),
            ("x-csrf-token", csrf),
            ("sec-fetch-site", "cross-site"),
        ],
    ] {
        let mut headers = vec![
            ("host", "127.0.0.1:18123"),
            ("cookie", cookie),
            ("content-type", "application/json"),
        ];
        headers.extend(extra);
        assert_eq!(
            local_request(&app, "POST", "/api/auth/login", &headers, &body)
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    let mutation = [
        ("host", "127.0.0.1:18123"),
        ("cookie", cookie),
        ("content-type", "application/json"),
        ("origin", "http://127.0.0.1:18123"),
        ("x-csrf-token", csrf),
    ];
    let (code, _, result) = local_request(&app, "POST", "/api/auth/login", &mutation, &body).await;
    assert_eq!(code, StatusCode::OK);
    let (_, _, read) = local_request(
        &app,
        "GET",
        "/api/status",
        &[("host", "127.0.0.1:18123"), ("cookie", cookie)],
        "",
    )
    .await;
    for text in [&context, &result, &read] {
        for value in [
            "private-central-token",
            "never-persist-this-password",
            "private-upstream-token",
        ] {
            assert!(!text.contains(value));
        }
    }
    let (code, _, malformed) = local_request(
        &app,
        "POST",
        "/api/auth/login",
        &mutation,
        "{\"password\":\"sensitive-malformed",
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(!malformed.contains("sensitive-malformed"));
    assert_eq!(
        local_request(
            &app,
            "GET",
            "/oauth/callback?code=untrusted&state=untrusted",
            &[
                ("host", "127.0.0.1:18123"),
                ("sec-fetch-site", "cross-site")
            ],
            ""
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        local_request(
            &app,
            "POST",
            "/oauth/callback",
            &[
                ("host", "127.0.0.1:18123"),
                ("sec-fetch-site", "cross-site")
            ],
            ""
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let generation = f
        .manager
        .load()
        .expect("logged in")
        .metadata
        .generation
        .to_string();
    let mut logout_headers = mutation.to_vec();
    logout_headers.push(("x-endpoint-generation", &generation));
    assert_eq!(
        local_request(&app, "POST", "/api/auth/logout", &logout_headers, "")
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        local_request(
            &app,
            "GET",
            "/api/status",
            &[("host", "127.0.0.1:18123"), ("cookie", cookie)],
            ""
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let captured = String::from_utf8(logs.0.lock().expect("logs").clone()).expect("UTF8 logs");
    assert!(captured.contains("endpoint_login"));
    assert!(captured.contains("endpoint_logout"));
    for value in [
        "private-central-token",
        "never-persist-this-password",
        csrf,
        "sensitive-malformed",
        raw,
    ] {
        assert!(!captured.contains(value));
    }
}

#[tokio::test]
async fn in_flight_login_cannot_resurrect_logout() {
    for selecting_other_server in [false, true] {
        let f = Fixture::new(Arc::new(Memory::default()));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let revoked = Arc::new(AtomicU16::new(0));
        let entered_handler = entered.clone();
        let release_handler = release.clone();
        let revoked_handler = revoked.clone();
        let app = Router::new()
            .route(
                "/api/endpoint/login",
                post(move || {
                    let entered = entered_handler.clone();
                    let release = release_handler.clone();
                    async move {
                        entered.notify_one();
                        release.notified().await;
                        axum::Json(EndpointSession {
                            token: "stale-central-session".into(),
                            user: UserInfo {
                                name: "Fixture User".into(),
                                id: Uuid::new_v4().to_string(),
                                email: "alice@example.test".into(),
                                role: Role::User,
                                active: true,
                            },
                            deployment_id: Uuid::new_v4().to_string(),
                            expires_at: epoch().expect("clock") + 43200,
                        })
                    }
                }),
            )
            .route(
                "/api/auth/logout",
                post(move || {
                    let revoked = revoked_handler.clone();
                    async move {
                        revoked.fetch_add(1, Ordering::Relaxed);
                        StatusCode::NO_CONTENT
                    }
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let client = CentralClient::new(
            &format!("http://{}", listener.local_addr().expect("address")),
            true,
        )
        .expect("fixture client");
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("server");
        });
        let manager = f.manager.clone();
        let pending = tokio::spawn(async move { manager.login(&client, login("alice")).await });
        entered.notified().await;
        if selecting_other_server {
            let new_server =
                CentralClient::new("http://127.0.0.1:2", true).expect("other configured origin");
            f.manager
                .select_server(&new_server)
                .await
                .expect("select other origin during login");
        } else {
            f.manager.logout().await.expect("logout during login");
        }
        release.notify_one();
        assert!(matches!(
            pending.await.expect("login task"),
            Err(EndpointError::IdentityChanged)
        ));
        assert_eq!(f.manager.status().expect("status").state, "login_required");
        assert_eq!(revoked.load(Ordering::Relaxed), 1);
        server.abort();
    }
}

#[test]
#[cfg(target_os = "macos")]
fn mcp_stdout_and_command_contract() {
    let binary = env!("CARGO_BIN_EXE_mcp-gateway");
    let root = std::env::temp_dir()
        .canonicalize()
        .expect("fixture parent")
        .join(format!("gateway-cold-mcp-owned-{}", Uuid::new_v4()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .expect("private fixture");
    }
    #[cfg(not(unix))]
    std::fs::create_dir(&root).expect("fixture");
    let output = std::process::Command::new(binary)
        .args(["mcp", "--allow-fixture-http", "--fixture-state-path"])
        .arg(&root)
        .output()
        .expect("MCP command");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No usable authenticated configuration")
    );
    let output = std::process::Command::new(binary)
        .arg("--help")
        .output()
        .expect("CLI help");
    let help = String::from_utf8_lossy(&output.stdout);
    for command in ["ui", "mcp", "status", "logout"] {
        assert!(help.contains(command));
    }
    for args in [
        ["--concurrency", "17"],
        ["--startup-timeout-seconds", "31"],
        ["--call-timeout-seconds", "61"],
    ] {
        let output = std::process::Command::new(binary)
            .arg("mcp")
            .args(args)
            .output()
            .expect("invalid bound request");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    std::fs::remove_dir_all(root).expect("owned cold fixture cleanup");
}

struct FailingDeletion {
    memory: Memory,
    fail: std::sync::atomic::AtomicBool,
}
impl CredentialStore for FailingDeletion {
    fn get(&self, key: &str) -> gateway_cli::Result<Option<Zeroizing<Vec<u8>>>> {
        self.memory.get(key)
    }
    fn set(&self, key: &str, bytes: &[u8]) -> gateway_cli::Result<()> {
        self.memory.set(key, bytes)
    }
    fn delete(&self, key: &str) -> gateway_cli::Result<()> {
        if self.fail.load(Ordering::Relaxed) {
            return Err(EndpointError::Keychain);
        }
        self.memory.delete(key)
    }
}
#[tokio::test]
async fn failed_keychain_cleanup_revokes_first_and_retries_owned_entries() {
    let credentials = Arc::new(FailingDeletion {
        memory: Memory::default(),
        fail: std::sync::atomic::AtomicBool::new(false),
    });
    let f = Fixture::new(credentials.clone());
    let s = server().await;
    f.manager
        .login(&s.client, login("alice"))
        .await
        .expect("login");
    let old = f.manager.load().expect("session");
    credentials.fail.store(true, Ordering::Relaxed);
    assert!(matches!(
        f.manager.logout().await,
        Err(EndpointError::Keychain)
    ));
    let metadata = f.manager.store.read().expect("recoverable cleanup journal");
    assert!(metadata.active.is_none());
    assert_ne!(old.metadata.generation, metadata.generation);
    assert_eq!(metadata.pending_cleanup.len(), 1);
    assert!(matches!(
        f.manager.load(),
        Err(EndpointError::LoginRequired)
    ));
    credentials.fail.store(false, Ordering::Relaxed);
    f.manager.logout().await.expect("retry cleanup");
    assert!(
        f.manager
            .store
            .read()
            .expect("metadata")
            .pending_cleanup
            .is_empty()
    );
    assert!(
        credentials
            .memory
            .0
            .lock()
            .expect("fixture lock")
            .is_empty()
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn real_cli_loopback_development_page_and_clean_stdio() {
    use std::io::{BufRead, BufReader};
    let fixture_home = std::env::temp_dir()
        .canonicalize()
        .expect("canonical temp")
        .join(format!("gateway-cli-home-owned-{}", Uuid::new_v4()));
    std::fs::create_dir(&fixture_home).expect("fixture home");
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(fixture_home.join("state"))
        .expect("private fixture state");
    let binary = env!("CARGO_BIN_EXE_mcp-gateway");
    let mut child = std::process::Command::new(binary)
        .args([
            "ui",
            "--server",
            "http://127.0.0.1:1",
            "--allow-fixture-http",
            "--no-open",
        ])
        .arg("--fixture-state-path")
        .arg(fixture_home.join("state"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("owned UI CLI");
    let stderr = child.stderr.take().expect("stderr");
    let mut line = String::new();
    BufReader::new(stderr)
        .read_line(&mut line)
        .expect("local URL");
    let url = line
        .strip_prefix("Local management: ")
        .expect("plain local URL")
        .trim();
    assert!(!url.contains('?'));
    assert!(!url.contains('#'));
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("fixture HTTP");
    let page = client.get(url).send().await.expect("real local HTTP");
    assert!(page.status().is_success());
    let cookie = page.headers()["set-cookie"]
        .to_str()
        .expect("cookie")
        .split(';')
        .next()
        .expect("cookie pair")
        .to_owned();
    let context = client
        .get(format!("{}api/context", url))
        .header("cookie", &cookie)
        .send()
        .await
        .expect("local context");
    assert!(context.status().is_success());
    assert!(
        !context
            .text()
            .await
            .expect("body")
            .contains("private-central-token")
    );
    let rejected = client
        .get(url)
        .header("host", "evil.example")
        .send()
        .await
        .expect("Host rejection");
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);
    child.kill().expect("terminate only owned UI CLI");
    let output = child.wait_with_output().expect("owned exit");
    assert!(output.stdout.is_empty());
    let status = std::process::Command::new(binary)
        .arg("status")
        .arg("--allow-fixture-http")
        .arg("--fixture-state-path")
        .arg(fixture_home.join("state"))
        .output()
        .expect("status");
    assert!(status.status.success());
    assert!(String::from_utf8_lossy(&status.stdout).contains("login_required"));
    std::fs::remove_dir_all(fixture_home).expect("clean fixture home");
}

#[tokio::test]
async fn configured_server_selection_preserves_same_origin_and_isolates_other_ui() {
    use gateway_cli::management::{ManagementState, router};
    let credentials = Arc::new(Memory::default());
    let f = Fixture::new(credentials.clone());
    let old = server().await;
    let new = server().await;
    let old_sessions = f
        .manager
        .select_server(&old.client)
        .await
        .expect("old startup selection");
    old_sessions
        .login(&old.client, login("alice"))
        .await
        .expect("old login");
    let first = old_sessions.load().expect("old session");
    let owned = old_sessions
        .save_credential(
            first.metadata.generation,
            CredentialKind::Upstream {
                mcp_id: Uuid::new_v4(),
                version: 1,
            },
            b"private-old-origin-token",
        )
        .expect("old credential");
    f.manager
        .store
        .locked(|_| {
            f.manager
                .store
                .atomic_write("snapshot.enc", b"old-origin-owned-cache-fixture")
        })
        .expect("old cache fixture");
    let same = f
        .manager
        .select_server(&old.client)
        .await
        .expect("same origin startup");
    assert_eq!(
        same.load().expect("preserved session").metadata.generation,
        first.metadata.generation
    );
    assert!(f.root.join("snapshot.enc").exists());
    assert!(
        credentials
            .get(&owned)
            .expect("preserved credential")
            .is_some()
    );
    assert_eq!(old.state.revoked.load(Ordering::Relaxed), 0);
    let old_app = router(
        ManagementState::new(
            old_sessions.clone(),
            old.client.clone(),
            "127.0.0.1:18125".parse().expect("address"),
            None,
        )
        .expect("old management"),
    );
    let (_, headers, _) =
        local_request(&old_app, "GET", "/", &[("host", "127.0.0.1:18125")], "").await;
    let cookie = headers["set-cookie"]
        .to_str()
        .expect("cookie")
        .split(';')
        .next()
        .expect("cookie pair");
    let selected = f
        .manager
        .select_server(&new.client)
        .await
        .expect("explicit new origin startup");
    assert!(matches!(selected.load(), Err(EndpointError::LoginRequired)));
    assert!(matches!(
        old_sessions.load(),
        Err(EndpointError::ServerMismatch)
    ));
    assert!(!f.root.join("snapshot.enc").exists());
    assert!(
        credentials
            .get(&owned)
            .expect("old credential retired")
            .is_none()
    );
    assert_eq!(old.state.revoked.load(Ordering::Relaxed), 1);
    selected
        .login(&new.client, login("bob"))
        .await
        .expect("new login");
    let current = selected.load().expect("new active identity");
    let before = std::fs::read(f.root.join("identity.json")).expect("current metadata");
    let (code, _, context) = local_request(
        &old_app,
        "GET",
        "/api/context",
        &[("host", "127.0.0.1:18125"), ("cookie", cookie)],
        "",
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    let context: serde_json::Value = serde_json::from_str(&context).expect("context");
    assert_eq!(context["status"]["state"], "server_mismatch");
    assert_eq!(context["serverUrl"], old.client.origin());
    assert!(context["status"]["identity"].is_null());
    assert!(matches!(
        old_sessions
            .request(reqwest::Method::GET, "/api/config", None)
            .await,
        Err(EndpointError::ServerMismatch)
    ));
    assert!(matches!(
        old_sessions.logout().await,
        Err(EndpointError::ServerMismatch)
    ));
    assert_eq!(
        std::fs::read(f.root.join("identity.json")).expect("read-only older UI"),
        before
    );
    assert_eq!(
        selected
            .load()
            .expect("new identity unaffected")
            .metadata
            .generation,
        current.metadata.generation
    );
    assert_eq!(new.state.revoked.load(Ordering::Relaxed), 0);
    old_sessions
        .login(&old.client, login("alice"))
        .await
        .expect("explicit old UI login may select old server");
    assert_eq!(new.state.revoked.load(Ordering::Relaxed), 1);
    assert!(matches!(
        selected.load(),
        Err(EndpointError::ServerMismatch)
    ));
}
