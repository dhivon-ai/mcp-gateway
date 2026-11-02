# One STDIO gateway for a coding agent

Install the Apple Silicon CLI and authenticate using the local management command:

```sh
dhivon-mcp-gateway ui --server https://gateway.example.com
```

The local page supports login and assigned MCP management. An administrator must grant access, and the user must explicitly enable an MCP before it exposes tools. An optional installer runs only through the explicit management action. Finish remote authentication in management before starting the coding agent; agent STDIO never opens a browser or asks for a password.

Configure the agent with one STDIO server. For clients using a JSON `mcpServers` configuration, the entry has this shape; replace the executable with the actual absolute installed path and adapt the enclosing file format to your agent:

```json
{
  "mcpServers": {
    "enterprise-gateway": {
      "command": "/absolute/path/to/dhivon-mcp-gateway",
      "args": ["mcp"]
    }
  }
}
```

Do not put the central URL, passwords, bearer tokens, or OAuth tokens in agent configuration or environment variables. The gateway uses the deployment/account already selected by `ui`. A different login generation requires restarting the agent's gateway process; an old process cannot silently adopt a new user's identity. The gateway supports native pipes and the connected Unix stream socket pairs used by Node child_process on macOS. It does not provide terminal-based interactive login or file-based protocol playback.

The tested client is the standard official rmcp 3.5.1 client configured with only this CLI entry point on macOS 27.0.1 arm64. The negotiated protocol is MCP 2025-11-25. Named commercial coding agents and other macOS versions have not yet been independently verified. See [deployment](deployment.md) and [Mac installation](mac-install.md) for the self-hosted service and locally built unsigned arm64 archive. Signing/notarization remains later scope.

The gateway advertises tools and tool-list changes. Resources, prompts, sampling, elicitation, subscriptions, and tasks are not forwarded. Tool names are deterministic aliases; [gateway aggregation](gateway-aggregation.md) describes mapping, original-name policy, bounds, and error isolation. Both input and output schemas and supported typed results are retained. The agent should follow `nextCursor` until exhausted; a changed catalog invalidates an old cursor and requires restarting discovery.

At startup, the gateway attempts one complete configuration refresh, then uses only a usable encrypted cache if that request fails. Without usable authorization it exits with a fixed diagnostic on stderr and no protocol bytes on stdout. After the handshake, discovery runs in the background. The first listing waits up to 30 seconds for initial discovery; slower remaining upstreams appear through tool-list notifications when the bounded discovery wave finishes. Individual MCP startup has a 30-second bound and at most four starts run concurrently. Existing healthy routes remain usable while rediscovery runs.

Every running gateway refreshes at startup and every 60 seconds with a 10-second control-plane deadline. Outage retries and reconnection attempts cannot renew freshness. Only a complete, validated, durably committed snapshot can renew the lease. The encrypted cache is usable for at most 3,600 seconds from server validation, further shortened by session and credential expiry; exact equality is expired. New listing/call authorization reads the latest shared snapshot, including same-Mac confirmed settings. During a control-plane outage the still-valid lease permits eligible tool use, while all configuration/install/auth mutations remain online-only.

Calls have a 60-second native sleep-aware deadline and at most 16 are admitted concurrently. Excess calls are refused immediately; there is no local call queue. Caller cancellation, learned policy denial, token/policy expiry, and gateway shutdown cancel the exact upstream request where possible. A dropped handler also sends best-effort cancellation. The runtime checks running-call policy/deadline at most every 50 ms and connection retirement at most every 250 ms while awake, and checks again before accepting a completed result. A side effect already performed upstream cannot be undone. Calls are never automatically replayed after reconnect or HTTP session expiry.

SIGTERM, Ctrl-C, and agent EOF trigger owned cleanup. Shutdown shares a five-second deadline; only gateway-owned upstream processes/connections are closed. Mac process-group ownership retains unreaped leaders through termination and reap. Nonblocking native stdin/stdout adapters avoid waiting indefinitely for an idle or blocked agent. Escaped or inaccessible descendants require external supervision, and abrupt SIGKILL cannot execute graceful shutdown handlers.

You can reduce the call/startup/concurrency limits:

```sh
dhivon-mcp-gateway mcp --call-timeout-seconds 30 --startup-timeout-seconds 15 --concurrency 8
```

The CLI rejects zero or larger-than-default bounds. `dhivon-mcp-gateway status` prints redacted identity/freshness status; `dhivon-mcp-gateway logout` clears local credentials/cache and requests central session revocation when reachable. Ordinary gateway diagnostics stay on stderr and include fixed policy/outcome categories, correlation IDs, durations, and validated identifiers without arguments, results, or credential values.

Isolated tests can use `--allow-fixture-http --fixture-state-path <existing absolute private directory>`. This creates a separate real-Keychain service and state scope; never use that flag for production. Fixture cleanup must run through the creating CLI's `logout`, and retain the journal if cleanup fails. No existing user agent configuration is modified by these tests.

Verification covers actual Mac Keychain login, real CLI STDIO local/SDK HTTP tools, current shared settings, cold offline restart and immediate reconnect refresh, SDK capability accuracy, cursor/schema changes, native call deadlines, caller cancellation, the 16-call admission limit, direct-call denial, one-hour expiry, malformed upstream isolation, no HTTP call replay, and native signal/owned descendant cleanup with idle input and blocked output. See [testing](testing.md) for the full test entry points and the limits of platform evidence.
