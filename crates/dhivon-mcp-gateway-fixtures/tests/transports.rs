use dhivon_mcp_gateway_fixtures::{FixtureState, http_router};
use rmcp::{
    ServiceExt,
    model::{
        CallToolRequestParams, ClientCapabilities, ClientConfig, Implementation, ProtocolVersion,
    },
    transport::{StreamableHttpClientTransport, TokioChildProcess},
};
use serde_json::json;
use tokio::process::Command;

fn client() -> ClientConfig {
    ClientConfig::new(
        ClientCapabilities::default(),
        Implementation::new("fixture-test", "0.1"),
    )
    .with_protocol_version(ProtocolVersion::V_2025_11_25)
}

#[tokio::test]
async fn stdio_discovery_pagination_echo_and_failure() -> anyhow::Result<()> {
    let transport = TokioChildProcess::new(Command::new(env!(
        "CARGO_BIN_EXE_dhivon-mcp-gateway-fixture"
    )))?;
    let peer = client().serve(transport).await?;
    let tools = peer.list_all_tools().await?;
    assert_eq!(
        tools.iter().map(|t| t.name.as_ref()).collect::<Vec<_>>(),
        ["echo", "fail"]
    );
    let args = json!({"message":"stdio fixture"})
        .as_object()
        .cloned()
        .expect("literal object");
    let result = peer
        .call_tool(CallToolRequestParams::new("echo").with_arguments(args))
        .await?;
    assert_eq!(
        result
            .content
            .first()
            .and_then(|c| c.as_text())
            .map(|c| c.text.as_str()),
        Some("stdio fixture")
    );
    assert_eq!(
        peer.call_tool(CallToolRequestParams::new("fail"))
            .await?
            .is_error,
        Some(true)
    );
    peer.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn streamable_http_discovery_and_auth_challenge() -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let base = format!("http://{addr}");
    let router = http_router(
        FixtureState::new(base.clone(), "none".into()),
        addr.to_string(),
    );
    let task = tokio::spawn(async move { axum::serve(listener, router).await });
    let transport = StreamableHttpClientTransport::from_uri(format!("{base}/mcp"));
    let peer = client().serve(transport).await?;
    assert_eq!(peer.list_all_tools().await?.len(), 2);
    peer.cancel().await?;
    task.abort();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let base = format!("http://{addr}");
    let router = http_router(
        FixtureState::new(base.clone(), "bearer".into()),
        addr.to_string(),
    );
    let task = tokio::spawn(async move { axum::serve(listener, router).await });
    let result = reqwest::Client::new()
        .post(format!("{base}/mcp"))
        .json(&json!({}))
        .send()
        .await?;
    assert_eq!(result.status(), 401);
    assert!(result.headers().get("www-authenticate").is_some());
    task.abort();
    Ok(())
}
