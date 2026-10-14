//! Generation-pinned management consent and control-plane coordinated token replacement.
use crate::{
    EndpointError, Result,
    bridge::{ControlBridge, segment},
    central::bounded_body,
    clock::ClockSample,
    oauth_pending::{PendingToken, pending, registration},
    oauth_protocol::{Consent, OAuthProtocol},
    session::NativeSession,
    sync::SyncEngine,
    upstream_credentials::{CredentialFault, NativeCredentials},
};
use gateway_core::{
    AuthMode, EffectiveMcp, OAuthCredentials, OAuthDisconnect, OAuthStatus, OAuthStoreRequest,
    RefreshLease, RefreshRelease, RefreshRequest, Transport, UpstreamCredential,
};
use reqwest::Method;
use serde::de::DeserializeOwned;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

pub type ConsentStart = gateway_core::OAuthConsent;
pub type ConnectInput = gateway_core::OAuthConnectInput;
pub type DisconnectInput = gateway_core::OAuthDisconnectInput;
struct Flow {
    generation: Uuid,
    revision: u32,
    mcp: EffectiveMcp,
    version: Option<u32>,
    consent: Consent,
    cancelled: tokio_util::sync::CancellationToken,
    processing: std::sync::atomic::AtomicBool,
}
struct TokenUpdate {
    version: Option<u32>,
    lease: Option<String>,
    credentials: OAuthCredentials,
    observed: ClockSample,
    cancelled: Option<tokio_util::sync::CancellationToken>,
}
#[derive(Clone)]
pub struct OAuthControl {
    pub sync: SyncEngine,
    callback: String,
    fixture_http: bool,
    flows: Arc<Mutex<BTreeMap<String, Arc<Flow>>>>,
}
impl OAuthControl {
    pub fn new(sync: SyncEngine, callback: String, fixture_http: bool) -> Self {
        Self {
            sync,
            callback,
            fixture_http,
            flows: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
    async fn prepare(
        &self,
        generation: &str,
        id: &str,
        revision: Option<u32>,
    ) -> Result<(NativeSession, EffectiveMcp, u32)> {
        segment(id).map_err(|_| EndpointError::Invalid)?;
        let prepared = ControlBridge::new(self.sync.clone())
            .prepare(generation, revision)
            .await
            .map_err(|error| {
                if error.status == reqwest::StatusCode::CONFLICT {
                    EndpointError::IdentityChanged
                } else {
                    EndpointError::Offline
                }
            })?;
        let mcp = prepared
            .snapshot
            .mcps
            .iter()
            .find(|mcp| {
                mcp.definition.id == id
                    && mcp.definition.active
                    && matches!(mcp.definition.auth, AuthMode::OAuth { .. })
            })
            .cloned()
            .ok_or(EndpointError::UpstreamRequired)?;
        Ok((prepared.session, mcp, prepared.snapshot.revision))
    }
    async fn request<T: DeserializeOwned>(
        &self,
        session: &NativeSession,
        method: Method,
        path: &str,
        input: Option<serde_json::Value>,
    ) -> Result<T> {
        let response = self
            .sync
            .cache
            .sessions
            .control_request_for(session, method, path, input.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(if response.status() == reqwest::StatusCode::CONFLICT {
                EndpointError::IdentityChanged
            } else {
                EndpointError::UpstreamRequired
            });
        }
        let body = bounded_body(response).await?;
        let decoded = serde_json::from_slice(&body).map_err(|_| EndpointError::Response)?;
        self.sync
            .cache
            .sessions
            .guard_generation(session.metadata.generation)?;
        Ok(decoded)
    }
    async fn central_status(&self, session: &NativeSession, id: &str) -> Result<OAuthStatus> {
        self.request(
            session,
            Method::GET,
            &format!(
                "/api/oauth/{id}/status/{}",
                session.metadata.installation_id
            ),
            None,
        )
        .await
    }
    pub async fn status(
        &self,
        generation: &str,
        id: &str,
    ) -> Result<gateway_core::LocalOAuthStatus> {
        self.recover(
            Uuid::parse_str(generation).map_err(|_| EndpointError::Invalid)?,
            id,
        )
        .await?;
        let (session, mcp, revision) = self.prepare(generation, id, None).await?;
        let status = self.central_status(&session, id).await?;
        let readiness = if !status.connected {
            "disconnected"
        } else {
            let credential:UpstreamCredential=self.request(&session,Method::POST,&format!("/api/endpoint/oauth/{id}/credentials"),Some(serde_json::json!({"expectedRevision":revision,"installationId":session.metadata.installation_id.to_string()}))).await?;
            let native = NativeCredentials::new(self.sync.cache.clone());
            let proof = self.sync.cache.latest()?;
            let scope = native.management_scope(&proof, &mcp)?;
            let outcome = native
                .publish(&scope, credential, self.sync.cache.sample()?)
                .and_then(|()| native.load(&scope))
                .and_then(|head| head.access(self.sync.cache.sample()?));
            match outcome {
                Ok(_) => "ready",
                Err(EndpointError::UpstreamRejected) => "auth_rejected",
                Err(EndpointError::UpstreamExpired) => "token_expired",
                Err(_) => "credential_sync_required",
            }
        };
        Ok(gateway_core::LocalOAuthStatus {
            connected: status.connected,
            version: status.version,
            expires_at: status.expires_at,
            readiness: readiness.into(),
        })
    }
    pub async fn start(
        &self,
        generation: &str,
        id: &str,
        input: ConnectInput,
    ) -> Result<ConsentStart> {
        self.recover(
            Uuid::parse_str(generation).map_err(|_| EndpointError::Invalid)?,
            id,
        )
        .await?;
        let (session, mcp, revision) = self
            .prepare(generation, id, Some(input.expected_revision))
            .await?;
        if !pending(&self.sync.cache.sessions, &session, &mcp)?.is_empty() {
            return Err(EndpointError::SyncRequired);
        }
        let status = self.central_status(&session, id).await?;
        let (AuthMode::OAuth { client_id, scopes }, Transport::Http { url }) =
            (&mcp.definition.auth, &mcp.definition.transport)
        else {
            return Err(EndpointError::Invalid);
        };
        let protocol =
            OAuthProtocol::discover(url, client_id, scopes, &self.callback, self.fixture_http)
                .await?;
        let began = self.sync.cache.sample()?;
        let consent = protocol.consent(began).await?;
        self.sync
            .cache
            .sessions
            .guard_generation(session.metadata.generation)?;
        let output = ConsentStart {
            flow_id: Uuid::new_v4().to_string(),
            authorization_url: consent.authorization_url.clone(),
        };
        let mut flows = self.flows.lock().await;
        // At most one consent per MCP/generation; replacing it is an explicit new connect action.
        flows.retain(|_, flow| {
            let keep = flow.generation == session.metadata.generation
                && flow.mcp.definition.id != id
                && flow.consent.live(began);
            if !keep {
                flow.cancelled.cancel();
            }
            keep
        });
        if flows.len() >= 16 {
            return Err(EndpointError::Invalid);
        }
        flows.insert(
            output.flow_id.clone(),
            Arc::new(Flow {
                generation: session.metadata.generation,
                revision,
                mcp,
                version: status.version,
                consent,
                cancelled: tokio_util::sync::CancellationToken::new(),
                processing: std::sync::atomic::AtomicBool::new(false),
            }),
        );
        Ok(output)
    }
    pub async fn cancel(&self, generation: &str, flow_id: &str) -> Result<()> {
        let generation = Uuid::parse_str(generation).map_err(|_| EndpointError::Invalid)?;
        self.sync.cache.sessions.guard_generation(generation)?;
        let mut flows = self.flows.lock().await;
        if flows
            .get(flow_id)
            .is_some_and(|flow| flow.generation == generation)
        {
            if let Some(flow) = flows.get(flow_id) {
                flow.cancelled.cancel();
            }
            flows.remove(flow_id);
        }
        Ok(())
    }
    pub async fn cancel_generation(&self, generation: Uuid) {
        self.flows.lock().await.retain(|_, flow| {
            if flow.generation == generation {
                flow.cancelled.cancel();
                false
            } else {
                true
            }
        });
    }
    pub async fn callback(&self, raw: &str) -> Result<OAuthStatus> {
        let url = url::Url::parse(raw).map_err(|_| EndpointError::Invalid)?;
        let states: Vec<_> = url
            .query_pairs()
            .filter(|(key, _)| key == "state")
            .map(|(_, value)| value.into_owned())
            .collect();
        if states.len() != 1 {
            return Err(EndpointError::Invalid);
        }
        let now = self.sync.cache.sample()?;
        let (key, flow) = {
            let flows = self.flows.lock().await;
            flows
                .iter()
                .find(|(_, flow)| flow.consent.state() == states[0])
                .map(|(key, flow)| (key.clone(), flow.clone()))
                .ok_or(EndpointError::Invalid)?
        };
        flow.consent.callback(raw, now)?;
        let (session, current, revision) = self
            .prepare(
                &flow.generation.to_string(),
                &flow.mcp.definition.id,
                Some(flow.revision),
            )
            .await?;
        if registration(&current)? != registration(&flow.mcp)? {
            return Err(EndpointError::IdentityChanged);
        }
        // Consume once after validation and online authorization; no token exchange on rejected callbacks.
        if flow.cancelled.is_cancelled()
            || flow
                .processing
                .compare_exchange(
                    false,
                    true,
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                )
                .is_err()
        {
            return Err(EndpointError::Cancelled);
        }
        let observed = self.sync.cache.sample()?;
        let credential = flow.consent.exchange(raw, observed).await?;
        if flow.cancelled.is_cancelled() {
            return Err(EndpointError::Cancelled);
        }
        let outcome = self
            .persist(
                &session,
                &current,
                revision,
                TokenUpdate {
                    version: flow.version,
                    lease: None,
                    credentials: credential,
                    observed,
                    cancelled: Some(flow.cancelled.clone()),
                },
            )
            .await;
        self.flows.lock().await.remove(&key);
        outcome
    }
    async fn persist(
        &self,
        session: &NativeSession,
        mcp: &EffectiveMcp,
        revision: u32,
        update: TokenUpdate,
    ) -> Result<OAuthStatus> {
        let TokenUpdate {
            version,
            lease,
            credentials,
            observed,
            cancelled,
        } = update;
        if cancelled
            .as_ref()
            .is_some_and(|cancel| cancel.is_cancelled())
        {
            return Err(EndpointError::Cancelled);
        }
        self.sync
            .cache
            .sessions
            .guard_generation(session.metadata.generation)?;
        let native = NativeCredentials::new(self.sync.cache.clone());
        let proof = self.sync.cache.latest()?;
        let scope = native.management_scope(&proof, mcp)?;
        if proof.snapshot.revision != revision {
            return Err(EndpointError::IdentityChanged);
        }
        let journal = PendingToken::new(
            session,
            mcp,
            version,
            lease.clone(),
            credentials.clone(),
            observed,
        )?;
        journal.remember(&self.sync.cache, proof.receipt)?;
        // A request with an unknown outcome must not leave an old native token dispatchable.
        native.reject(&scope, None, CredentialFault::SyncRequired)?;
        if cancelled
            .as_ref()
            .is_some_and(|cancel| cancel.is_cancelled())
        {
            journal.forget(&self.sync.cache.sessions)?;
            return Err(EndpointError::Cancelled);
        }
        let input = OAuthStoreRequest {
            expected_revision: revision,
            installation_id: session.metadata.installation_id.to_string(),
            expected_version: version,
            refresh_lease: lease,
            credentials: credentials.clone(),
        };
        let status: OAuthStatus = self
            .request(
                session,
                Method::PUT,
                &format!("/api/endpoint/oauth/{}", mcp.definition.id),
                Some(serde_json::to_value(input).map_err(|_| EndpointError::Invalid)?),
            )
            .await?;
        let accepted = version
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(EndpointError::Invalid)?;
        if !status.connected
            || status.version != Some(accepted)
            || status.expires_at != Some(credentials.expires_at)
        {
            return Err(EndpointError::Response);
        }
        self.sync.refresh().await?;
        self.sync
            .cache
            .sessions
            .guard_generation(session.metadata.generation)?;
        let proof = self.sync.cache.latest()?;
        let scope = native.management_scope(&proof, mcp)?;
        native.publish(
            &scope,
            UpstreamCredential::OAuth {
                credentials,
                version: accepted,
                installation_id: session.metadata.installation_id.to_string(),
            },
            observed,
        )?;
        journal.forget(&self.sync.cache.sessions)?;
        tracing::info!(event="oauth_connection_confirmed",mcp_id=%mcp.definition.id,version=accepted);
        Ok(status)
    }
    /// Automatic refresh remains non-interactive and requires complete online authorization first.
    pub async fn recover(&self, generation: Uuid, id: &str) -> Result<()> {
        let (session, mcp, revision) = self.prepare(&generation.to_string(), id, None).await?;
        let mut entries = pending(&self.sync.cache.sessions, &session, &mcp)?;
        let Some(journal) = entries.pop() else {
            return Ok(());
        };
        let now = self.sync.cache.sample()?;
        let response=self.sync.cache.sessions.control_request_for(&session,Method::POST,&format!("/api/endpoint/oauth/{id}/credentials"),Some(&serde_json::json!({"expectedRevision":revision,"installationId":session.metadata.installation_id.to_string()}))).await?;
        let central = if response.status() == reqwest::StatusCode::PRECONDITION_REQUIRED {
            None
        } else {
            if !response.status().is_success() {
                return Err(EndpointError::UpstreamRequired);
            }
            let bytes = bounded_body(response).await?;
            Some(
                serde_json::from_slice::<UpstreamCredential>(&bytes)
                    .map_err(|_| EndpointError::Response)?,
            )
        };
        let accepted = journal
            .previous_version
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(EndpointError::Invalid)?;
        let native = NativeCredentials::new(self.sync.cache.clone());
        if let Some(UpstreamCredential::OAuth {
            credentials,
            version,
            installation_id,
        }) = &central
        {
            if installation_id != &session.metadata.installation_id.to_string() {
                return Err(EndpointError::Invalid);
            }
            if *version >= accepted {
                let same = serde_json::to_vec(credentials).map_err(|_| EndpointError::Invalid)?
                    == serde_json::to_vec(&journal.credentials)
                        .map_err(|_| EndpointError::Invalid)?;
                let observed = if same { journal.observed } else { now };
                let proof = self.sync.cache.latest()?;
                let scope = native.management_scope(&proof, &mcp)?;
                native.publish(&scope, central.ok_or(EndpointError::Invalid)?, observed)?;
                journal.forget(&self.sync.cache.sessions)?;
                return Ok(());
            }
            if Some(*version) != journal.previous_version {
                return Err(EndpointError::RevisionRegression);
            }
        } else if central.is_some() || journal.previous_version.is_some() {
            return Err(EndpointError::UpstreamRequired);
        }
        if !journal.usable(now)? {
            journal.forget(&self.sync.cache.sessions)?;
            return Err(EndpointError::UpstreamExpired);
        }
        let mut input = OAuthStoreRequest {
            expected_revision: revision,
            installation_id: session.metadata.installation_id.to_string(),
            expected_version: journal.previous_version,
            refresh_lease: journal.lease.clone(),
            credentials: journal.credentials.clone(),
        };
        let path = format!("/api/endpoint/oauth/{id}");
        let status = self
            .request::<OAuthStatus>(
                &session,
                Method::PUT,
                &path,
                Some(serde_json::to_value(&input).map_err(|_| EndpointError::Invalid)?),
            )
            .await;
        let status = match status {
            Err(EndpointError::IdentityChanged) if journal.lease.is_some() => {
                let version = journal.previous_version.ok_or(EndpointError::Invalid)?;
                let lease:RefreshLease=self.request(&session,Method::POST,&format!("{path}/lease"),Some(serde_json::json!({"installationId":session.metadata.installation_id.to_string(),"expectedVersion":version}))).await?;
                if lease.version != version {
                    return Err(EndpointError::Invalid);
                }
                input.refresh_lease = Some(lease.lease_id.clone());
                let _release = RefreshOwner {
                    sync: self.sync.clone(),
                    session: Arc::new(NativeSession {
                        metadata: session.metadata.clone(),
                        credentials: session.credentials.clone(),
                    }),
                    id: id.into(),
                    lease: lease.lease_id,
                };
                self.request::<OAuthStatus>(
                    &session,
                    Method::PUT,
                    &path,
                    Some(serde_json::to_value(&input).map_err(|_| EndpointError::Invalid)?),
                )
                .await?
            }
            other => other?,
        };
        if !status.connected
            || status.version != Some(accepted)
            || status.expires_at != Some(journal.credentials.expires_at)
        {
            return Err(EndpointError::Response);
        }
        self.sync.refresh().await?;
        self.sync.cache.sessions.guard_generation(generation)?;
        let proof = self.sync.cache.latest()?;
        let scope = native.management_scope(&proof, &mcp)?;
        native.publish(
            &scope,
            UpstreamCredential::OAuth {
                credentials: journal.credentials.clone(),
                version: accepted,
                installation_id: session.metadata.installation_id.to_string(),
            },
            journal.observed,
        )?;
        journal.forget(&self.sync.cache.sessions)
    }
    /// Automatic refresh remains non-interactive and requires complete online authorization first.
    pub async fn refresh(&self, generation: Uuid, id: &str) -> Result<()> {
        let (session, mcp, revision) = self.prepare(&generation.to_string(), id, None).await?;
        if !mcp.enabled {
            return Err(EndpointError::UpstreamRequired);
        }
        if !pending(&self.sync.cache.sessions, &session, &mcp)?.is_empty() {
            return self.recover(generation, id).await;
        }
        let previous:UpstreamCredential = self.request(&session,Method::POST,&format!("/api/endpoint/credentials/{id}"),Some(serde_json::json!({"expectedRevision":revision,"installationId":session.metadata.installation_id.to_string()}))).await?;
        let UpstreamCredential::OAuth {
            credentials,
            version,
            installation_id,
        } = previous
        else {
            return Err(EndpointError::Invalid);
        };
        if installation_id != session.metadata.installation_id.to_string() {
            return Err(EndpointError::Invalid);
        }
        let (AuthMode::OAuth { client_id, scopes }, Transport::Http { url }) =
            (&mcp.definition.auth, &mcp.definition.transport)
        else {
            return Err(EndpointError::Invalid);
        };
        let protocol =
            OAuthProtocol::discover(url, client_id, scopes, &self.callback, self.fixture_http)
                .await?;
        let input = RefreshRequest {
            installation_id,
            expected_version: version,
        };
        let lease: RefreshLease = self
            .request(
                &session,
                Method::POST,
                &format!("/api/endpoint/oauth/{id}/lease"),
                Some(serde_json::to_value(input).map_err(|_| EndpointError::Invalid)?),
            )
            .await?;
        if lease.version != version || Uuid::parse_str(&lease.lease_id).is_err() {
            return Err(EndpointError::Invalid);
        }
        let release = RefreshOwner {
            sync: self.sync.clone(),
            session: Arc::new(session),
            id: id.to_owned(),
            lease: lease.lease_id.clone(),
        };
        let observed = self.sync.cache.sample()?;
        if lease.expires_at <= observed.wall_seconds()? {
            return Err(EndpointError::UpstreamRequired);
        }
        let updated = protocol.refresh(&credentials, observed).await;
        let updated = match updated {
            Err(EndpointError::UpstreamRejected) => {
                let native = NativeCredentials::new(self.sync.cache.clone());
                let proof = self.sync.cache.latest()?;
                let scope = native.scope(&proof, &mcp)?;
                if let Ok(head) = native.load(&scope)
                    && head.version() == version
                {
                    native.reject(&scope, Some(&head.signature()), CredentialFault::Rejected)?;
                }
                return Err(EndpointError::UpstreamRejected);
            }
            outcome => outcome?,
        };
        self.persist(
            &release.session,
            &mcp,
            revision,
            TokenUpdate {
                version: Some(version),
                lease: Some(lease.lease_id),
                credentials: updated,
                observed,
                cancelled: None,
            },
        )
        .await?;
        Ok(())
    }
    pub async fn disconnect(
        &self,
        generation: &str,
        id: &str,
        input: DisconnectInput,
    ) -> Result<()> {
        let (session, mcp, revision) = self
            .prepare(generation, id, Some(input.expected_revision))
            .await?;
        let native = NativeCredentials::new(self.sync.cache.clone());
        let proof = self.sync.cache.latest()?;
        let scope = native.management_scope(&proof, &mcp)?;
        native.disconnect_oauth(&scope)?;
        for journal in pending(&self.sync.cache.sessions, &session, &mcp)? {
            journal.forget(&self.sync.cache.sessions)?;
        }
        self.flows.lock().await.retain(|_, flow| {
            let keep =
                flow.generation != session.metadata.generation || flow.mcp.definition.id != id;
            if !keep {
                flow.cancelled.cancel();
            }
            keep
        });
        let input = OAuthDisconnect {
            expected_revision: revision,
            installation_id: session.metadata.installation_id.to_string(),
            expected_version: input.expected_version,
        };
        let response = self
            .sync
            .cache
            .sessions
            .control_request_for(
                &session,
                Method::DELETE,
                &format!("/api/endpoint/oauth/{id}"),
                Some(&serde_json::to_value(input).map_err(|_| EndpointError::Invalid)?),
            )
            .await?;
        if !response.status().is_success() {
            return Err(EndpointError::UpstreamRequired);
        }
        self.sync.refresh().await?;
        self.sync
            .cache
            .sessions
            .guard_generation(session.metadata.generation)?;
        for journal in pending(&self.sync.cache.sessions, &session, &mcp)? {
            journal.forget(&self.sync.cache.sessions)?;
        }
        self.flows.lock().await.retain(|_, flow| {
            let keep =
                flow.generation != session.metadata.generation || flow.mcp.definition.id != id;
            if !keep {
                flow.cancelled.cancel();
            }
            keep
        });
        Ok(())
    }
}
struct RefreshOwner {
    sync: SyncEngine,
    session: Arc<NativeSession>,
    id: String,
    lease: String,
}
impl Drop for RefreshOwner {
    fn drop(&mut self) {
        let sync = self.sync.clone();
        let session = self.session.clone();
        let path = format!("/api/endpoint/oauth/{}/lease", self.id);
        let input = RefreshRelease {
            installation_id: session.metadata.installation_id.to_string(),
            lease_id: self.lease.clone(),
        };
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                if let Ok(body) = serde_json::to_value(input) {
                    let _ = sync
                        .cache
                        .sessions
                        .control_request_for(&session, Method::DELETE, &path, Some(&body))
                        .await;
                }
            });
        }
        // If the process exits, the server's exclusive lease expires without mutating the credential.
    }
}
