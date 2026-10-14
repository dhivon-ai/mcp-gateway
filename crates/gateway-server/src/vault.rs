use crate::{
    AppError, AppState, Input, bump_revision,
    configuration::{self, revision},
    epoch, identity, uuid,
};
use axum::{
    Json,
    extract::{Path as ApiPath, State},
    http::{HeaderMap, StatusCode},
};
use gateway_core::{
    AuthMode, CredentialRequest, OAuthCredentials, OAuthDisconnect, OAuthStatus, OAuthStoreRequest,
    RefreshLease, RefreshRelease, RefreshRequest, SecretMetadata, SecretRename, SecretWrite,
    Transport, UpstreamCredential, VersionInput,
    crypto::{KeyRing, Sealed},
};
use sqlx::{FromRow, Postgres, Transaction};
use std::{io::Read, path::Path};
use uuid::Uuid;

pub fn load_key_file(path: &Path) -> anyhow::Result<KeyRing> {
    let metadata = std::fs::symlink_metadata(path)?;
    anyhow::ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= 16384,
        "Vault key must be a bounded regular file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        anyhow::ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "Vault key file must have owner-only permissions (0600)"
        );
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(16385)
        .read_to_end(&mut bytes)?;
    KeyRing::from_json(&bytes).map_err(|_| anyhow::anyhow!("Invalid vault key file"))
}
fn vault_error() -> AppError {
    tracing::warn!(event = "vault_integrity_or_key_failure");
    AppError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        gateway_core::ErrorCode::Internal,
        "Vault key is unavailable or encrypted data is invalid. Restore the matching key.",
    )
}
fn aad(kind: &str, owner: Uuid, id: &str, version: u32) -> Vec<u8> {
    format!("{kind}:{owner}:{id}:{version}").into_bytes()
}
fn next_version(version: u32) -> Result<u32, AppError> {
    version
        .checked_add(1)
        .ok_or_else(|| AppError::conflict("Credential version exhausted."))
}
fn key(raw: &str) -> Result<String, AppError> {
    let key = raw.trim();
    if key.is_empty() || key.len() > 128 || key.chars().any(char::is_control) {
        return Err(AppError::bad(
            "Secret key must contain 1–128 bytes without control characters.",
        ));
    }
    Ok(key.to_string())
}
fn value(raw: &str) -> Result<(), AppError> {
    if raw.is_empty() || raw.len() > 16384 {
        return Err(AppError::bad("Secret value must contain 1–16,384 bytes."));
    }
    Ok(())
}
#[derive(FromRow)]
struct SecretRow {
    id: Uuid,
    owner_id: Uuid,
    key: String,
    version: i64,
    key_id: String,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}
