# Why Dhivon MCP Gateway exists

Teams adopting MCP need a consistent answer to three questions. Which users may
use an MCP, which tools may they call, and which assigned MCPs does each developer
want enabled? Configuring every upstream separately in every agent spreads those
choices across machines and client files.

Dhivon MCP Gateway brings team assignments and tool policies into one self-hosted
control plane. Developers manage their assigned catalog locally and connect each
coding agent to one gateway entry. This gives administrators one place to change
access while preserving an explicit developer action before an MCP is enabled.

## Components and execution locations

```text
Administrator browser ── HTTPS ──> Self-hosted control plane
                                    │
                                    ├── PostgreSQL configuration and encrypted secrets
                                    └── Operator-held vault key file
                                    │
                              HTTPS policy and credential synchronization
                                    │
Developer browser ── loopback ──> Mac management process
                                    │
                                    └── Keychain and protected local policy snapshot
                                               │
Coding agent ── local STDIO ──> Mac gateway process
                                    ├── child STDIO MCP running as the developer
                                    └── HTTPS Streamable HTTP MCP at its remote service
```

The administrator uses the central website to manage identities, catalog entries,
assignments and original-name tool policies. PostgreSQL holds the authoritative
configuration and per-user enablement. The central service also owns encrypted
user secrets and installation-scoped OAuth records.

The Mac management process serves a protected loopback website. It handles login,
explicit installation, enablement, secret selection, OAuth consent and status.
Native credentials and cache keys live in macOS Keychain. The agent starts a
separate `dhivon-mcp-gateway mcp` process that reads the selected identity and
protected authorization state. Its stdout carries MCP protocol messages.

Local MCP child processes execute on the developer's Mac with that user's
privileges. Remote MCP requests go from the Mac gateway to the exact configured
upstream HTTPS endpoint. The central container manages access and credentials.
The diagram's policy connection supplies authorization state to the Mac, while
the agent's tool calls pass through its local gateway.

## A tool becomes usable through several decisions

1. An administrator creates a catalog entry. New entries have no assignments and
   an empty tool allowlist.
2. The administrator assigns it to a user, group or all users, then permits the
   intended original tool names.
3. The developer explicitly installs the MCP if needed, prepares its supported
   authentication and enables it through local management.
4. The gateway synchronizes a complete authenticated snapshot and discovers
   available tools. The agent sees deterministic aliases for eligible tools.
5. Each listing and call checks assignment, active definition, enablement,
   original-name policy, identity, lease and credential readiness again.

The [echo quickstart](quickstart.md) follows this path. Disabling the MCP removes
eligibility. Removing its final assignment clears enablement centrally, and
reassignment needs another opt-in. Running endpoints learn changes through
synchronization. During an outage, an existing snapshot permits eligible work
only until its bounded lease expires.

## Boundaries that shape deployment

One enterprise operates each self-hosted deployment. The current Mac endpoint
supports Apple Silicon, local STDIO and remote Streamable HTTP with the tested
MCP tools profile. The [compatibility matrix](compatibility.md) records the exact
protocol, platform, browser and distribution evidence.

Policy aliases keep routing stable but grant no authority of their own. A client
cannot bypass a deny decision by retaining a previous alias. Side effects already
performed upstream cannot be undone by later cancellation or revocation.
[Aggregation](gateway-aggregation.md) covers bounds, cancellation and process
ownership. [Security](security.md) explains trust, secret handling and expiry.
