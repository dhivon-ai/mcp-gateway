//! Native format selection and one-time cutover, serialized with v1 state writers.
use super::*;
use crate::{credentials::MacKeychain, vault::Vault};
use security_framework::item::{ItemClass, ItemSearchOptions, Limit};
use zeroize::Zeroizing;

const FENCE: &[u8] = br#"{"schemaVersion":2}"#;
const RETIRED:&[u8]=br#"{"schemaVersion":2,"fixtureRetired":true,"active":null,"credentialAccounts":[],"pendingCleanup":[]}"#;
const RETIRING: &[u8] = br#"{"schemaVersion":2,"fixtureRetiring":true}"#;
const CLEANUP: &str = "upgrade/legacy-accounts";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Upgrade {
    names: BTreeSet<String>,
    source: Vec<u8>,
    retiring: bool,
    handoff: bool,
    // A v1-readable, signed-out journal reserved for an explicitly requested
    // handoff to the creating executable. It carries no credentials.
    cleanup: Metadata,
}
impl MacKeychain {
    fn accounts(&self) -> Result<BTreeSet<String>> {
        let entries = match ItemSearchOptions::new()
            .class(ItemClass::generic_password())
            .service(&self.service)
            .load_attributes(true)
            .limit(Limit::Max(4097))
            .search()
        {
            Ok(v) => v,
            Err(e) if e.code() == -25300 => return Ok(BTreeSet::new()),
            Err(_) => return Err(EndpointError::Keychain),
        };
        if entries.len() > 4096 {
            return Err(EndpointError::CredentialCapacity);
        }
        entries
            .into_iter()
            .map(|item| {
                item.simplify_dict()
                    .and_then(|mut a| a.remove("acct"))
                    .ok_or(EndpointError::Invalid)
            })
            .collect()
    }
}
impl StateStore {
    pub fn native(root: PathBuf, legacy: MacKeychain) -> Result<Self> {
        Self::cutover(root, legacy, |_| Ok(()))
    }
    fn cutover(
        root: PathBuf,
        legacy: MacKeychain,
        checkpoint: impl Fn(&str) -> Result<()>,
    ) -> Result<Self> {
        Self::private_root(&root)?;
        let vault = Arc::new(Vault::open(&root, format!("{}.master.v2", legacy.service))?);
        let store = Self {
            root,
            credentials: vault.clone(),
            protected: true,
            vault: Some(vault.clone()),
        };
        let _guard = store.lock_file()?;
        let identity = read_legacy(&store.root.join("identity.json"), 65536)?;
        let fenced = identity.as_ref().is_some_and(|v| v.as_slice() == FENCE);
        let source = Sha256::digest(identity.as_ref().map_or(&[][..], |b| b.as_slice())).to_vec();
        let old = vault.current()?;
        if !fenced && old.is_none() {
            let abandoned = fs::read_dir(store.root.join("endpoint-v2"))
                .map_err(|_| EndpointError::Storage)?
                .try_fold(false, |seen, e| {
                    e.map(|e| seen || e.file_name() != "store.lock")
                })
                .map_err(|_| EndpointError::Storage)?;
            if abandoned {
                if !legacy.accounts()?.is_empty() {
                    return Err(EndpointError::LegacyCleanup);
                }
                // Losing both the fence and slot is still a recovery, not a clean initialization.
                return Ok(store);
            }
            let mut metadata: Metadata = match identity {
                Some(v) if v.as_slice() == RETIRED => Metadata::default(),
                Some(v) => serde_json::from_slice(&v).map_err(|_| EndpointError::Invalid)?,
                None => Metadata::default(),
            };
            metadata.validate()?;
            if metadata.cache_checkpoint.is_some() {
                return Err(EndpointError::Invalid);
            }
            let owned: BTreeSet<_> = metadata
                .credential_accounts
                .iter()
                .chain(metadata.pending_cleanup.iter().flat_map(|c| &c.accounts))
                .cloned()
                .collect();
            let accounts = legacy.accounts()?;
            if !accounts.is_subset(&owned) {
                return Err(EndpointError::LegacyCleanup);
            }
            let mut values = BTreeMap::new();
            let mut total = 0usize;
            for name in &accounts {
                let bytes = legacy
                    .get(name)
                    .map_err(|_| EndpointError::LegacyCleanup)?
                    .ok_or(EndpointError::LegacyCleanup)?;
                total = total
                    .checked_add(bytes.len())
                    .ok_or(EndpointError::Invalid)?;
                if bytes.len() > 65536 || total > 64 * 1024 * 1024 {
                    return Err(EndpointError::CredentialCapacity);
                }
                if let Some(guard) = crate::vault::guard_name(name) {
                    values.insert(
                        guard,
                        crate::upstream_credentials::recovery_evidence(&bytes)?,
                    );
                }
                if metadata.cache_head_account.as_ref() == Some(name) {
                    metadata.cache_checkpoint = Some(crate::cache::import_checkpoint(&bytes)?);
                }
                values.insert(name.clone(), bytes);
            }
            if let Some(bytes) = read_legacy(&store.root.join("installations.json"), 262144)? {
                crate::install::validate_receipts(&bytes)?;
                values.insert("installations.json".into(), bytes);
            }
            values.insert(
                "identity.json".into(),
                Zeroizing::new(serde_json::to_vec(&metadata).map_err(|_| EndpointError::Invalid)?),
            );
            let mut cleanup = Metadata {
                installation_id: metadata.installation_id,
                pending_cleanup: metadata.pending_cleanup.clone(),
                ..Default::default()
            };
            if let Some(identity) = metadata.active {
                cleanup.pending_cleanup.push(Cleanup {
                    identity,
                    generation: metadata.generation,
                    accounts: metadata.credential_accounts,
                });
            }
            cleanup.validate()?;
            let journal = Upgrade {
                names: accounts,
                source: source.clone(),
                retiring: false,
                handoff: false,
                cleanup,
            };
            values.insert(
                CLEANUP.into(),
                Zeroizing::new(serde_json::to_vec(&journal).map_err(|_| EndpointError::Invalid)?),
            );
            vault.replace_all(None, &values)?;
            checkpoint("prepared")?;
        }
        // The fence is meaningful to old binaries only. New binaries consult the
        // protected root exclusively once present, even if v1 files are restored.
        let journal = match vault.get(CLEANUP) {
            Ok(j) => j,
            Err(EndpointError::InvalidCache | EndpointError::LoginRequired)
                if fenced && legacy.accounts()?.is_empty() =>
            {
                return Ok(store);
            }
            Err(e) => return Err(e),
        };
        if let Some(bytes) = journal {
            let mut upgrade: Upgrade =
                serde_json::from_slice(&bytes).map_err(|_| EndpointError::InvalidCache)?;
            let remaining = legacy.accounts()?;
            if !remaining.is_subset(&upgrade.names) {
                return Err(EndpointError::LegacyCleanup);
            }
            if !upgrade.handoff {
                if upgrade.retiring && !fenced {
                    return Err(EndpointError::LegacyCleanup);
                }
                if !upgrade.retiring {
                    if (!fenced && source != upgrade.source) || remaining != upgrade.names {
                        return Err(EndpointError::LegacyCleanup);
                    }
                    if old.is_some() {
                        for name in &remaining {
                            let prior = legacy.get(name)?.ok_or(EndpointError::LegacyCleanup)?;
                            if vault
                                .get(name)?
                                .is_none_or(|v| v.as_slice() != prior.as_slice())
                            {
                                return Err(EndpointError::LegacyCleanup);
                            }
                        }
                    }
                }
            }
            if !fenced {
                store.write_file("identity.json", FENCE)?;
            }
            if !upgrade.retiring {
                upgrade.retiring = true;
                vault.set(
                    CLEANUP,
                    &serde_json::to_vec(&upgrade).map_err(|_| EndpointError::Invalid)?,
                )?;
            }
            checkpoint("fenced")?;
            for name in &upgrade.names {
                legacy
                    .delete(name)
                    .map_err(|_| EndpointError::LegacyCleanup)?;
                checkpoint("retired-entry")?;
            }
            if !legacy.accounts()?.is_empty() {
                return Err(EndpointError::LegacyCleanup);
            }
            vault.delete(CLEANUP)?;
        } else {
            if !legacy.accounts()?.is_empty() {
                return Err(EndpointError::LegacyCleanup);
            }
            if !fenced {
                store.write_file("identity.json", FENCE)?;
            }
        }
        for entry in fs::read_dir(&store.root).map_err(|_| EndpointError::Storage)? {
            let entry = entry.map_err(|_| EndpointError::Storage)?;
            let name = entry.file_name();
            if name == "installations.json"
                || name
                    .to_str()
                    .and_then(|n| n.strip_prefix(".write-"))
                    .is_some_and(|s| Uuid::parse_str(s).is_ok())
            {
                check_private(&entry.path(), false)?;
                fs::remove_file(entry.path()).map_err(|_| EndpointError::Storage)?;
            }
        }
        vault.initialize()?;
        Ok(store)
    }
    /// Enable a v1 logout only after atomically discarding all prepared v2
    /// authority. Restoring either format can never resurrect the imported login.
    pub fn legacy_handoff(fixture: Option<PathBuf>) -> Result<()> {
        let (root, legacy) = match fixture {
            Some(root) => {
                let legacy = MacKeychain::fixture(&root)?;
                (root, legacy)
            }
            None => (
                PathBuf::from(std::env::var_os("HOME").ok_or(EndpointError::Storage)?)
                    .join("Library/Application Support/com.dhivon.mcp-gateway"),
                MacKeychain::production(),
            ),
        };
        check_private(&root, true)?;
        let vault = Arc::new(Vault::open(&root, format!("{}.master.v2", legacy.service))?);
        let store = Self {
            root,
            protected: true,
            credentials: vault.clone(),
            vault: Some(vault.clone()),
        };
        let _lock = store.lock_file()?;
        let bytes = vault.get(CLEANUP)?.ok_or(EndpointError::Invalid)?;
        let mut journal: Upgrade =
            serde_json::from_slice(&bytes).map_err(|_| EndpointError::InvalidCache)?;
        journal.cleanup.validate()?;
        if journal.cleanup.active.is_some() || !legacy.accounts()?.is_subset(&journal.names) {
            return Err(EndpointError::LegacyCleanup);
        }
        let m = Metadata {
            installation_id: journal.cleanup.installation_id,
            ..Default::default()
        };
        journal.handoff = true;
        let mut records = BTreeMap::from([
            (
                CLEANUP.into(),
                Zeroizing::new(serde_json::to_vec(&journal).map_err(|_| EndpointError::Invalid)?),
            ),
            (
                "identity.json".into(),
                Zeroizing::new(serde_json::to_vec(&m).map_err(|_| EndpointError::Invalid)?),
            ),
        ]);
        if let Some(receipts) = vault.get("installations.json")? {
            records.insert("installations.json".into(), receipts);
        }
        vault.replace_all(vault.current()?, &records)?;
        store.write_file(
            "identity.json",
            &serde_json::to_vec(&journal.cleanup).map_err(|_| EndpointError::Invalid)?,
        )
    }
    /// Take a CAS witness only when the bootstrap itself cannot authenticate.
    /// Never convert a locked Keychain or filesystem permission error into a reset.
    pub(crate) fn recovery_witness(&self) -> Result<Option<Option<Uuid>>> {
        let Some(v) = &self.vault else {
            return Ok(None);
        };
        let _lock = self.lock_file()?;
        match self.metadata() {
            Err(EndpointError::InvalidCache | EndpointError::LoginRequired) => {
                Ok(Some(v.current()?))
            }
            Ok(_) => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub(crate) fn recover_authenticated(
        &self,
        witness: Option<Uuid>,
        origin: &str,
    ) -> Result<Uuid> {
        let v = self.vault.as_ref().ok_or(EndpointError::Unsupported)?;
        let _lock = self.lock_file()?;
        if !matches!(
            self.metadata(),
            Err(EndpointError::InvalidCache | EndpointError::LoginRequired)
        ) || v.current()? != witness
        {
            return Err(EndpointError::IdentityChanged);
        }
        let m = Metadata {
            selected_server_origin: Some(origin.to_owned()),
            ..Default::default()
        };
        let bytes = Zeroizing::new(serde_json::to_vec(&m).map_err(|_| EndpointError::Invalid)?);
        // Preserve local-only receipts if their record still authenticates. A new
        // installation ID prevents inherited receipts from granting install credit.
        let mut records = BTreeMap::from([("identity.json".into(), bytes)]);
        match v.get("installations.json") {
            Ok(Some(receipts)) => {
                records.insert("installations.json".into(), receipts);
            }
            Ok(None) | Err(EndpointError::LoginRequired | EndpointError::InvalidCache) => (),
            Err(e) => return Err(e),
        }
        v.replace_all(witness, &records)?;
        self.write_file("identity.json", FENCE)?;
        Ok(m.generation)
    }
    pub fn finish_fixture_retirement(root: &Path) -> Result<bool> {
        let _ = MacKeychain::fixture(root)?;
        let marker = read_legacy(&root.join("identity.json"), 65536)?;
        if marker
            .as_ref()
            .is_some_and(|m| matches!(m.as_slice(), RETIRED | RETIRING))
        {
            Self::retire_fixture(root.to_owned())?;
            return Ok(true);
        }
        Ok(false)
    }
    /// Only a fixture namespace derived from this exact private directory can be retired.
    pub fn retire_fixture(root: PathBuf) -> Result<()> {
        let legacy = MacKeychain::fixture(&root)?;
        let service = format!("{}.master.v2", legacy.service);
        let vault = Vault::open(&root, service)?;
        let store = Self {
            root,
            credentials: Arc::new(legacy),
            protected: true,
            vault: None,
        };
        let _guard = store.lock_file()?;
        if !MacKeychain::fixture(store.root())?.accounts()?.is_empty() {
            return Err(EndpointError::LegacyCleanup);
        }
        if vault.current()?.is_none() {
            if !read_legacy(&store.root.join("identity.json"), 65536)?
                .is_some_and(|b| matches!(b.as_slice(), RETIRED | RETIRING))
            {
                return Err(EndpointError::LoginRequired);
            }
            return store.write_file("identity.json", RETIRED);
        }
        let bytes = vault.get("identity.json")?.ok_or(EndpointError::Invalid)?;
        let m: Metadata = serde_json::from_slice(&bytes).map_err(|_| EndpointError::Invalid)?;
        m.validate()?;
        if m.active.is_some() || !m.pending_cleanup.is_empty() || !m.credential_accounts.is_empty()
        {
            return Err(EndpointError::Invalid);
        }
        store.write_file("identity.json", RETIRING)?;
        vault.retire_records(&["installations.json", "identity.json"])?;
        if vault.current()?.is_some() {
            return Err(EndpointError::Keychain);
        }
        store.write_file("identity.json", RETIRED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::DirBuilderExt;
    fn legacy(root: &Path) -> MacKeychain {
        MacKeychain::fixture(root).expect("generated service")
    }
    #[test]
    fn resume_v1_cutover_or_hand_off_without_restoring_retired_authority() {
        for step in [
            "prepared",
            "fenced",
            "retired-entry",
            "handoff",
            "v1-logout",
        ] {
            let root = std::env::temp_dir()
                .canonicalize()
                .expect("temp")
                .join(format!("dhivon-upgrade-{}", Uuid::new_v4()));
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&root)
                .expect("private fixture");
            let v1 = StateStore::new(root.clone(), Arc::new(legacy(&root))).expect("v1");
            let identity = Identity {
                server_origin: "https://owned.example.test".into(),
                deployment_id: Uuid::new_v4().to_string(),
                user_id: Uuid::new_v4().to_string(),
            };
            let name = v1
                .locked(|m| {
                    m.active = Some(identity.clone());
                    let name = StateStore::account(
                        &identity,
                        m.generation,
                        &format!("oauth-save:{}", Uuid::new_v4()),
                    )?;
                    m.credential_accounts.push(name.clone());
                    v1.write(m)?;
                    v1.credentials().set(&name, b"issued-only-once")?;
                    v1.atomic_write("installations.json", b"[]")?;
                    Ok(name)
                })
                .expect("old pending save");
            let fault = if step == "handoff" {
                "fenced"
            } else if step == "v1-logout" {
                "prepared"
            } else {
                step
            };
            assert!(
                StateStore::cutover(root.clone(), legacy(&root), |p| if p == fault {
                    Err(EndpointError::Storage)
                } else {
                    Ok(())
                })
                .is_err()
            );
            if step == "v1-logout" {
                v1.locked(|m| v1.clear_locked(m))
                    .expect("previous executable completed offline logout");
                assert!(matches!(
                    StateStore::native(root.clone(), legacy(&root)),
                    Err(EndpointError::LegacyCleanup)
                ));
            }
            let handoff = matches!(step, "v1-logout" | "handoff");
            if handoff {
                StateStore::legacy_handoff(Some(root.clone()))
                    .expect("explicit signed-out handoff");
                v1.locked(|m| {
                    assert!(m.active.is_none());
                    v1.clear_locked(m)
                })
                .expect("creator cleanup");
            }
            let v2 = StateStore::native(root.clone(), legacy(&root)).expect("finish cutover");
            assert!(legacy(&root).accounts().expect("old service").is_empty());
            assert!(v1.read().is_err());
            assert!(!root.join("installations.json").exists());
            assert_eq!(
                v2.read().expect("encrypted identity").active.is_some(),
                !handoff
            );
            assert_eq!(
                v2.credentials()
                    .get(&name)
                    .expect("owned pending save")
                    .map(|v| v.to_vec()),
                (!handoff).then(|| b"issued-only-once".to_vec())
            );
            v2.locked(|m| v2.clear_locked(m)).expect("scoped cleanup");
            StateStore::retire_fixture(root.clone()).expect("retire sole slot");
            fs::remove_dir_all(root).expect("fixture removed");
        }
    }
    #[test]
    fn distinguish_unavailable_files_and_lost_master_and_compare_rebuild_witness() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir()
            .canonicalize()
            .expect("temp")
            .join(format!("dhivon-recovery-{}", Uuid::new_v4()));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .expect("fixture");
        let store = StateStore::native(root.clone(), legacy(&root)).expect("new");
        let vault = store.vault.as_ref().expect("v2");
        let id = vault.current().expect("slot").expect("master");
        let index = root.join("endpoint-v2").join(format!("index-{id}"));
        fs::set_permissions(&index, fs::Permissions::from_mode(0o644)).expect("inject mode fault");
        assert!(matches!(
            store.recovery_witness(),
            Err(EndpointError::Storage)
        ));
        fs::set_permissions(&index, fs::Permissions::from_mode(0o600))
            .expect("restore private mode");
        security_framework::passwords::delete_generic_password(
            &format!("{}.master.v2", legacy(&root).service),
            &id.to_string(),
        )
        .expect("lose owned key");
        let restart = StateStore::native(root.clone(), legacy(&root)).expect("recovery available");
        assert!(restart.read().is_err());
        let witness = restart
            .recovery_witness()
            .expect("missing")
            .expect("login needed");
        assert!(witness.is_none());
        assert!(
            vault
                .current()
                .expect("no silent reinitialization")
                .is_none()
        );
        restart
            .recover_authenticated(witness, "https://owned.example.test")
            .expect("rebuild after authentication");
        assert!(matches!(
            store.recover_authenticated(witness, "https://other.example.test"),
            Err(EndpointError::IdentityChanged)
        ));
        assert_eq!(
            store
                .read()
                .expect("other reader")
                .selected_server_origin
                .as_deref(),
            Some("https://owned.example.test")
        );
        StateStore::retire_fixture(root.clone()).expect("retire");
        fs::remove_dir_all(root).expect("cleanup");
    }
}
