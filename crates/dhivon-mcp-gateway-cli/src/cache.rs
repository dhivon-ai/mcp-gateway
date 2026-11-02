//! Complete encrypted snapshots, replay-resistant heads and immutable restart-safe deadlines.
use crate::{
    EndpointError, Result,
    clock::{Clock, ClockSample, NANOS_PER_SECOND, epoch_ns},
    session::{NativeSession, SessionManager},
    store::{Identity, Metadata, StateStore, check_private},
};
use dhivon_mcp_gateway_core::{
    ConfigSnapshot,
    crypto::{KeyRing, Sealed},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs::File, io::Read, sync::Arc};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

const MAGIC: &[u8; 8] = b"MCPCACH1";
const KEY_ID: &str = "endpoint-v1";
const MAX_FILE_BYTES: usize = 2 * 1024 * 1024 + 36;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CacheHead {
    format: u32,
    #[serde(skip)]
    key: Vec<u8>,
    receipt: Option<Uuid>,
    revision: Option<u32>,
    validated_at: Option<i64>,
    policy_hash: Option<Vec<u8>>,
    last_seen: Option<ClockSample>,
    committed: bool,
}
impl Drop for CacheHead {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CachedSnapshot {
    format: u32,
    receipt: Uuid,
    identity: Identity,
    generation: Uuid,
    installation_id: Uuid,
    anchor: ClockSample,
    deadline_continuous_ns: u64,
    snapshot: ConfigSnapshot,
}
#[derive(Clone)]
pub struct SnapshotCache {
    pub sessions: SessionManager,
    clock: Arc<dyn Clock>,
}
/// Native-only result. A consumer must call latest again before each new dispatch.
pub struct ValidatedSnapshot {
    pub snapshot: ConfigSnapshot,
    pub generation: Uuid,
    pub receipt: Uuid,
    pub sampled_at: ClockSample,
    pub now: i64,
    pub remaining_ns: u64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatus {
    pub state: &'static str,
    pub revision: Option<u32>,
    pub validated_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub remaining_seconds: Option<u64>,
}
#[cfg(target_os = "macos")]
pub(crate) fn import_checkpoint(bytes: &[u8]) -> Result<serde_json::Value> {
    let head: CacheHead = serde_json::from_slice(bytes).map_err(|_| EndpointError::InvalidCache)?;
    serde_json::to_value(head).map_err(|_| EndpointError::InvalidCache)
}
impl SnapshotCache {
    pub fn new(sessions: SessionManager, clock: Arc<dyn Clock>) -> Self {
        Self { sessions, clock }
    }
    pub fn sample(&self) -> Result<ClockSample> {
        self.clock.sample()
    }
    fn account(m: &Metadata) -> Result<String> {
        StateStore::account(
            m.active.as_ref().ok_or(EndpointError::LoginRequired)?,
            m.generation,
            "cache-key",
        )
    }
    fn head(&self, m: &Metadata) -> Result<Option<CacheHead>> {
        let key_account = Self::account(m)?;
        if !m.credential_accounts.contains(&key_account) {
            return Ok(None);
        }
        let Some(key) = self.sessions.store.credentials().get(&key_account)? else {
            return Ok(None);
        };
        if key.len() != 32 {
            return Err(EndpointError::InvalidCache);
        }
        if self.sessions.store.is_protected()
            && let Some(checkpoint) = &m.cache_checkpoint
        {
            let mut head: CacheHead = serde_json::from_value(checkpoint.clone())
                .map_err(|_| EndpointError::InvalidCache)?;
            head.key = key.to_vec();
            return Ok(Some(head));
        }
        let Some(account) = &m.cache_head_account else {
            return Ok(None);
        };
        let bytes = self
            .sessions
            .store
            .credentials()
            .get(account)?
            .ok_or(EndpointError::InvalidCache)?;
        if bytes.len() > 16384 {
            return Err(EndpointError::InvalidCache);
        }
        let mut head: CacheHead =
            serde_json::from_slice(&bytes).map_err(|_| EndpointError::InvalidCache)?;
        head.key = key.to_vec();
        if head.format != 1
            || head.policy_hash.as_ref().is_some_and(|h| h.len() != 32)
            || head.receipt.is_some() != head.revision.is_some()
            || head.receipt.is_some() != head.validated_at.is_some()
            || head.receipt.is_some() != head.policy_hash.is_some()
            || head.receipt.is_some() != head.last_seen.is_some()
        {
            return Err(EndpointError::InvalidCache);
        }
        Ok(Some(head))
    }
    fn save_head(&self, m: &mut Metadata, head: &CacheHead, create_key: bool) -> Result<()> {
        let key_account = Self::account(m)?;
        if self.sessions.store.is_protected() {
            let store = &self.sessions.store;
            if create_key {
                if !m.credential_accounts.contains(&key_account) {
                    m.credential_accounts.push(key_account.clone());
                }
                m.cache_publication_pending = true;
                store.write(m)?;
                store.credentials().set(&key_account, &head.key)?;
            }
            m.cache_checkpoint =
                Some(serde_json::to_value(head).map_err(|_| EndpointError::InvalidCache)?);
            m.cache_publication_pending = !head.committed;
            if let Some(old) = m.cache_head_account.take() {
                store.write(m)?;
                store.credentials().delete(&old)?;
                m.credential_accounts.retain(|c| c != &old);
            }
            return store.write(m);
        }
        match self.sessions.store.credentials().get(&key_account)? {
            Some(key) if key.as_slice() == head.key.as_slice() => {}
            None if create_key => {
                if !m.credential_accounts.contains(&key_account) {
                    m.credential_accounts.push(key_account.clone());
                    self.sessions.store.write(m)?;
                }
                self.sessions
                    .store
                    .credentials()
                    .set(&key_account, &head.key)?;
            }
            _ => return Err(EndpointError::InvalidCache),
        }
        // Existing-item updates can leave stale lookups in another Darwin client process.
        // Immutable checkpoints plus an atomic scoped pointer keep every reader on the same protected head.
        let identity = m.active.as_ref().ok_or(EndpointError::LoginRequired)?;
        let checkpoint_prefix = format!(
            "{}cache-head:",
            StateStore::account_prefix(identity, m.generation)
        );
        m.cache_publication_pending = true;
        self.sessions.store.write(m)?;
        // Clear journaled orphan checkpoints before allocating another one; persistent retirement failure stays bounded.
        let orphaned: Vec<_> = m
            .credential_accounts
            .iter()
            .filter(|account| {
                account.starts_with(&checkpoint_prefix)
                    && Some(*account) != m.cache_head_account.as_ref()
            })
            .cloned()
            .collect();
        for account in orphaned {
            self.sessions.store.credentials().delete(&account)?;
            m.credential_accounts
                .retain(|candidate| candidate != &account);
        }
        self.sessions.store.write(m)?;
        if m.credential_accounts.len() >= 2048 {
            return Err(EndpointError::Storage);
        }
        let checkpoint = StateStore::account(
            identity,
            m.generation,
            &format!("cache-head:{}", Uuid::new_v4()),
        )?;
        m.cache_publication_pending = true;
        m.credential_accounts.push(checkpoint.clone());
        self.sessions.store.write(m)?;
        let bytes =
            Zeroizing::new(serde_json::to_vec(head).map_err(|_| EndpointError::InvalidCache)?);
        self.sessions.store.credentials().set(&checkpoint, &bytes)?;
        m.cache_head_account = Some(checkpoint.clone());
        self.sessions.store.write(m)?;
        let obsolete: Vec<_> = m
            .credential_accounts
            .iter()
            .filter(|a| a.starts_with(&checkpoint_prefix) && **a != checkpoint)
            .cloned()
            .collect();
        for account in obsolete {
            // Retirement is part of publication: an older usable checkpoint may not remain replayable.
            self.sessions.store.credentials().delete(&account)?;
            m.credential_accounts.retain(|a| a != &account);
        }
        m.cache_publication_pending = !head.committed;
        self.sessions.store.write(m)
    }
    fn ring(head: &CacheHead) -> Result<KeyRing> {
        let key: [u8; 32] = head
            .key
            .as_slice()
            .try_into()
            .map_err(|_| EndpointError::InvalidCache)?;
        KeyRing::new(KEY_ID.into(), BTreeMap::from([(KEY_ID.into(), key)]))
            .map_err(|_| EndpointError::InvalidCache)
    }
    fn binding(m: &Metadata) -> Result<Vec<u8>> {
        serde_json::to_vec(&(
            "endpoint-cache-v1",
            1u32,
            &m.active,
            m.generation,
            m.installation_id,
        ))
        .map_err(|_| EndpointError::InvalidCache)
    }
    fn read_payload(&self, m: &Metadata, head: &CacheHead) -> Result<CachedSnapshot> {
        let path = self.sessions.store.root().join("snapshot.enc");
        if !path.exists() {
            return Err(EndpointError::SyncRequired);
        }
        check_private(&path, false)?;
        let mut bytes = Vec::with_capacity(MAX_FILE_BYTES + 1);
        File::open(path)
            .map_err(|_| EndpointError::Storage)?
            .take((MAX_FILE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| EndpointError::Storage)?;
        if bytes.len() < 36 || bytes.len() > MAX_FILE_BYTES || &bytes[..8] != MAGIC {
            return Err(EndpointError::InvalidCache);
        }
        let sealed = Sealed {
            format: 1,
            key_id: KEY_ID.into(),
            nonce: bytes[8..20].to_vec(),
            ciphertext: bytes[20..].to_vec(),
        };
        let plaintext = Zeroizing::new(
            Self::ring(head)?
                .open(&Self::binding(m)?, &sealed)
                .map_err(|_| EndpointError::InvalidCache)?,
        );
        let payload: CachedSnapshot =
            serde_json::from_slice(&plaintext).map_err(|_| EndpointError::InvalidCache)?;
        if payload.format != 1
            || Some(payload.receipt) != head.receipt
            || Some(payload.snapshot.revision) != head.revision
            || Some(payload.snapshot.validated_at) != head.validated_at
            || Some(&payload.identity) != m.active.as_ref()
            || payload.generation != m.generation
            || payload.installation_id != m.installation_id
            || Some(policy_hash(&payload.snapshot)?) != head.policy_hash
        {
            return Err(EndpointError::InvalidCache);
        }
        Ok(payload)
    }
    fn freshness(
        payload: &CachedSnapshot,
        head: &CacheHead,
        now: ClockSample,
        session: &NativeSession,
    ) -> Result<u64> {
        let elapsed = now.elapsed_since(payload.anchor)?;
        now.elapsed_since(head.last_seen.ok_or(EndpointError::InvalidCache)?)?;
        let expiry_ns = epoch_ns(payload.snapshot.expires_at)?;
        let anchor_remaining = expiry_ns
            .checked_sub(payload.anchor.wall_ns)
            .ok_or(EndpointError::InvalidCache)?;
        let maximum_deadline = payload
            .anchor
            .continuous_ns
            .checked_add(anchor_remaining)
            .ok_or(EndpointError::InvalidCache)?;
        if payload.deadline_continuous_ns > maximum_deadline
            || anchor_remaining > 3600 * NANOS_PER_SECOND
            || payload.snapshot.session_expires_at != session.credentials.expires_at
        {
            return Err(EndpointError::InvalidCache);
        }
        payload
            .snapshot
            .validate(
                &payload.identity.deployment_id,
                &payload.identity.user_id,
                now.wall_seconds()?,
            )
            .map_err(|e| match e {
                dhivon_mcp_gateway_core::PolicyError::Expired => EndpointError::CacheExpired,
                dhivon_mcp_gateway_core::PolicyError::ClockRollback => {
                    EndpointError::ClockUncertain
                }
                _ => EndpointError::InvalidCache,
            })?;
        let remaining = payload
            .deadline_continuous_ns
            .checked_sub(now.continuous_ns)
            .filter(|r| *r > 0)
            .ok_or(EndpointError::CacheExpired)?;
        if elapsed >= anchor_remaining || now.wall_ns >= expiry_ns {
            return Err(EndpointError::CacheExpired);
        }
        Ok(remaining.min(expiry_ns - now.wall_ns))
    }
    pub fn latest(&self) -> Result<ValidatedSnapshot> {
        let session = self.sessions.load()?;
        self.sessions.store.locked(|m| {
            if m.generation != session.metadata.generation {
                return Err(EndpointError::IdentityChanged);
            }
            if m.cache_publication_pending {
                return Err(EndpointError::InvalidCache);
            }
            let mut head = self.head(m)?.ok_or_else(|| {
                if self.sessions.store.root().join("snapshot.enc").exists() {
                    EndpointError::InvalidCache
                } else {
                    EndpointError::SyncRequired
                }
            })?;
            if !head.committed {
                return Err(EndpointError::InvalidCache);
            }
            let checked = (|| {
                let payload = self.read_payload(m, &head)?;
                let now = self.clock.sample()?;
                let remaining_ns = Self::freshness(&payload, &head, now, &session)?;
                Ok((payload, now, remaining_ns))
            })();
            match checked {
                Ok((payload, now, _)) => {
                    // A protected high-water mark detects rollback across process restart. It never changes the lease anchor.
                    head.last_seen = Some(now);
                    self.save_head(m, &head, false)?;
                    let checked = self.clock.sample().and_then(|now| {
                        Self::freshness(&payload, &head, now, &session)
                            .map(|remaining| (now, remaining))
                    });
                    let (after_publication, remaining_ns) = match checked {
                        Ok(checked) => checked,
                        Err(error) => {
                            if !matches!(error, EndpointError::CacheExpired) {
                                head.committed = false;
                                self.save_head(m, &head, false)?;
                            }
                            return Err(error);
                        }
                    };
                    Ok(ValidatedSnapshot {
                        snapshot: payload.snapshot,
                        generation: m.generation,
                        receipt: payload.receipt,
                        sampled_at: after_publication,
                        now: after_publication.wall_seconds()?,
                        remaining_ns,
                    })
                }
                Err(e) => {
                    if !matches!(e, EndpointError::CacheExpired) {
                        head.committed = false;
                        self.save_head(m, &head, false)?;
                    }
                    Err(e)
                }
            }
        })
    }
    /// Native credential publication compares the protected snapshot receipt, not rotating read watermarks.
    pub(crate) fn matches_receipt_locked(
        &self,
        metadata: &Metadata,
        receipt: Uuid,
    ) -> Result<bool> {
        Ok(!metadata.cache_publication_pending
            && self
                .head(metadata)?
                .is_some_and(|head| head.committed && head.receipt == Some(receipt)))
    }
    pub(crate) fn proof_usable(&self, proof: &ValidatedSnapshot) -> Result<bool> {
        let now = self.sample()?;
        Ok(now.elapsed_since(proof.sampled_at)? < proof.remaining_ns
            && now.wall_ns < epoch_ns(proof.snapshot.expires_at)?)
    }
    /// Complete validated online response only. Request-start anchor charges response/persistence time against the lease.
    pub fn install(
        &self,
        session: &NativeSession,
        snapshot: ConfigSnapshot,
        request_start: ClockSample,
    ) -> Result<()> {
        self.sessions.store.locked(|m| {
            if !self.sessions.matches_selection(m) {
                return Err(EndpointError::ServerMismatch);
            }
            if m.generation != session.metadata.generation || m.active != session.metadata.active {
                return Err(EndpointError::IdentityChanged);
            }
            let identity = m.active.clone().ok_or(EndpointError::LoginRequired)?;
            if snapshot.user.id == identity.user_id && !snapshot.user.active {
                self.sessions.store.clear_locked(m)?;
                return Err(EndpointError::Rejected);
            }
            let now = self.clock.sample()?;
            now.elapsed_since(request_start)?;
            snapshot
                .validate(
                    &identity.deployment_id,
                    &identity.user_id,
                    now.wall_seconds()?,
                )
                .map_err(|_| EndpointError::InvalidCache)?;
            if snapshot.session_expires_at != session.credentials.expires_at {
                return Err(EndpointError::InvalidCache);
            }
            let hash = policy_hash(&snapshot)?;
            // A live authenticated pull can replace a lost data key. Keep revision,
            // clock and policy floors from the separately protected checkpoint.
            let prior = if self.sessions.store.is_protected()
                && let Some(floor) = &m.cache_checkpoint
            {
                Some(
                    serde_json::from_value(floor.clone())
                        .map_err(|_| EndpointError::InvalidCache)?,
                )
            } else {
                self.head(m)?
            };
            let mut head = match prior {
                Some(head) => head,
                None => {
                    let mut key = vec![0u8; 32];
                    getrandom::fill(&mut key).map_err(|_| EndpointError::Keychain)?;
                    CacheHead {
                        format: 1,
                        key,
                        receipt: None,
                        revision: None,
                        validated_at: None,
                        policy_hash: None,
                        last_seen: None,
                        committed: false,
                    }
                }
            };
            if self.sessions.store.is_protected() {
                head.key.resize(32, 0);
                getrandom::fill(&mut head.key).map_err(|_| EndpointError::Storage)?;
            }
            if let Some(revision) = head.revision
                && (snapshot.revision < revision
                    || Some(snapshot.validated_at) < head.validated_at
                    || (snapshot.revision == revision && head.policy_hash.as_ref() != Some(&hash)))
            {
                return Err(EndpointError::RevisionRegression);
            }
            if let Some(last_seen) = head.last_seen.filter(|s| s.boot_id == now.boot_id) {
                now.elapsed_since(last_seen)?;
            }
            let expiry_ns = epoch_ns(snapshot.expires_at)?;
            let initial_budget = expiry_ns
                .checked_sub(request_start.wall_ns)
                .ok_or(EndpointError::CacheExpired)?;
            let receipt_budget = expiry_ns
                .checked_sub(now.wall_ns)
                .ok_or(EndpointError::CacheExpired)?;
            if receipt_budget == 0 || receipt_budget > 3600 * NANOS_PER_SECOND {
                return Err(EndpointError::InvalidCache);
            }
            let deadline = request_start
                .continuous_ns
                .checked_add(initial_budget)
                .ok_or(EndpointError::ClockUncertain)?
                .min(
                    now.continuous_ns
                        .checked_add(receipt_budget)
                        .ok_or(EndpointError::ClockUncertain)?,
                );
            if now.continuous_ns >= deadline {
                return Err(EndpointError::CacheExpired);
            }
            let receipt = Uuid::new_v4();
            let payload = CachedSnapshot {
                format: 1,
                receipt,
                identity,
                generation: m.generation,
                installation_id: m.installation_id,
                anchor: now,
                deadline_continuous_ns: deadline,
                snapshot,
            };
            let plaintext = Zeroizing::new(
                serde_json::to_vec(&payload).map_err(|_| EndpointError::InvalidCache)?,
            );
            let sealed = Self::ring(&head)?
                .seal(&Self::binding(m)?, &plaintext)
                .map_err(|_| EndpointError::InvalidCache)?;
            let bytes = [MAGIC.as_slice(), &sealed.nonce, &sealed.ciphertext].concat();
            head.receipt = Some(receipt);
            head.revision = Some(payload.snapshot.revision);
            head.validated_at = Some(payload.snapshot.validated_at);
            head.policy_hash = Some(hash);
            head.last_seen = Some(now);
            head.committed = false;
            self.save_head(m, &head, true)?;
            self.sessions.store.atomic_write("snapshot.enc", &bytes)?;
            // Expiry or clock uncertainty during durable publication must not produce a usable head.
            let committed_at = self.clock.sample()?;
            Self::freshness(&payload, &head, committed_at, session)?;
            head.last_seen = Some(committed_at);
            head.committed = true;
            self.save_head(m, &head, false)?;
            let confirmed = self.clock.sample().and_then(|after_publication| {
                Self::freshness(&payload, &head, after_publication, session)
            });
            if let Err(error) = confirmed {
                head.committed = false;
                self.save_head(m, &head, false)?;
                return Err(error);
            }
            tracing::info!(
                event = "endpoint_config_committed",
                revision = payload.snapshot.revision
            );
            Ok(())
        })
    }
    pub fn status(&self) -> CacheStatus {
        match self.latest() {
            Ok(valid) => CacheStatus {
                state: "valid",
                revision: Some(valid.snapshot.revision),
                validated_at: Some(valid.snapshot.validated_at),
                expires_at: Some(valid.snapshot.expires_at),
                remaining_seconds: Some(valid.remaining_ns / NANOS_PER_SECOND),
            },
            Err(e) => CacheStatus {
                state: match e {
                    EndpointError::LoginRequired | EndpointError::Rejected => "login_required",
                    EndpointError::SyncRequired => "sync_required",
                    EndpointError::CacheExpired => "expired",
                    EndpointError::ClockUncertain => "clock_uncertain",
                    EndpointError::ServerMismatch => "server_mismatch",
                    _ => "invalid",
                },
                revision: None,
                validated_at: None,
                expires_at: None,
                remaining_seconds: None,
            },
        }
    }
}
fn policy_hash(snapshot: &ConfigSnapshot) -> Result<Vec<u8>> {
    let mut policy = snapshot.clone();
    policy.validated_at = 0;
    policy.expires_at = 0;
    Ok(
        Sha256::digest(serde_json::to_vec(&policy).map_err(|_| EndpointError::InvalidCache)?)
            .to_vec(),
    )
}
