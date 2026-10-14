use clap::Parser;
use gateway_fixtures::{EchoServer, FixtureState, http_router};
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.transport == "stdio" {
        if args.auth != "none" {
            anyhow::bail!("STDIO fixture does not use HTTP authentication");
        }
        EchoServer.serve(stdio()).await?.waiting().await?;
    } else {
        if !args.bind.ip().is_loopback() {
            anyhow::bail!("Fixtures must bind to loopback");
        }
        let listener = tokio::net::TcpListener::bind(args.bind).await?;
        let addr = listener.local_addr()?;
        let state = FixtureState::new(format!("http://{addr}"), args.auth);
        axum::serve(listener, http_router(state, addr.to_string()))
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await?;
    }
    Ok(())
}
