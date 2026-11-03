//! First-run and restart checks for the root Compose stack.
use anyhow::{Context, Result, ensure};
use dhivon_mcp_gateway_core::crypto::{KeyRing, Sealed};
use dhivon_mcp_gateway_server::{AppState, identity};
use sqlx::PgPool;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
    time::Duration,
};

const ROOT: &str = "/setup";
const RECOVERY: &str = "Inconsistent setup. Restore the matching database and private setup directory before retrying.";
const PROOF: &[u8] = b"Dhivon Compose first-admin provisioning";

fn private_directory(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    ensure!(
        meta.is_dir()
            && meta.uid() == fs::metadata("/proc/self")?.uid()
            && meta.permissions().mode() & 0o777 == 0o700,
        "{} must be a 0700 directory owned by the deployment user.",
        path.display()
    );
    Ok(())
}

fn read_private(path: &Path, limit: u64) -> Result<String> {
    let meta = fs::symlink_metadata(path).with_context(|| format!("Missing {}", path.display()))?;
    ensure!(
        meta.is_file()
            && meta.uid() == Path::new("/proc/self").metadata()?.uid()
            && meta.len() <= limit
            && meta.permissions().mode() & 0o077 == 0,
        "{} must be a bounded owner-only regular file.",
        path.display()
    );
    Ok(fs::read_to_string(path)?
        .trim_end_matches(['\r', '\n'])
        .into())
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub async fn prepare(database_url: &str) -> Result<File> {
    let root = Path::new(ROOT);
    private_directory(root)?;
    private_directory(&root.join("gateway"))?;
    private_directory(&root.join("postgres"))?;
    let lock = File::open(root)?;
    lock.try_lock()
        .context("Another setup owns this directory. Retry after it finishes.")?;
    // A full Compose restart can start setup before PostgreSQL is accepting
    // connections. The normal up path already waits for the database health check.
    let pool = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Ok(pool) = PgPool::connect(database_url).await {
                return pool;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await
    .context("Database unavailable. Check PostgreSQL health and its matching private password.")?;
    let gateway = root.join("gateway/keys");
    let cluster = read_private(&root.join("postgres/cluster-id"), 64)?;
    if gateway.try_exists()? {
        private_directory(&gateway)?;
        ensure!(
            read_private(&gateway.join("cluster-id"), 64)? == cluster,
            "{RECOVERY}"
        );
        return Ok(lock);
    }
    // A missing directory must never manufacture replacement key material for
    // an existing deployment, even when its database still accepts our password.
    let fresh: bool =
        sqlx::query_scalar("SELECT NOT EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public')")
            .fetch_one(&pool)
            .await?;
    pool.close().await;
    ensure!(fresh, "{RECOVERY}");
    let stage = root.join(format!("gateway/.new-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&stage)?;
    fs::set_permissions(&stage, fs::Permissions::from_mode(0o700))?;
    write_private(&stage.join("cluster-id"), cluster.as_bytes())?;
    write_private(
        &stage.join("vault-keys.json"),
        &KeyRing::generate_key_file(&format!("key-{}", uuid::Uuid::new_v4().simple()))?,
    )?;
    File::open(&stage)?.sync_all()?;
    fs::rename(stage, gateway)?;
    File::open(root.join("gateway"))?.sync_all()?;
    Ok(lock)
}

pub async fn verify_or_bootstrap(state: &AppState, provision: bool) -> Result<()> {
    let root = Path::new(ROOT);
    let gateway = root.join("gateway/keys");
    ensure!(
        read_private(&root.join("postgres/cluster-id"), 64)?
            == read_private(&gateway.join("cluster-id"), 64)?,
        "{RECOVERY}"
    );
    private_directory(&gateway)?;
    let deployment = gateway.join("deployment-id");
    let id = state.deployment_id.to_string();
    if deployment.try_exists()? {
        ensure!(read_private(&deployment, 64)? == id, "{RECOVERY}");
    } else {
        ensure!(provision, "{RECOVERY}");
        // Publish the intended database identity before creating an account.
        let stage = gateway.join(format!(".deployment-{}", uuid::Uuid::new_v4()));
        write_private(&stage, id.as_bytes())?;
        fs::rename(stage, &deployment)?;
        File::open(&gateway)?.sync_all()?;
    }
    let proof: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT proof FROM compose_setup WHERE singleton")
            .fetch_optional(&state.pool)
            .await?;
    let vault = &state.options.vault;
    if let Some(proof) = proof {
        let sealed: Sealed = serde_json::from_value(proof).context(RECOVERY)?;
        ensure!(
            vault
                .open(id.as_bytes(), &sealed)
                .context(RECOVERY)?
                .as_slice()
                == PROOF,
            "{RECOVERY}"
        );
    } else {
        ensure!(provision, "{RECOVERY}");
        let sealed = serde_json::to_value(vault.seal(id.as_bytes(), PROOF)?)?;
        let email = read_private(&root.join("admin-email"), 256)?;
        let password = read_private(&root.join("admin-password"), 130)?;
        identity::bootstrap_with_receipt(state, &email, "Administrator", password, Some(sealed))
            .await
            .context("First-admin provisioning failed. Check the private inputs; restore matching state for an existing deployment.")?;
    }
    if provision && root.join("admin-password").try_exists()? {
        fs::remove_file(root.join("admin-password"))
            .context("Provisioning succeeded. Remove the private admin-password file and retry.")?;
    }
    Ok(())
}
