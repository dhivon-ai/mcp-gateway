# Security and trust boundaries

Dhivon MCP Gateway enforces administrator assignments, explicit user enablement
and tool policy at the central service and the Mac gateway. Administrators should
review a server definition before assigning it because installation and local MCP
commands execute with the developer's OS privileges after explicit action.

## Identity and permissions

An operator bootstraps the first administrator explicitly. Users sign in with a
canonical email and password. Display names never authenticate. Passwords use
Argon2id, sessions expire within 12 hours, and account/password edits revoke the
affected sessions. The service enforces administrator roles independently of the
browser. See [identity](identity.md) for validation, rate limits and exact APIs.

Grants are additive across user, group and all-user assignments. No assignment
means no MCP access. A grant still requires the user's explicit enablement and a
permitted original tool name. An empty allowlist exposes no tools. The same
checks govern discovery and direct calls. [Configuration](configuration.md)
defines revocation, changed transport/authentication and revision conflicts.

## Credentials and data at rest

| Data                                                           | Owner and protection                                                 |
| -------------------------------------------------------------- | -------------------------------------------------------------------- |
| Central catalog, assignments, enablement and ordinary metadata | PostgreSQL, under operator database controls                         |
| User bearer secrets and managed OAuth values                   | Owner/version-bound authenticated encryption in PostgreSQL           |
| Central encryption keys                                        | External operator-held private key file, backed up separately        |
| Native session tokens, upstream credentials and cache keys     | macOS Keychain in a deployment/user/installation/login scope         |
| Native authorization snapshot                                  | Authenticated encryption in private user-scoped files                |
| Browser state                                                  | HttpOnly session cookies and in-memory CSRF, metadata-only read APIs |

Administrator access does not reveal another user's secret values. Browser APIs
expose credential metadata only. Native credential resolution checks current
assignment, enablement, ownership and version, then pins the result to the exact
configured upstream. A central login token is never sent as upstream
authentication. [Vault operations](vault.md) and
[native credentials](remote-credentials.md) specify publication, ownership,
rotation, capacity and recovery.

Keep database dumps and their matching key backups under separate access
controls. Losing the matching vault keys makes encrypted values unusable. Test a
fresh restore and credential resolution before switching traffic. The
[deployment recovery procedure](deployment.md#backup-and-restore) includes copied
session invalidation and review of policy changes since the restore point.

## Network and browser protection

Use verified HTTPS for the enterprise origin and remote production MCPs. The
Compose service binds loopback behind the operator's HTTPS proxy. Preserve the
public Host and Origin. The service and loopback management server enforce exact
Host/Origin, session and CSRF checks. The local management API binds loopback and
requires the active login generation on account-scoped actions.

Both protected interfaces serve their bundled assets locally with restrictive
content security policy. Secrets and session capabilities belong outside URLs,
frontend storage, logs and ordinary configuration. Controlled HTTP fixtures use
an explicit loopback exception limited to the
[fixture workflow](testing.md#mcp-and-oauth-fixtures).

## Policy leases and offline behavior

A complete authenticated, validated and durably installed snapshot can authorize
eligible tool use for at most 3,600 seconds from server validation. Session and
credential deadlines can shorten that interval. Only successful complete
synchronization renews it. Failed requests, process restart, token refresh and
file copying cannot extend the lease. Equality with the expiry deadline denies
use. Unknown clock or protected-state integrity requires online recovery.

Each gateway synchronizes at startup and normally every 60 seconds, with a
10-second control-plane request bound. A known session rejection invalidates the
matching identity immediately. A connectivity outage permits only previously
valid authorization within its existing lease. Settings, installation, secret
writes and new OAuth connections require connectivity and have no offline queue.
Logout and local cancellation remain available.

Read [policy cache](endpoint-cache.md) for precise clock, generation and
publication fences. The OS and Keychain are trusted. Current evidence covers
actual Mac process restart and deterministic time cases, with platform limits
listed in [compatibility](compatibility.md).

## Tool execution

Inputs, schemas, names and results from upstream MCPs are untrusted. The gateway
bounds discovery, frames, startup, calls and concurrency. Tool calls are never
automatically replayed. Disablement, expiry, cancellation and shutdown retire
owned work where possible. Cancellation cannot undo an upstream side effect.
Escaped descendants or abrupt termination require the operator's process
supervision policy. See [gateway aggregation](gateway-aggregation.md) for the
implemented transport profile and exact limits.
