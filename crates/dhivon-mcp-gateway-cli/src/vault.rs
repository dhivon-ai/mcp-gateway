//! The Keychain account UUID selects the only committed encrypted index. Renaming
//! it with the key bytes avoids stale same-account reads across Darwin processes.
use crate::{EndpointError, Result, credentials::CredentialStore, store::check_private};
use core_foundation::data::CFData;
use dhivon_mcp_gateway_core::crypto::{KeyRing, Sealed};
use hkdf::Hkdf;
use security_framework::{
    item::{ItemClass, ItemSearchOptions, ItemUpdateOptions, ItemUpdateValue, Limit, update_item},
    os::macos::keychain::SecKeychain,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

pub(crate) const UPGRADE_JOURNAL: &str = "upgrade/legacy-accounts";
const MAX_RECORD: usize = 2 * 1024 * 1024;
const MAX_NAMES: usize = 4096;
const HEADER: &[u8; 8] = b"DHVENC02";

pub struct Vault {
    root: PathBuf,
    service: String,
    keychain: SecKeychain,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    seed: [u8; 32],
    records: BTreeMap<String, Uuid>,
}

impl Drop for Index {
    fn drop(&mut self) {
        self.seed.zeroize();
    }
}

pub(crate) fn guard_name(name: &str) -> Option<String> {
    let (scope, id) = name.rsplit_once(":upstream-head:")?;
    Uuid::parse_str(id).ok()?;
    Some(format!("{scope}:upstream-guard:{id}"))
}

fn random_key() -> Result<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0; 32]);
    getrandom::fill(&mut *key).map_err(|_| EndpointError::Storage)?;
    Ok(key)
}

fn ring(secret: &[u8], id: Uuid, purpose: &[u8]) -> Result<KeyRing> {
    let mut key = Zeroizing::new([0; 32]);
    Hkdf::<Sha256>::new(Some(id.as_bytes()), secret)
        .expand(purpose, &mut *key)
        .map_err(|_| EndpointError::Invalid)?;
    KeyRing::new("v2".into(), BTreeMap::from([("v2".into(), *key)]))
        .map_err(|_| EndpointError::Invalid)
}

fn read_sealed(path: &Path, secret: &[u8], id: Uuid, purpose: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if matches!(path.symlink_metadata(),Err(e) if e.kind()==std::io::ErrorKind::NotFound) {
        return Err(EndpointError::InvalidCache);
    }
    check_private(path, false)?;
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| EndpointError::Storage)?
        .take((MAX_RECORD + 37) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| EndpointError::Storage)?;
    if bytes.len() < 36 || bytes.len() > MAX_RECORD + 36 || &bytes[..8] != HEADER {
        return Err(EndpointError::InvalidCache);
    }
    let sealed = Sealed {
        format: 1,
        key_id: "v2".into(),
        nonce: bytes[8..20].to_vec(),
        ciphertext: bytes.split_off(20),
    };
    ring(secret, id, purpose)?
        .open(purpose, &sealed)
        .map(Zeroizing::new)
        .map_err(|_| EndpointError::InvalidCache)
}

fn write_sealed(path: &Path, secret: &[u8], id: Uuid, purpose: &[u8], bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_RECORD {
        return Err(EndpointError::Storage);
    }
    let sealed = ring(secret, id, purpose)?
        .seal(purpose, bytes)
        .map_err(|_| EndpointError::Invalid)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| EndpointError::Storage)?;
    file.write_all(HEADER)
        .and_then(|_| file.write_all(&sealed.nonce))
        .and_then(|_| file.write_all(&sealed.ciphertext))
        .and_then(|_| file.sync_all())
        .map_err(|_| EndpointError::Storage)
}

