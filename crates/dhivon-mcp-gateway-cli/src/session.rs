use crate::{
    EndpointError, Result,
    central::CentralClient,
    credentials::CredentialKind,
    epoch,
    store::{Identity, Metadata, StateStore},
};
use dhivon_mcp_gateway_core::{EndpointSession, LoginRequest};
use reqwest::{Method, Response};
use serde::Serialize;
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactedStatus {
    pub state: &'static str,
    pub identity: Option<Identity>,
    pub generation: Option<String>,
    pub session_expires_at: Option<i64>,
    pub configuration: &'static str,
    pub email: Option<String>,
    pub name: Option<String>,
}
pub struct NativeSession {
    pub metadata: Metadata,
    pub credentials: EndpointSession,
}
#[derive(Clone)]
pub struct SessionManager {
    pub store: StateStore,
    allow_fixture_http: bool,
    selected_origin: Option<String>,
}
impl SessionManager {
    pub fn new(store: StateStore, allow_fixture_http: bool) -> Self {
        Self {
            store,
            allow_fixture_http,
            selected_origin: None,
        }
    }
    /// Bind this process to one configured origin without mutating shared authorization.
    pub fn for_server(&self, central: &CentralClient) -> Self {
        let mut selected = self.clone();
        selected.selected_origin = Some(central.origin());
        selected
    }
    pub fn is_selected_for(&self, central: &CentralClient) -> bool {
        self.selected_origin.as_deref() == Some(central.origin().as_str())
    }
    pub fn matches_selection(&self, metadata: &Metadata) -> bool {
        self.selected_origin.as_ref().is_none_or(|origin| {
            metadata
                .selected_server_origin
                .as_ref()
                .is_none_or(|selected| selected == origin)
                && metadata
                    .active
                    .as_ref()
                    .is_none_or(|identity| identity.server_origin == *origin)
        })
    }
    /// Explicit CLI startup selection. Different origins invalidate active and pending login generations.
    pub async fn select_server(&self, central: &CentralClient) -> Result<Self> {
        let origin = central.origin();
        if self.store.recovery_witness()?.is_some() {
            return Ok(self.for_server(central));
        }
        let retired = self.store.locked(|m| {
            let changed = m
                .selected_server_origin
                .as_ref()
                .is_some_and(|selected| selected != &origin)
                || m.active
                    .as_ref()
                    .is_some_and(|identity| identity.server_origin != origin);
            let previous = if changed {
                self.load_locked(m, false).ok()
            } else {
                None
            };
            if changed {
                self.store.clear_locked(m)?;
            }
            if m.selected_server_origin.as_ref() != Some(&origin) {
                m.selected_server_origin = Some(origin.clone());
                self.store.write(m)?;
            }
            Ok(previous)
        })?;
        if let Some(previous) = retired {
            let previous_origin = &previous
                .metadata
                .active
                .as_ref()
                .ok_or(EndpointError::LoginRequired)?
                .server_origin;
            if let Ok(previous_client) =
                CentralClient::new(previous_origin, self.allow_fixture_http)
            {
                let _ = previous_client.revoke(&previous.credentials).await;
            }
        }
        Ok(self.for_server(central))
    }
    pub fn load(&self) -> Result<NativeSession> {
        self.store.locked(|m| self.load_locked(m, true))
    }
    fn load_locked(&self, m: &mut Metadata, enforce_selection: bool) -> Result<NativeSession> {
        if enforce_selection && !self.matches_selection(m) {
            return Err(EndpointError::ServerMismatch);
        }
        let identity = m.active.as_ref().ok_or(EndpointError::LoginRequired)?;
        let account = StateStore::account(identity, m.generation, "session")?;
        if !m.credential_accounts.contains(&account) {
            return Err(EndpointError::LoginRequired);
        }
        let bytes = self
            .store
            .credentials()
            .get(&account)?
            .ok_or(EndpointError::LoginRequired)?;
        let credentials: EndpointSession =
            serde_json::from_slice(&bytes).map_err(|_| EndpointError::Invalid)?;
        if credentials.user.id != identity.user_id
            || credentials.deployment_id != identity.deployment_id
            || !credentials.user.active
            || credentials.expires_at <= epoch()?
        {
            self.store.clear_locked(m)?;
            return Err(EndpointError::LoginRequired);
        }
        Ok(NativeSession {
            metadata: m.clone(),
            credentials,
        })
    }
    pub fn status(&self) -> Result<RedactedStatus> {
        match self.load() {
            Ok(s) => Ok(RedactedStatus {
                state: "authenticated",
                generation: Some(s.metadata.generation.to_string()),
                identity: s.metadata.active,
                session_expires_at: Some(s.credentials.expires_at),
                configuration: "sync_required",
                email: Some(s.credentials.user.email),
                name: Some(s.credentials.user.name),
            }),
            Err(EndpointError::ServerMismatch) => Ok(RedactedStatus {
                state: "server_mismatch",
                identity: None,
                generation: None,
                session_expires_at: None,
                configuration: "server_mismatch",
                email: None,
                name: None,
            }),
            Err(EndpointError::LoginRequired | EndpointError::InvalidCache) => Ok(RedactedStatus {
                state: "login_required",
                identity: None,
                generation: None,
                session_expires_at: None,
                configuration: "sync_required",
                email: None,
                name: None,
            }),
            Err(e) => Err(e),
        }
    }
    pub async fn login(&self, central: &CentralClient, input: LoginRequest) -> Result<()> {
        if self
            .selected_origin
            .as_ref()
            .is_some_and(|origin| *origin != central.origin())
        {
            return Err(EndpointError::ServerMismatch);
        }
        let recovery = self.store.recovery_witness()?;
        let (expected, delayed_clear) = if recovery.is_none() {
            let (expected, damaged, previous) = self.store.locked(|m| {
                let previous = match self.load_locked(m, false) {
                    Ok(s) => Some(s),
                    Err(EndpointError::InvalidCache | EndpointError::LoginRequired)
                        if self.store.is_protected() && m.active.is_some() =>
                    {
                        return Ok((m.generation, true, None));
                    }
                    Err(EndpointError::Keychain | EndpointError::Storage)
                        if self.store.is_protected() =>
                    {
                        return Err(EndpointError::Keychain);
                    }
                    Err(_) => None,
                };
                self.store.clear_locked(m)?;
                m.selected_server_origin = Some(central.origin());
                self.store.write(m)?;
                Ok((m.generation, false, previous))
            })?;
            if let Some(previous) = previous
                && let Ok(client) = self.client(&previous)
            {
                let _ = client.revoke(&previous.credentials).await;
            }
            (Some(expected), damaged)
        } else {
            (None, false)
        };
        let session = central.login(input).await?;
        let identity = Identity {
            server_origin: central.origin(),
            deployment_id: session.deployment_id.clone(),
            user_id: session.user.id.clone(),
        };
        identity.validate()?;
        let installed = (|| {
            let expected = match (expected, recovery) {
                (Some(g), None) => g,
                (None, Some(witness)) => self
                    .store
                    .recover_authenticated(witness, &central.origin())?,
                _ => return Err(EndpointError::IdentityChanged),
            };
            self.store.locked(|m| {
                if m.generation != expected {
                    return Err(EndpointError::IdentityChanged);
                }
                if delayed_clear {
                    self.store.clear_locked(m)?;
                    m.selected_server_origin = Some(central.origin());
                }
                let account = StateStore::account(&identity, m.generation, "session")?;
                // Journal owned entries before Keychain insertion for crash-safe cleanup.
                m.active = Some(identity.clone());
                m.credential_accounts.push(account.clone());
                self.store.write(m)?;
                let bytes = Zeroizing::new(
                    serde_json::to_vec(&session).map_err(|_| EndpointError::Invalid)?,
                );
                if let Err(e) = self.store.credentials().set(&account, &bytes) {
                    self.store.clear_locked(m)?;
                    return Err(e);
                }
                Ok(())
            })
        })();
        if installed.is_err() {
            let _ = central.revoke(&session).await;
        }
        installed?;
        tracing::info!(event="endpoint_login", user_id=%identity.user_id, deployment_id=%identity.deployment_id);
        Ok(())
    }
    pub fn invalidate(&self, generation: Uuid) -> Result<()> {
        self.store.locked(|m| {
            if m.generation == generation {
                self.store.clear_locked(m)?;
            }
            Ok(())
        })
    }
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<(NativeSession, Response)> {
        let session = self.load()?;
        let central = self.client(&session)?;
        match central
            .request(&session.credentials, method, path, body)
            .await
        {
            Err(EndpointError::Rejected) => {
                self.invalidate(session.metadata.generation)?;
                tracing::info!(event = "endpoint_session_rejected");
                Err(EndpointError::Rejected)
            }
            Ok(response) => Ok((session, response)),
            Err(e) => Err(e),
        }
    }
    pub fn guard_generation(&self, generation: Uuid) -> Result<()> {
        self.store.locked(|m| {
            if !self.matches_selection(m) {
                return Err(EndpointError::ServerMismatch);
            }
            if m.generation != generation || m.active.is_none() {
                return Err(EndpointError::IdentityChanged);
            }
            Ok(())
        })
    }
    /// Pin the original principal for a local action; never reload another user's token after a login race.
    pub async fn control_request_for(
        &self,
        session: &NativeSession,
        method: Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<Response> {
        self.guard_generation(session.metadata.generation)?;
        let result = self
            .client(session)?
            .control_request(&session.credentials, method, path, body)
            .await;
        if matches!(result, Err(EndpointError::Rejected)) {
            self.invalidate(session.metadata.generation)?;
        }
        result
    }
    pub fn client(&self, session: &NativeSession) -> Result<CentralClient> {
        CentralClient::new(
            &session
                .metadata
                .active
                .as_ref()
                .ok_or(EndpointError::LoginRequired)?
                .server_origin,
            self.allow_fixture_http,
        )
    }
    /// Browser logout compares and invalidates under one lock; revocation uses only that old principal.
    pub async fn logout_generation(&self, generation: Uuid) -> Result<()> {
        let session = self.store.locked(|m| {
            if !self.matches_selection(m) {
                return Err(EndpointError::ServerMismatch);
            }
            if m.generation != generation {
                return Err(EndpointError::IdentityChanged);
            }
            let session = self.load_locked(m, false).ok();
            self.store.clear_locked(m)?;
            Ok(session)
        })?;
        if let Some(session) = session
            && let Ok(client) = self.client(&session)
        {
            let _ = client.revoke(&session.credentials).await;
        }
        tracing::info!(event = "endpoint_logout");
        Ok(())
    }
    pub async fn logout(&self) -> Result<()> {
        let session = self.load().ok();
        self.store.locked(|m| {
            if !self.matches_selection(m) {
                return Err(EndpointError::ServerMismatch);
            }
            self.store.clear_locked(m)
        })?;
        if let Some(session) = session
            && let Ok(client) = self.client(&session)
        {
            let _ = client.revoke(&session.credentials).await;
        }
        tracing::info!(event = "endpoint_logout");
        Ok(())
    }
    /// Guard every credential/cache installation after network work against logout and identity change.
    pub fn save_credential(
        &self,
        generation: Uuid,
        kind: CredentialKind,
        bytes: &[u8],
    ) -> Result<String> {
        self.store.locked(|m| {
            if !self.matches_selection(m) {
                return Err(EndpointError::ServerMismatch);
            }
            if m.generation != generation {
                return Err(EndpointError::IdentityChanged);
            }
            let identity = m.active.as_ref().ok_or(EndpointError::LoginRequired)?;
            let account = StateStore::account(identity, generation, &kind.label()?)?;
            if !m.credential_accounts.contains(&account) {
                m.credential_accounts.push(account.clone());
                self.store.write(m)?;
            }
            self.store.credentials().set(&account, bytes)?;
            Ok(account)
        })
    }
}
