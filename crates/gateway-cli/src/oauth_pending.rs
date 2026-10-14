//! A complete issued token may await online CAS recovery; it is never dispatch authority.
use crate::{
    EndpointError, Result,
    clock::{ClockSample, epoch_ns},
    session::{NativeSession, SessionManager},
    store::{Identity, StateStore},
};
use gateway_core::{EffectiveMcp, OAuthCredentials};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingToken {
    id: Uuid,
    identity: Identity,
    generation: Uuid,
    installation: Uuid,
    mcp_id: String,
    registration: Vec<u8>,
    pub previous_version: Option<u32>,
    pub lease: Option<String>,
    pub credentials: OAuthCredentials,
    pub observed: ClockSample,
}
pub fn registration(mcp: &EffectiveMcp) -> Result<Vec<u8>> {
    serde_json::to_vec(&(
        &mcp.definition.id,
        &mcp.definition.auth,
        &mcp.definition.transport,
    ))
    .map_err(|_| EndpointError::Invalid)
}
impl PendingToken {
    pub fn new(
        session: &NativeSession,
        mcp: &EffectiveMcp,
        previous_version: Option<u32>,
        lease: Option<String>,
        credentials: OAuthCredentials,
        observed: ClockSample,
    ) -> Result<Self> {
        Ok(Self {
            id: Uuid::new_v4(),
            identity: session
                .metadata
                .active
                .clone()
                .ok_or(EndpointError::LoginRequired)?,
            generation: session.metadata.generation,
            installation: session.metadata.installation_id,
            mcp_id: mcp.definition.id.clone(),
            registration: registration(mcp)?,
            previous_version,
            lease,
            credentials,
            observed,
        })
    }
    pub fn usable(&self, now: ClockSample) -> Result<bool> {
        let expiry = epoch_ns(self.credentials.expires_at)?;
        Ok(now.wall_ns < expiry
            && now.elapsed_since(self.observed)?
                < expiry
                    .checked_sub(self.observed.wall_ns)
                    .ok_or(EndpointError::UpstreamExpired)?)
    }
    pub fn account(&self) -> Result<String> {
        StateStore::account(
            &self.identity,
            self.generation,
            &format!("oauth-save:{}", self.id),
        )
    }
    pub fn remember(&self, cache: &crate::cache::SnapshotCache, receipt: Uuid) -> Result<()> {
        let sessions = &cache.sessions;
        let account = self.account()?;
        sessions.store.locked(|metadata| {
            if !cache.matches_receipt_locked(metadata, receipt)? {
                return Err(EndpointError::IdentityChanged);
            }
            if sessions
                .store
                .credentials()
                .get(&crate::upstream_credentials::oauth_disconnect_account(
                    metadata,
                    &self.mcp_id,
                    receipt,
                )?)?
                .is_some()
            {
                return Err(EndpointError::Cancelled);
            }
            if metadata.generation != self.generation
                || metadata.active.as_ref() != Some(&self.identity)
                || metadata.installation_id != self.installation
            {
                return Err(EndpointError::IdentityChanged);
            }
            if metadata.credential_accounts.len() >= 2048 {
                return Err(EndpointError::Storage);
            }
            let prefix = format!(
                "{}oauth-save:",
                StateStore::account_prefix(&self.identity, self.generation)
            );
            for old in metadata
                .credential_accounts
                .iter()
                .filter(|old| old.starts_with(&prefix))
            {
                if let Some(bytes) = sessions.store.credentials().get(old)? {
                    let candidate: PendingToken =
                        serde_json::from_slice(&bytes).map_err(|_| EndpointError::Invalid)?;
                    if candidate.mcp_id == self.mcp_id {
                        return Err(EndpointError::UpstreamRequired);
                    }
                }
            }
            metadata.credential_accounts.push(account.clone());
            sessions.store.write(metadata)?;
            let bytes =
                Zeroizing::new(serde_json::to_vec(self).map_err(|_| EndpointError::Invalid)?);
            sessions.store.credentials().set(&account, &bytes)
        })
    }
    pub fn forget(&self, sessions: &SessionManager) -> Result<()> {
        let account = self.account()?;
        sessions.store.locked(|metadata| {
            if metadata.generation != self.generation {
                return Ok(());
            }
            sessions.store.credentials().delete(&account)?;
            metadata
                .credential_accounts
                .retain(|tracked| tracked != &account);
            sessions.store.write(metadata)
        })
    }
}
pub fn pending(
    sessions: &SessionManager,
    session: &NativeSession,
    mcp: &EffectiveMcp,
) -> Result<Vec<PendingToken>> {
    let identity = session
        .metadata
        .active
        .as_ref()
        .ok_or(EndpointError::LoginRequired)?;
    let prefix = format!(
        "{}oauth-save:",
        StateStore::account_prefix(identity, session.metadata.generation)
    );
    let expected = registration(mcp)?;
    sessions.store.locked(|metadata| {
        if metadata.generation != session.metadata.generation
            || metadata.active.as_ref() != Some(identity)
        {
            return Err(EndpointError::IdentityChanged);
        }
        let mut pending = Vec::new();
        let mut obsolete = Vec::new();
        for account in metadata
            .credential_accounts
            .iter()
            .filter(|account| account.starts_with(&prefix))
        {
            let Some(bytes) = sessions.store.credentials().get(account)? else {
                continue;
            };
            if bytes.len() > 65536 {
                return Err(EndpointError::Invalid);
            }
            let item: PendingToken =
                serde_json::from_slice(&bytes).map_err(|_| EndpointError::Invalid)?;
            if item.generation != session.metadata.generation
                || &item.identity != identity
                || item.installation != session.metadata.installation_id
                || item.account()? != *account
            {
                return Err(EndpointError::Invalid);
            }
            if item.mcp_id == mcp.definition.id {
                if item.registration != expected {
                    obsolete.push(account.clone());
                    continue;
                }
                pending.push(item);
            }
        }
        if !obsolete.is_empty() {
            for account in &obsolete {
                sessions.store.credentials().delete(account)?;
            }
            metadata
                .credential_accounts
                .retain(|account| !obsolete.contains(account));
            sessions.store.write(metadata)?;
        }
        if pending.len() > 1 {
            return Err(EndpointError::UpstreamRequired);
        }
        Ok(pending)
    })
}
