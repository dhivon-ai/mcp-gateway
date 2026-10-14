use std::net::SocketAddr;

use axum::{Json, Router, routing::get};
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "mcp-gateway-server",
    version,
    about = "Enterprise MCP control plane"
)]
struct Args {
    #[arg(long, env = "GATEWAY_BIND", default_value = "127.0.0.1:8080")]
    bind: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let args = Args::parse();
    let router = Router::new().route(
        "/healthz",
        get(|| async { Json(serde_json::json!({ "status": "ok" })) }),
    );
    let listener = tokio::net::TcpListener::bind(args.bind).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
