use crate::{
    EndpointError, Result,
    central::CentralClient,
    credentials::CredentialKind,
    epoch,
    store::{Identity, Metadata, StateStore},
};
use gateway_core::{EndpointSession, LoginRequest};
use reqwest::{Method, Response};
use serde::Serialize;
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactedStatus {
    pub state: &'static str,
    pub identity: Option<Identity>,
    pub session_expires_at: Option<i64>,
    pub configuration: &'static str,
}
pub struct NativeSession {
    pub metadata: Metadata,
    pub credentials: EndpointSession,
}
#[derive(Clone)]
pub struct SessionManager {
    pub store: StateStore,
    allow_fixture_http: bool,
}
impl SessionManager {
    pub fn new(store: StateStore, allow_fixture_http: bool) -> Self {
        Self {
            store,
            allow_fixture_http,
        }
    }
    pub fn load(&self) -> Result<NativeSession> {
        self.store.locked(|m| {
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
        })
    }
    pub fn status(&self) -> Result<RedactedStatus> {
        match self.load() {
            Ok(s) => Ok(RedactedStatus {
                state: "authenticated",
                identity: s.metadata.active,
                session_expires_at: Some(s.credentials.expires_at),
                configuration: "sync_required",
            }),
            Err(EndpointError::LoginRequired) => Ok(RedactedStatus {
                state: "login_required",
                identity: None,
                session_expires_at: None,
                configuration: "sync_required",
            }),
            Err(e) => Err(e),
        }
    }
    pub async fn login(&self, central: &CentralClient, input: LoginRequest) -> Result<()> {
        let previous = self.load().ok();
        // An identity transition invalidates old authorization before network work.
        let expected = self.store.locked(|m| {
            self.store.clear_locked(m)?;
            Ok(m.generation)
        })?;
        if let Some(previous) = previous
            && let Ok(previous_client) = self.client(&previous)
        {
            let _ = previous_client.revoke(&previous.credentials).await;
        }
        let session = central.login(input).await?;
        let identity = Identity {
            server_origin: central.origin(),
            deployment_id: session.deployment_id.clone(),
            user_id: session.user.id.clone(),
        };
        identity.validate()?;
        let installed = self.store.locked(|m| {
            if m.generation != expected {
                return Err(EndpointError::IdentityChanged);
            }
            let account = StateStore::account(&identity, expected, "session")?;
            // Journal owned entries before Keychain insertion for crash-safe cleanup.
            m.active = Some(identity.clone());
            m.credential_accounts.push(account.clone());
            self.store.write(m)?;
            let bytes =
                Zeroizing::new(serde_json::to_vec(&session).map_err(|_| EndpointError::Invalid)?);
            if let Err(e) = self.store.credentials().set(&account, &bytes) {
                self.store.clear_locked(m)?;
                return Err(e);
            }
            Ok(())
        });
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
    pub async fn logout(&self) -> Result<()> {
        let session = self.load().ok();
        self.store.locked(|m| self.store.clear_locked(m))?;
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
