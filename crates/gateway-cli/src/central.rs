//! Native-only central traffic. No redirects, cookie jar, unsafe TLS, or raw transport errors.
use crate::{EndpointError, Result, epoch};
use gateway_core::{EndpointSession, LoginRequest};
use reqwest::{Client, Method, Response, StatusCode};
use std::time::Duration;
use url::Url;

#[derive(Clone)]
pub struct CentralClient {
    client: Client,
    origin: Url,
}
impl CentralClient {
    pub fn new(server: &str, allow_fixture_http: bool) -> Result<Self> {
        let origin = gateway_core::validate_remote_url(server, allow_fixture_http)
            .map_err(|_| EndpointError::Invalid)?;
        if origin.path() != "/" || origin.query().is_some() {
            return Err(EndpointError::Invalid);
        }
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(10))
            .no_proxy()
            .build()
            .map_err(|_| EndpointError::Invalid)?;
        Ok(Self { client, origin })
    }
    pub fn origin(&self) -> String {
        self.origin.origin().ascii_serialization()
    }
    pub async fn login(&self, input: LoginRequest) -> Result<EndpointSession> {
        let response = self
            .client
            .post(
                self.origin
                    .join("/api/endpoint/login")
                    .map_err(|_| EndpointError::Invalid)?,
            )
            .json(&input)
            .send()
            .await
            .map_err(|_| EndpointError::Offline)?;
        let response = classify(response)?;
        let bytes = bounded_body(response).await?;
        let session: EndpointSession =
            serde_json::from_slice(&bytes).map_err(|_| EndpointError::Response)?;
        let now = epoch()?;
        if session.token.is_empty()
            || session.token.len() > 512
            || !session
                .token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
            || !session.user.active
            || session.expires_at <= now
            || session.expires_at > now + 43200
            || uuid::Uuid::parse_str(&session.user.id).is_err()
            || uuid::Uuid::parse_str(&session.deployment_id).is_err()
        {
            return Err(EndpointError::Response);
        }
        Ok(session)
    }
    /// Only fixed central API paths are accepted; upstream URLs can never receive this token.
    pub async fn request(
        &self,
        session: &EndpointSession,
        method: Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<Response> {
        self.request_inner(session, method, path, body, false).await
    }
    pub async fn control_request(
        &self,
        session: &EndpointSession,
        method: Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<Response> {
        self.request_inner(session, method, path, body, true).await
    }
    async fn request_inner(
        &self,
        session: &EndpointSession,
        method: Method,
        path: &str,
        body: Option<&serde_json::Value>,
        preserve_errors: bool,
    ) -> Result<Response> {
        if !path.starts_with("/api/") || path.contains(['?', '#', '\\']) || path.contains("..") {
            return Err(EndpointError::Invalid);
        }
        let mut request = self
            .client
            .request(
                method,
                self.origin.join(path).map_err(|_| EndpointError::Invalid)?,
            )
            .bearer_auth(&session.token);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await.map_err(|_| EndpointError::Offline)?;
        if preserve_errors
            && response.status().is_client_error()
            && response.status() != StatusCode::UNAUTHORIZED
        {
            return Ok(response);
        }
        classify(response)
    }
    pub async fn revoke(&self, session: &EndpointSession) -> Result<()> {
        self.request(session, Method::POST, "/api/auth/logout", None)
            .await
            .map(|_| ())
    }
}
fn classify(response: Response) -> Result<Response> {
    match response.status() {
        StatusCode::UNAUTHORIZED => Err(EndpointError::Rejected),
        s if s.is_success() || s == StatusCode::CONFLICT => Ok(response),
        _ => Err(EndpointError::Response),
    }
}
pub async fn bounded_body(mut response: Response) -> Result<zeroize::Zeroizing<Vec<u8>>> {
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    while let Some(chunk) = response.chunk().await.map_err(|_| EndpointError::Offline)? {
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
            return Err(EndpointError::Response);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