impl Vault {
    /// Caller supplies a private installation directory and its fixed Keychain namespace.
    pub fn open(parent: &Path, service: String) -> Result<Self> {
        let chain = SecKeychain::default().map_err(|_| EndpointError::Keychain)?;
        Self::in_keychain(parent, service, chain)
    }
    fn in_keychain(parent: &Path, service: String, keychain: SecKeychain) -> Result<Self> {
        check_private(parent, true)?;
        let root = parent.join("endpoint-v2");
        match fs::DirBuilder::new().mode(0o700).create(&root) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(_) => return Err(EndpointError::Storage),
        }
        check_private(&root, true)?;
        File::open(parent)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| EndpointError::Storage)?;
        Ok(Self {
            root,
            service,
            keychain,
        })
    }

    fn lock(&self) -> Result<File> {
        check_private(&self.root, true)?;
        let path = self.root.join("store.lock");
        if path.symlink_metadata().is_ok() {
            check_private(&path, false)?;
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&path)
            .map_err(|_| EndpointError::Storage)?;
        lock.lock().map_err(|_| EndpointError::Storage)?;
        Ok(lock)
    }

    fn query(&self) -> ItemSearchOptions {
        let mut query = ItemSearchOptions::new();
        query
            .keychains(std::slice::from_ref(&self.keychain))
            .class(ItemClass::generic_password())
            .service(&self.service);
        query
    }
    pub(crate) fn current(&self) -> Result<Option<Uuid>> {
        let entries = match self
            .query()
            .load_attributes(true)
            .limit(Limit::Max(2))
            .search()
        {
            Ok(v) => v,
            Err(e) if e.code() == -25300 => return Ok(None),
            Err(_) => return Err(EndpointError::Keychain),
        };
        let [entry] = entries.as_slice() else {
            return Err(EndpointError::InvalidCache);
        };
        let attrs = entry.simplify_dict().ok_or(EndpointError::InvalidCache)?;
        let account = attrs.get("acct").ok_or(EndpointError::InvalidCache)?;
        Uuid::parse_str(account)
            .map(Some)
            .map_err(|_| EndpointError::InvalidCache)
    }

    fn index_path(&self, id: Uuid) -> PathBuf {
        self.root.join(format!("index-{id}"))
    }
    fn blob_path(&self, id: Uuid) -> PathBuf {
        self.root.join(format!("record-{id}"))
    }

    fn load(&self, id: Uuid) -> Result<Index> {
        let mut results = self
            .query()
            .account(&id.to_string())
            .load_data(true)
            .skip_authenticated_items(true)
            .search()
            .map_err(|_| EndpointError::Keychain)?;
        let Some(security_framework::item::SearchResult::Data(value)) = results.pop() else {
            return Err(EndpointError::Keychain);
        };
        let key = Zeroizing::new(value);
        if key.len() != 32 {
            return Err(EndpointError::InvalidCache);
        }
        let bytes = read_sealed(&self.index_path(id), &key, id, b"endpoint-index/v2")?;
        let index: Index =
            serde_json::from_slice(&bytes).map_err(|_| EndpointError::InvalidCache)?;
        if index.records.len() > MAX_NAMES {
            return Err(EndpointError::InvalidCache);
        }
        Ok(index)
    }

    fn commit(&self, previous: Option<Uuid>, index: &Index) -> Result<()> {
        let id = Uuid::new_v4();
        let key = random_key()?;
        let bytes = Zeroizing::new(serde_json::to_vec(index).map_err(|_| EndpointError::Invalid)?);
        write_sealed(
            &self.index_path(id),
            &*key,
            id,
            b"endpoint-index/v2",
            &bytes,
        )?;
        File::open(&self.root)
            .and_then(|d| d.sync_all())
            .map_err(|_| EndpointError::Storage)?;
        let result = if let Some(old) = previous {
            update_item(
                self.query().account(&old.to_string()),
                ItemUpdateOptions::new()
                    .set_account_name(id.to_string())
                    .set_value(ItemUpdateValue::Data(CFData::from_buffer(&*key))),
            )
        } else {
            self.keychain
                .add_generic_password(&self.service, &id.to_string(), &*key)
        };
        result.map_err(|_| EndpointError::Keychain)?;
        if let Some(old) = previous {
            self.unlink(&self.index_path(old))?;
        }
        Ok(())
    }

    fn unlink(&self, path: &Path) -> Result<()> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(EndpointError::Storage),
        }
    }

    /// Only initialize a genuinely new store. Losing the slot or the committed
    /// files is a recovery event and cannot silently create an empty identity.
    pub fn initialize(&self) -> Result<()> {
        let _guard = self.lock()?;
        if let Some(id) = self.current()? {
            let index = self.load(id)?;
            return self.collect_orphans(id, &index);
        }
        for entry in fs::read_dir(&self.root).map_err(|_| EndpointError::Storage)? {
            if entry.map_err(|_| EndpointError::Storage)?.file_name() != "store.lock" {
                return Err(EndpointError::InvalidCache);
            }
        }
        self.commit(
            None,
            &Index {
                seed: *random_key()?,
                records: BTreeMap::new(),
            },
        )
    }

    // Caller holds state.lock. Stage all imported records and verify them before the
    // single Keychain transition. A failed preparation leaves the old format usable.
    pub(crate) fn replace_all(
        &self,
        expected: Option<Uuid>,
        values: &BTreeMap<String, Zeroizing<Vec<u8>>>,
    ) -> Result<()> {
        let _guard = self.lock()?;
        if self.current()? != expected {
            return Err(EndpointError::IdentityChanged);
        }
        if values.len() > MAX_NAMES {
            return Err(EndpointError::CredentialCapacity);
        }
        let mut index = Index {
            seed: *random_key()?,
            records: BTreeMap::new(),
        };
        for (name, value) in values {
            let id = Uuid::new_v4();
            write_sealed(&self.blob_path(id), &index.seed, id, name.as_bytes(), value)?;
            if read_sealed(&self.blob_path(id), &index.seed, id, name.as_bytes())?.as_slice()
                != value.as_slice()
            {
                return Err(EndpointError::InvalidCache);
            }
            index.records.insert(name.clone(), id);
        }
        self.commit(expected, &index)?;
        let head = self.current()?.ok_or(EndpointError::Keychain)?;
        let verified = self.load(head)?;
        self.collect_orphans(head, &verified)
    }
    fn collect_orphans(&self, head: Uuid, index: &Index) -> Result<()> {
        let live: std::collections::BTreeSet<_> = index.records.values().copied().collect();
        for entry in fs::read_dir(&self.root).map_err(|_| EndpointError::Storage)? {
            let entry = entry.map_err(|_| EndpointError::Storage)?;
            let name = entry.file_name();
            let Some((kind, id)) = name.to_str().and_then(|s| s.split_once('-')) else {
                continue;
            };
            let Ok(id) = Uuid::parse_str(id) else {
                continue;
            };
            if (kind == "index" && id != head) || (kind == "record" && !live.contains(&id)) {
                check_private(&entry.path(), false)?;
                self.unlink(&entry.path())?;
            }
        }
        Ok(())
    }

    fn change(&self, name: &str, value: Option<&[u8]>) -> Result<()> {
        if name.len() > 512 || name.is_empty() {
            return Err(EndpointError::Invalid);
        }
        let _guard = self.lock()?;
        let head = self.current()?.ok_or(EndpointError::LoginRequired)?;
        let mut index = self.load(head)?;
        let mut changes = vec![(name.to_owned(), value)];
        let evidence;
        if let Some(guard) = guard_name(name) {
            evidence = value
                .map(crate::upstream_credentials::recovery_evidence)
                .transpose()?;
            changes.push((guard, evidence.as_ref().map(|v| v.as_slice())));
        }
        let mut obsolete = Vec::new();
        let mut wrote = false;
        for (name, value) in changes {
            obsolete.extend(index.records.remove(&name));
            if let Some(value) = value {
                if index.records.len() >= MAX_NAMES {
                    return Err(EndpointError::CredentialCapacity);
                }
                let id = Uuid::new_v4();
                write_sealed(&self.blob_path(id), &index.seed, id, name.as_bytes(), value)?;
                index.records.insert(name, id);
                wrote = true;
            }
        }
        if wrote || !obsolete.is_empty() {
            self.commit(Some(head), &index)?;
            for id in obsolete {
                self.unlink(&self.blob_path(id))?;
            }
        }
        Ok(())
    }

    /// Explicit teardown for isolated installs. Caller has already stopped writers.
    pub fn remove_empty(&self) -> Result<()> {
        self.retire_records(&[])
    }
    pub(crate) fn retire_records(&self, allowed: &[&str]) -> Result<()> {
        let _guard = self.lock()?;
        let Some(id) = self.current()? else {
            return Ok(());
        };
        let index = self.load(id)?;
        if !index.records.keys().all(|k| allowed.contains(&k.as_str())) {
            return Err(EndpointError::Invalid);
        }
        self.collect_orphans(id, &index)?;
        self.query()
            .account(&id.to_string())
            .delete()
            .map_err(|_| EndpointError::Keychain)?;
        for id in index.records.values() {
            self.unlink(&self.blob_path(*id))?;
        }
        self.unlink(&self.index_path(id))
    }
    // Ordinary readers must never see a prepared import, even if somebody
    // restores a previously missing index while the process is already alive.
    pub(crate) fn read_record(
        &self,
        name: &str,
        allow_prepared: bool,
    ) -> Result<Option<Zeroizing<Vec<u8>>>> {
        let _guard = self.lock()?;
        let head = self.current()?.ok_or(EndpointError::LoginRequired)?;
        let index = self.load(head)?;
        if !allow_prepared && index.records.contains_key(UPGRADE_JOURNAL) {
            return Err(EndpointError::LegacyCleanup);
        }
        index
            .records
            .get(name)
            .map(|id| read_sealed(&self.blob_path(*id), &index.seed, *id, name.as_bytes()))
            .transpose()
    }
}
impl CredentialStore for Vault {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        self.read_record(name, false)
    }
    fn set(&self, name: &str, value: &[u8]) -> Result<()> {
        self.change(name, Some(value))
    }
    fn delete(&self, name: &str) -> Result<()> {
        self.change(name, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "manual macOS locked-Keychain check: a SecurityAgent prompt may appear; run explicitly with --ignored --nocapture"]
    fn locked_isolated_keychain_cannot_replace_a_committed_master() {
        use security_framework::os::macos::keychain::CreateOptions;
        let root = std::env::temp_dir()
            .canonicalize()
            .expect("temp")
            .join(format!("dhivon-locked-{}", Uuid::new_v4()));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .expect("fixture");
        let path = root.join("isolated.keychain-db");
        let pass = "Dhivon-Test-Only-27!";
        eprintln!(
            "Creating disposable Keychain: {}. Test password: {pass}. For the locked-access check choose Cancel or Deny; unlocking would change the condition under test.",
            path.display()
        );
        let chain = CreateOptions::new()
            .password(pass)
            .create(&path)
            .expect("isolated test keychain");
        let v = Vault::in_keychain(
            &root,
            format!("dhivon.fixture.{}", Uuid::new_v4()),
            chain.clone(),
        )
        .expect("fixture vault");
        v.initialize().expect("create");
        v.set("a", b"synthetic").expect("publish");
        let before = v.current().expect("head");
        assert!(
            std::process::Command::new("/usr/bin/security")
                .arg("lock-keychain")
                .arg(&path)
                .status()
                .expect("lock only test keychain")
                .success()
        );
        let no_dialog = SecKeychain::disable_user_interaction().expect("local noninteractive read");
        let denied = v.get("a");
        let init = v.initialize();
        let after = v.current().expect("locked slot attributes remain visible");
        drop(no_dialog);
        drop(v);
        drop(chain);
        assert!(
            std::process::Command::new("/usr/bin/security")
                .arg("delete-keychain")
                .arg(&path)
                .status()
                .expect("remove only test keychain")
                .success()
        );
        fs::remove_dir_all(root).expect("remove fixture");
        assert!(matches!(denied, Err(EndpointError::Keychain)));
        assert!(matches!(init, Err(EndpointError::Keychain)));
        assert_eq!(before, after);
    }

    #[test]
    fn committed_index_selects_records_and_discards_abandoned_writes() {
        let root = std::env::temp_dir()
            .canonicalize()
            .expect("temp")
            .join(Uuid::new_v4().to_string());
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .expect("private scope");
        let vault = Vault::open(
            &root,
            format!("com.dhivon.mcp-gateway.fixture.vault.{}", Uuid::new_v4()),
        )
        .expect("open");
        vault.initialize().expect("initialize");
        vault.set("a", b"first").expect("first");
        vault.set("b", b"second").expect("second");
        let head = vault.current().expect("slot").expect("one key");
        let index = vault.load(head).expect("current manifest");
        let record_a = vault.blob_path(index.records["a"]);
        let record_b = vault.blob_path(index.records["b"]);
        let wrong = fs::read(&record_b).expect("encrypted b");
        fs::write(&record_a, wrong).expect("substitute");
        assert!(vault.get("a").is_err());
        assert!(
            vault
                .get("b")
                .expect("unaffected")
                .is_some_and(|b| b.as_slice() == b"second")
        );
        vault.set("a", b"fresh").expect("replace damaged record");
        let orphan = Uuid::new_v4();
        let path = vault.index_path(orphan);
        write_sealed(
            &path,
            &*random_key().expect("key"),
            orphan,
            b"endpoint-index/v2",
            b"staged",
        )
        .expect("simulate precommit crash");
        vault.initialize().expect("recover");
        assert!(!path.exists());
        assert!(
            vault
                .get("a")
                .expect("recovered")
                .is_some_and(|b| b.as_slice() == b"fresh")
        );
        vault.delete("a").expect("delete");
        vault.delete("b").expect("delete");
        vault.remove_empty().expect("cleanup");
        assert!(vault.current().expect("confirm absence").is_none());
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
