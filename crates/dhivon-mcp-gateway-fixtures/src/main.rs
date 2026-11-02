use clap::Parser;
use dhivon_mcp_gateway_fixtures::{EchoServer, FixtureState, RecordedServer, http_router};
use rmcp::{ServiceExt, transport::stdio};
use std::net::SocketAddr;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value="stdio", value_parser=["stdio","http"])]
    transport: String,
    #[arg(long, default_value = "127.0.0.1:18081")]
    bind: SocketAddr,
    #[arg(long, default_value="none", value_parser=["none","bearer","oauth"])]
    auth: String,
    #[arg(long)]
    tag: Option<String>,
    #[arg(long)]
    call_log: Option<std::path::PathBuf>,
    /// Loopback HTTPS origin advertised behind an isolated TLS test proxy.
    #[arg(long)]
    public_url: Option<url::Url>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.transport == "stdio" {
        if args.auth != "none" || args.public_url.is_some() {
            anyhow::bail!("STDIO fixture does not use HTTP authentication");
        }
        if args.tag.is_some() || args.call_log.is_some() {
            let tag = args.tag.unwrap_or_else(|| "fixture".into());
            if tag.len() > 64 || !tag.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                anyhow::bail!("Invalid fixture tag");
            }
            RecordedServer {
                tag,
                call_log: args.call_log,
            }
            .serve(stdio())
            .await?
            .waiting()
            .await?;
        } else {
            EchoServer.serve(stdio()).await?.waiting().await?;
        }
    } else {
        if !args.bind.ip().is_loopback() {
            anyhow::bail!("Fixtures must bind to loopback");
        }
        let listener = tokio::net::TcpListener::bind(args.bind).await?;
        let addr = listener.local_addr()?;
        let base = if let Some(url) = args.public_url {
            if url.scheme() != "https"
                || !matches!(url.host_str(), Some("localhost" | "127.0.0.1"))
                || url.port().is_none()
                || url.path() != "/"
                || url.query().is_some()
                || url.fragment().is_some()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                anyhow::bail!("TLS fixtures require a loopback HTTPS origin with an explicit port");
            }
            url.origin().ascii_serialization()
        } else {
            format!("http://{addr}")
        };
        let state = FixtureState::new(base, args.auth);
        axum::serve(listener, http_router(state, addr.to_string()))
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await?;
    }
    Ok(())
}
