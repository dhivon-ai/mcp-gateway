//! Cross-process identity and authorization guards. The native store encrypts metadata and receipts.
use crate::{EndpointError, Result, credentials::CredentialStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

pub(crate) const MAX_UPSTREAM_HEADS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Identity {
    pub server_origin: String,
    pub deployment_id: String,
    pub user_id: String,
}
impl Identity {
    pub fn validate(&self) -> Result<()> {
        Uuid::parse_str(&self.deployment_id).map_err(|_| EndpointError::Invalid)?;
        Uuid::parse_str(&self.user_id).map_err(|_| EndpointError::Invalid)?;
        let url = url::Url::parse(&self.server_origin).map_err(|_| EndpointError::Invalid)?;
        if url.origin().ascii_serialization() != self.server_origin {
            return Err(EndpointError::Invalid);
        }
        Ok(())
    }
    pub fn scope(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(self.server_origin.as_bytes());
        hash.update(b"\0");
        hash.update(self.deployment_id.as_bytes());
        hash.update(b"\0");
        hash.update(self.user_id.as_bytes());
        hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Cleanup {
    pub identity: Identity,
    pub generation: Uuid,
    pub accounts: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Metadata {
    pub schema_version: u32,
    pub installation_id: Uuid,
    pub generation: Uuid,
    pub active: Option<Identity>,
    #[serde(default)]
    pub selected_server_origin: Option<String>,
    pub credential_accounts: Vec<String>,
    #[serde(default)]
    pub cache_head_account: Option<String>,
    #[serde(default)]
    pub cache_publication_pending: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) cache_checkpoint: Option<serde_json::Value>,
    #[serde(default)]
    pub upstream_heads: BTreeMap<String, String>,
    #[serde(default)]
    pub upstream_publication_pending: BTreeSet<String>,
    pub pending_cleanup: Vec<Cleanup>,
}
impl Default for Metadata {
    fn default() -> Self {
        Self {
            schema_version: 1,
            installation_id: Uuid::new_v4(),
            generation: Uuid::new_v4(),
            active: None,
            selected_server_origin: None,
            credential_accounts: Vec::new(),
            cache_head_account: None,
            cache_publication_pending: false,
            cache_checkpoint: None,
            upstream_heads: BTreeMap::new(),
            upstream_publication_pending: BTreeSet::new(),
            pending_cleanup: Vec::new(),
        }
    }
}
impl Metadata {
    fn validate(&self) -> Result<()> {
        if self.schema_version != 1 || self.credential_accounts.len() > 2048 {
            return Err(EndpointError::Invalid);
        }
        if self.upstream_heads.len() > MAX_UPSTREAM_HEADS
            || self.upstream_publication_pending.len() > MAX_UPSTREAM_HEADS
        {
            return Err(EndpointError::Invalid);
        }
        for id in self
            .upstream_heads
            .keys()
            .chain(&self.upstream_publication_pending)
        {
            if id.is_empty() || id.len() > 128 || id.contains(['\0', '\r', '\n']) {
                return Err(EndpointError::Invalid);
            }
        }
        if let Some(origin) = &self.selected_server_origin {
            let url = url::Url::parse(origin).map_err(|_| EndpointError::Invalid)?;
            if !matches!(url.scheme(), "https" | "http")
                || url.origin().ascii_serialization() != *origin
            {
                return Err(EndpointError::Invalid);
            }
        }
        if let Some(identity) = &self.active {
            identity.validate()?;
            if self
                .selected_server_origin
                .as_ref()
                .is_some_and(|origin| *origin != identity.server_origin)
            {
                return Err(EndpointError::Invalid);
            }
            let prefix = StateStore::account_prefix(identity, self.generation);
            if self
                .credential_accounts
                .iter()
                .any(|s| !s.starts_with(&prefix) || s.len() > 256)
            {
                return Err(EndpointError::Invalid);
            }
        } else if !self.credential_accounts.is_empty() {
            return Err(EndpointError::Invalid);
        }
        if let Some(head) = &self.cache_head_account {
            let identity = self.active.as_ref().ok_or(EndpointError::Invalid)?;
            let prefix = format!(
                "{}cache-head:",
                StateStore::account_prefix(identity, self.generation)
            );
            if head
                .strip_prefix(&prefix)
                .is_none_or(|suffix| Uuid::parse_str(suffix).is_err())
                || !self.credential_accounts.contains(head)
            {
                return Err(EndpointError::Invalid);
            }
        }
        for head in self.upstream_heads.values() {
            let identity = self.active.as_ref().ok_or(EndpointError::Invalid)?;
            let prefix = format!(
                "{}upstream-head:",
                StateStore::account_prefix(identity, self.generation)
            );
            if head
                .strip_prefix(&prefix)
                .is_none_or(|suffix| Uuid::parse_str(suffix).is_err())
                || !self.credential_accounts.contains(head)
            {
                return Err(EndpointError::Invalid);
            }
        }
        if self.active.is_none() && !self.upstream_publication_pending.is_empty() {
            return Err(EndpointError::Invalid);
        }
        if self.pending_cleanup.len() > 16 {
            return Err(EndpointError::Invalid);
        }
        for cleanup in &self.pending_cleanup {
            cleanup.identity.validate()?;
            let prefix = StateStore::account_prefix(&cleanup.identity, cleanup.generation);
            if cleanup.accounts.len() > 2048
                || cleanup
                    .accounts
                    .iter()
                    .any(|s| !s.starts_with(&prefix) || s.len() > 256)
            {
                return Err(EndpointError::Invalid);
            }
        }
        Ok(())
    }
}
#[derive(Clone)]
pub struct StateStore {
    root: PathBuf,
    credentials: Arc<dyn CredentialStore>,
    protected: bool,
    #[cfg(target_os = "macos")]
    vault: Option<Arc<crate::vault::Vault>>,
}
impl StateStore {
    pub fn production() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var_os("HOME").ok_or(EndpointError::Storage)?;
            Self::native(
                PathBuf::from(home).join("Library/Application Support/com.dhivon.mcp-gateway"),
                crate::credentials::MacKeychain::production(),
            )
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(EndpointError::Unsupported)
        }
    }
    pub fn new(root: PathBuf, credentials: Arc<dyn CredentialStore>) -> Result<Self> {
        Self::private_root(&root)?;
        let store = Self {
            root,
            credentials,
            protected: false,
            #[cfg(target_os = "macos")]
            vault: None,
        };
        store.locked(|m| store.write(m))?;
        Ok(store)
    }
    pub fn is_protected(&self) -> bool {
        self.protected
    }
    fn private_root(root: &Path) -> Result<()> {
        if !root.is_absolute() {
            return Err(EndpointError::Storage);
        }
        for ancestor in root.ancestors() {
            if fs::symlink_metadata(ancestor).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(EndpointError::Storage);
            }
        }
        if !root.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(root)
                    .map_err(|_| EndpointError::Storage)?;
            }
            #[cfg(not(unix))]
            {
                fs::create_dir_all(root).map_err(|_| EndpointError::Storage)?;
            }
        }
        check_private(root, true)
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn credentials(&self) -> &dyn CredentialStore {
        self.credentials.as_ref()
    }
    /// Own the process lock for the entire read/compare/write or cache transaction.
    pub fn locked<T>(&self, f: impl FnOnce(&mut Metadata) -> Result<T>) -> Result<T> {
        let _lock = self.lock_file()?;
        let mut metadata = self.metadata()?;
        f(&mut metadata)
    }
    pub(super) fn lock_file(&self) -> Result<File> {
        check_private(&self.root, true)?;
        let lock_path = self.root.join("state.lock");
        if fs::symlink_metadata(&lock_path).is_ok() {
            check_private(&lock_path, false)?;
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options
            .open(lock_path)
            .map_err(|_| EndpointError::Storage)?;
        lock.lock().map_err(|_| EndpointError::Storage)?;
        Ok(lock)
    }
    fn metadata(&self) -> Result<Metadata> {
        let metadata = match self.read_component("identity.json", 65536)? {
            Some(bytes) => serde_json::from_slice::<Metadata>(&bytes)
                .map_err(|_| EndpointError::InvalidCache)?,
            None if self.protected => return Err(EndpointError::LoginRequired),
            None => Metadata::default(),
        };
        metadata.validate()?;
        Ok(metadata)
    }
    pub(crate) fn read_component(
        &self,
        name: &str,
        limit: usize,
    ) -> Result<Option<zeroize::Zeroizing<Vec<u8>>>> {
        if self.protected {
            let value = self.credentials.get(name)?;
            if value.as_ref().is_some_and(|v| v.len() > limit) {
                return Err(EndpointError::InvalidCache);
            }
            return Ok(value);
        }
        read_legacy(&self.root.join(name), limit)
    }
    pub fn read(&self) -> Result<Metadata> {
        self.locked(|m| Ok(m.clone()))
    }
    pub fn write(&self, metadata: &Metadata) -> Result<()> {
        metadata.validate()?;
        let bytes = serde_json::to_vec(metadata).map_err(|_| EndpointError::Invalid)?;
        if bytes.len() > 65536 {
            return Err(EndpointError::Storage);
        }
        self.atomic_write("identity.json", &bytes)
    }
    /// Caller holds locked(); cache group may reuse this for protected ciphertext.
    pub fn atomic_write(&self, name: &str, bytes: &[u8]) -> Result<()> {
        if !matches!(
            name,
            "identity.json" | "snapshot.enc" | "installations.json"
        ) {
            return Err(EndpointError::Invalid);
        }
        if self.protected && name != "snapshot.enc" {
            return self.credentials.set(name, bytes);
        }
        self.write_file(name, bytes)
    }
    pub(super) fn write_file(&self, name: &str, bytes: &[u8]) -> Result<()> {
        let path = self.root.join(name);
        if fs::symlink_metadata(&path).is_ok() {
            check_private(&path, false)?;
        }
        let temporary = self.root.join(format!(".write-{}", Uuid::new_v4()));
        let result = (|| {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options
                .open(&temporary)
                .map_err(|_| EndpointError::Storage)?;
            file.write_all(bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| EndpointError::Storage)?;
            fs::rename(&temporary, path).map_err(|_| EndpointError::Storage)?;
            File::open(&self.root)
                .and_then(|d| d.sync_all())
                .map_err(|_| EndpointError::Storage)
        })();
        let _ = fs::remove_file(temporary);
        result
    }
    pub fn account_prefix(identity: &Identity, generation: Uuid) -> String {
        format!("v1:{}:{generation}:", identity.scope())
    }
    pub fn account(identity: &Identity, generation: Uuid, kind: &str) -> Result<String> {
        if kind.is_empty()
            || kind.len() > 100
            || !kind
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
        {
            return Err(EndpointError::Invalid);
        }
        Ok(format!(
            "{}{kind}",
            Self::account_prefix(identity, generation)
        ))
    }
    /// Called under lock. Persist revocation before Keychain work so failures still deny access.
    pub fn clear_locked(&self, metadata: &mut Metadata) -> Result<()> {
        if let Some(identity) = metadata.active.take() {
            metadata.pending_cleanup.push(Cleanup {
                identity,
                generation: metadata.generation,
                accounts: std::mem::take(&mut metadata.credential_accounts),
            });
        }
        metadata.cache_head_account = None;
        metadata.cache_checkpoint = None;
        metadata.cache_publication_pending = false;
        metadata.upstream_heads.clear();
        metadata.upstream_publication_pending.clear();
        metadata.generation = Uuid::new_v4();
        self.write(metadata)?;
        let cache = self.root.join("snapshot.enc");
        if fs::symlink_metadata(&cache).is_ok() {
            check_private(&cache, false)?;
            fs::remove_file(cache).map_err(|_| EndpointError::Storage)?;
        }
        for cleanup in &metadata.pending_cleanup {
            for account in &cleanup.accounts {
                self.credentials.delete(account)?;
            }
        }
        metadata.pending_cleanup.clear();
        self.write(metadata)
    }
}
pub fn check_private(path: &Path, directory: bool) -> Result<()> {
    let m = fs::symlink_metadata(path).map_err(|_| EndpointError::Storage)?;
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::MetadataExt;
        if m.uid() != rustix::process::geteuid().as_raw() {
            return Err(EndpointError::Storage);
        }
    }
    if m.file_type().is_symlink() || (directory && !m.is_dir()) || (!directory && !m.is_file()) {
        return Err(EndpointError::Storage);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if m.permissions().mode() & 0o077 != 0 {
            return Err(EndpointError::Storage);
        }
    }
    Ok(())
}

