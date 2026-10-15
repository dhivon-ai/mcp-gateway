mod common;
use common::{Harness, PASSWORD};
use gateway_core::EndpointSession;
use std::{process::Stdio, time::Duration};
use tokio::{io::AsyncWriteExt, process::Command};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires isolated TEST_DATABASE_URL"]
async fn operator_commands_and_live_http_keep_credentials_private() {
    let h = Harness::new().await;
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&h.state.pool)
        .await
        .expect("owned database");
    let mut dsn =
        url::Url::parse(&std::env::var("TEST_DATABASE_URL").expect("fixture DSN")).expect("DSN");
    dsn.set_path(&database);
    let file = std::env::temp_dir().join(format!(
        "mcp-gateway-operator-fixture-{}",
        Uuid::new_v4().simple()
    ));
    let binary = env!("CARGO_BIN_EXE_mcp-gateway-server");
    let output = Command::new(binary)
        .args(["generate-vault-key", "--output"])
        .arg(&file)
        .output()
        .await
        .expect("key generator");
    assert!(output.status.success());
    assert!(gateway_server::vault::load_key_file(&file).is_ok());
    let private: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).expect("key file")).expect("key format");
    let encoded = private["keys"]
        .as_object()
        .expect("keys")
        .values()
        .next()
        .expect("active key")
        .as_str()
        .expect("encoded key");
    assert!(!String::from_utf8_lossy(&output.stdout).contains(encoded));
    assert!(
        !Command::new(binary)
            .args(["generate-vault-key", "--output"])
            .arg(&file)
            .output()
            .await
            .expect("repeat generator")
            .status
            .success()
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("owned port");
    let bind = listener.local_addr().expect("address").to_string();
    drop(listener);
    let origin = format!("http://{bind}");
    for attempt in 0..2 {
        let mut child = Command::new(binary)
            .env("DATABASE_URL", dsn.as_str())
            .env("GATEWAY_PUBLIC_URL", &origin)
            .env("GATEWAY_VAULT_KEY_FILE", &file)
            .args([
                "--allow-loopback-http",
                "bootstrap-admin",
                "--email",
                "operator@example.test",
                "--name",
                "Operator",
                "--password-stdin",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("bootstrap process");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(PASSWORD.as_bytes())
            .await
            .expect("password stdin");
        let output = tokio::time::timeout(Duration::from_secs(10), child.wait_with_output())
            .await
            .expect("bootstrap deadline")
            .expect("bootstrap output");
        assert_eq!(output.status.success(), attempt == 0);
        assert!(!String::from_utf8_lossy(&output.stdout).contains(PASSWORD));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(PASSWORD));
    }
    let child = Command::new(binary)
        .env("DATABASE_URL", dsn.as_str())
        .env("GATEWAY_PUBLIC_URL", &origin)
        .env("GATEWAY_VAULT_KEY_FILE", &file)
        .args(["--allow-loopback-http", "--bind", &bind])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("service process");
    let pid = child.id().expect("owned process ID");
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("client");
    let mut ready = false;
    for _ in 0..100 {
        if http
            .get(format!("{origin}/healthz"))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(ready, "Service health did not become ready");
    let response = http
        .post(format!("{origin}/api/endpoint/login"))
        .json(&serde_json::json!({"email":"operator@example.test","password":PASSWORD}))
        .send()
        .await
        .expect("live login");
    assert!(response.status().is_success());
    let session: EndpointSession = response.json().await.expect("native session");
    let before = gateway_server::epoch().expect("time");
    assert!(session.expires_at <= before + 43200 && session.expires_at >= before + 43190);
    let response = http
        .get(format!("{origin}/api/config"))
        .bearer_auth(&session.token)
        .send()
        .await
        .expect("snapshot");
    assert!(response.status().is_success());
    let config: gateway_core::ConfigSnapshot = response.json().await.expect("config");
    assert!(config.mcps.is_empty());
    assert_eq!(config.expires_at - config.validated_at, 3600);
    let exit = Command::new("/bin/kill")
        .arg("-INT")
        .arg(pid.to_string())
        .status()
        .await
        .expect("interrupt owned child");
    assert!(exit.success());
    let output = tokio::time::timeout(Duration::from_secs(5), child.wait_with_output())
        .await
        .expect("shutdown deadline")
        .expect("shutdown");
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let logs = String::from_utf8_lossy(&output.stderr);
    assert!(!logs.contains(PASSWORD));
    assert!(!logs.contains(encoded));
    assert!(!logs.contains(&session.token));
    std::fs::remove_file(&file).expect("owned key cleanup");
    h.cleanup().await;
}
