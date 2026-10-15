mod common;
use common::{Harness, PASSWORD};
#[cfg(not(target_os = "macos"))]
use gateway_cli::credentials::CredentialStore;
use gateway_cli::{
    aggregation::{Aggregator, alias},
    cache::SnapshotCache,
    central::CentralClient,
    clock::{Clock, ClockSample, NANOS_PER_SECOND},
    oauth_control::{ConnectInput, DisconnectInput, OAuthControl},
    remote_credentials::RemoteCredentials,
    session::SessionManager,
    store::StateStore,
    sync::{NativeTransport, SyncEngine},
};
use gateway_core::*;
use gateway_fixtures::{FixtureState, http_router};
use serde_json::json;
use std::sync::Arc;
#[cfg(not(target_os = "macos"))]
use std::{collections::HashMap, sync::Mutex};
use uuid::Uuid;
#[cfg(not(target_os = "macos"))]
use zeroize::Zeroizing;
#[cfg(not(target_os = "macos"))]
#[derive(Default)]
struct Memory(Mutex<HashMap<String, Vec<u8>>>);
#[cfg(not(target_os = "macos"))]
impl CredentialStore for Memory {
    fn get(&self, k: &str) -> gateway_cli::Result<Option<Zeroizing<Vec<u8>>>> {
        Ok(self
            .0
            .lock()
            .expect("keys")
            .get(k)
            .cloned()
            .map(Zeroizing::new))
    }
    fn set(&self, k: &str, v: &[u8]) -> gateway_cli::Result<()> {
        self.0.lock().expect("keys").insert(k.into(), v.into());
        Ok(())
    }
    fn delete(&self, k: &str) -> gateway_cli::Result<()> {
        self.0.lock().expect("keys").remove(k);
        Ok(())
    }
}
struct Time {
    boot: Uuid,
    start: std::time::Instant,
}
impl Clock for Time {
    fn sample(&self) -> gateway_cli::Result<ClockSample> {
        Ok(ClockSample {
            boot_id: self.boot,
            continuous_ns: NANOS_PER_SECOND + self.start.elapsed().as_nanos() as u64,
            wall_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("fixture wall")
                .as_nanos() as u64,
        })
    }
}
struct NativeFixture {
    root: std::path::PathBuf,
    store: StateStore,
}
impl Drop for NativeFixture {
    fn drop(&mut self) {
        if self.store.locked(|m| self.store.clear_locked(m)).is_err() {
            eprintln!(
                "Owned OAuth fixture cleanup journal retained: {}",
                self.root.display()
            );
            return;
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
#[ignore = "requires isolated TEST_DATABASE_URL"]
async fn consent_central_encryption_gateway_refresh_coordination_and_disconnect() {
    let remote = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("remote");
    let remote_address = remote.local_addr().expect("address");
    let remote_base = format!("http://{remote_address}");
    let delay_token = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let token_entered = Arc::new(tokio::sync::Notify::new());
    let token_release = Arc::new(tokio::sync::Notify::new());
    let delay = delay_token.clone();
    let entered = token_entered.clone();
    let release = token_release.clone();
    let remote_app = http_router(
        FixtureState::new(remote_base.clone(), "oauth".into()),
        remote_address.to_string(),
    )
    .layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let delay = delay.clone();
            let entered = entered.clone();
            let release = release.clone();
            async move {
                let held = request.uri().path() == "/token"
                    && delay.load(std::sync::atomic::Ordering::Relaxed);
                let response = next.run(request).await;
                if held {
                    entered.notify_one();
                    release.notified().await;
                }
                response
            }
        },
    ));
    let remote_task = tokio::spawn(axum::serve(remote, remote_app).into_future());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("central");
    let base = format!("http://{}", listener.local_addr().expect("address"));
    let h = Harness::with_public_url(&base).await;
    let admin = h.bootstrap().await;
    h.user(&admin.token, "alice").await;
    let alice = h.login("alice", PASSWORD).await;
    let (_, _, snapshot) = h
        .request("GET", "/api/config", Some(&admin.token), None)
        .await;
    let id = Uuid::new_v4().to_string();
    assert_eq!(h.request("POST","/api/admin/mcps",Some(&admin.token),Some(json!({"expectedRevision":snapshot["revision"],"definition":{"id":id,"name":"OAuth fixture","description":"PKCE protected MCP","active":true,"transport":{"kind":"http","url":format!("{remote_base}/mcp")},"auth":{"kind":"oauth","clientId":"gateway-fixture-client","scopes":["tools"]},"install":null,"toolPolicy":{"mode":"allowlist","tools":["echo"]}}}))).await.0,200);
    let (_, _, snapshot) = h
        .request("GET", "/api/config", Some(&admin.token), None)
        .await;
    assert_eq!(
        h.request(
            "POST",
            "/api/admin/assignments",
            Some(&admin.token),
            Some(
                json!({"expectedRevision":snapshot["revision"],"mcpId":id,"target":{"kind":"all"}})
            )
        )
        .await
        .0,
        200
    );
    let save_fault = Arc::new(std::sync::atomic::AtomicU8::new(0));
    let fault = save_fault.clone();
    let app = h.app.clone().layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let fault = fault.clone();
            async move {
                use axum::response::IntoResponse;
                let saving = request.method() == axum::http::Method::PUT
                    && request.uri().path().starts_with("/api/endpoint/oauth/");
                let mode = fault.load(std::sync::atomic::Ordering::Relaxed);
                if saving && mode == 1 {
                    return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
                let response = next.run(request).await;
                if saving && mode == 2 {
                    axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response()
                } else {
                    response
                }
            }
        },
    ));
    let central_task = tokio::spawn(axum::serve(listener, app).into_future());
    let root = std::env::temp_dir()
        .canonicalize()
        .expect("parent")
        .join(format!("gateway-oauth-owned-{}", Uuid::new_v4()));
    #[cfg(not(target_os = "macos"))]
    let memory = Arc::new(Memory::default());
    #[cfg(target_os = "macos")]
    let memory = {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir(&root).expect("owned private root");
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            .expect("permissions");
        Arc::new(
            gateway_cli::credentials::MacKeychain::fixture(&root)
                .expect("isolated real Mac Keychain"),
        )
    };
    let store = StateStore::new(root.clone(), memory).expect("store");
    let native = NativeFixture {
        root,
        store: store.clone(),
    };
    let central = CentralClient::new(&base, true).expect("client");
    let sessions = SessionManager::new(store, true)
        .select_server(&central)
        .await
        .expect("select");
    sessions
        .login(
            &central,
            LoginRequest {
                email: "alice@example.test".into(),
                password: PASSWORD.into(),
            },
        )
        .await
        .expect("native login");
    let time = Arc::new(Time {
        boot: Uuid::new_v4(),
        start: std::time::Instant::now(),
    });
    let sync = SyncEngine::new(
        SnapshotCache::new(sessions.clone(), time.clone()),
        Arc::new(NativeTransport),
    );
    sync.refresh().await.expect("sync");
    let generation = sessions
        .load()
        .expect("session")
        .metadata
        .generation
        .to_string();
    let control = OAuthControl::new(
        sync.clone(),
        "http://127.0.0.1:19001/oauth/callback".into(),
        true,
    );
    let browser = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("browser");
    let before = sync.cache.latest().expect("proof").snapshot.revision;
    let abandoned = control
        .start(
            &generation,
            &id,
            ConnectInput {
                expected_revision: before,
            },
        )
        .await
        .expect("pending consent without a stored record");
    let response = browser
        .get(&abandoned.authorization_url)
        .send()
        .await
        .expect("consent redirect");
    let abandoned_callback = response.headers()["location"]
        .to_str()
        .expect("callback")
        .to_owned();
    control
        .disconnect(
            &generation,
            &id,
            DisconnectInput {
                expected_revision: before,
                expected_version: 0,
            },
        )
        .await
        .expect("disconnect before any token exists");
    assert!(
        control.callback(&abandoned_callback).await.is_err(),
        "disconnect must fence an uncommitted consent"
    );
    assert!(
        !control
            .status(&generation, &id)
            .await
            .expect("disconnected")
            .connected
    );
    let revision = sync.cache.latest().expect("proof").snapshot.revision;
    let start = control
        .start(
            &generation,
            &id,
            ConnectInput {
                expected_revision: revision,
            },
        )
        .await
        .expect("start consent");
    let before_poll = sync.cache.latest().expect("pending proof").snapshot;
    let pending_status = control
        .status(&generation, &id)
        .await
        .expect("pending status");
    assert_eq!(pending_status.readiness, "authorization_pending");
    assert!(!pending_status.connected);
    assert_eq!(
        sync.cache
            .latest()
            .expect("proof after poll")
            .snapshot
            .validated_at,
        before_poll.validated_at,
        "ephemeral consent polling must not rotate the callback's protected policy receipt"
    );
    let response = browser
        .get(start.authorization_url)
        .send()
        .await
        .expect("consent");
    let callback = response.headers()["location"].to_str().expect("callback");
    let status = control
        .callback(callback)
        .await
        .expect("confirmed persisted consent");
    assert_eq!(status.version, Some(1));
    let proof = sync.cache.latest().expect("proof");
    let installed = sessions
        .load()
        .expect("session")
        .metadata
        .installation_id
        .to_string();
    assert_eq!(
        h.request(
            "POST",
            &format!("/api/endpoint/credentials/{id}"),
            Some(&alice.token),
            Some(json!({"expectedRevision":proof.snapshot.revision,"installationId":installed}))
        )
        .await
        .0,
        403,
        "normal agent credential endpoint requires enablement"
    );
    assert_eq!(
        h.request(
            "POST",
            &format!("/api/endpoint/oauth/{id}/credentials"),
            None,
            Some(json!({"expectedRevision":proof.snapshot.revision,"installationId":installed}))
        )
        .await
        .0,
        401,
        "consent recovery requires native identity"
    );
    let retry = control
        .start(
            &generation,
            &id,
            ConnectInput {
                expected_revision: proof.snapshot.revision,
            },
        )
        .await
        .expect("reconsent start");
    let response = browser
        .get(&retry.authorization_url)
        .send()
        .await
        .expect("provider consent");
    let cancelled_callback = response.headers()["location"].to_str().expect("callback");
    control
        .cancel(&generation, &retry.flow_id)
        .await
        .expect("cancel consent");
    assert!(control.callback(cancelled_callback).await.is_err());
    let proof = sync.cache.latest().expect("proof");
    let in_flight = control
        .start(
            &generation,
            &id,
            ConnectInput {
                expected_revision: proof.snapshot.revision,
            },
        )
        .await
        .expect("in-flight consent");
    let response = browser
        .get(&in_flight.authorization_url)
        .send()
        .await
        .expect("consent");
    let callback = response.headers()["location"]
        .to_str()
        .expect("callback")
        .to_owned();
    delay_token.store(true, std::sync::atomic::Ordering::Relaxed);
    let exchange_control = control.clone();
    let exchange = tokio::spawn(async move { exchange_control.callback(&callback).await });
    tokio::time::timeout(std::time::Duration::from_secs(10), token_entered.notified())
        .await
        .expect("provider processed the code before cancellation");
    assert_eq!(
        control
            .status(&generation, &id)
            .await
            .expect("status during exchange")
            .readiness,
        "authorization_pending"
    );
    control
        .cancel(&generation, &in_flight.flow_id)
        .await
        .expect("cancel an exchange in progress");
    delay_token.store(false, std::sync::atomic::Ordering::Relaxed);
    token_release.notify_one();
    assert!(
        exchange.await.expect("owned exchange").is_err(),
        "issued response after cancellation cannot replace the working central credential"
    );
    assert_eq!(
        control
            .status(&generation, &id)
            .await
            .expect("working credential preserved")
            .version,
        Some(1)
    );
    let cipher: Vec<u8> = sqlx::query_scalar(
        "SELECT ciphertext FROM oauth_credentials WHERE user_id=$1 AND mcp_id=$2",
    )
    .bind(Uuid::parse_str(&alice.user.id).expect("user"))
    .bind(Uuid::parse_str(&id).expect("MCP"))
    .fetch_one(&h.state.pool)
    .await
    .expect("encrypted token");
    assert!(!String::from_utf8_lossy(&cipher).contains("accessToken"));
    let proof = sync.cache.latest().expect("confirmed config");
    assert!(
        !proof.snapshot.mcps[0].enabled,
        "consent never silently enables"
    );
    gateway_cli::bridge::ControlBridge::new(sync.clone())
        .settings(
            &generation,
            &id,
            SettingsUpdate {
                expected_revision: proof.snapshot.revision,
                enabled: true,
                secret_id: None,
            },
        )
        .await
        .expect("explicit enable");
    let gateway = Aggregator::new(sync.cache.clone(), true)
        .expect("gateway")
        .with_credentials(Arc::new(RemoteCredentials::new(sync.clone())));
    gateway.reconcile().await.expect("OAuth discovery");
    assert_eq!(gateway.list().expect("tools").len(), 1);
    gateway
        .call(
            rmcp::model::CallToolRequestParams::new(alias(&id, "echo")).with_arguments(
                serde_json::Map::from_iter([("message".into(), json!("protected"))]),
            ),
        )
        .await
        .expect("OAuth tool call");
    let generation_id = Uuid::parse_str(&generation).expect("generation");
    let (first, second) = tokio::join!(
        control.refresh(generation_id, &id),
        control.refresh(generation_id, &id)
    );
    assert!(
        first.is_ok() || second.is_ok(),
        "one coordinated refresh wins"
    );
    let status = control.status(&generation, &id).await.expect("status");
    assert_eq!(
        status.version,
        Some(2),
        "one-use provider refresh is serialized centrally"
    );
    gateway.reconcile().await.expect("rotated connection");
    assert_eq!(gateway.list().expect("rotated tools").len(), 1);
    save_fault.store(1, std::sync::atomic::Ordering::Relaxed);
    assert!(
        control.refresh(generation_id, &id).await.is_err(),
        "failed central persist retains complete protected candidate"
    );
    assert!(
        sessions
            .store
            .read()
            .expect("metadata")
            .credential_accounts
            .iter()
            .any(|account| account.contains(":oauth-save:"))
    );
    assert!(gateway.list().expect("pending local fault").is_empty());
    save_fault.store(0, std::sync::atomic::Ordering::Relaxed);
    control
        .recover(generation_id, &id)
        .await
        .expect("recover rotated response without another provider exchange");
    assert_eq!(
        control
            .status(&generation, &id)
            .await
            .expect("recovered")
            .version,
        Some(3)
    );
    save_fault.store(2, std::sync::atomic::Ordering::Relaxed);
    assert!(
        control.refresh(generation_id, &id).await.is_err(),
        "unknown committed result retains journal"
    );
    save_fault.store(0, std::sync::atomic::Ordering::Relaxed);
    let restarted = OAuthControl::new(
        sync.clone(),
        "http://127.0.0.1:19001/oauth/callback".into(),
        true,
    );
    restarted
        .recover(generation_id, &id)
        .await
        .expect("restart confirms already committed rotated credentials");
    assert_eq!(
        control
            .status(&generation, &id)
            .await
            .expect("status")
            .version,
        Some(4),
        "no duplicate central commit"
    );
    assert!(
        !sessions
            .store
            .read()
            .expect("metadata")
            .credential_accounts
            .iter()
            .any(|account| account.contains(":oauth-save:"))
    );
    save_fault.store(1, std::sync::atomic::Ordering::Relaxed);
    assert!(
        control.refresh(generation_id, &id).await.is_err(),
        "disconnecting must also remove a complete pending save"
    );
    save_fault.store(0, std::sync::atomic::Ordering::Relaxed);
    let proof = sync.cache.latest().expect("proof");
    control
        .disconnect(
            &generation,
            &id,
            DisconnectInput {
                expected_revision: proof.snapshot.revision,
                expected_version: 4,
            },
        )
        .await
        .expect("disconnect");
    control
        .recover(generation_id, &id)
        .await
        .expect("no pending candidate can resurrect disconnect");
    assert!(
        !sessions
            .store
            .read()
            .expect("metadata")
            .credential_accounts
            .iter()
            .any(|account| account.contains(":oauth-save:"))
    );
    assert!(gateway.list().expect("confirmed disabled list").is_empty());
    assert!(
        !control
            .status(&generation, &id)
            .await
            .expect("disconnected")
            .connected
    );
    let proof = sync.cache.latest().expect("proof after disconnect");
    let reconnect = control
        .start(
            &generation,
            &id,
            ConnectInput {
                expected_revision: proof.snapshot.revision,
            },
        )
        .await
        .expect("same-generation reconnect");
    let response = browser
        .get(&reconnect.authorization_url)
        .send()
        .await
        .expect("new consent");
    let callback = response.headers()["location"].to_str().expect("callback");
    assert_eq!(
        control
            .callback(callback)
            .await
            .expect("new record after deletion")
            .version,
        Some(1)
    );
    let proof = sync.cache.latest().expect("reconnected config");
    gateway_cli::bridge::ControlBridge::new(sync.clone())
        .settings(
            &generation,
            &id,
            SettingsUpdate {
                expected_revision: proof.snapshot.revision,
                enabled: true,
                secret_id: None,
            },
        )
        .await
        .expect("explicit enable after reconnect");
    gateway.reconcile().await.expect("reconnect discovery");
    assert_eq!(
        gateway.list().expect("reconnected tools").len(),
        1,
        "retired native record must not reject the new record's version reset"
    );
    gateway.close().await;
    sessions.logout().await.expect("cleanup");
    drop(native);
    central_task.abort();
    remote_task.abort();
    h.cleanup().await;
}