fn read_legacy(path: &Path, limit: usize) -> Result<Option<zeroize::Zeroizing<Vec<u8>>>> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(EndpointError::Storage),
        Ok(_) => check_private(path, false)?,
    }
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    File::open(path)
        .and_then(|f| f.take(limit as u64 + 1).read_to_end(&mut bytes))
        .map_err(|_| EndpointError::Storage)?;
    if bytes.len() > limit {
        return Err(EndpointError::Invalid);
    }
    Ok(Some(bytes))
}
#[cfg(target_os = "macos")]
#[path = "cutover.rs"]
mod cutover;
#[cfg(not(target_os = "macos"))]
impl StateStore {
    pub(crate) fn recovery_witness(&self) -> Result<Option<Option<Uuid>>> {
        Ok(None)
    }
    pub(crate) fn recover_authenticated(&self, _: Option<Uuid>, _: &str) -> Result<Uuid> {
        Err(EndpointError::Unsupported)
    }
    pub fn legacy_handoff(_: Option<PathBuf>) -> Result<()> {
        Err(EndpointError::Unsupported)
    }
    pub fn finish_fixture_retirement(_: &Path) -> Result<bool> {
        Err(EndpointError::Unsupported)
    }
    pub fn retire_fixture(_: PathBuf) -> Result<()> {
        Err(EndpointError::Unsupported)
    }
    pub fn native(_: PathBuf, _: crate::credentials::MacKeychain) -> Result<Self> {
        Err(EndpointError::Unsupported)
    }
}
