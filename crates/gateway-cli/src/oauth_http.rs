//! Bounded SDK OAuth I/O. Discovery establishes destinations before any token-bearing POST.
use crate::{EndpointError, Result};
use futures::StreamExt;
use rmcp::transport::auth::{
    AuthorizationMetadata, OAuthHttpClient, OAuthHttpClientFuture, OAuthHttpRequest,
};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::Duration,
};
use url::Url;

const LIMIT: usize = 1024 * 1024;
#[derive(Default)]
struct Destinations {
    issuers: BTreeSet<String>,
    token: Option<String>,
    resource_seen: bool,
}
#[derive(Clone)]
pub struct OAuthNetwork {
    resource: Url,
    fixture_http: bool,
    client: reqwest::Client,
    destinations: Arc<Mutex<Destinations>>,
}
impl OAuthNetwork {
    pub fn new(resource: &str, fixture_http: bool) -> Result<Self> {
        let raw = resource;
        let resource = gateway_core::validate_remote_url(raw, fixture_http)
            .map_err(|_| EndpointError::Invalid)?;
        if resource.as_str() != raw || resource.query().is_some() {
            return Err(EndpointError::Invalid);
        }
        Ok(Self {
            resource,
            fixture_http,
            client: reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(|_| EndpointError::Invalid)?,
            destinations: Arc::new(Mutex::new(Destinations::default())),
        })
    }
    fn url(&self, raw: &str) -> Result<Url> {
        gateway_core::validate_remote_url(raw, self.fixture_http)
            .map_err(|_| EndpointError::Invalid)
    }
    pub async fn challenge(&self) -> Result<Option<String>> {
        let response = self
            .client
            .get(self.resource.clone())
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-11-25")
            .send()
            .await
            .map_err(|_| EndpointError::Offline)?;
        if response.status().is_redirection() {
            return Err(EndpointError::Invalid);
        }
        if response.status() != reqwest::StatusCode::UNAUTHORIZED {
            return Ok(None);
        }
        let values: Vec<_> = response
            .headers()
            .get_all(http::header::WWW_AUTHENTICATE)
            .iter()
            .collect();
        if values.len() > 1 {
            return Err(EndpointError::Invalid);
        }
        match values.first() {
            Some(value) => {
                let challenge = value.to_str().map_err(|_| EndpointError::Invalid)?;
                if challenge.len() > 8192 {
                    return Err(EndpointError::Invalid);
                }
                Ok(Some(challenge.into()))
            }
            None => Ok(None),
        }
    }
    /// Pin only published metadata from a protected-resource discovery chain.
    pub fn pin(&self, metadata: &AuthorizationMetadata) -> Result<()> {
        let issuer = metadata
            .issuer
            .as_deref()
            .ok_or(EndpointError::OAuthUnsupported)?;
        let issuer_url = self.url(issuer)?;
        if issuer_url.query().is_some()
            || !self
                .destinations
                .lock()
                .map_err(|_| EndpointError::Invalid)?
                .issuers
                .contains(issuer)
        {
            return Err(EndpointError::Invalid);
        }
        self.url(&metadata.authorization_endpoint)?;
        let token = self.url(&metadata.token_endpoint)?;
        if token.query().is_some()
            || metadata
                .code_challenge_methods_supported
                .as_ref()
                .is_none_or(|methods| !methods.iter().any(|m| m == "S256"))
            || metadata
                .additional_fields
                .get("token_endpoint_auth_methods_supported")
                .and_then(Value::as_array)
                .is_some_and(|methods| {
                    !methods.iter().any(|method| method.as_str() == Some("none"))
                })
        {
            return Err(EndpointError::OAuthUnsupported);
        }
        let mut bound = self
            .destinations
            .lock()
            .map_err(|_| EndpointError::Invalid)?;
        if !bound.resource_seen {
            return Err(EndpointError::OAuthUnsupported);
        }
        bound.token = Some(token.to_string());
        Ok(())
    }
    fn check(&self, request: &oauth2::HttpRequest) -> Result<Url> {
        let url = self.url(&request.uri().to_string())?;
        let bound = self
            .destinations
            .lock()
            .map_err(|_| EndpointError::Invalid)?;
        if request.body().len() > 65536
            || request.headers().contains_key(http::header::AUTHORIZATION)
        {
            return Err(EndpointError::Invalid);
        }
        match *request.method() {
            http::Method::GET if request.body().is_empty() => {
                let allowed = url.origin() == self.resource.origin()
                    || bound.issuers.iter().any(|issuer| {
                        Url::parse(issuer).is_ok_and(|issuer| issuer.origin() == url.origin())
                    });
                if !allowed {
                    return Err(EndpointError::Invalid);
                }
            }
            http::Method::POST if bound.token.as_deref() == Some(url.as_str()) => {
                let mut resource = Vec::new();
                for (key, value) in url::form_urlencoded::parse(request.body()) {
                    if key == "resource" {
                        resource.push(value.into_owned());
                    }
                    if key == "client_secret" {
                        return Err(EndpointError::OAuthUnsupported);
                    }
                }
                if resource != [self.resource.to_string()] {
                    return Err(EndpointError::Invalid);
                }
            }
            _ => return Err(EndpointError::Invalid),
        }
        Ok(url)
    }
    fn observe(&self, url: &Url, body: &[u8]) -> Result<()> {
        let Ok(value) = serde_json::from_slice::<Value>(body) else {
            return Ok(());
        };
        if value.get("authorization_servers").is_none() {
            return Ok(());
        }
        if url.origin() != self.resource.origin()
            || value.get("resource").and_then(Value::as_str) != Some(self.resource.as_str())
        {
            return Err(EndpointError::Invalid);
        }
        let issuers = value
            .get("authorization_servers")
            .and_then(Value::as_array)
            .ok_or(EndpointError::Invalid)?;
        if issuers.is_empty() || issuers.len() > 16 {
            return Err(EndpointError::Invalid);
        }
        let mut validated = BTreeSet::new();
        for issuer in issuers {
            let issuer = issuer.as_str().ok_or(EndpointError::Invalid)?;
            let parsed = self.url(issuer)?;
            if parsed.query().is_some() {
                return Err(EndpointError::Invalid);
            }
            validated.insert(issuer.to_owned());
        }
        let mut bound = self
            .destinations
            .lock()
            .map_err(|_| EndpointError::Invalid)?;
        bound.issuers.extend(validated);
        bound.resource_seen = true;
        Ok(())
    }
}
impl OAuthHttpClient for OAuthNetwork {
    fn execute(&self, input: OAuthHttpRequest) -> OAuthHttpClientFuture<'_> {
        Box::pin(async move {
            let url = self.check(&input.request)?;
            let mut request = self
                .client
                .request(input.request.method().clone(), url.clone());
            // SDK-generated public-client requests carry no ambient credentials or cookies.
            for name in [http::header::ACCEPT, http::header::CONTENT_TYPE] {
                if let Some(value) = input.request.headers().get(&name) {
                    request = request.header(name, value);
                }
            }
            let response = request
                .body(input.request.body().clone())
                .send()
                .await
                .map_err(|_| EndpointError::Offline)?;
            if response.status().is_redirection() {
                return Err(EndpointError::Invalid.into());
            }
            if response
                .content_length()
                .is_some_and(|size| size > LIMIT as u64)
            {
                return Err(EndpointError::Response.into());
            }
            let status = response.status();
            let headers = response.headers().clone();
            let mut body = Vec::new();
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|_| EndpointError::Offline)?;
                if body.len().saturating_add(chunk.len()) > LIMIT {
                    return Err(EndpointError::Response.into());
                }
                body.extend_from_slice(&chunk);
            }
            if input.request.method() == http::Method::GET && status.is_success() {
                self.observe(&url, &body)?;
            }
            let mut output = http::Response::builder().status(status).body(body)?;
            *output.headers_mut() = headers;
            Ok(output)
        })
    }
}
