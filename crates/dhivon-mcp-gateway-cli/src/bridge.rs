//! Online-only, generation-pinned settings and write-only vault bridge.
use crate::{EndpointError, central::bounded_body, session::NativeSession, sync::SyncEngine};
use dhivon_mcp_gateway_core::{
    ApiError, AuthMode, ConfigSnapshot, ErrorCode, LocalApiError, LocalCatalog, LocalMcpMetadata,
    SecretMetadata, SecretMutationOutcome, SettingsUpdate, Transport,
};
use reqwest::{Method, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use uuid::Uuid;

pub type BridgeResult<T> = std::result::Result<T, BridgeError>;
#[derive(Debug)]
pub struct BridgeError {
    pub status: StatusCode,
    pub body: LocalApiError,
}
impl From<EndpointError> for BridgeError {
    fn from(error: EndpointError) -> Self {
        let (status, code) = match error {
            EndpointError::LoginRequired | EndpointError::Rejected => {
                (StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized)
            }
            EndpointError::IdentityChanged | EndpointError::ServerMismatch => {
                (StatusCode::CONFLICT, ErrorCode::Conflict)
            }
            EndpointError::Invalid => (StatusCode::BAD_REQUEST, ErrorCode::InvalidRequest),
            EndpointError::CacheExpired => {
                (StatusCode::SERVICE_UNAVAILABLE, ErrorCode::ConfigExpired)
            }
            EndpointError::InvalidCache | EndpointError::ClockUncertain => {
                (StatusCode::SERVICE_UNAVAILABLE, ErrorCode::InvalidCache)
            }
            EndpointError::UpstreamRequired
            | EndpointError::UpstreamRejected
            | EndpointError::UpstreamExpired => {
                (StatusCode::PRECONDITION_REQUIRED, ErrorCode::AuthRequired)
            }
            EndpointError::CredentialCapacity => (
                StatusCode::SERVICE_UNAVAILABLE,
                ErrorCode::UpstreamUnavailable,
            ),
            EndpointError::OAuthUnsupported => {
                (StatusCode::UNPROCESSABLE_ENTITY, ErrorCode::Unsupported)
            }
            _ => (StatusCode::SERVICE_UNAVAILABLE, ErrorCode::Offline),
        };
        Self {
            status,
            body: LocalApiError {
                code,
                message: error.to_string(),
                revision: None,
                current: None,
            },
        }
    }
}
impl BridgeError {
    pub fn conflict(current: LocalCatalog) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            body: LocalApiError {
                code: ErrorCode::Conflict,
                message: "Configuration changed; review and retry explicitly".into(),
                revision: Some(current.revision),
                current: Some(current),
            },
        }
    }
    pub fn invalid() -> Self {
        EndpointError::Invalid.into()
    }
}
pub struct Prepared {
    pub session: NativeSession,
    pub snapshot: ConfigSnapshot,
}
#[derive(Clone)]
pub struct ControlBridge {
    pub sync: SyncEngine,
}
impl ControlBridge {
    pub fn new(sync: SyncEngine) -> Self {
        Self { sync }
    }
    pub fn generation(value: &str) -> BridgeResult<Uuid> {
        Uuid::parse_str(value).map_err(|_| BridgeError::invalid())
    }
    pub fn check(&self, generation: &str) -> BridgeResult<()> {
        self.sync
            .cache
            .sessions
            .guard_generation(Self::generation(generation)?)
            .map_err(Into::into)
    }
    pub fn catalog(&self, generation: &str) -> BridgeResult<LocalCatalog> {
        self.check(generation)?;
        let validated = self.sync.cache.latest()?;
        if validated.generation != Self::generation(generation)? {
            return Err(EndpointError::IdentityChanged.into());
        }
        self.check(generation)?;
        Ok(metadata(&validated.snapshot, generation))
    }
    /// A valid offline lease is intentionally insufficient for every mutation and vault read.
    pub async fn prepare(&self, generation: &str, revision: Option<u32>) -> BridgeResult<Prepared> {
        self.check(generation)?;
        self.sync.refresh().await?;
        self.check(generation)?;
        let session = self.sync.cache.sessions.load()?;
        let validated = self.sync.cache.latest()?;
        require_prepared_generation(
            Self::generation(generation)?,
            session.metadata.generation,
            validated.generation,
        )?;
        if revision.is_some_and(|r| r != validated.snapshot.revision) {
            return Err(BridgeError::conflict(metadata(
                &validated.snapshot,
                generation,
            )));
        }
        Ok(Prepared {
            session,
            snapshot: validated.snapshot,
        })
    }
    pub async fn settings(
        &self,
        generation: &str,
        id: &str,
        input: SettingsUpdate,
    ) -> BridgeResult<LocalCatalog> {
        self.settings_guarded(generation, id, input, || true).await
    }
    pub(crate) async fn settings_guarded(
        &self,
        generation: &str,
        id: &str,
        input: SettingsUpdate,
        allowed: impl Fn() -> bool,
    ) -> BridgeResult<LocalCatalog> {
        let correlation = Uuid::new_v4();
        let began = std::time::Instant::now();
        let result = self.settings_inner(generation, id, input, allowed).await;
        record("settings", correlation, began, &result);
        result
    }
    async fn settings_inner(
        &self,
        generation: &str,
        id: &str,
        input: SettingsUpdate,
        allowed: impl Fn() -> bool,
    ) -> BridgeResult<LocalCatalog> {
        if !allowed() {
            return Err(EndpointError::Cancelled.into());
        }
        segment(id)?;
        let prepared = self
            .prepare(generation, Some(input.expected_revision))
            .await?;
        if !prepared
            .snapshot
            .mcps
            .iter()
            .any(|m| m.definition.id == id && m.definition.active)
        {
            return Err(BridgeError::invalid());
        }
        let start = self.sync.cache.sample()?;
        if !allowed() {
            return Err(EndpointError::Cancelled.into());
        }
        let response = self
            .sync
            .cache
            .sessions
            .control_request_for(
                &prepared.session,
                Method::PUT,
                &format!("/api/settings/{id}"),
                Some(&serde_json::to_value(&input).map_err(|_| BridgeError::invalid())?),
            )
            .await?;
        if !response.status().is_success() {
            return Err(self
                .central_error(&prepared.session, response, start)
                .await?);
        }
        let snapshot = decode::<ConfigSnapshot>(response).await?;
        if snapshot.revision <= input.expected_revision
            || !snapshot.mcps.iter().any(|m| {
                m.definition.id == id
                    && m.enabled == input.enabled
                    && m.secret.as_ref().map(|s| &s.id) == input.secret_id.as_ref()
            })
        {
            return Err(EndpointError::Response.into());
        }
        // A confirmed server write is not acknowledged until the complete shared snapshot is durable.
        self.sync
            .cache
            .install(&prepared.session, snapshot, start)?;
        self.catalog(generation)
    }
    pub async fn secrets(&self, generation: &str) -> BridgeResult<Vec<SecretMetadata>> {
        let prepared = self.prepare(generation, None).await?;
        let start = self.sync.cache.sample()?;
        let response = self
            .sync
            .cache
            .sessions
            .control_request_for(&prepared.session, Method::GET, "/api/secrets", None)
            .await?;
        if !response.status().is_success() {
            return Err(self
                .central_error(&prepared.session, response, start)
                .await?);
        }
        let metadata = decode(response).await?;
        self.check(generation)?;
        Ok(metadata)
    }
    pub async fn secret<T: Serialize>(
        &self,
        generation: &str,
        method: Method,
        id: Option<&str>,
        rename: bool,
        input: T,
    ) -> BridgeResult<SecretMutationOutcome> {
        let correlation = Uuid::new_v4();
        let began = std::time::Instant::now();
        let result = self
            .secret_inner(generation, method, id, rename, input)
            .await;
        record("vault_mutation", correlation, began, &result);
        result
    }
    async fn secret_inner<T: Serialize>(
        &self,
        generation: &str,
        method: Method,
        id: Option<&str>,
        rename: bool,
        input: T,
    ) -> BridgeResult<SecretMutationOutcome> {
        if !matches!(method, Method::POST | Method::PUT | Method::DELETE)
            || (rename && method != Method::PUT)
        {
            return Err(BridgeError::invalid());
        }
        let path = match id {
            Some(id) => {
                segment(id)?;
                format!("/api/secrets/{id}{}", if rename { "/key" } else { "" })
            }
            None if method == Method::POST && !rename => "/api/secrets".into(),
            _ => return Err(BridgeError::invalid()),
        };
        let prepared = self.prepare(generation, None).await?;
        let start = self.sync.cache.sample()?;
        let body = serde_json::to_value(input).map_err(|_| BridgeError::invalid())?;
        let response = self
            .sync
            .cache
            .sessions
            .control_request_for(&prepared.session, method, &path, Some(&body))
            .await?;
        if !response.status().is_success() {
            return Err(self
                .central_error(&prepared.session, response, start)
                .await?);
        }
        let metadata = if response.status() == StatusCode::NO_CONTENT {
            None
        } else {
            Some(decode::<SecretMetadata>(response).await?)
        };
        self.check(generation)?;
        let synchronized = self.sync.refresh().await.is_ok();
        self.check(generation)?;
        Ok(SecretMutationOutcome {
            confirmed: true,
            synchronized,
            metadata,
        })
    }
    async fn central_error(
        &self,
        session: &NativeSession,
        response: reqwest::Response,
        start: crate::clock::ClockSample,
    ) -> BridgeResult<BridgeError> {
        let status = response.status();
        let error = decode::<ApiError>(response).await?;
        self.sync
            .cache
            .sessions
            .guard_generation(session.metadata.generation)?;
        let current = if let Some(snapshot) = error.snapshot {
            self.sync.cache.install(session, snapshot, start)?;
            Some(self.catalog(&session.metadata.generation.to_string())?)
        } else {
            None
        };
        Ok(BridgeError {
            status,
            body: LocalApiError {
                message: fixed_message(&error.code).into(),
                code: error.code,
                revision: error.revision,
                current,
            },
        })
    }
}
pub fn segment(id: &str) -> BridgeResult<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
    {
        return Err(BridgeError::invalid());
    }
    Ok(())
}
async fn decode<T: DeserializeOwned>(response: reqwest::Response) -> BridgeResult<T> {
    let body = bounded_body(response).await?;
    serde_json::from_slice(&body).map_err(|_| EndpointError::Response.into())
}
fn fixed_message(code: &ErrorCode) -> &'static str {
    match code {
        ErrorCode::Conflict => {
            "Current server state prevents this change; review configuration, active bindings and versions before retrying"
        }
        ErrorCode::Forbidden => "The control plane refused this operation",
        ErrorCode::InvalidRequest => "The control plane rejected the input",
        _ => "The control plane could not confirm this operation",
    }
}
pub fn metadata(snapshot: &ConfigSnapshot, generation: &str) -> LocalCatalog {
    LocalCatalog {
        generation: generation.into(),
        revision: snapshot.revision,
        expires_at: snapshot.expires_at,
        mcps: snapshot
            .mcps
            .iter()
            .map(|m| {
                let (auth, required) = match m.definition.auth {
                    AuthMode::None {} => ("none", false),
                    AuthMode::Bearer {} => ("bearer", m.secret.is_none()),
                    AuthMode::OAuth { .. } => ("oauth", true),
                };
                LocalMcpMetadata {
                    id: m.definition.id.clone(),
                    name: m.definition.name.clone(),
                    description: m.definition.description.clone(),
                    transport: match m.definition.transport {
                        Transport::Stdio { .. } => "stdio",
                        Transport::Http { .. } => "http",
                    }
                    .into(),
                    auth: auth.into(),
                    enabled: m.enabled,
                    secret: m.secret.clone(),
                    installation_available: m.definition.install.is_some(),
                    installed: false,
                    availability: if !m.enabled {
                        "disabled"
                    } else if required {
                        "auth_required"
                    } else {
                        "configured"
                    }
                    .into(),
                    auth_required: required,
                    tool_policy: m.definition.tool_policy.clone(),
                }
            })
            .collect(),
    }
}

fn require_prepared_generation(expected: Uuid, session: Uuid, snapshot: Uuid) -> BridgeResult<()> {
    if expected != session || expected != snapshot {
        return Err(EndpointError::IdentityChanged.into());
    }
    Ok(())
}
fn record<T>(
    operation: &'static str,
    correlation: Uuid,
    began: std::time::Instant,
    result: &BridgeResult<T>,
) {
    tracing::info!(event = "endpoint_management_outcome", operation, correlation_id = %correlation, duration_ms = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX), status = result.as_ref().err().map_or(200, |e| e.status.as_u16()), confirmed = result.is_ok());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_old_generation_then_loaded_new_session_and_snapshot_is_rejected() {
        let displayed = Uuid::new_v4();
        let concurrently_logged_in = Uuid::new_v4();
        assert!(
            require_prepared_generation(displayed, concurrently_logged_in, concurrently_logged_in)
                .is_err()
        );
        assert!(require_prepared_generation(displayed, displayed, concurrently_logged_in).is_err());
        assert!(require_prepared_generation(displayed, displayed, displayed).is_ok());
    }
}
