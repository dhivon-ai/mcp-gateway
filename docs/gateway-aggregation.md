# Gateway aggregation contract

The endpoint uses the pinned official Rust MCP SDK, rmcp 3.5.1, with the tested MCP 2025-11-25 profile. The gateway forwards tool discovery and ordinary tool calls. It does not forward resources, prompts, sampling, elicitation, subscriptions, or tasks. Upstream clients advertise no optional client capabilities. Task-required tools are removed before SDK decoding; task requests from agents are rejected before they can become ordinary calls.

Local STDIO commands run only when assigned, centrally enabled, and active. Startup never runs an installation command. The endpoint owns the process group and keeps its leader unreaped while the SDK uses detached protocol pipes. Environment inheritance is restricted, stderr is discarded, and shutdown follows the bounded ownership contract in [endpoint management](endpoint-management.md). Missing executables and execution-permission failures have separate redacted status categories.

Remote servers use Streamable HTTP. Requests pin the configured URL, validate production TLS, reject redirects and caller-controlled headers, and normalize the protocol header to 2025-11-25. Central login tokens are used only with the control plane. Native upstream credentials have a separate resolver interface; the default resolver supports unauthenticated MCPs and marks bearer/OAuth entries as requiring authentication. Authentication integration follows in Phase 5. An expired HTTP session fails the request without reinitializing and replaying its POST.

Each agent-visible name is `gw_<first 32 hex characters of SHA-256(MCP ID)>_<SHA-256(original tool name)>`. The name is fixed ASCII, 100 bytes, and stable across tool-policy and schema changes. The route registry rejects ambiguous names instead of overwriting a route. Duplicate original names within one upstream's paginated catalog reject that upstream. Input/output schemas, descriptions, annotations, metadata, structured results, content, and tool-error results pass through the SDK's supported typed representation; only the tool name changes. Unknown protocol extensions are not promised to survive SDK decoding.

Every listing and new dispatch reads the latest committed, identity-bound encrypted snapshot. Listing and dispatch share the effective-tool predicate: assignment, active entry, central enablement, valid policy lease, current runtime/credential binding, credential readiness, and the original-name allowlist/denylist. Direct original names, unknown aliases, and hidden tools cannot reach an upstream. Learned disablement or runtime changes cancel pending discovery before publication. Expired credentials cannot gain extra time from a policy refresh.

Resource bounds are explicit:

| Resource | Maximum |
| --- | --- |
| Startup/discovery wave per MCP | 30 seconds, including native sleep-aware elapsed time |
| Simultaneously starting MCPs | 4 |
| Active aggregated MCPs | 32 |
| Paginated upstream discovery | 32 pages, distinct opaque cursors of at most 1,024 bytes |
| Tools in one upstream | 2,048 and 2 MiB serialized total |
| One original tool name / schema entry | 512 bytes / 256 KiB |
| Whole incoming/outgoing RPC or retained SSE event | 2 MiB; a smaller SDK SSE limit is honored |
| Aggregate tool catalog | 2,048 tools and 16 MiB |
| Owned aggregate shutdown | 5 seconds, concurrent across upstreams |

Partial STDIO lines, JSON bodies, and retained SSE event data are bounded before typed SDK decoding. Oversized outgoing STDIO messages are rejected before any bytes are written. Malformed catalogs, looping cursors, unsupported negotiation, and individual unavailable servers have isolated status outcomes; healthy servers remain usable. Tool-list notifications coalesce into a rediscovery signal. The agent runtime adds bounded downstream pagination, call cancellation/concurrency, refresh, and notification delivery in the next group.

After successful discovery, the endpoint may asynchronously report only the assigned MCP ID and original tool names through the native inventory API. The server rechecks assignment and treats this as untrusted metadata; it does not change grants, policies, revisions, or the cache lease. Reports never include schemas, arguments, results, commands, or credentials.

Endpoint logging enables only application INFO/WARN/ERROR targets. SDK/network and application DEBUG/TRACE events are excluded even when `RUST_LOG` is set, because SDK events may contain protocol payloads. Diagnostics use fixed categories rather than upstream response bodies.

Verification uses actual SDK STDIO/HTTP fixture servers and raw controlled HTTP fixtures: duplicate upstream names, multi-page discovery, schema/result preservation, allow/deny and direct-call witnesses, malformed/looping/duplicate catalogs, task capability boundaries, sleep-aware startup and learned disablement, bounded fragmented frames/SSE, destination isolation, and a 404-session no-replay witness. See [testing](testing.md) for prerequisites. These tests establish the stated profile; they do not claim arbitrary remote-provider or coding-agent compatibility.
