//! Exact-destination, bounded Streamable HTTP client for the official SDK worker.
use crate::wire::{Direction, MAX_MESSAGE_BYTES, sanitize};
use futures::{StreamExt, stream::BoxStream};
use http::{HeaderName, HeaderValue};
use rmcp::{
    model::{ClientJsonRpcMessage, JsonRpcMessage, ServerJsonRpcMessage},
    transport::streamable_http_client::{
        SseError, StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
    },
};
use sse_stream::{Sse, SseStream};
use std::{collections::HashMap, io, sync::Arc, time::Duration};
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum RemoteFailure {
    #[error("Remote MCP is unavailable")]
    Unavailable,
    #[error("Remote MCP returned an invalid or oversized response")]
    Protocol,
    #[error("Remote MCP rejected authentication")]
    Unauthorized,
    #[error("Remote MCP destination or headers were rejected")]
    Destination,
}
type HttpResult<T> = Result<T, StreamableHttpError<RemoteFailure>>;
fn failure(error: RemoteFailure) -> StreamableHttpError<RemoteFailure> {
    StreamableHttpError::Client(error)
}

#[derive(Clone)]
pub struct BoundedHttpClient {
    client: reqwest::Client,
    uri: Arc<str>,
    access: Option<Arc<Zeroizing<String>>>,
    rejected: Option<Arc<dyn Fn() + Send + Sync>>,
}
impl BoundedHttpClient {
    pub fn new(uri: &str, allow_fixture_http: bool) -> Result<Self, RemoteFailure> {
        let uri = dhivon_mcp_gateway_core::validate_remote_url(uri, allow_fixture_http)
            .map_err(|_| RemoteFailure::Destination)?
            .to_string();
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(60))
            .build()
            .map_err(|_| RemoteFailure::Unavailable)?;
        Ok(Self {
            client,
            uri: uri.into(),
            access: None,
            rejected: None,
        })
    }
    /// Only native scoped credential resolution may supply this value; no central session token.
    pub fn with_access(mut self, token: Zeroizing<String>) -> Result<Self, RemoteFailure> {
        if token.is_empty() || token.len() > 16384 || token.contains(['\0', '\r', '\n']) {
            return Err(RemoteFailure::Unauthorized);
        }
        self.access = Some(Arc::new(token));
        Ok(self)
    }
    pub fn uri(&self) -> Arc<str> {
        self.uri.clone()
    }
    pub fn with_rejection_handler(mut self, rejected: Arc<dyn Fn() + Send + Sync>) -> Self {
        self.rejected = Some(rejected);
        self
    }
    fn request(
        &self,
        method: reqwest::Method,
        uri: &str,
        session: Option<&str>,
        auth: Option<&str>,
        headers: HashMap<HeaderName, HeaderValue>,
    ) -> HttpResult<reqwest::RequestBuilder> {
        if uri != self.uri.as_ref()
            || auth.is_some_and(|v| self.access.as_ref().is_none_or(|a| a.as_str() != v))
        {
            return Err(failure(RemoteFailure::Destination));
        }
        // No caller-controlled header forwarding. The worker's version header is normalized to our profile.
        if headers
            .keys()
            .any(|name| name.as_str() != "mcp-protocol-version")
        {
            return Err(failure(RemoteFailure::Destination));
        }
        let mut request = self
            .client
            .request(method, uri)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-11-25");
        if let Some(session) = session {
            if session.is_empty()
                || session.len() > 256
                || !session.bytes().all(|b| (0x21..=0x7e).contains(&b))
            {
                return Err(failure(RemoteFailure::Protocol));
            }
            request = request.header("mcp-session-id", session);
        }
        if let Some(access) = &self.access {
            request = request.bearer_auth(access.as_str());
        }
        Ok(request)
    }
    async fn send(
        request: reqwest::RequestBuilder,
        deadline: Duration,
    ) -> HttpResult<reqwest::Response> {
        tokio::time::timeout(deadline, request.send())
            .await
            .map_err(|_| failure(RemoteFailure::Unavailable))?
            .map_err(|_| failure(RemoteFailure::Unavailable))
    }
    fn status(&self, response: &reqwest::Response) -> HttpResult<()> {
        if response.status() == reqwest::StatusCode::UNAUTHORIZED
            || response.status() == reqwest::StatusCode::FORBIDDEN
        {
            if let Some(rejected) = &self.rejected {
                rejected();
            }
            return Err(failure(RemoteFailure::Unauthorized));
        }
        if !response.status().is_success() {
            return Err(failure(RemoteFailure::Unavailable));
        }
        Ok(())
    }
}
fn session(response: &reqwest::Response) -> HttpResult<Option<String>> {
    match response.headers().get("mcp-session-id") {
        None => Ok(None),
        Some(value) => {
            let value = value
                .to_str()
                .map_err(|_| failure(RemoteFailure::Protocol))?;
            if value.is_empty()
                || value.len() > 256
                || !value.bytes().all(|b| (0x21..=0x7e).contains(&b))
            {
                return Err(failure(RemoteFailure::Protocol));
            }
            Ok(Some(value.into()))
        }
    }
}
fn content_type(response: &reqwest::Response) -> Option<&str> {
    response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .map(str::trim)
}
async fn body(mut response: reqwest::Response) -> HttpResult<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|n| n > MAX_MESSAGE_BYTES as u64)
    {
        return Err(failure(RemoteFailure::Protocol));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| failure(RemoteFailure::Unavailable))?
    {
        if chunk.len() > MAX_MESSAGE_BYTES.saturating_sub(bytes.len()) {
            return Err(failure(RemoteFailure::Protocol));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

impl StreamableHttpClient for BoundedHttpClient {
    type Error = RemoteFailure;
    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth: Option<String>,
        headers: HashMap<HeaderName, HeaderValue>,
    ) -> HttpResult<StreamableHttpPostResponse> {
        self.post_message_with_max_sse_event_size(
            uri,
            message,
            session_id,
            auth,
            headers,
            MAX_MESSAGE_BYTES,
        )
        .await
    }
    async fn post_message_with_max_sse_event_size(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth: Option<String>,
        headers: HashMap<HeaderName, HeaderValue>,
        max: usize,
    ) -> HttpResult<StreamableHttpPostResponse> {
        let encoded = serde_json::to_vec(&message).map_err(|_| failure(RemoteFailure::Protocol))?;
        if encoded.len() > MAX_MESSAGE_BYTES {
            return Err(failure(RemoteFailure::Protocol));
        }
        let request = self
            .request(
                reqwest::Method::POST,
                &uri,
                session_id.as_deref(),
                auth.as_deref(),
                headers,
            )?
            .header("content-type", "application/json")
            .body(encoded);
        let call = matches!(&message, JsonRpcMessage::Request(request) if matches!(&request.request, rmcp::model::ClientRequest::CallToolRequest(_)));
        let response = Self::send(request, Duration::from_secs(if call { 60 } else { 10 })).await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND && session_id.is_some() {
            return Err(StreamableHttpError::SessionExpired);
        }
        self.status(&response)?;
        if matches!(
            response.status(),
            reqwest::StatusCode::ACCEPTED | reqwest::StatusCode::NO_CONTENT
        ) {
            return Ok(StreamableHttpPostResponse::Accepted);
        }
        let session = session(&response)?;
        match content_type(&response) {
            Some("text/event-stream") => Ok(StreamableHttpPostResponse::Sse(
                event_stream(response, max.min(MAX_MESSAGE_BYTES)),
                session,
            )),
            Some("application/json") => {
                let bytes = body(response).await?;
                if bytes.is_empty() && !matches!(message, JsonRpcMessage::Request(_)) {
                    return Ok(StreamableHttpPostResponse::Accepted);
                }
                let bytes = sanitize(&bytes, Direction::Upstream)
                    .map_err(|_| failure(RemoteFailure::Protocol))?;
                let message: ServerJsonRpcMessage =
                    serde_json::from_slice(&bytes).map_err(|_| failure(RemoteFailure::Protocol))?;
                Ok(StreamableHttpPostResponse::Json(message, session))
            }
            _ => Err(failure(RemoteFailure::Protocol)),
        }
    }
    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Option<Arc<str>>,
        last_event_id: Option<String>,
        auth: Option<String>,
        headers: HashMap<HeaderName, HeaderValue>,
    ) -> HttpResult<BoxStream<'static, Result<Sse, SseError>>> {
        self.get_stream_with_max_sse_event_size(
            uri,
            session_id,
            last_event_id,
            auth,
            headers,
            MAX_MESSAGE_BYTES,
        )
        .await
    }
    async fn get_stream_with_max_sse_event_size(
        &self,
        uri: Arc<str>,
        session_id: Option<Arc<str>>,
        last_event_id: Option<String>,
        auth: Option<String>,
        headers: HashMap<HeaderName, HeaderValue>,
        max: usize,
    ) -> HttpResult<BoxStream<'static, Result<Sse, SseError>>> {
        let mut request = self.request(
            reqwest::Method::GET,
            &uri,
            session_id.as_deref(),
            auth.as_deref(),
            headers,
        )?;
        if let Some(id) = last_event_id {
            if id.len() > 512 || id.contains(['\0', '\r', '\n']) {
                return Err(failure(RemoteFailure::Protocol));
            }
            request = request.header("last-event-id", id);
        }
        let response = Self::send(request, Duration::from_secs(10)).await?;
        if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(StreamableHttpError::ServerDoesNotSupportSse);
        }
        self.status(&response)?;
        if content_type(&response) != Some("text/event-stream") {
            return Err(failure(RemoteFailure::Protocol));
        }
        Ok(event_stream(response, max.min(MAX_MESSAGE_BYTES)))
    }
    async fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth: Option<String>,
        headers: HashMap<HeaderName, HeaderValue>,
    ) -> HttpResult<()> {
        let request = self
            .request(
                reqwest::Method::DELETE,
                &uri,
                Some(&session_id),
                auth.as_deref(),
                headers,
            )?
            .timeout(Duration::from_secs(1));
        let response = Self::send(request, Duration::from_secs(10)).await?;
        if matches!(
            response.status(),
            reqwest::StatusCode::METHOD_NOT_ALLOWED | reqwest::StatusCode::NOT_FOUND
        ) {
            return Ok(());
        }
        self.status(&response)
    }
}

