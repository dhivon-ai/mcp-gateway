use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use gateway_server::{AppState, Options};

#[derive(Parser)]
#[command(
    name = "mcp-gateway-server",
    version,
    about = "Enterprise MCP control plane"
)]
struct Args {
    #[arg(long, env = "GATEWAY_BIND", default_value = "127.0.0.1:8080")]
    bind: SocketAddr,
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,
    #[arg(long, env = "GATEWAY_PUBLIC_URL")]
    public_url: String,
    #[arg(long, hide = true)]
    allow_loopback_http: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Bootstrap the first administrator in an empty deployment.
    BootstrapAdmin {
        #[arg(long)]
        username: String,
        /// Read password from standard input. Never pass it as a CLI argument.
        #[arg(long, required = true)]
        password_stdin: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let args = Args::parse();
    let state = AppState::connect(
        &args.database_url,
        Options {
            public_url: args.public_url,
            allow_loopback_http: args.allow_loopback_http,
        },
    )
    .await?;
    if let Some(Command::BootstrapAdmin {
        username,
        password_stdin: _,
    }) = args.command
    {
        use std::io::Read;
        let mut password = String::new();
        std::io::stdin().take(1024).read_to_string(&mut password)?;
        let user = gateway_server::identity::bootstrap(
            &state,
            &username,
            password.trim_end_matches(['\n', '\r']).into(),
        )
        .await?;
        println!("Administrator created: {} ({})", user.username, user.id);
        state.pool.close().await;
        return Ok(());
    }
    let router = gateway_server::router(state.clone());
    let listener = tokio::net::TcpListener::bind(args.bind).await?;
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    state.pool.close().await;
    Ok(())
}
