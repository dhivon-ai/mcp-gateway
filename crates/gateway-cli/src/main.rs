use clap::{Parser, Subcommand};
use gateway_cli::{
    cache::SnapshotCache,
    central::CentralClient,
    clock::NativeClock,
    credentials::MacKeychain,
    management::{ManagementState, router},
    session::SessionManager,
    store::StateStore,
    sync::{NativeTransport, SyncEngine},
};
use std::sync::Arc;

#[derive(Parser)]
#[command(
    name = "mcp-gateway",
    version,
    about = "Enterprise MCP gateway for macOS"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Start browser-based login and MCP management on loopback.
    Ui {
        #[arg(long)]
        server: String,
        /// Explicit isolated loopback HTTP fixture exception. Never use for production.
        #[arg(long)]
        allow_fixture_http: bool,
        /// Bind a fixed loopback port; zero chooses an available port.
        #[arg(long, default_value_t = 0)]
        port: u16,
        /// Print the local page address without opening the default browser.
        #[arg(long)]
        no_open: bool,
    },
    /// Serve coding agents over STDIO using an authenticated configuration.
    Mcp,
    /// Print redacted local configuration status.
    Status,
    /// Remove local authentication and request online session revocation.
    Logout {
        #[arg(long)]
        allow_fixture_http: bool,
    },
}
fn manager(allow_fixture_http: bool) -> gateway_cli::Result<SessionManager> {
    Ok(SessionManager::new(
        StateStore::production(Arc::new(MacKeychain::production()))?,
        allow_fixture_http,
    ))
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    match Args::parse().command {
        Command::Status => {
            let sessions = manager(false)?;
            let engine = SyncEngine::new(
                SnapshotCache::new(sessions, Arc::new(NativeClock::new()?)),
                Arc::new(NativeTransport),
            );
            println!("{}", serde_json::to_string(&engine.status()?)?);
        }
        Command::Mcp => {
            // P4 provides aggregation. No stdin interaction or browser opening, including cold startup.
            anyhow::bail!(
                "No usable authenticated configuration. Run mcp-gateway ui --server <url> and synchronize."
            )
        }
        Command::Logout { allow_fixture_http } => manager(allow_fixture_http)?.logout().await?,
        Command::Ui {
            server,
            allow_fixture_http,
            port,
            no_open,
        } => {
            let central = CentralClient::new(&server, allow_fixture_http)?;
            let listener =
                tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
            let sessions = manager(allow_fixture_http)?.select_server(&central).await?;
            let engine = SyncEngine::new(
                SnapshotCache::new(sessions.clone(), Arc::new(NativeClock::new()?)),
                Arc::new(NativeTransport),
            );
            let state = ManagementState::new(sessions, central, listener.local_addr()?, None)?
                .with_sync(engine.clone())?;
            let url = format!("{}/", state.origin());
            eprintln!("Local management: {url}");
            if !no_open {
                open_browser(&url).await?;
            }
            let (stop, stopped) = tokio::sync::watch::channel(false);
            let sync_task = tokio::spawn(async move {
                engine.run(stopped).await;
            });
            let serving = axum::serve(listener, router(state))
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await;
            let _ = stop.send(true);
            sync_task.abort();
            serving?;
        }
    }
    Ok(())
}
async fn open_browser(url: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let status = tokio::process::Command::new("/usr/bin/open")
            .arg(url)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await?;
        anyhow::ensure!(
            status.success(),
            "Could not open the browser; use the displayed local address."
        );
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = url;
        anyhow::bail!("Browser management requires macOS")
    }
}
