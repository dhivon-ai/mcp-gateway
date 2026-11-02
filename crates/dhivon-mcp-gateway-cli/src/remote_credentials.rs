//! Scoped native resolution; credential traffic never renews the policy lease or enters browser APIs.
use crate::{
    EndpointError,
    aggregation::{AggregateError, AuthLease, CredentialResolver},
    cache::ValidatedSnapshot,
    central::bounded_body,
    sync::SyncEngine,
    upstream_credentials::{CredentialFault, NativeCredentials, validate_credential},
};
use dhivon_mcp_gateway_core::{AuthMode, CredentialRequest, EffectiveMcp, UpstreamCredential};
use futures::future::BoxFuture;
use std::sync::Arc;
use uuid::Uuid;

pub struct RemoteCredentials {
    pub native: NativeCredentials,
    engine: SyncEngine,
    oauth: crate::oauth_control::OAuthControl,
}
impl RemoteCredentials {
    async fn refresh_expired(
        &self,
        generation: Uuid,
        id: &str,
    ) -> Result<AuthLease, AggregateError> {
        self.oauth.refresh(generation, id).await.map_err(category)?;
        let latest = self.native.cache.latest().map_err(category)?;
        if latest.generation != generation {
            return Err(AggregateError::Denied);
        }
        let current = latest
            .snapshot
            .mcps
            .iter()
            .find(|mcp| mcp.definition.id == id)
            .ok_or(AggregateError::Denied)?;
        self.cached(&latest, current)
    }
    pub fn new(engine: SyncEngine) -> Self {
        let fixture_http = engine
            .cache
            .sessions
            .load()
            .ok()
            .and_then(|session| session.metadata.active)
            .is_some_and(|identity| identity.server_origin.starts_with("http://"));
        Self {
            native: NativeCredentials::new(engine.cache.clone()),
            oauth: crate::oauth_control::OAuthControl::new(
                engine.clone(),
                "http://127.0.0.1:19001/oauth/callback".into(),
                fixture_http,
            ),
            engine,
        }
    }
    fn cached(
        &self,
        snapshot: &ValidatedSnapshot,
        mcp: &EffectiveMcp,
    ) -> Result<AuthLease, AggregateError> {
        let scope = self.native.cached_scope(snapshot, mcp).map_err(category)?;
        let record = self.native.load(&scope).map_err(category)?;
        let access = record
            .access(self.native.cache.sample().map_err(category)?)
            .map_err(category)?;
        if !self.native.cache.proof_usable(snapshot).map_err(category)? {
            return Err(AggregateError::Denied);
        }
        let expires_at = match record.credential.as_ref() {
            Some(UpstreamCredential::OAuth { credentials, .. }) => Some(credentials.expires_at),
            _ => None,
        };
        Ok(AuthLease {
            access: Some(access),
            binding: record.signature(),
            expires_at,
        })
    }
}
impl CredentialResolver for RemoteCredentials {
    fn resolve<'a>(
        &'a self,
        snapshot: &'a ValidatedSnapshot,
        mcp: &'a EffectiveMcp,
    ) -> BoxFuture<'a, Result<AuthLease, AggregateError>> {
        Box::pin(async move {
            if matches!(mcp.definition.auth, AuthMode::None {}) {
                return Ok(AuthLease {
                    access: None,
                    binding: "none".into(),
                    expires_at: None,
                });
            }
            if matches!(mcp.definition.auth, AuthMode::OAuth { .. }) {
                let session = self.native.cache.sessions.load().map_err(category)?;
                if session.metadata.generation != snapshot.generation {
                    return Err(AggregateError::Denied);
                }
                if !crate::oauth_pending::pending(&self.native.cache.sessions, &session, mcp)
                    .map_err(category)?
                    .is_empty()
                {
                    self.oauth
                        .recover(snapshot.generation, &mcp.definition.id)
                        .await
                        .map_err(category)?;
                }
            }
            let scope = self.native.scope(snapshot, mcp).map_err(category)?;
            let start = self.native.cache.sample().map_err(category)?;
            let input = CredentialRequest {
                expected_revision: scope.revision,
                installation_id: scope.session.metadata.installation_id.to_string(),
            };
            let body = serde_json::to_value(input).map_err(|_| AggregateError::Protocol)?;
            let path = format!("/api/endpoint/credentials/{}", scope.mcp.definition.id);
            let outcome = scope.session.metadata.generation;
            let response = self
                .native
                .cache
                .sessions
                .control_request_for(&scope.session, reqwest::Method::POST, &path, Some(&body))
                .await;
            let mut online_resolution = false;
            match response {
                Ok(response) if response.status().is_success() => {
                    online_resolution = true;
                    let prepared = async {
                        let bytes = bounded_body(response).await.map_err(category)?;
                        let credential: UpstreamCredential =
                            serde_json::from_slice(&bytes).map_err(|_| AggregateError::Protocol)?;
                        validate_credential(&scope, &credential).map_err(category)?;
                        // A provider can reject an expired access token before reconciliation.
                        // Its rejection marker must still deny that exact token, but must not
                        // prevent a version/lease-checked refresh of the expired central record.
                        if matches!(&credential, UpstreamCredential::OAuth { credentials, .. }
                            if credentials.expires_at <= self.native.cache.sample().map_err(category)?.wall_seconds().map_err(category)?)
                        {
                            return self.refresh_expired(outcome, &mcp.definition.id).await.map(Some);
                        }
                        self.native
                            .publish(&scope, credential, start)
                            .map_err(category)?;
                        Ok(None)
                    }
                    .await;
                    match prepared {
                        Ok(Some(lease)) => return Ok(lease),
                        Ok(None) => (),
                        Err(error) => {
                            let _ = self.native.reject(&scope, None, CredentialFault::Required);
                            return Err(error);
                        }
                    }
                }
                Ok(response) => {
                    let fault = if response.status() == reqwest::StatusCode::CONFLICT {
                        self.engine.notify_reconnected();
                        CredentialFault::SyncRequired
                    } else {
                        CredentialFault::Required
                    };
                    // Known scoped refusal cannot fall back to a previously usable token.
                    self.native.reject(&scope, None, fault).map_err(category)?;
                    return Err(AggregateError::Authentication);
                }
                Err(EndpointError::Offline | EndpointError::Response) => {
                    // Network/server failure may use only a current, unexpired protected credential.
                }
                Err(error) => return Err(category(error)),
            }
            let latest = self.native.cache.latest().map_err(category)?;
            if latest.generation != outcome {
                return Err(AggregateError::Denied);
            }
            let current = latest
                .snapshot
                .mcps
                .iter()
                .find(|current| {
                    current.definition.id == mcp.definition.id
                        && current.enabled
                        && current.definition.active
                })
                .ok_or(AggregateError::Denied)?;
            match self.cached(&latest, current) {
                Err(AggregateError::TokenExpired)
                    if online_resolution
                        && matches!(current.definition.auth, AuthMode::OAuth { .. }) =>
                {
                    self.refresh_expired(outcome, &current.definition.id).await
                }
                result => result,
            }
        })
    }
    fn cached_ready(
        &self,
        snapshot: &ValidatedSnapshot,
        mcp: &EffectiveMcp,
        binding: &str,
    ) -> bool {
        if matches!(mcp.definition.auth, AuthMode::None {}) {
            return binding == "none";
        }
        self.cached(snapshot, mcp)
            .is_ok_and(|lease| lease.binding == binding)
    }
    fn rejected(&self, generation: Uuid, id: &str, signature: &str) {
        let Ok(latest) = self.native.cache.latest() else {
            return;
        };
        if latest.generation != generation {
            return;
        }
        let Some(mcp) = latest
            .snapshot
            .mcps
            .iter()
            .find(|mcp| mcp.definition.id == id)
        else {
            return;
        };
        let Ok(scope) = self.native.cached_scope(&latest, mcp) else {
            return;
        };
        let _ = self
            .native
            .reject(&scope, Some(signature), CredentialFault::Rejected);
        tracing::info!(
            event = "upstream_authentication_rejected",
            mcp_id = id,
            category = "upstream_auth_rejected"
        );
    }
}
fn category(error: EndpointError) -> AggregateError {
    match error {
        EndpointError::UpstreamRejected => AggregateError::AuthenticationRejected,
        EndpointError::UpstreamExpired => AggregateError::TokenExpired,
        EndpointError::LoginRequired
        | EndpointError::Rejected
        | EndpointError::IdentityChanged
        | EndpointError::CacheExpired
        | EndpointError::InvalidCache
        | EndpointError::ClockUncertain => AggregateError::Denied,
        _ => AggregateError::Authentication,
    }
}
pub fn resolver(engine: SyncEngine) -> Arc<dyn CredentialResolver> {
    Arc::new(RemoteCredentials::new(engine))
}
