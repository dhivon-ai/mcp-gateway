use clap::{Parser, Subcommand};

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
    },
    /// Serve coding agents over STDIO using an authenticated configuration.
    Mcp,
    /// Print redacted local configuration status.
    Status,
    /// Remove local authentication and request online session revocation.
    Logout,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    match Args::parse().command {
        Command::Status => println!("{}", serde_json::json!({ "state": "login_required" })),
        Command::Mcp => {
            anyhow::bail!("No authenticated configuration. Run mcp-gateway ui --server <url>.")
        }
        Command::Ui { .. } | Command::Logout => {
            anyhow::bail!("Local management is not yet configured.")
        }
    }
    Ok(())
}