#[derive(Default)]
struct EventBound {
    retained: usize,
    line: usize,
    comment: bool,
    cr: bool,
}
impl EventBound {
    fn observe(&mut self, bytes: &[u8], max: usize) -> io::Result<()> {
        for &byte in bytes {
            if self.cr {
                self.cr = false;
                if byte == b'\n' {
                    continue;
                }
            }
            if byte == b'\r' || byte == b'\n' {
                if self.line == 0 {
                    self.retained = 0;
                } else if !self.comment {
                    self.retained = self.retained.saturating_add(self.line).saturating_add(1);
                }
                self.line = 0;
                self.comment = false;
                self.cr = byte == b'\r';
            } else {
                if self.line == 0 {
                    self.comment = byte == b':';
                }
                self.line = self.line.saturating_add(1);
            }
            if self.retained.saturating_add(self.line) > max {
                return Err(io::Error::other("MCP event exceeded its bound"));
            }
        }
        Ok(())
    }
}
fn event_stream(
    response: reqwest::Response,
    max: usize,
) -> BoxStream<'static, Result<Sse, SseError>> {
    let source = futures::stream::unfold(
        (
            response.bytes_stream().boxed(),
            EventBound::default(),
            false,
        ),
        move |(mut stream, mut bound, failed)| async move {
            if failed {
                return None;
            }
            match stream.next().await {
                None => None,
                Some(Ok(bytes)) => {
                    let result = bound.observe(&bytes, max).map(|_| bytes);
                    let failed = result.is_err();
                    Some((result, (stream, bound, failed)))
                }
                Some(Err(_)) => Some((
                    Err(io::Error::other("MCP event stream closed")),
                    (stream, bound, true),
                )),
            }
        },
    );
    SseStream::from_bytes_stream(source)
        .map(|event| {
            let mut event = event?;
            if event.event.as_deref().is_some_and(|s| s != "message")
                || event
                    .id
                    .as_ref()
                    .is_some_and(|s| s.len() > 512 || s.contains(['\0', '\r', '\n']))
            {
                return Err(SseError::Body(Box::new(io::Error::other(
                    "Unsupported MCP event",
                ))));
            }
            if let Some(data) = event.data.as_ref().filter(|s| !s.is_empty()) {
                let bytes = sanitize(data.as_bytes(), Direction::Upstream)
                    .map_err(|_| SseError::Body(Box::new(io::Error::other("Invalid MCP event"))))?;
                event.data = Some(String::from_utf8(bytes).map_err(|_| {
                    SseError::Body(Box::new(io::Error::other("Invalid MCP event")))
                })?);
            }
            Ok(event)
        })
        .boxed()
}
