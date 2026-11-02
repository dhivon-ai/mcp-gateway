//! Explicit SDK public-client exchanges; callers own online policy, persistence and refresh leases.
use crate::{EndpointError, Result, clock::ClockSample, oauth_http::OAuthNetwork};
use dhivon_mcp_gateway_core::OAuthCredentials;
use oauth2::{AccessToken, RefreshToken, Scope, TokenResponse, basic::BasicTokenType};
use rmcp::transport::auth::{
    AuthorizationManager, AuthorizationMetadataSource, CredentialStore, InMemoryCredentialStore,
    OAuthClientConfig, OAuthTokenResponse, StoredCredentials,
};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use url::Url;

pub struct OAuthProtocol {
    manager: AuthorizationManager,
    memory: InMemoryCredentialStore,
    resource: String,
    client_id: String,
    scopes: Vec<String>,
    issuer: String,
    issuer_required: bool,
    callback: Url,
}
pub struct Consent {
    pub protocol: OAuthProtocol,
    pub authorization_url: String,
    state: String,
    began: ClockSample,
}
impl OAuthProtocol {
    pub async fn discover(
        resource: &str,
        client_id: &str,
        scopes: &[String],
        callback: &str,
        fixture_http: bool,
    ) -> Result<Self> {
        let callback = Url::parse(callback).map_err(|_| EndpointError::Invalid)?;
        if callback.scheme() != "http"
            || callback.host_str() != Some("127.0.0.1")
            || callback.port().is_none()
            || callback.path() != "/oauth/callback"
            || callback.query().is_some()
            || callback.fragment().is_some()
            || !callback.username().is_empty()
            || callback.password().is_some()
            || client_id.is_empty()
            || client_id.len() > 4096
            || scopes.len() > 64
        {
            return Err(EndpointError::Invalid);
        }
        let network = Arc::new(OAuthNetwork::new(resource, fixture_http)?);
        let mut manager =
            AuthorizationManager::new_with_oauth_http_client(resource, network.clone())
                .await
                .map_err(|_| EndpointError::OAuthUnsupported)?;
        let resolution = tokio::time::timeout(Duration::from_secs(10), async {
            let challenge = network.challenge().await?;
            manager
                .resolve_metadata_from_challenge(challenge.as_deref())
                .await
                .map_err(|_| EndpointError::OAuthUnsupported)
        })
        .await
        .map_err(|_| EndpointError::Offline)??;
        if resolution.source != AuthorizationMetadataSource::ProtectedResourceMetadata {
            return Err(EndpointError::OAuthUnsupported);
        }
        network.pin(&resolution.metadata)?;
        let issuer = resolution
            .metadata
            .issuer
            .clone()
            .ok_or(EndpointError::OAuthUnsupported)?;
        let issuer_required = resolution
            .metadata
            .additional_fields
            .get("authorization_response_iss_parameter_supported")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        manager.set_metadata(resolution.metadata);
        manager
            .configure_client(
                OAuthClientConfig::new(client_id, callback.as_str()).with_scopes(scopes.to_vec()),
            )
            .map_err(|_| EndpointError::OAuthUnsupported)?;
        let memory = InMemoryCredentialStore::new();
        manager.set_credential_store(memory.clone());
        Ok(Self {
            manager,
            memory,
            resource: resource.into(),
            client_id: client_id.into(),
            scopes: scopes.to_vec(),
            issuer,
            issuer_required,
            callback,
        })
    }
    pub async fn consent(self, began: ClockSample) -> Result<Consent> {
        let refs: Vec<_> = self.scopes.iter().map(String::as_str).collect();
        let authorization_url = self
            .manager
            .get_authorization_url(&refs)
            .await
            .map_err(|_| EndpointError::OAuthUnsupported)?;
        let url = Url::parse(&authorization_url).map_err(|_| EndpointError::Invalid)?;
        let parameters: BTreeMap<_, _> = url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        if parameters.get("resource").map(String::as_str) != Some(self.resource.as_str())
            || parameters.get("client_id").map(String::as_str) != Some(self.client_id.as_str())
            || parameters.get("redirect_uri").map(String::as_str) != Some(self.callback.as_str())
            || parameters.get("code_challenge_method").map(String::as_str) != Some("S256")
        {
            return Err(EndpointError::Invalid);
        }
        let state = parameters
            .get("state")
            .cloned()
            .ok_or(EndpointError::Invalid)?;
        Ok(Consent {
            protocol: self,
            authorization_url,
            state,
            began,
        })
    }
    /// The central lease must span this call and the full credential CAS save.
    pub async fn refresh(
        &self,
        previous: &OAuthCredentials,
        began: ClockSample,
    ) -> Result<OAuthCredentials> {
        if previous.issuer != self.issuer
            || previous.resource != self.resource
            || previous.client_id != self.client_id
            || previous
                .scopes
                .iter()
                .any(|scope| !self.scopes.contains(scope))
        {
            return Err(EndpointError::UpstreamRequired);
        }
        let mut token = OAuthTokenResponse::new(
            AccessToken::new(previous.access_token.clone()),
            BasicTokenType::Bearer,
            rmcp::transport::auth::VendorExtraTokenFields::default(),
        );
        token.set_refresh_token(previous.refresh_token.clone().map(RefreshToken::new));
        token.set_scopes(Some(
            previous.scopes.iter().cloned().map(Scope::new).collect(),
        ));
        // No implicit SDK get_access_token: only this explicitly coordinated exchange may refresh.
        self.memory
            .save(
                StoredCredentials::new(
                    self.client_id.clone(),
                    Some(token),
                    previous.scopes.clone(),
                    None,
                )
                .with_issuer(Some(self.issuer.clone())),
            )
            .await
            .map_err(|_| EndpointError::Storage)?;
        let result = self
            .manager
            .refresh_token()
            .await
            .map_err(|error| match error {
                rmcp::transport::auth::AuthError::TokenRefreshRejected(_)
                | rmcp::transport::auth::AuthError::AuthorizationRequired => {
                    EndpointError::UpstreamRejected
                }
                _ => EndpointError::Offline,
            })?;
        self.complete(result, began).await
    }
    async fn complete(
        &self,
        token: OAuthTokenResponse,
        began: ClockSample,
    ) -> Result<OAuthCredentials> {
        if token.token_type() != &BasicTokenType::Bearer {
            return Err(EndpointError::OAuthUnsupported);
        }
        let lifetime = token.expires_in().ok_or(EndpointError::OAuthUnsupported)?;
        if lifetime.is_zero() || lifetime.as_secs() > 31_536_000 {
            return Err(EndpointError::OAuthUnsupported);
        }
        let access_token = token.access_token().secret().clone();
        let refresh_token = token.refresh_token().map(|token| token.secret().clone());
        if access_token.is_empty()
            || access_token.len() > 16384
            || !access_token
                .bytes()
                .all(|byte| byte.is_ascii() && !byte.is_ascii_whitespace() && byte != 0)
            || refresh_token.as_ref().is_some_and(|token| {
                token.is_empty() || token.len() > 16384 || token.contains(['\0', '\r', '\n'])
            })
        {
            return Err(EndpointError::Invalid);
        }
        let scopes = self.manager.get_current_scopes().await;
        if scopes.iter().any(|scope| !self.scopes.contains(scope)) {
            return Err(EndpointError::OAuthUnsupported);
        }
        let expires_at = began
            .wall_seconds()?
            .checked_add(i64::try_from(lifetime.as_secs()).map_err(|_| EndpointError::Invalid)?)
            .ok_or(EndpointError::Invalid)?;
        Ok(OAuthCredentials {
            access_token,
            refresh_token,
            expires_at,
            issuer: self.issuer.clone(),
            resource: self.resource.clone(),
            client_id: self.client_id.clone(),
            scopes,
        })
    }
}
impl Consent {
    pub fn live(&self, now: ClockSample) -> bool {
        now.elapsed_since(self.began)
            .is_ok_and(|elapsed| elapsed < 300_000_000_000)
    }
    pub fn state(&self) -> &str {
        &self.state
    }
    pub fn callback(
        &self,
        raw: &str,
        now: ClockSample,
    ) -> Result<(String, String, Option<String>)> {
        if now.elapsed_since(self.began)? >= 300_000_000_000 || raw.len() > 16384 {
            return Err(EndpointError::Cancelled);
        }
        let url = Url::parse(raw).map_err(|_| EndpointError::Invalid)?;
        if url.origin() != self.protocol.callback.origin()
            || url.path() != self.protocol.callback.path()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(EndpointError::Invalid);
        }
        let mut parameters = BTreeMap::new();
        for (key, value) in url.query_pairs() {
            if parameters
                .insert(key.into_owned(), value.into_owned())
                .is_some()
            {
                return Err(EndpointError::Invalid);
            }
        }
        if parameters.get("state") != Some(&self.state) {
            return Err(EndpointError::Invalid);
        }
        if parameters
            .get("iss")
            .is_some_and(|issuer| issuer != &self.protocol.issuer)
        {
            return Err(EndpointError::Invalid);
        }
        if self.protocol.issuer_required && !parameters.contains_key("iss") {
            return Err(EndpointError::Invalid);
        }
        if parameters.contains_key("error") {
            return Err(EndpointError::Cancelled);
        }
        let code = parameters
            .get("code")
            .filter(|code| !code.is_empty() && code.len() <= 8192)
            .cloned()
            .ok_or(EndpointError::Invalid)?;
        Ok((code, self.state.clone(), parameters.get("iss").cloned()))
    }
    pub async fn exchange(&self, raw: &str, now: ClockSample) -> Result<OAuthCredentials> {
        let (code, state, issuer) = self.callback(raw, now)?;
        let result = self
            .protocol
            .manager
            .exchange_code_for_token_with_issuer(&code, &state, issuer.as_deref())
            .await
            .map_err(|_| EndpointError::UpstreamRequired)?;
        self.protocol.complete(result, now).await
    }
}
