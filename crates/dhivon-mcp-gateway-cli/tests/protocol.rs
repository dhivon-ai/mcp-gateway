use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
};
use dhivon_mcp_gateway_cli::{
    upstream_http::BoundedHttpClient,
    wire::{BoundedRead, BoundedTransport, Direction, MAX_MESSAGE_BYTES},
};
use rmcp::{
    ServiceExt,
    model::*,
    transport::{
        StreamableHttpClientTransport,
        async_rw::AsyncRwTransport,
        streamable_http_client::{StreamableHttpClient, StreamableHttpClientTransportConfig},
    },
};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zeroize::Zeroizing;

#[derive(Clone, Default)]
struct Logs(Arc<std::sync::Mutex<Vec<u8>>>);
impl std::io::Write for Logs {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("captured fixture logs")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Logs {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}
#[test]
fn endpoint_logging_excludes_sdk_network_and_application_debug_payloads() {
    let logs = Logs::default();
    tracing::subscriber::with_default(
        dhivon_mcp_gateway_cli::logging::subscriber(logs.clone()),
        || {
            tracing::info!(target: "rmcp::service", raw_message = "private-sdk-fixture");
            tracing::warn!(target: "reqwest", body = "private-network-fixture");
            tracing::debug!(target: "dhivon_mcp_gateway_cli::aggregation", payload = "private-debug-fixture");
            tracing::info!(target: "dhivon_mcp_gateway_cli::aggregation", event = "gateway_policy_allowed", correlation_id = "owned-fixture");
        },
    );
    let text = String::from_utf8(logs.0.lock().expect("logs").clone()).expect("UTF8 logs");
    assert!(text.contains("gateway_policy_allowed"));
    assert!(!text.contains("private-"));
}

#[tokio::test]
async fn fragmented_cancelled_reads_keep_complete_frames_and_task_requests_never_reach_sdk() {
    let (mut source, read) = tokio::io::duplex(128);
    let mut read = BoundedRead::new(read, Direction::Agent);
    source
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,")
        .await
        .expect("fragment");
    let mut byte = [0u8; 1];
    assert!(
        tokio::time::timeout(Duration::from_millis(20), read.read(&mut byte))
            .await
            .is_err()
    );
    source
        .write_all(b"\"method\":\"ping\"}\n")
        .await
        .expect("remaining frame");
    source.shutdown().await.expect("EOF");
    let mut received = Vec::new();
    read.read_to_end(&mut received).await.expect("resumed read");
    assert_eq!(
        serde_json::from_slice::<Value>(&received).expect("complete JSON"),
        json!({"jsonrpc":"2.0","id":1,"method":"ping"})
    );
    let task = b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"execute\",\"task\":{}}}\n";
    let mut rejected = BoundedRead::new(task.as_slice(), Direction::Agent);
    let mut output = Vec::new();
    assert!(rejected.read_to_end(&mut output).await.is_err());
    assert!(output.is_empty());
    let required = json!({"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"task-only","execution":{"taskSupport":"required"},"inputSchema":{"type":"object"}},{"name":"normal","execution":{"taskSupport":"optional"},"inputSchema":{"type":"object"}}]}}).to_string() + "\n";
    let mut upstream = BoundedRead::new(required.as_bytes(), Direction::Upstream);
    let mut output = Vec::new();
    upstream
        .read_to_end(&mut output)
        .await
        .expect("supported frame");
    let frame: Value = serde_json::from_slice(&output).expect("filtered JSON");
    assert_eq!(frame["result"]["tools"].as_array().expect("tools").len(), 1);
    assert_eq!(frame["result"]["tools"][0]["name"], "normal");
    assert!(frame["result"]["tools"][0].get("execution").is_none());
}

#[tokio::test]
async fn incoming_unterminated_and_outgoing_oversized_frames_are_rejected_before_output() {
    let oversized = vec![b'x'; MAX_MESSAGE_BYTES + 1];
    let mut read = BoundedRead::new(oversized.as_slice(), Direction::Upstream);
    let mut output = Vec::new();
    assert!(read.read_to_end(&mut output).await.is_err());
    assert!(output.is_empty());
    let (writer, mut reader) = tokio::io::duplex(128);
    let mut transport =
        BoundedTransport::new(AsyncRwTransport::new_client(tokio::io::empty(), writer));
    let message = ClientJsonRpcMessage::request(
        ClientRequest::CallToolRequest(CallToolRequest::new(
            CallToolRequestParams::new("execute").with_arguments(
                json!({"message":"private-oversize".repeat(MAX_MESSAGE_BYTES / 8)})
                    .as_object()
                    .cloned()
                    .expect("literal arguments"),
            ),
        )),
        RequestId::Number(1),
    );
    assert!(
        rmcp::transport::Transport::<rmcp::RoleClient>::send(&mut transport, message)
            .await
            .is_err()
    );
    let mut byte = [0u8; 1];
    assert!(
        tokio::time::timeout(Duration::from_millis(20), reader.read(&mut byte))
            .await
            .is_err(),
        "no partial oversized frame was written"
    );
}

fn ping() -> ClientJsonRpcMessage {
    ClientJsonRpcMessage::request(
        ClientRequest::PingRequest(PingRequest::default()),
        RequestId::Number(1),
    )
}

#[test]
fn downstream_pages_fit_frames_and_changed_catalogs_reject_old_cursors() {
    let tools: Vec<_> = (0..12)
        .map(|index| {
            Tool::new(
                format!("tool-{index:02}"),
                "d".repeat(120_000),
                serde_json::Map::new(),
            )
        })
        .collect();
    let first = dhivon_mcp_gateway_cli::runtime::paginate(tools.clone(), None).expect("first page");
    assert!(first.tools.len() < tools.len());
    assert!(serde_json::to_vec(&first).expect("page bytes").len() < MAX_MESSAGE_BYTES);
    let cursor = first.next_cursor.clone().expect("bounded cursor");
    let next = dhivon_mcp_gateway_cli::runtime::paginate(tools.clone(), Some(cursor.clone()))
        .expect("second page");
    assert!(next.next_cursor.is_none());
    assert_eq!(first.tools.len() + next.tools.len(), tools.len());
    let mut changed = tools;
    changed[0].description = Some("new schema description".into());
    assert!(dhivon_mcp_gateway_cli::runtime::paginate(changed, Some(cursor)).is_err());
    for cursor in ["x".repeat(100), "bogus:1".into(), ":0".into()] {
        assert!(dhivon_mcp_gateway_cli::runtime::paginate(Vec::new(), Some(cursor)).is_err());
    }
}

#[tokio::test]
async fn sse_limits_cover_fragmented_events_and_respect_smaller_sdk_bounds() {
    use futures::StreamExt;
    use rmcp::transport::streamable_http_client::StreamableHttpPostResponse;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("SSE fixture");
    let address = listener.local_addr().expect("fixture address");
    let app = Router::new().route(
        "/mcp",
        post(|| async {
            // Each line is small, but the retained event crosses the requested limit.
            let chunks = [
                "data: {\"jsonrpc\":\"2.0\",\r",
                "\ndata: \"id\":1,\r\n",
                "data: \"result\":{}}\r\n\r\n",
            ];
            let source = futures::stream::iter(
                chunks
                    .into_iter()
                    .map(|s| Ok::<_, std::io::Error>(bytes::Bytes::from(s))),
            );
            (
                [("content-type", "text/event-stream")],
                axum::body::Body::from_stream(source),
            )
        })
        .get(|| async {
            (
                [("content-type", "text/event-stream")],
                format!("data: {}", "x".repeat(MAX_MESSAGE_BYTES + 1)),
            )
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("owned SSE fixture");
    });
    let client = BoundedHttpClient::new(&format!("http://{address}/mcp"), true).expect("client");
    let response = client
        .post_message_with_max_sse_event_size(
            client.uri(),
            ping(),
            None,
            None,
            Default::default(),
            32,
        )
        .await
        .expect("SSE headers");
    let StreamableHttpPostResponse::Sse(mut stream, _) = response else {
        panic!("expected SSE")
    };
    assert!(stream.next().await.expect("bound failure").is_err());
    assert!(stream.next().await.is_none());
    let response = client
        .post_message_with_max_sse_event_size(
            client.uri(),
            ping(),
            None,
            None,
            Default::default(),
            256,
        )
        .await
        .expect("SSE headers");
    let StreamableHttpPostResponse::Sse(mut stream, _) = response else {
        panic!("expected SSE")
    };
    let event = stream
        .next()
        .await
        .expect("complete event")
        .expect("supported event");
    assert_eq!(
        serde_json::from_str::<Value>(event.data.as_deref().expect("RPC data")).expect("JSON"),
        json!({"jsonrpc":"2.0","id":1,"result":{}})
    );
    let mut stream = client
        .get_stream_with_max_sse_event_size(
            client.uri(),
            None,
            None,
            None,
            Default::default(),
            usize::MAX,
        )
        .await
        .expect("GET SSE");
    assert!(stream.next().await.expect("hard event bound").is_err());
    server.abort();
}

#[tokio::test]
async fn redirect_cannot_forward_native_upstream_credentials_and_large_http_body_is_bounded() {
    let destination_calls = Arc::new(AtomicUsize::new(0));
    let counted = destination_calls.clone();
    let destination = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("redirect target");
    let target = destination.local_addr().expect("target address");
    let target_task = tokio::spawn(async move {
        axum::serve(
            destination,
            Router::new().route(
                "/other",
                post(move || {
                    let counted = counted.clone();
                    async move {
                        counted.fetch_add(1, Ordering::Relaxed);
                        StatusCode::OK
                    }
                }),
            ),
        )
        .await
        .expect("owned redirect target");
    });
    let origin = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("origin");
    let address = origin.local_addr().expect("address");
    let app = Router::new()
        .route(
            "/mcp",
            post(move |headers: HeaderMap| async move {
                assert_eq!(headers["authorization"], "Bearer private-upstream-fixture");
                (
                    StatusCode::TEMPORARY_REDIRECT,
                    [("location", format!("http://{target}/other"))],
                )
            }),
        )
        .route(
            "/large",
            post(|| async {
                (
                    [("content-type", "application/json")],
                    "x".repeat(MAX_MESSAGE_BYTES + 1),
                )
            }),
        );
    let task = tokio::spawn(async move {
        axum::serve(origin, app).await.expect("owned origin");
    });
    let uri = format!("http://{address}/mcp");
    let client = BoundedHttpClient::new(&uri, true)
        .expect("bounded client")
        .with_access(Zeroizing::new("private-upstream-fixture".into()))
        .expect("native credential");
    assert!(
        client
            .post_message(client.uri(), ping(), None, None, Default::default())
            .await
            .is_err()
    );
    assert_eq!(destination_calls.load(Ordering::Relaxed), 0);
    let large =
        BoundedHttpClient::new(&format!("http://{address}/large"), true).expect("large fixture");
    assert!(
        large
            .post_message(large.uri(), ping(), None, None, Default::default())
            .await
            .is_err()
    );
    assert!(
        client
            .post_message(large.uri(), ping(), None, None, Default::default())
            .await
            .is_err(),
        "exact destination is pinned"
    );
    task.abort();
    target_task.abort();
}

#[derive(Clone, Default)]
struct Replay {
    initializes: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
}
async fn expire_session(
    State(state): State<Replay>,
    axum::Json(request): axum::Json<Value>,
) -> axum::response::Response {
    match request["method"].as_str() {
        Some("initialize") => {
            state.initializes.fetch_add(1, Ordering::Relaxed);
            ([("mcp-session-id", "owned-session")], axum::Json(json!({"jsonrpc":"2.0","id":request["id"],"result":{"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"expiry-fixture","version":"1"}}}))).into_response()
        }
        Some("tools/call") => {
            state.calls.fetch_add(1, Ordering::Relaxed);
            StatusCode::NOT_FOUND.into_response()
        }
        _ => StatusCode::ACCEPTED.into_response(),
    }
}
#[tokio::test]
async fn expired_http_session_never_reinitializes_and_replays_a_side_effecting_call() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("session fixture");
    let address = listener.local_addr().expect("address");
    let state = Replay::default();
    let app = Router::new()
        .route("/mcp", post(expire_session))
        .with_state(state.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("owned session fixture");
    });
    let client =
        BoundedHttpClient::new(&format!("http://{address}/mcp"), true).expect("bounded client");
    let config = StreamableHttpClientTransportConfig::with_uri(client.uri())
        .reinit_on_expired_session(false)
        .max_sse_event_size(MAX_MESSAGE_BYTES);
    let peer = ClientConfig::new(
        ClientCapabilities::default(),
        Implementation::new("gateway-protocol-test", "1"),
    )
    .with_protocol_version(ProtocolVersion::V_2025_11_25)
    .serve(StreamableHttpClientTransport::with_client(client, config))
    .await
    .expect("session initialize");
    assert!(
        peer.call_tool(CallToolRequestParams::new("execute"))
            .await
            .is_err()
    );
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(state.calls.load(Ordering::Relaxed), 1);
    assert_eq!(state.initializes.load(Ordering::Relaxed), 1);
    let _ = peer.cancel().await;
    task.abort();
}