impl SecretRow {
    fn sealed(&self) -> Sealed {
        Sealed {
            format: 1,
            key_id: self.key_id.clone(),
            nonce: self.nonce.clone(),
            ciphertext: self.ciphertext.clone(),
        }
    }
    fn open(&self, keys: &KeyRing) -> Result<Vec<u8>, AppError> {
        keys.open(
            &aad(
                "secret",
                self.owner_id,
                &self.id.to_string(),
                revision(self.version)?,
            ),
            &self.sealed(),
        )
        .map_err(|_| vault_error())
    }
}
async fn secret_row(
    tx: &mut Transaction<'_, Postgres>,
    owner: Uuid,
    id: Uuid,
) -> Result<SecretRow, AppError> {
    sqlx::query_as("SELECT id,owner_id,key,version,key_id,nonce,ciphertext,updated_at FROM user_secrets WHERE id=$1 AND owner_id=$2").bind(id).bind(owner).fetch_optional(&mut **tx).await?.ok_or_else(AppError::forbidden)
}
pub async fn list_secrets(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<SecretMetadata>>, AppError> {
    let p = identity::authenticate(&state, &headers, false).await?;
    let (mut tx, p, _) = identity::begin_user(&state, &p).await?;
    let rows: Vec<(Uuid, String, i64, i64)> = sqlx::query_as(
        "SELECT id,key,version,updated_at FROM user_secrets WHERE owner_id=$1 ORDER BY key",
    )
    .bind(uuid(&p.user.id)?)
    .fetch_all(&mut *tx)
    .await?;
    let result = rows
        .into_iter()
        .map(|(id, key, v, updated_at)| {
            Ok(SecretMetadata {
                id: id.to_string(),
                key,
                version: revision(v)?,
                updated_at,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    tx.commit().await?;
    Ok(Json(result))
}
pub async fn create_secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    Input(input): Input<SecretWrite>,
) -> Result<Json<SecretMetadata>, AppError> {
    let p = identity::authenticate(&state, &headers, true).await?;
    let key = key(&input.key)?;
    value(&input.value)?;
    if input.expected_version.is_some() {
        return Err(AppError::bad("New secrets have no expected version."));
    }
    let (mut tx, p, _) = identity::begin_user(&state, &p).await?;
    let owner = uuid(&p.user.id)?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM user_secrets WHERE owner_id=$1")
        .bind(owner)
        .fetch_one(&mut *tx)
        .await?;
    if count >= 256 {
        return Err(AppError::bad("Secret store limit is 256 keys."));
    }
    let id = Uuid::new_v4();
    let blob = state
        .options
        .vault
        .seal(
            &aad("secret", owner, &id.to_string(), 1),
            input.value.as_bytes(),
        )
        .map_err(|_| vault_error())?;
    let now = epoch()?;
    sqlx::query("INSERT INTO user_secrets(id,owner_id,key,version,key_id,nonce,ciphertext,updated_at) VALUES($1,$2,$3,1,$4,$5,$6,$7)").bind(id).bind(owner).bind(&key).bind(blob.key_id).bind(blob.nonce).bind(blob.ciphertext).bind(now).execute(&mut *tx).await?;
    bump_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="secret_created",user_id=%owner,secret_id=%id);
    Ok(Json(SecretMetadata {
        id: id.to_string(),
        key,
        version: 1,
        updated_at: now,
    }))
}
async fn update_secret(
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
    row: &SecretRow,
    key: String,
    plaintext: &[u8],
) -> Result<SecretMetadata, AppError> {
    let version = next_version(revision(row.version)?)?;
    let now = epoch()?;
    let blob = state
        .options
        .vault
        .seal(
            &aad("secret", row.owner_id, &row.id.to_string(), version),
            plaintext,
        )
        .map_err(|_| vault_error())?;
    sqlx::query("UPDATE user_secrets SET key=$2,version=$3,key_id=$4,nonce=$5,ciphertext=$6,updated_at=$7 WHERE id=$1").bind(row.id).bind(&key).bind(i64::from(version)).bind(blob.key_id).bind(blob.nonce).bind(blob.ciphertext).bind(now).execute(&mut **tx).await?;
    bump_revision(tx).await?;
    Ok(SecretMetadata {
        id: row.id.to_string(),
        key,
        version,
        updated_at: now,
    })
}
pub async fn replace_secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiPath(raw): ApiPath<String>,
    Input(input): Input<SecretWrite>,
) -> Result<Json<SecretMetadata>, AppError> {
    let p = identity::authenticate(&state, &headers, true).await?;
    let id = uuid(&raw)?;
    let key = key(&input.key)?;
    value(&input.value)?;
    let (mut tx, p, _) = identity::begin_user(&state, &p).await?;
    let row = secret_row(&mut tx, uuid(&p.user.id)?, id).await?;
    if input.expected_version != Some(revision(row.version)?) {
        return Err(AppError::conflict(
            "Secret changed. Reload its metadata before replacing it.",
        ));
    }
    if key != row.key {
        return Err(AppError::bad(
            "Use the rename action to change a secret key.",
        ));
    }
    let result = update_secret(&state, &mut tx, &row, key, input.value.as_bytes()).await?;
    tx.commit().await?;
    tracing::info!(event="secret_replaced",user_id=%p.user.id,secret_id=%id,version=result.version);
    Ok(Json(result))
}
pub async fn rename_secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiPath(raw): ApiPath<String>,
    Input(input): Input<SecretRename>,
) -> Result<Json<SecretMetadata>, AppError> {
    let p = identity::authenticate(&state, &headers, true).await?;
    let id = uuid(&raw)?;
    let key = key(&input.key)?;
    let (mut tx, p, _) = identity::begin_user(&state, &p).await?;
    let row = secret_row(&mut tx, uuid(&p.user.id)?, id).await?;
    if input.expected_version != revision(row.version)? {
        return Err(AppError::conflict(
            "Secret changed. Reload its metadata before renaming it.",
        ));
    }
    let plaintext = row.open(&state.options.vault)?;
    let result = update_secret(&state, &mut tx, &row, key, &plaintext).await?;
    tx.commit().await?;
    tracing::info!(event="secret_renamed",user_id=%p.user.id,secret_id=%id,version=result.version);
    Ok(Json(result))
}
pub async fn delete_secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiPath(raw): ApiPath<String>,
    Input(input): Input<VersionInput>,
) -> Result<StatusCode, AppError> {
    let p = identity::authenticate(&state, &headers, true).await?;
    let id = uuid(&raw)?;
    let (mut tx, p, _) = identity::begin_user(&state, &p).await?;
    let row = secret_row(&mut tx, uuid(&p.user.id)?, id).await?;
    if input.expected_version != revision(row.version)? {
        return Err(AppError::conflict(
            "Secret changed. Reload its metadata before deleting it.",
        ));
    }
    let bound: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM user_settings WHERE secret_id=$1)")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if bound {
        return Err(AppError::conflict(
            "Disable or change every MCP binding before deleting this key.",
        ));
    }
    sqlx::query("DELETE FROM user_secrets WHERE id=$1 AND owner_id=$2")
        .bind(id)
        .bind(uuid(&p.user.id)?)
        .execute(&mut *tx)
        .await?;
    bump_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="secret_deleted",user_id=%p.user.id,secret_id=%id);
    Ok(StatusCode::NO_CONTENT)
}
#[derive(FromRow)]
struct OAuthRow {
    user_id: Uuid,
    mcp_id: Uuid,
    installation_id: Uuid,
    version: i64,
    key_id: String,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
    refresh_holder: Option<Uuid>,
    refresh_until: Option<i64>,
}
impl OAuthRow {
    fn binding(&self) -> Result<Vec<u8>, AppError> {
        Ok(aad(
            "oauth",
            self.user_id,
            &format!("{}:{}", self.mcp_id, self.installation_id),
            revision(self.version)?,
        ))
    }
    fn sealed(&self) -> Sealed {
        Sealed {
            format: 1,
            key_id: self.key_id.clone(),
            nonce: self.nonce.clone(),
            ciphertext: self.ciphertext.clone(),
        }
    }
    fn open(&self, keys: &KeyRing) -> Result<OAuthCredentials, AppError> {
        let bytes = keys
            .open(&self.binding()?, &self.sealed())
            .map_err(|_| vault_error())?;
        serde_json::from_slice(&bytes).map_err(|_| vault_error())
    }
}
async fn oauth_row(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    mcp: Uuid,
    installation: Uuid,
) -> Result<Option<OAuthRow>, AppError> {
    Ok(sqlx::query_as("SELECT user_id,mcp_id,installation_id,version,key_id,nonce,ciphertext,refresh_holder,refresh_until FROM oauth_credentials WHERE user_id=$1 AND mcp_id=$2 AND installation_id=$3").bind(user).bind(mcp).bind(installation).fetch_optional(&mut **tx).await?)
}
async fn native(state: &AppState, headers: &HeaderMap) -> Result<identity::Principal, AppError> {
    let p = identity::authenticate(state, headers, true).await?;
    if !p.endpoint {
        return Err(AppError::forbidden());
    }
    Ok(p)
}
async fn require_enabled(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    mcp: Uuid,
) -> Result<(), AppError> {
    if !configuration::assigned(tx, user, mcp).await? {
        return Err(AppError::forbidden());
    }
    let enabled: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_settings WHERE user_id=$1 AND mcp_id=$2)",
    )
    .bind(user)
    .bind(mcp)
    .fetch_one(&mut **tx)
    .await?;
    if !enabled {
        return Err(AppError::forbidden());
    }
    Ok(())
}
pub async fn resolve_credential(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiPath(raw): ApiPath<String>,
    Input(input): Input<CredentialRequest>,
) -> Result<Json<UpstreamCredential>, AppError> {
    resolve_inner(state, headers, raw, input, false).await
}
/// Consent recovery can inspect its installation's managed record before explicit enablement.
/// This native-only route never returns generic bearer secrets or grants MCP execution.
pub async fn resolve_oauth_credential(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiPath(raw): ApiPath<String>,
    Input(input): Input<CredentialRequest>,
) -> Result<Json<UpstreamCredential>, AppError> {
    resolve_inner(state, headers, raw, input, true).await
}
async fn resolve_inner(
    state: AppState,
    headers: HeaderMap,
    raw: String,
    input: CredentialRequest,
    consent: bool,
) -> Result<Json<UpstreamCredential>, AppError> {
    let p = native(&state, &headers).await?;
    let mcp = uuid(&raw)?;
    let installation = uuid(&input.installation_id)?;
    let (mut tx, p, current) = identity::begin_user(&state, &p).await?;
    configuration::check_revision(input.expected_revision, current)?;
    let owner = uuid(&p.user.id)?;
    let def = configuration::definition(&mut tx, mcp).await?;
    if consent {
        if !matches!(def.auth, AuthMode::OAuth { .. })
            || !configuration::assigned(&mut tx, owner, mcp).await?
        {
            return Err(AppError::forbidden());
        }
    } else {
        require_enabled(&mut tx, owner, mcp).await?;
    }
    let result = match def.auth {
        AuthMode::Bearer {} => {
            let id: Option<Uuid> = sqlx::query_scalar(
                "SELECT secret_id FROM user_settings WHERE user_id=$1 AND mcp_id=$2",
            )
            .bind(owner)
            .bind(mcp)
            .fetch_one(&mut *tx)
            .await?;
            let row = secret_row(&mut tx, owner, id.ok_or_else(AppError::forbidden)?).await?;
            let bytes = row.open(&state.options.vault)?;
            let token = String::from_utf8(bytes).map_err(|_| vault_error())?;
            if token.is_empty()
                || token
                    .bytes()
                    .any(|b| !b.is_ascii() || b.is_ascii_whitespace() || b == 0)
            {
                return Err(AppError::bad(
                    "The selected secret is not a valid bearer token.",
                ));
            }
            UpstreamCredential::Bearer {
                token,
                secret_id: row.id.to_string(),
                version: revision(row.version)?,
            }
        }
        AuthMode::OAuth { .. } => {
            let row = oauth_row(&mut tx, owner, mcp, installation)
                .await?
                .ok_or_else(|| {
                    AppError::new(
                        StatusCode::PRECONDITION_REQUIRED,
                        gateway_core::ErrorCode::AuthRequired,
                        "Connect OAuth on this Mac first.",
                    )
                })?;
            UpstreamCredential::OAuth {
                credentials: row.open(&state.options.vault)?,
                version: revision(row.version)?,
                installation_id: installation.to_string(),
            }
        }
        AuthMode::None {} => return Err(AppError::bad("This MCP requires no credential.")),
    };
    tx.commit().await?;
    tracing::info!(event="credential_resolved",user_id=%owner,mcp_id=%mcp);
    Ok(Json(result))
}
fn validate_oauth(
    state: &AppState,
    def: &gateway_core::McpDefinition,
    c: &OAuthCredentials,
) -> Result<(), AppError> {
    let (AuthMode::OAuth { client_id, scopes }, Transport::Http { url }) =
        (&def.auth, &def.transport)
    else {
        return Err(AppError::bad("This MCP is not configured for OAuth."));
    };
    if c.client_id != *client_id
        || c.issuer.len() > 4096
        || c.resource != *url
        || c.access_token.is_empty()
        || c.access_token.len() > 16384
        || c.access_token.contains(['\r', '\n', '\0'])
        || c.refresh_token
            .as_ref()
            .is_some_and(|t| t.is_empty() || t.len() > 16384)
        || c.expires_at <= epoch()?
        || c.expires_at > 253402300799
        || c.scopes.len() > 64
        || c.scopes.iter().any(|s| !scopes.contains(s))
    {
        return Err(AppError::bad(
            "OAuth credential registration, resource, scope, token or expiry is invalid.",
        ));
    }
    let issuer = gateway_core::validate_remote_url(&c.issuer, state.options.allow_loopback_http)
        .map_err(|_| AppError::bad("OAuth issuer must use verified HTTPS."))?;
    if issuer.query().is_some() {
        return Err(AppError::bad("OAuth issuer cannot contain a query."));
    }
    Ok(())
}
pub async fn store_oauth(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiPath(raw): ApiPath<String>,
    Input(input): Input<OAuthStoreRequest>,
) -> Result<Json<OAuthStatus>, AppError> {
    let p = native(&state, &headers).await?;
    let mcp = uuid(&raw)?;
    let installation = uuid(&input.installation_id)?;
    let (mut tx, p, current) = identity::begin_user(&state, &p).await?;
    configuration::check_revision(input.expected_revision, current)?;
    let owner = uuid(&p.user.id)?;
    if !configuration::assigned(&mut tx, owner, mcp).await? {
        return Err(AppError::forbidden());
    }
    validate_oauth(
        &state,
        &configuration::definition(&mut tx, mcp).await?,
        &input.credentials,
    )?;
    let old = oauth_row(&mut tx, owner, mcp, installation).await?;
    let now = epoch()?;
    let version = if let Some(old) = old {
        let previous = revision(old.version)?;
        if input.expected_version != Some(previous) {
            return Err(AppError::conflict(
                "OAuth credentials changed. Reload before retrying.",
            ));
        }
        let supplied = input.refresh_lease.as_deref().map(uuid).transpose()?;
        if supplied.is_some() {
            if supplied != old.refresh_holder || old.refresh_until.is_none_or(|until| until <= now)
            {
                return Err(AppError::conflict(
                    "OAuth refresh lease has expired or changed.",
                ));
            }
        } else if old.refresh_holder.is_some() && old.refresh_until.is_some_and(|until| until > now)
        {
            return Err(AppError::conflict("OAuth refresh is already in progress."));
        }
        next_version(previous)?
    } else {
        if input.expected_version.is_some() || input.refresh_lease.is_some() {
            return Err(AppError::conflict(
                "OAuth credential record no longer exists.",
            ));
        }
        1
    };
    let binding = aad("oauth", owner, &format!("{mcp}:{installation}"), version);
    let bytes = serde_json::to_vec(&input.credentials).map_err(|_| AppError::internal())?;
    let sealed = state
        .options
        .vault
        .seal(&binding, &bytes)
        .map_err(|_| vault_error())?;
    sqlx::query("INSERT INTO oauth_credentials(user_id,mcp_id,installation_id,version,key_id,nonce,ciphertext) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(user_id,mcp_id,installation_id) DO UPDATE SET version=excluded.version,key_id=excluded.key_id,nonce=excluded.nonce,ciphertext=excluded.ciphertext,refresh_holder=NULL,refresh_until=NULL").bind(owner).bind(mcp).bind(installation).bind(i64::from(version)).bind(sealed.key_id).bind(sealed.nonce).bind(sealed.ciphertext).execute(&mut *tx).await?;
    bump_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="oauth_credentials_stored",user_id=%owner,mcp_id=%mcp,version);
    Ok(Json(OAuthStatus {
        connected: true,
        version: Some(version),
        expires_at: Some(input.credentials.expires_at),
    }))
}
pub async fn oauth_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiPath((raw, installation)): ApiPath<(String, String)>,
) -> Result<Json<OAuthStatus>, AppError> {
    let p = identity::authenticate(&state, &headers, false).await?;
    let mcp = uuid(&raw)?;
    let installation = uuid(&installation)?;
    let (mut tx, p, _) = identity::begin_user(&state, &p).await?;
    let owner = uuid(&p.user.id)?;
    if !configuration::assigned(&mut tx, owner, mcp).await? {
        return Err(AppError::forbidden());
    }
    let result = match oauth_row(&mut tx, owner, mcp, installation).await? {
        Some(row) => OAuthStatus {
            connected: true,
            version: Some(revision(row.version)?),
            expires_at: Some(row.open(&state.options.vault)?.expires_at),
        },
        None => OAuthStatus {
            connected: false,
            version: None,
            expires_at: None,
        },
    };
    tx.commit().await?;
    Ok(Json(result))
}
pub async fn acquire_refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiPath(raw): ApiPath<String>,
    Input(input): Input<RefreshRequest>,
) -> Result<Json<RefreshLease>, AppError> {
    let p = native(&state, &headers).await?;
    let mcp = uuid(&raw)?;
    let installation = uuid(&input.installation_id)?;
    let (mut tx, p, _) = identity::begin_user(&state, &p).await?;
    let owner = uuid(&p.user.id)?;
    require_enabled(&mut tx, owner, mcp).await?;
    let row = oauth_row(&mut tx, owner, mcp, installation)
        .await?
        .ok_or_else(AppError::forbidden)?;
    if revision(row.version)? != input.expected_version {
        return Err(AppError::conflict("OAuth credentials changed."));
    }
    let now = epoch()?;
    if row.refresh_holder.is_some() && row.refresh_until.is_some_and(|until| until > now) {
        return Err(AppError::conflict("OAuth refresh is already in progress."));
    }
    let lease = Uuid::new_v4();
    let expires = now + 60;
    sqlx::query("UPDATE oauth_credentials SET refresh_holder=$4,refresh_until=$5 WHERE user_id=$1 AND mcp_id=$2 AND installation_id=$3").bind(owner).bind(mcp).bind(installation).bind(lease).bind(expires).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(RefreshLease {
        lease_id: lease.to_string(),
        expires_at: expires,
        version: input.expected_version,
    }))
}
pub async fn release_refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiPath(raw): ApiPath<String>,
    Input(input): Input<RefreshRelease>,
) -> Result<StatusCode, AppError> {
    let p = native(&state, &headers).await?;
    let mcp = uuid(&raw)?;
    let installation = uuid(&input.installation_id)?;
    let holder = uuid(&input.lease_id)?;
    let (mut tx, p, _) = identity::begin_user(&state, &p).await?;
    sqlx::query("UPDATE oauth_credentials SET refresh_holder=NULL,refresh_until=NULL WHERE user_id=$1 AND mcp_id=$2 AND installation_id=$3 AND refresh_holder=$4").bind(uuid(&p.user.id)?).bind(mcp).bind(installation).bind(holder).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
pub async fn disconnect_oauth(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiPath(raw): ApiPath<String>,
    Input(input): Input<OAuthDisconnect>,
) -> Result<StatusCode, AppError> {
    let p = native(&state, &headers).await?;
    let mcp = uuid(&raw)?;
    let installation = uuid(&input.installation_id)?;
    let (mut tx, p, current) = identity::begin_user(&state, &p).await?;
    configuration::check_revision(input.expected_revision, current)?;
    let owner = uuid(&p.user.id)?;
    if !configuration::assigned(&mut tx, owner, mcp).await?
        || !matches!(
            configuration::definition(&mut tx, mcp).await?.auth,
            AuthMode::OAuth { .. }
        )
    {
        return Err(AppError::forbidden());
    }
    if let Some(row) = oauth_row(&mut tx, owner, mcp, installation).await? {
        if revision(row.version)? != input.expected_version {
            return Err(AppError::conflict("OAuth credentials changed."));
        }
        sqlx::query(
            "DELETE FROM oauth_credentials WHERE user_id=$1 AND mcp_id=$2 AND installation_id=$3",
        )
        .bind(owner)
        .bind(mcp)
        .bind(installation)
        .execute(&mut *tx)
        .await?;
    }
    // Disconnect also fences a consent exchange when no record exists yet.
    sqlx::query("DELETE FROM user_settings WHERE user_id=$1 AND mcp_id=$2")
        .bind(owner)
        .bind(mcp)
        .execute(&mut *tx)
        .await?;
    bump_revision(&mut tx).await?;
    tx.commit().await?;
    tracing::info!(event="oauth_disconnected",user_id=%owner,mcp_id=%mcp);
    Ok(StatusCode::NO_CONTENT)
}

/// Operator-only rotation in bounded batches. No credential leaves this process.
pub async fn rotate_keys(state: &AppState) -> Result<u64, AppError> {
    let mut total = 0;
    loop {
        let mut tx = state.pool.begin().await?;
        crate::lock_revision(&mut tx).await?;
        let secrets:Vec<SecretRow>=sqlx::query_as("SELECT id,owner_id,key,version,key_id,nonce,ciphertext,updated_at FROM user_secrets WHERE key_id<>$1 ORDER BY id LIMIT 100").bind(state.options.vault.active_key_id()).fetch_all(&mut *tx).await?;
        let oauth:Vec<OAuthRow>=sqlx::query_as("SELECT user_id,mcp_id,installation_id,version,key_id,nonce,ciphertext,refresh_holder,refresh_until FROM oauth_credentials WHERE key_id<>$1 ORDER BY user_id,mcp_id,installation_id LIMIT 100").bind(state.options.vault.active_key_id()).fetch_all(&mut *tx).await?;
        let count = secrets.len() + oauth.len();
        for row in secrets {
            let plain = row.open(&state.options.vault)?;
            let blob = state
                .options
                .vault
                .seal(
                    &aad(
                        "secret",
                        row.owner_id,
                        &row.id.to_string(),
                        revision(row.version)?,
                    ),
                    &plain,
                )
                .map_err(|_| vault_error())?;
            sqlx::query("UPDATE user_secrets SET key_id=$2,nonce=$3,ciphertext=$4 WHERE id=$1")
                .bind(row.id)
                .bind(blob.key_id)
                .bind(blob.nonce)
                .bind(blob.ciphertext)
                .execute(&mut *tx)
                .await?;
        }
        for row in oauth {
            let plain = row.open(&state.options.vault)?;
            let bytes = serde_json::to_vec(&plain).map_err(|_| AppError::internal())?;
            let blob = state
                .options
                .vault
                .seal(&row.binding()?, &bytes)
                .map_err(|_| vault_error())?;
            sqlx::query("UPDATE oauth_credentials SET key_id=$4,nonce=$5,ciphertext=$6 WHERE user_id=$1 AND mcp_id=$2 AND installation_id=$3").bind(row.user_id).bind(row.mcp_id).bind(row.installation_id).bind(blob.key_id).bind(blob.nonce).bind(blob.ciphertext).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        total += count as u64;
        if count == 0 {
            break;
        }
    }
    Ok(total)
}
