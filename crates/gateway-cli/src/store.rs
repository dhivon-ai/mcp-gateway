//! Cross-process generation guards and atomic, ordinary metadata. No credentials in files.
use crate::{EndpointError, Result, credentials::CredentialStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

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
            pending_cleanup: Vec::new(),
        }
    }
}
#[derive(Clone)]
pub struct StateStore {
    root: PathBuf,
    credentials: Arc<dyn CredentialStore>,
}
impl StateStore {
    pub fn production(credentials: Arc<dyn CredentialStore>) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var_os("HOME").ok_or(EndpointError::Storage)?;
            Self::new(
                PathBuf::from(home).join("Library/Application Support/com.cadre.mcp-gateway"),
                credentials,
            )
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = credentials;
            Err(EndpointError::Unsupported)
        }
    }
    pub fn new(root: PathBuf, credentials: Arc<dyn CredentialStore>) -> Result<Self> {
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
                    .create(&root)
                    .map_err(|_| EndpointError::Storage)?;
            }
            #[cfg(not(unix))]
            {
                fs::create_dir_all(&root).map_err(|_| EndpointError::Storage)?;
            }
        }
        check_private(&root, true)?;
        let store = Self { root, credentials };
        store.locked(|metadata| {
            store.write(metadata)?;
            Ok(())
        })?;
        Ok(store)
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn credentials(&self) -> &dyn CredentialStore {
        self.credentials.as_ref()
    }
    /// Own the process lock for the entire read/compare/write or cache transaction.
    pub fn locked<T>(&self, f: impl FnOnce(&mut Metadata) -> Result<T>) -> Result<T> {
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
        let path = self.root.join("identity.json");
        let mut metadata = if fs::symlink_metadata(&path).is_ok() {
            check_private(&path, false)?;
            let mut bytes = Vec::with_capacity(65537);
            File::open(&path)
                .map_err(|_| EndpointError::Storage)?
                .take(65537)
                .read_to_end(&mut bytes)
                .map_err(|_| EndpointError::Storage)?;
            if bytes.len() > 65536 {
                return Err(EndpointError::Invalid);
            }
            serde_json::from_slice::<Metadata>(&bytes).map_err(|_| EndpointError::Invalid)?
        } else {
            Metadata::default()
        };
        if metadata.schema_version != 1 || metadata.credential_accounts.len() > 2048 {
            return Err(EndpointError::Invalid);
        }
        if let Some(origin) = &metadata.selected_server_origin {
            let url = url::Url::parse(origin).map_err(|_| EndpointError::Invalid)?;
            if !matches!(url.scheme(), "https" | "http")
                || url.origin().ascii_serialization() != *origin
            {
                return Err(EndpointError::Invalid);
            }
        }
        if let Some(identity) = &metadata.active {
            identity.validate()?;
            if metadata
                .selected_server_origin
                .as_ref()
                .is_some_and(|origin| *origin != identity.server_origin)
            {
                return Err(EndpointError::Invalid);
            }
            let prefix = Self::account_prefix(identity, metadata.generation);
            if metadata
                .credential_accounts
                .iter()
                .any(|s| !s.starts_with(&prefix) || s.len() > 256)
            {
                return Err(EndpointError::Invalid);
            }
        } else if !metadata.credential_accounts.is_empty() {
            return Err(EndpointError::Invalid);
        }
        if let Some(head) = &metadata.cache_head_account {
            let identity = metadata.active.as_ref().ok_or(EndpointError::Invalid)?;
            let prefix = format!(
                "{}cache-head:",
                Self::account_prefix(identity, metadata.generation)
            );
            if head
                .strip_prefix(&prefix)
                .is_none_or(|suffix| Uuid::parse_str(suffix).is_err())
                || !metadata.credential_accounts.contains(head)
            {
                return Err(EndpointError::Invalid);
            }
        }
        if metadata.pending_cleanup.len() > 16 {
            return Err(EndpointError::Invalid);
        }
        for cleanup in &metadata.pending_cleanup {
            cleanup.identity.validate()?;
            let prefix = Self::account_prefix(&cleanup.identity, cleanup.generation);
            if cleanup.accounts.len() > 2048
                || cleanup
                    .accounts
                    .iter()
                    .any(|s| !s.starts_with(&prefix) || s.len() > 256)
            {
                return Err(EndpointError::Invalid);
            }
        }
        f(&mut metadata)
    }
    pub fn read(&self) -> Result<Metadata> {
        self.locked(|m| Ok(m.clone()))
    }
    pub fn write(&self, metadata: &Metadata) -> Result<()> {
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
        metadata.cache_publication_pending = false;
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
