//! Native-only immutable Keychain credential heads. Ordinary files contain scoped pointers only.
use crate::{
    EndpointError, Result,
    cache::{SnapshotCache, ValidatedSnapshot},
    clock::{ClockSample, epoch_ns},
    session::NativeSession,
    store::{Identity, Metadata, StateStore},
};
use gateway_core::{AuthMode, EffectiveMcp, UpstreamCredential};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::Zeroizing;

pub struct CredentialScope {
    pub session: NativeSession,
    pub mcp: EffectiveMcp,
    pub revision: u32,
    receipt: Uuid,
    binding: String,
}
impl CredentialScope {
    pub(crate) fn receipt(&self) -> Uuid {
        self.receipt
    }
    pub(crate) fn revision(&self) -> u32 {
        self.revision
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialFault {
    Required,
    Rejected,
    SyncRequired,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialHead {
    schema_version: u32,
    head_id: Uuid,
    identity: Identity,
    generation: Uuid,
    installation: Uuid,
    mcp_id: String,
    runtime_binding: String,
    source_revision: u32,
    observed: ClockSample,
    version: u32,
    fingerprint: String,
    pub credential: Option<UpstreamCredential>,
    fault: Option<CredentialFault>,
}
impl CredentialHead {
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn signature(&self) -> String {
        digest(
            format!(
                "{}:{}:{}",
                self.runtime_binding, self.version, self.fingerprint
            )
            .as_bytes(),
        )
    }
    pub fn access(&self, now: ClockSample) -> Result<Zeroizing<String>> {
        match self.fault {
            Some(CredentialFault::Rejected) => return Err(EndpointError::UpstreamRejected),
            Some(_) => return Err(EndpointError::UpstreamRequired),
            None => (),
        }
        let elapsed = now.elapsed_since(self.observed)?;
        match self
            .credential
            .as_ref()
            .ok_or(EndpointError::UpstreamRequired)?
        {
            UpstreamCredential::Bearer { token, .. } => Ok(Zeroizing::new(token.clone())),
            UpstreamCredential::OAuth { credentials, .. } => {
                let expiry = epoch_ns(credentials.expires_at)?;
                let budget = expiry
                    .checked_sub(self.observed.wall_ns)
                    .ok_or(EndpointError::UpstreamExpired)?;
                if now.wall_ns >= expiry || elapsed >= budget {
                    return Err(EndpointError::UpstreamExpired);
                }
                Ok(Zeroizing::new(credentials.access_token.clone()))
            }
        }
    }
}
#[derive(Clone)]
pub struct NativeCredentials {
    pub cache: SnapshotCache,
}
impl NativeCredentials {
    pub fn new(cache: SnapshotCache) -> Self {
        Self { cache }
    }
    /// Capture the protected snapshot receipt; read watermarks may rotate without replacing policy.
    pub fn scope(
        &self,
        snapshot: &ValidatedSnapshot,
        mcp: &EffectiveMcp,
    ) -> Result<CredentialScope> {
        self.scope_inner(snapshot, mcp, false)
    }
    /// Consent can precede enablement; this never makes an agent credential ready.
    pub fn management_scope(
        &self,
        snapshot: &ValidatedSnapshot,
        mcp: &EffectiveMcp,
    ) -> Result<CredentialScope> {
        if !matches!(mcp.definition.auth, AuthMode::OAuth { .. }) {
            return Err(EndpointError::UpstreamRequired);
        }
        self.scope_inner(snapshot, mcp, true)
    }
    fn scope_inner(
        &self,
        snapshot: &ValidatedSnapshot,
        mcp: &EffectiveMcp,
        consent: bool,
    ) -> Result<CredentialScope> {
        let before = self.cache.sessions.load()?;
        let latest = self.cache.latest()?;
        let session = self.cache.sessions.load()?;
        if before.metadata.generation != snapshot.generation
            || latest.generation != snapshot.generation
            || session.metadata.generation != snapshot.generation
        {
            return Err(EndpointError::IdentityChanged);
        }
        let current = latest
            .snapshot
            .mcps
            .iter()
            .find(|item| {
                item.definition.id == mcp.definition.id
                    && (item.enabled || consent)
                    && item.definition.active
            })
            .ok_or(EndpointError::UpstreamRequired)?;
        let binding = runtime_binding(current)?;
        if binding != runtime_binding(mcp)? || matches!(current.definition.auth, AuthMode::None {})
        {
            return Err(EndpointError::UpstreamRequired);
        }
        Ok(CredentialScope {
            session,
            mcp: current.clone(),
            revision: latest.snapshot.revision,
            receipt: latest.receipt,
            binding,
        })
    }
    pub fn load(&self, scope: &CredentialScope) -> Result<CredentialHead> {
        self.cache.sessions.store.locked(|metadata| {
            self.check_scope(metadata, scope)?;
            if metadata
                .upstream_publication_pending
                .contains(&scope.mcp.definition.id)
            {
                return Err(EndpointError::UpstreamRequired);
            }
            let account = metadata
                .upstream_heads
                .get(&scope.mcp.definition.id)
                .ok_or(EndpointError::UpstreamRequired)?;
            let record = self.read(account, scope)?;
            if record.source_revision != scope.revision {
                return Err(EndpointError::UpstreamRequired);
            }
            Ok(record)
        })
    }
    /// Cancel local consent and retire this record even if central follow-up synchronization fails.
    pub fn disconnect_oauth(&self, scope: &CredentialScope) -> Result<()> {
        if !matches!(scope.mcp.definition.auth, AuthMode::OAuth { .. }) {
            return Err(EndpointError::Invalid);
        }
        self.cache.sessions.store.locked(|metadata| {
            self.check_scope(metadata, scope)?;
            let marker = oauth_disconnect_account(
                &scope.session.metadata,
                &scope.mcp.definition.id,
                scope.receipt,
            )?;
            if !metadata.credential_accounts.contains(&marker) {
                if metadata.credential_accounts.len() >= 2048 {
                    return Err(EndpointError::Storage);
                }
                metadata.credential_accounts.push(marker.clone());
                self.cache.sessions.store.write(metadata)?;
            }
            if self
                .cache
                .sessions
                .store
                .credentials()
                .get(&marker)?
                .is_none()
            {
                self.cache
                    .sessions
                    .store
                    .credentials()
                    .set(&marker, b"disconnected")?;
            }
            if let Some(account) = metadata.upstream_heads.remove(&scope.mcp.definition.id) {
                self.cache.sessions.store.credentials().delete(&account)?;
                metadata
                    .credential_accounts
                    .retain(|tracked| tracked != &account);
            }
            metadata
                .upstream_publication_pending
                .remove(&scope.mcp.definition.id);
            self.cache.sessions.store.write(metadata)
        })
    }
    fn consent_cancelled(&self, scope: &CredentialScope) -> Result<bool> {
        if !matches!(scope.mcp.definition.auth, AuthMode::OAuth { .. }) {
            return Ok(false);
        }
        Ok(self
            .cache
            .sessions
            .store
            .credentials()
            .get(&oauth_disconnect_account(
                &scope.session.metadata,
                &scope.mcp.definition.id,
                scope.receipt,
            )?)?
            .is_some())
    }
    fn read(&self, account: &str, scope: &CredentialScope) -> Result<CredentialHead> {
        if self.consent_cancelled(scope)? {
            return Err(EndpointError::UpstreamRequired);
        }
        let record = self.read_raw(account, scope, false)?;
        if record.runtime_binding != scope.binding {
            return Err(EndpointError::UpstreamRequired);
        }
        if let Some(credential) = &record.credential {
            validate_credential(scope, credential)?;
        }
        let rejected = rejection_account(scope, &record.signature())?;
        if self
            .cache
            .sessions
            .store
            .credentials()
            .get(&rejected)?
            .is_some()
        {
            return Err(EndpointError::UpstreamRejected);
        }
        Ok(record)
    }
    fn read_raw(
        &self,
        account: &str,
        scope: &CredentialScope,
        allow_retired: bool,
    ) -> Result<CredentialHead> {
        let bytes = self
            .cache
            .sessions
            .store
            .credentials()
            .get(account)?
            .ok_or(EndpointError::UpstreamRequired)?;
        if bytes.len() > 65536 {
            return Err(EndpointError::Invalid);
        }
        let record: CredentialHead =
            serde_json::from_slice(&bytes).map_err(|_| EndpointError::Invalid)?;
        let identity = scope
            .session
            .metadata
            .active
            .as_ref()
            .ok_or(EndpointError::IdentityChanged)?;
        let expected = StateStore::account(
            identity,
            scope.session.metadata.generation,
            &format!("upstream-head:{}", record.head_id),
        )?;
        if record.schema_version != 1
            || record.identity != *identity
            || record.generation != scope.session.metadata.generation
            || record.installation != scope.session.metadata.installation_id
            || record.mcp_id != scope.mcp.definition.id
            || expected != account
        {
            return Err(EndpointError::UpstreamRequired);
        }
        let marker = StateStore::account(
            identity,
            record.generation,
            &format!("upstream-retired:{}", record.head_id),
        )?;
        if !allow_retired
            && self
                .cache
                .sessions
                .store
                .credentials()
                .get(&marker)?
                .is_some()
        {
            return Err(EndpointError::UpstreamRequired);
        }
        Ok(record)
    }
    /// Publish only a complete authenticated resolution. Token lifetime keeps its original native anchor.
    pub fn publish(
        &self,
        scope: &CredentialScope,
        credential: UpstreamCredential,
        observed: ClockSample,
    ) -> Result<()> {
        validate_credential(scope, &credential)?;
        let version = credential_version(&credential);
        let encoded =
            Zeroizing::new(serde_json::to_vec(&credential).map_err(|_| EndpointError::Invalid)?);
        let fingerprint = digest(&encoded);
        self.cache.sessions.store.locked(|metadata| {
            self.check_scope(metadata, scope)?;
            if self.consent_cancelled(scope)? {
                return Err(EndpointError::UpstreamRequired);
            }
            let signature =
                digest(format!("{}:{}:{}", scope.binding, version, fingerprint).as_bytes());
            if self
                .cache
                .sessions
                .store
                .credentials()
                .get(&rejection_account(scope, &signature)?)?
                .is_some()
            {
                return Err(EndpointError::UpstreamRejected);
            }
            let mut old = match metadata.upstream_heads.get(&scope.mcp.definition.id) {
                None => None,
                Some(account) => match self.read_raw(
                    account,
                    scope,
                    metadata
                        .upstream_publication_pending
                        .contains(&scope.mcp.definition.id),
                ) {
                    Ok(record) => Some(record),
                    Err(EndpointError::UpstreamRequired)
                        if metadata
                            .upstream_publication_pending
                            .contains(&scope.mcp.definition.id) =>
                    {
                        None
                    }
                    Err(error) => return Err(error),
                },
            };
            if metadata
                .upstream_publication_pending
                .contains(&scope.mcp.definition.id)
            {
                let prefix = format!(
                    "{}upstream-head:",
                    StateStore::account_prefix(
                        scope
                            .session
                            .metadata
                            .active
                            .as_ref()
                            .ok_or(EndpointError::IdentityChanged)?,
                        scope.session.metadata.generation
                    )
                );
                for account in metadata
                    .credential_accounts
                    .iter()
                    .filter(|account| account.starts_with(&prefix))
                {
                    match self.read_raw(account, scope, true) {
                        Ok(candidate) if candidate.runtime_binding == scope.binding => {
                            let earlier = old.as_ref().is_none_or(|old| {
                                candidate.version > old.version
                                    || (candidate.version == old.version
                                        && candidate.observed.boot_id == old.observed.boot_id
                                        && candidate.observed.continuous_ns
                                            < old.observed.continuous_ns)
                            });
                            if earlier {
                                old = Some(candidate);
                            }
                        }
                        Ok(_) | Err(EndpointError::UpstreamRequired) => (),
                        Err(error) => return Err(error),
                    }
                }
            }
            let mut observed = observed;
            if let Some(old) = old
                .as_ref()
                .filter(|old| old.runtime_binding == scope.binding)
            {
                if version < old.version {
                    return Err(EndpointError::RevisionRegression);
                }
                if version == old.version {
                    if old.fault == Some(CredentialFault::Rejected) {
                        return Err(EndpointError::UpstreamRejected);
                    }
                    if old.fingerprint != fingerprint {
                        return Err(EndpointError::Invalid);
                    }
                    let anchor_valid = self.cache.sample()?.elapsed_since(old.observed).is_ok();
                    if anchor_valid {
                        observed = old.observed;
                    }
                    if anchor_valid
                        && old.fault.is_none()
                        && old.source_revision == scope.revision
                        && !metadata
                            .upstream_publication_pending
                            .contains(&scope.mcp.definition.id)
                    {
                        return Ok(());
                    }
                }
            }
            let record = CredentialHead {
                schema_version: 1,
                head_id: Uuid::new_v4(),
                identity: scope
                    .session
                    .metadata
                    .active
                    .clone()
                    .ok_or(EndpointError::IdentityChanged)?,
                generation: scope.session.metadata.generation,
                installation: scope.session.metadata.installation_id,
                mcp_id: scope.mcp.definition.id.clone(),
                runtime_binding: scope.binding.clone(),
                source_revision: scope.revision,
                observed,
                version,
                fingerprint,
                credential: Some(credential),
                fault: None,
            };
            self.replace(metadata, scope, record)
        })
    }
    /// A late rejection cannot invalidate a newer credential or another login generation.
    pub fn reject(
        &self,
        scope: &CredentialScope,
        expected_signature: Option<&str>,
        fault: CredentialFault,
    ) -> Result<()> {
        self.cache.sessions.store.locked(|metadata| {
            self.check_scope(metadata, scope)?;
            let old = metadata
                .upstream_heads
                .get(&scope.mcp.definition.id)
                .map(|account| self.read_raw(account, scope, false))
                .transpose()?;
            let old_bound = old
                .as_ref()
                .filter(|old| old.runtime_binding == scope.binding);
            if expected_signature
                .is_some_and(|signature| old_bound.is_none_or(|old| old.signature() != signature))
            {
                return Ok(());
            }
            if fault == CredentialFault::Rejected
                && let Some(old) = old_bound
            {
                let account = rejection_account(scope, &old.signature())?;
                if !metadata.credential_accounts.contains(&account) {
                    if metadata.credential_accounts.len() >= 2048 {
                        return Err(EndpointError::Storage);
                    }
                    metadata.credential_accounts.push(account.clone());
                    self.cache.sessions.store.write(metadata)?;
                }
                if self
                    .cache
                    .sessions
                    .store
                    .credentials()
                    .get(&account)?
                    .is_none()
                {
                    self.cache
                        .sessions
                        .store
                        .credentials()
                        .set(&account, b"rejected")?;
                }
            }
            if old_bound.is_some_and(|old| {
                old.fault == Some(fault) && old.source_revision == scope.revision
            }) {
                return Ok(());
            }
            let record = CredentialHead {
                schema_version: 1,
                head_id: Uuid::new_v4(),
                identity: scope
                    .session
                    .metadata
                    .active
                    .clone()
                    .ok_or(EndpointError::IdentityChanged)?,
                generation: scope.session.metadata.generation,
                installation: scope.session.metadata.installation_id,
                mcp_id: scope.mcp.definition.id.clone(),
                runtime_binding: scope.binding.clone(),
                source_revision: scope.revision,
                observed: old
                    .as_ref()
                    .map_or(self.cache.sample()?, |old| old.observed),
                version: old_bound.map_or(0, |old| old.version),
                fingerprint: old_bound.map_or_else(String::new, |old| old.fingerprint.clone()),
                credential: None,
                fault: Some(fault),
            };
            self.replace(metadata, scope, record)
        })
    }
    fn check_scope(&self, metadata: &Metadata, scope: &CredentialScope) -> Result<()> {
        check_identity(metadata, scope)?;
        if !self.cache.matches_receipt_locked(metadata, scope.receipt)? {
            return Err(EndpointError::IdentityChanged);
        }
        Ok(())
    }
    /// The caller must have just obtained this private native proof through cache.latest().
    pub fn cached_scope(
        &self,
        snapshot: &ValidatedSnapshot,
        mcp: &EffectiveMcp,
    ) -> Result<CredentialScope> {
        if !self.cache.proof_usable(snapshot)? {
            return Err(EndpointError::CacheExpired);
        }
        let session = self.cache.sessions.load()?;
        if session.metadata.generation != snapshot.generation
            || !mcp.enabled
            || !mcp.definition.active
            || matches!(mcp.definition.auth, AuthMode::None {})
        {
            return Err(EndpointError::UpstreamRequired);
        }
        let member = snapshot
            .snapshot
            .mcps
            .iter()
            .find(|member| {
                member.definition.id == mcp.definition.id
                    && member.enabled
                    && member.definition.active
            })
            .ok_or(EndpointError::UpstreamRequired)?;
        if runtime_binding(member)? != runtime_binding(mcp)? {
            return Err(EndpointError::UpstreamRequired);
        }
        let scope = CredentialScope {
            session,
            mcp: mcp.clone(),
            revision: snapshot.snapshot.revision,
            receipt: snapshot.receipt,
            binding: runtime_binding(mcp)?,
        };
        self.cache
            .sessions
            .store
            .locked(|metadata| self.check_scope(metadata, &scope))?;
        Ok(scope)
    }
    fn replace(
        &self,
        metadata: &mut Metadata,
        scope: &CredentialScope,
        record: CredentialHead,
    ) -> Result<()> {
        let store = &self.cache.sessions.store;
        let account = StateStore::account(
            &record.identity,
            record.generation,
            &format!("upstream-head:{}", record.head_id),
        )?;
        let old = metadata.upstream_heads.get(&record.mcp_id).cloned();
        let marker = old
            .as_ref()
            .map(|account| {
                let id = account.rsplit(':').next().ok_or(EndpointError::Invalid)?;
                StateStore::account(
                    &record.identity,
                    record.generation,
                    &format!("upstream-retired:{id}"),
                )
            })
            .transpose()?;
        metadata
            .upstream_publication_pending
            .insert(record.mcp_id.clone());
        for account in std::iter::once(&account).chain(marker.iter()) {
            if !metadata.credential_accounts.contains(account) {
                metadata.credential_accounts.push(account.clone());
            }
        }
        if metadata.credential_accounts.len() > 2048 {
            return Err(EndpointError::Storage);
        }
        store.write(metadata)?;
        if let Some(marker) = &marker
            && store.credentials().get(marker)?.is_none()
        {
            store.credentials().set(marker, b"retired")?;
        }
        let bytes =
            Zeroizing::new(serde_json::to_vec(&record).map_err(|_| EndpointError::Invalid)?);
        store.credentials().set(&account, &bytes)?;
        if let Some(old) = &old {
            store.credentials().delete(old)?;
        }
        self.check_scope(metadata, scope)?;
        metadata
            .upstream_heads
            .insert(record.mcp_id.clone(), account);
        metadata.upstream_publication_pending.remove(&record.mcp_id);
        store.write(metadata)?;
        if let Some(old) = &old {
            metadata
                .credential_accounts
                .retain(|account| account != old);
        }
        if let Some(marker) = &marker
            && store.credentials().delete(marker).is_ok()
        {
            metadata
                .credential_accounts
                .retain(|account| account != marker);
        }
        // Completed recovery can retire journaled candidates without touching another MCP's pending work.
        if metadata.upstream_publication_pending.is_empty() {
            let prefix = StateStore::account_prefix(&record.identity, record.generation);
            let head_prefix = format!("{prefix}upstream-head:");
            let retired_prefix = format!("{prefix}upstream-retired:");
            let disconnect_prefix = format!("{prefix}od:{}:", digest(record.mcp_id.as_bytes()));
            let current_disconnect =
                oauth_disconnect_account(&scope.session.metadata, &record.mcp_id, scope.receipt)?;
            let current: std::collections::HashSet<_> =
                metadata.upstream_heads.values().cloned().collect();
            let current_ids: std::collections::HashSet<_> = current
                .iter()
                .filter_map(|account| account.rsplit(':').next())
                .collect();
            let orphaned: Vec<_> = metadata
                .credential_accounts
                .iter()
                .filter(|account| {
                    (account.starts_with(&head_prefix) && !current.contains(*account))
                        || (account.starts_with(&disconnect_prefix)
                            && *account != &current_disconnect)
                        || (account.starts_with(&retired_prefix)
                            && account
                                .rsplit(':')
                                .next()
                                .is_some_and(|id| !current_ids.contains(id)))
                })
                .cloned()
                .collect();
            for account in orphaned {
                if store.credentials().delete(&account).is_ok() {
                    metadata
                        .credential_accounts
                        .retain(|tracked| *tracked != account);
                }
            }
        }
        store.write(metadata)
    }
}
fn check_identity(metadata: &Metadata, scope: &CredentialScope) -> Result<()> {
    if metadata.generation != scope.session.metadata.generation
        || metadata.installation_id != scope.session.metadata.installation_id
        || metadata.active != scope.session.metadata.active
        || metadata.cache_publication_pending
    {
        return Err(EndpointError::IdentityChanged);
    }
    Ok(())
}
fn runtime_binding(mcp: &EffectiveMcp) -> Result<String> {
    Ok(digest(
        &serde_json::to_vec(&(
            &mcp.definition.id,
            &mcp.definition.transport,
            &mcp.definition.auth,
            &mcp.secret,
        ))
        .map_err(|_| EndpointError::Invalid)?,
    ))
}
fn credential_version(credential: &UpstreamCredential) -> u32 {
    match credential {
        UpstreamCredential::Bearer { version, .. } | UpstreamCredential::OAuth { version, .. } => {
            *version
        }
    }
}
fn validate_credential(scope: &CredentialScope, credential: &UpstreamCredential) -> Result<()> {
    let token = match (&scope.mcp.definition.auth, credential) {
        (
            AuthMode::Bearer {},
            UpstreamCredential::Bearer {
                token,
                secret_id,
                version,
            },
        ) if scope
            .mcp
            .secret
            .as_ref()
            .is_some_and(|secret| secret.id == *secret_id && secret.version == *version) =>
        {
            token
        }
        (
            AuthMode::OAuth { client_id, scopes },
            UpstreamCredential::OAuth {
                credentials,
                version,
                installation_id,
            },
        ) if *version > 0
            && installation_id == &scope.session.metadata.installation_id.to_string()
            && credentials.client_id == *client_id
            && matches!(&scope.mcp.definition.transport, gateway_core::Transport::Http { url } if *url == credentials.resource)
            && credentials
                .scopes
                .iter()
                .all(|scope| scopes.contains(scope)) =>
        {
            &credentials.access_token
        }
        _ => return Err(EndpointError::UpstreamRequired),
    };
    if token.is_empty()
        || token.len() > 16384
        || !token
            .bytes()
            .all(|byte| byte.is_ascii() && !byte.is_ascii_whitespace() && byte != 0)
    {
        return Err(EndpointError::UpstreamRequired);
    }
    Ok(())
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn rejection_account(scope: &CredentialScope, signature: &str) -> Result<String> {
    StateStore::account(
        scope
            .session
            .metadata
            .active
            .as_ref()
            .ok_or(EndpointError::IdentityChanged)?,
        scope.session.metadata.generation,
        &format!("upstream-reject:{signature}"),
    )
}
pub(crate) fn oauth_disconnect_account(
    metadata: &Metadata,
    id: &str,
    receipt: Uuid,
) -> Result<String> {
    StateStore::account(
        metadata
            .active
            .as_ref()
            .ok_or(EndpointError::LoginRequired)?,
        metadata.generation,
        &format!("od:{}:{}", digest(id.as_bytes()), receipt.simple()),
    )
}
