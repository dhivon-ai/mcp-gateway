use std::{net::SocketAddr, path::PathBuf};

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
    database_url: Option<String>,
    #[arg(long, env = "GATEWAY_PUBLIC_URL")]
    public_url: Option<String>,
    #[arg(long, env = "GATEWAY_VAULT_KEY_FILE")]
    vault_key_file: Option<PathBuf>,
    #[arg(long, hide = true)]
    allow_loopback_http: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a private external encryption key file without displaying its value.
    GenerateVaultKey {
        #[arg(long)]
        output: PathBuf,
    },
    /// Reencrypt records under the active key, retaining values and versions.
    RotateVault,
    /// Bootstrap the first administrator in an empty deployment.
    BootstrapAdmin {
        #[arg(long)]
        email: String,
        #[arg(long)]
        name: String,
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
    if let Some(Command::GenerateVaultKey { output }) = &args.command {
        use std::io::Write;
        #[cfg(unix)]
        use std::os::unix::fs::OpenOptionsExt;
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        opts.mode(0o600);
        let bytes = gateway_core::crypto::KeyRing::generate_key_file(&format!(
            "key-{}",
            uuid::Uuid::new_v4().simple()
        ))?;
        let mut file = opts.open(output)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        println!("Private vault key file created: {}", output.display());
        return Ok(());
    }
    let path = args.vault_key_file.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Set GATEWAY_VAULT_KEY_FILE to an external private key file")
    })?;
    let vault = gateway_server::vault::load_key_file(path)?;
    let database_url = args
        .database_url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Set DATABASE_URL"))?;
    let public_url = args
        .public_url
        .ok_or_else(|| anyhow::anyhow!("Set GATEWAY_PUBLIC_URL"))?;
    let state = AppState::connect(
        database_url,
        Options {
            public_url,
            allow_loopback_http: args.allow_loopback_http,
            vault,
        },
    )
    .await?;
    if matches!(args.command, Some(Command::RotateVault)) {
        let count = gateway_server::vault::rotate_keys(&state).await?;
        println!("Vault records reencrypted: {count}");
        state.pool.close().await;
        return Ok(());
    }
    if let Some(Command::BootstrapAdmin {
        email,
        name,
        password_stdin: _,
    }) = args.command
    {
        use std::io::Read;
        let mut password = String::new();
        std::io::stdin().take(1024).read_to_string(&mut password)?;
        let user = gateway_server::identity::bootstrap(
            &state,
            &email,
            &name,
            password.trim_end_matches(['\n', '\r']).into(),
        )
        .await?;
        println!("Administrator created: {} ({})", user.email, user.id);
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
