use dhivon_mcp_gateway_cli::{
    EndpointError,
    cache::SnapshotCache,
    clock::{Clock, ClockSample, NANOS_PER_SECOND},
    credentials::CredentialStore,
    epoch,
    session::SessionManager,
    store::{Identity, StateStore},
    upstream_credentials::{CredentialFault, NativeCredentials},
};
use dhivon_mcp_gateway_core::*;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Default)]
struct Memory {
    values: Mutex<HashMap<String, Vec<u8>>>,
    fail_head: Mutex<bool>,
}
impl CredentialStore for Memory {
    fn get(&self, key: &str) -> dhivon_mcp_gateway_cli::Result<Option<Zeroizing<Vec<u8>>>> {
        Ok(self
            .values
            .lock()
            .expect("fixture keys")
            .get(key)
            .cloned()
            .map(Zeroizing::new))
    }
    fn set(&self, key: &str, value: &[u8]) -> dhivon_mcp_gateway_cli::Result<()> {
        if *self.fail_head.lock().expect("fault switch") && key.contains(":upstream-head:") {
            return Err(EndpointError::Keychain);
        }
        self.values
            .lock()
            .expect("fixture keys")
            .insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, key: &str) -> dhivon_mcp_gateway_cli::Result<()> {
        self.values.lock().expect("fixture keys").remove(key);
        Ok(())
    }
}
struct Time(Mutex<ClockSample>);
impl Clock for Time {
    fn sample(&self) -> dhivon_mcp_gateway_cli::Result<ClockSample> {
        Ok(*self.0.lock().expect("fixture clock"))
    }
}
impl Time {
    fn advance(&self, seconds: u64, wall: bool) {
        let mut time = self.0.lock().expect("fixture clock");
        time.continuous_ns += seconds * NANOS_PER_SECOND;
        if wall {
            time.wall_ns += seconds * NANOS_PER_SECOND;
        }
    }
}
struct Fixture {
    root: PathBuf,
    cache: SnapshotCache,
    native: NativeCredentials,
    time: Arc<Time>,
    memory: Arc<Memory>,
    snapshot: ConfigSnapshot,
}
impl Fixture {
    fn new(oauth: bool) -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .expect("fixture parent")
            .join(format!(
                "gateway-native-credential-owned-{}",
                Uuid::new_v4()
            ));
        let memory = Arc::new(Memory::default());
        let store = StateStore::new(root.clone(), memory.clone()).expect("private fixture store");
        let now = epoch().expect("epoch");
        let user = UserInfo {
            name: "Fixture User".into(),
            id: Uuid::new_v4().to_string(),
            email: "owned-fixture@example.test".into(),
            role: Role::User,
            active: true,
        };
        let session = EndpointSession {
            token: "private-central-fixture".into(),
            user: user.clone(),
            deployment_id: Uuid::new_v4().to_string(),
            expires_at: now + 43200,
        };
        store
            .locked(|metadata| {
                metadata.selected_server_origin = Some("http://127.0.0.1:1".into());
                metadata.active = Some(Identity {
                    server_origin: "http://127.0.0.1:1".into(),
                    deployment_id: session.deployment_id.clone(),
                    user_id: user.id.clone(),
                });
                let account = StateStore::account(
                    metadata.active.as_ref().expect("identity"),
                    metadata.generation,
                    "session",
                )?;
                metadata.credential_accounts.push(account.clone());
                store.write(metadata)?;
                store.credentials().set(
                    &account,
                    &serde_json::to_vec(&session).expect("fixture encoding"),
                )
            })
            .expect("fixture identity");
        let mcp = EffectiveMcp {
            definition: McpDefinition {
                id: Uuid::new_v4().to_string(),
                name: "Credential fixture".into(),
                description: String::new(),
                active: true,
                transport: Transport::Http {
                    url: "http://127.0.0.1:2/mcp".into(),
                },
                auth: if oauth {
                    AuthMode::OAuth {
                        client_id: "public-fixture".into(),
                        scopes: vec!["tools".into()],
                    }
                } else {
                    AuthMode::Bearer {}
                },
                install: None,
                tool_policy: ToolPolicy::Allowlist(vec!["echo".into()]),
            },
            enabled: true,
            secret: if oauth {
                None
            } else {
                Some(SecretRef {
                    id: Uuid::new_v4().to_string(),
                    key: "OWNED_KEY".into(),
                    version: 1,
                })
            },
        };
        let snapshot = ConfigSnapshot {
            schema_version: 1,
            deployment_id: session.deployment_id,
            user,
            revision: 1,
            validated_at: now,
            expires_at: now + 3600,
            session_expires_at: session.expires_at,
            mcps: vec![mcp],
        };
        let time = Arc::new(Time(Mutex::new(ClockSample {
            boot_id: Uuid::new_v4(),
            continuous_ns: 10 * NANOS_PER_SECOND,
            wall_ns: now as u64 * NANOS_PER_SECOND,
        })));
        let cache = SnapshotCache::new(SessionManager::new(store, true), time.clone());
        cache
            .install(
                &cache.sessions.load().expect("session"),
                snapshot.clone(),
                time.sample().expect("time"),
            )
            .expect("cache");
        Self {
            root,
            native: NativeCredentials::new(cache.clone()),
            cache,
            time,
            memory,
            snapshot,
        }
    }
    fn credential(&self, version: u32) -> UpstreamCredential {
        if let AuthMode::Bearer {} = self.snapshot.mcps[0].definition.auth {
            UpstreamCredential::Bearer {
                token: format!("private-upstream-fixture-{version}"),
                secret_id: self.snapshot.mcps[0]
                    .secret
                    .as_ref()
                    .expect("secret")
                    .id
                    .clone(),
                version,
            }
        } else {
            UpstreamCredential::OAuth {
                credentials: OAuthCredentials {
                    access_token: format!("private-oauth-fixture-{version}"),
                    refresh_token: Some("private-refresh-fixture".into()),
                    expires_at: self.snapshot.validated_at + 10,
                    issuer: "http://127.0.0.1:3".into(),
                    resource: "http://127.0.0.1:2/mcp".into(),
                    client_id: "public-fixture".into(),
                    scopes: vec!["tools".into()],
                },
                version,
                installation_id: self
                    .cache
                    .sessions
                    .store
                    .read()
                    .expect("metadata")
                    .installation_id
                    .to_string(),
            }
        }
    }
    fn publish(&self, credential: UpstreamCredential) -> dhivon_mcp_gateway_cli::Result<()> {
        let latest = self.cache.latest()?;
        let scope = self.native.scope(&latest, &self.snapshot.mcps[0])?;
        self.native.publish(&scope, credential, self.time.sample()?)
    }
    fn change(&mut self, revision: bool) {
        if revision {
            self.snapshot.revision += 1;
        }
        let now = self.time.sample().expect("time");
        self.snapshot.validated_at = now.wall_seconds().expect("wall");
        self.snapshot.expires_at = self.snapshot.validated_at + 3600;
        self.cache
            .install(
                &self.cache.sessions.load().expect("session"),
                self.snapshot.clone(),
                now,
            )
            .expect("new snapshot");
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self
            .cache
            .sessions
            .store
            .locked(|metadata| self.cache.sessions.store.clear_locked(metadata));
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn immutable_heads_survive_watermarks_bind_versions_and_never_put_values_in_files() {
    let mut fixture = Fixture::new(false);
    fixture
        .publish(fixture.credential(1))
        .expect("first protected token");
    let proof = fixture.cache.latest().expect("fresh proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("scope");
    let head = fixture.native.load(&scope).expect("head");
    assert_eq!(
        head.access(fixture.time.sample().expect("time"))
            .expect("token")
            .as_str(),
        "private-upstream-fixture-1"
    );
    for _ in 0..5 {
        fixture
            .cache
            .latest()
            .expect("rotating protected watermark");
    }
    fixture
        .native
        .load(&scope)
        .expect("read watermark is not a new snapshot receipt");
    for entry in std::fs::read_dir(&fixture.root).expect("private files") {
        let path = entry.expect("file").path();
        if path.is_file() {
            let bytes = std::fs::read(path).expect("file bytes");
            assert!(!String::from_utf8_lossy(&bytes).contains("private-upstream"));
        }
    }
    fixture.snapshot.mcps[0]
        .secret
        .as_mut()
        .expect("key")
        .version = 2;
    fixture.change(true);
    assert!(
        fixture.native.load(&scope).is_err(),
        "old receipt cannot authorize new binding"
    );
    let latest = fixture.cache.latest().expect("new proof");
    let scope = fixture
        .native
        .cached_scope(&latest, &fixture.snapshot.mcps[0])
        .expect("new scope");
    assert!(fixture.native.load(&scope).is_err());
    assert!(
        fixture.publish(fixture.credential(1)).is_err(),
        "old key version cannot be republished"
    );
    fixture
        .publish(fixture.credential(2))
        .expect("replacement version");
    let latest = fixture.cache.latest().expect("replacement proof");
    let scope = fixture
        .native
        .cached_scope(&latest, &fixture.snapshot.mcps[0])
        .expect("replacement scope");
    assert_eq!(fixture.native.load(&scope).expect("new token").version(), 2);
}

#[test]
fn rejection_is_scoped_persistent_and_cannot_restore_the_same_binding() {
    let mut fixture = Fixture::new(false);
    fixture.publish(fixture.credential(1)).expect("token");
    let proof = fixture.cache.latest().expect("proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("scope");
    let signature = fixture.native.load(&scope).expect("head").signature();
    fixture
        .native
        .reject(&scope, Some(&signature), CredentialFault::Rejected)
        .expect("known rejection");
    assert!(matches!(
        fixture.native.load(&scope),
        Err(EndpointError::UpstreamRejected)
    ));
    assert!(matches!(
        fixture.publish(fixture.credential(1)),
        Err(EndpointError::UpstreamRejected)
    ));
    fixture.snapshot.mcps[0]
        .secret
        .as_mut()
        .expect("key")
        .version = 2;
    fixture.change(true);
    fixture
        .publish(fixture.credential(2))
        .expect("new credential");
    let proof = fixture.cache.latest().expect("proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("scope");
    fixture
        .native
        .reject(&scope, Some(&signature), CredentialFault::Rejected)
        .expect("old rejection ignored");
    assert!(
        fixture
            .native
            .load(&scope)
            .expect("new head")
            .access(fixture.time.sample().expect("time"))
            .is_ok()
    );
}

#[test]
fn switching_keys_can_restart_the_version_and_late_old_rejection_cannot_poison_it() {
    let mut fixture = Fixture::new(false);
    fixture.snapshot.mcps[0]
        .secret
        .as_mut()
        .expect("key")
        .version = 8;
    fixture.change(true);
    fixture.publish(fixture.credential(8)).expect("old key");
    let proof = fixture.cache.latest().expect("proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("scope");
    let old = fixture.native.load(&scope).expect("old head").signature();
    fixture.snapshot.mcps[0].secret = Some(SecretRef {
        id: Uuid::new_v4().to_string(),
        key: "OTHER_OWNED_KEY".into(),
        version: 1,
    });
    fixture.change(true);
    let proof = fixture.cache.latest().expect("proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("new key scope");
    fixture
        .native
        .reject(&scope, Some(&old), CredentialFault::Rejected)
        .expect("old response ignored");
    fixture
        .native
        .reject(&scope, None, CredentialFault::Required)
        .expect("known refusal before new-key download");
    fixture
        .publish(fixture.credential(1))
        .expect("new key's version1");
    let proof = fixture.cache.latest().expect("proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("scope");
    assert_eq!(
        fixture.native.load(&scope).expect("new key head").version(),
        1
    );
}

#[test]
fn failed_keychain_publication_denies_until_complete_online_recovery() {
    let fixture = Fixture::new(false);
    fixture.publish(fixture.credential(1)).expect("token");
    let proof = fixture.cache.latest().expect("proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("scope");
    *fixture.memory.fail_head.lock().expect("fault") = true;
    assert!(
        fixture
            .native
            .reject(&scope, None, CredentialFault::Required)
            .is_err()
    );
    assert!(fixture.native.load(&scope).is_err());
    assert!(
        fixture.cache.latest().is_ok(),
        "credential failure does not corrupt configuration"
    );
    *fixture.memory.fail_head.lock().expect("fault") = false;
    fixture
        .publish(fixture.credential(1))
        .expect("authenticated complete recovery");
    let proof = fixture.cache.latest().expect("proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("scope");
    assert!(
        fixture
            .native
            .load(&scope)
            .expect("recovered head")
            .access(fixture.time.sample().expect("time"))
            .is_ok()
    );
}

#[test]
fn oauth_sleep_and_unchanged_refresh_cannot_extend_token_or_policy_lifetime() {
    let mut fixture = Fixture::new(true);
    let credential = fixture.credential(1);
    fixture.publish(credential.clone()).expect("token");
    fixture.time.advance(9, false);
    fixture.change(false);
    fixture
        .publish(credential.clone())
        .expect("unchanged credential resolution");
    let proof = fixture.cache.latest().expect("proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("scope");
    assert!(
        fixture
            .native
            .load(&scope)
            .expect("head")
            .access(fixture.time.sample().expect("time"))
            .is_ok()
    );
    fixture.time.advance(1, false);
    fixture.change(false);
    fixture
        .publish(credential)
        .expect("unchanged credential stays anchored");
    let proof = fixture.cache.latest().expect("proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("scope");
    assert!(matches!(
        fixture
            .native
            .load(&scope)
            .expect("head")
            .access(fixture.time.sample().expect("time")),
        Err(EndpointError::UpstreamExpired)
    ));
    fixture.time.advance(3600, true);
    assert!(fixture.cache.latest().is_err());
}

#[test]
fn oauth_partial_publication_recovery_preserves_the_existing_sleep_aware_deadline() {
    let fixture = Fixture::new(true);
    let credential = fixture.credential(1);
    fixture.publish(credential.clone()).expect("token");
    fixture.time.advance(9, false);
    let proof = fixture.cache.latest().expect("proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("scope");
    *fixture.memory.fail_head.lock().expect("fault") = true;
    assert!(
        fixture
            .native
            .reject(&scope, None, CredentialFault::Required)
            .is_err()
    );
    *fixture.memory.fail_head.lock().expect("fault") = false;
    fixture.publish(credential).expect("complete recovery");
    fixture.time.advance(1, false);
    let proof = fixture.cache.latest().expect("proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("scope");
    assert!(matches!(
        fixture
            .native
            .load(&scope)
            .expect("head")
            .access(fixture.time.sample().expect("time")),
        Err(EndpointError::UpstreamExpired)
    ));
}

#[test]
fn sequential_mcp_capacity_keeps_identity_and_existing_credentials_usable() {
    let mut fixture = Fixture::new(false);
    let first = fixture.snapshot.mcps[0].clone();
    for index in 0..128 {
        if index > 0 {
            fixture.snapshot.mcps[0].definition.id = Uuid::new_v4().to_string();
            fixture.change(true);
        }
        fixture
            .publish(fixture.credential(1))
            .expect("bounded head");
    }
    let store = &fixture.cache.sessions.store;
    assert_eq!(
        store
            .read()
            .expect("readable boundary")
            .upstream_heads
            .len(),
        128
    );
    fixture.snapshot.mcps[0].definition.id = Uuid::new_v4().to_string();
    fixture.change(true);
    assert!(matches!(
        fixture.publish(fixture.credential(1)),
        Err(EndpointError::CredentialCapacity)
    ));
    let metadata = fixture
        .cache
        .sessions
        .store
        .read()
        .expect("capacity must not corrupt identity");
    assert_eq!(metadata.upstream_heads.len(), 128);
    assert!(metadata.upstream_publication_pending.is_empty());
    fixture
        .cache
        .sessions
        .load()
        .expect("session/status still usable");
    fixture.cache.latest().expect("shared cache still usable");
    // A complete subsequent sync can still refresh an existing identity at capacity.
    fixture.snapshot.mcps[0] = first;
    fixture.change(true);
    fixture
        .publish(fixture.credential(1))
        .expect("existing MCP revalidation at capacity");
    let proof = fixture.cache.latest().expect("proof");
    let scope = fixture
        .native
        .cached_scope(&proof, &fixture.snapshot.mcps[0])
        .expect("scope");
    fixture
        .native
        .load(&scope)
        .expect("existing credential")
        .access(fixture.time.sample().expect("time"))
        .expect("usable token");
    fixture
        .cache
        .sessions
        .store
        .locked(|m| fixture.cache.sessions.store.clear_locked(m))
        .expect("logout at capacity");
    let metadata = fixture
        .cache
        .sessions
        .store
        .read()
        .expect("cleared identity");
    assert!(
        metadata.active.is_none()
            && metadata.credential_accounts.is_empty()
            && metadata.pending_cleanup.is_empty()
    );
    assert!(fixture.memory.values.lock().expect("keys").is_empty());
}

#[test]
fn invalid_metadata_write_preserves_the_last_readable_identity() {
    let fixture = Fixture::new(false);
    let store = &fixture.cache.sessions.store;
    let good = std::fs::read(store.root().join("identity.json")).expect("original metadata");
    let original = store.read().expect("identity");
    let mut cases = Vec::new();
    let mut invalid = original.clone();
    invalid.schema_version = 2;
    cases.push(invalid);
    let mut invalid = original.clone();
    invalid.upstream_publication_pending = (0..129).map(|i| format!("mcp-{i}")).collect();
    cases.push(invalid);
    let mut invalid = original.clone();
    invalid
        .credential_accounts
        .push("foreign-owner-account".into());
    cases.push(invalid);
    let mut invalid = original.clone();
    invalid
        .upstream_heads
        .insert("mcp".into(), "untracked-head".into());
    cases.push(invalid);
    let mut invalid = original.clone();
    invalid.pending_cleanup = (0..17)
        .map(|_| dhivon_mcp_gateway_cli::store::Cleanup {
            identity: original.active.clone().expect("identity"),
            generation: original.generation,
            accounts: vec![],
        })
        .collect();
    cases.push(invalid);
    for invalid in cases {
        assert!(
            store.write(&invalid).is_err(),
            "invalid write must fail before replacement"
        );
        assert_eq!(
            std::fs::read(store.root().join("identity.json")).expect("bytes"),
            good
        );
        store
            .read()
            .expect("last valid state survives rejected write");
        fixture.cache.sessions.load().expect("login remains usable");
    }
}
