# Native endpoint identity and local browser boundary

This document retains the Phase 3 identity-slice verification below. The complete
MVP now serves the local React UI, synchronizes encrypted snapshots and runs MCP
aggregation; see [local UI](local-ui.md), [cache](endpoint-cache.md) and
[agent configuration](agent-configuration.md) for current behavior. Sign-in uses
email/password only; Name is display-only. Current status returns separate
`email` and `name` metadata. Older username accounts/API/cache are unsupported;
see [identity deployment compatibility](identity.md#fresh-identity-setup).

The identity slice provides real native login, Keychain storage and a protected
loopback API. The local product UI belongs to Phase 7; the current page explicitly
identifies itself as a development endpoint. Encrypted snapshot refresh belongs
to the next Phase 3 group, and STDIO aggregation to Phase 4. An authenticated
session alone does not authorize tools or establish a configuration lease.

```sh
dhivon-mcp-gateway ui --server https://gateway.example
dhivon-mcp-gateway status
dhivon-mcp-gateway logout
dhivon-mcp-gateway mcp
```

`ui` binds only `127.0.0.1`, chooses an ephemeral port and opens the plain local
page URL with `/usr/bin/open`. `--port` selects an explicit local port; `--no-open`
prints the address to stderr without browser interaction. The explicit
`--allow-fixture-http` flag accepts only isolated loopback HTTP control planes;
production requires HTTPS with certificate validation. `logout` accepts the same
fixture flag when revoking a fixture session. Control-plane URLs must be origins,
without credentials, query strings, fragments or path prefixes. Requests have a
10-second deadline, do not follow redirects and do not use an HTTP proxy.

`mcp` currently refuses startup with empty stdout and a fixed stderr diagnostic.
It never opens a browser, prompts on stdin or implies that an identity without a
validated snapshot permits tool execution. `status` emits ordinary identity and
absolute session expiry metadata; passwords, central tokens, upstream tokens,
local cookies and CSRF values are absent. It currently reports `sync_required`.

## Native state and Keychain ownership

The production directory is
`~/Library/Application Support/com.dhivon.mcp-gateway`. Directories are created
with mode 0700; metadata, lock files and atomically written files use 0600.
Existing permissive files/directories and symlinks are rejected. The state root
must be absolute and have no symlink ancestors. This follows the trusted OS/user
boundary; it does not claim protection from an attacker controlling the user
account or replacing the program.

`identity.json` contains only schema version, stable installation UUID, generation
UUID, the active origin/deployment/user IDs and owned Keychain account names. A
pending-cleanup journal contains only the same ordinary ownership metadata.
`state.lock` uses Rust 1.95 safe standard-library file locking across processes.
Atomic writes sync the temporary file, rename it and sync the containing directory.
Caller code must hold `StateStore::locked` across read/compare/write transactions.

The Keychain service is `com.dhivon.mcp-gateway.endpoint.v1`. Account names are
`v1:<identity SHA-256>:<generation UUID>:<credential kind>`, where the identity
hash includes canonical control-plane origin, deployment UUID and user UUID.
`session` stores the private native session JSON. `cache-key` is reserved for the
snapshot key; the typed upstream credential kind includes MCP UUID and a nonzero binding/version.
The ownership list makes all inserted entries discoverable for scoped cleanup,
without broad Keychain enumeration or deleting other applications' entries.

The production adapter calls the maintained security-framework 3.7.0 safe
`set_generic_password`, `get_generic_password` and `delete_generic_password`
APIs against the default macOS Keychain. Missing entries are distinct from locked
or failed Keychain access. Keychain failure has no plaintext or mock fallback.
A small `CredentialStore` trait supports deterministic tests; non-macOS production
adapters return unsupported, permitting Linux central CI without pretending to
support a Linux endpoint release. Credential structs have no Debug/TS export;
Keychain byte buffers and serialized native session buffers are zeroized on drop.

Login invalidates the old active identity locally before network work, removes
its cache and owned credentials and attempts to revoke its old central session.
Native login POSTs `/api/endpoint/login` and stores the returned private bearer
session only in Keychain. The origin, stable IDs, active account and absolute
maximum 12-hour expiry are validated before installation. A generation captured
before login prevents an in-flight response from resurrecting logout or replacing
a newer login. A rejected stale result is revoked centrally where reachable.

Logout persists a new generation and clears active authorization before deleting
Keychain entries. A Keychain deletion failure retains a bounded owned cleanup
journal for retry, while authorization remains absent. Successful cleanup removes
`snapshot.enc`, all owned active entries and the active identity, retaining the
installation UUID. Other state roots/deployments and unrelated Keychain entries
are untouched. Local cleanup succeeds independently of control-plane availability;
online revocation is best effort and an unreachable central session retains its
original absolute deadline.

Known central HTTP 401 immediately invalidates the matching local generation.
An HTTP 403 permission failure, redirect, malformed response, server failure or
network outage does not masquerade as session rejection. An expired/inactive
stored session also clears locally. Later consumers must independently enforce
the snapshot lease during outages.

## Local browser protocol

The plain same-origin GET `/` creates a random 256-bit, one-hour, HttpOnly,
SameSite=Strict cookie named `dhivon_mcp_gateway_local_<port>`. No capability appears in the
URL, HTML or browser storage. Cookie names isolate concurrent loopback servers
on different ports; the server keeps the cookie value and a separate random CSRF
challenge in a bounded in-memory table (maximum 128 live sessions). The page
contains no native token or password. Native and browser sessions are separate.

| Endpoint                | Authorization and result                                                                 |
| ----------------------- | ---------------------------------------------------------------------------------------- |
| GET `/`                 | Exact Host and permitted navigation context; establishes local cookie                    |
| GET `/api/context`      | Local cookie; redacted status, configured origin and CSRF challenge                      |
| GET `/api/status`       | Local cookie; redacted status only                                                       |
| POST `/api/auth/login`  | Cookie, exact Origin and CSRF; JSON email/password; redacted status                      |
| POST `/api/auth/logout` | Cookie, exact Origin and CSRF; local cleanup and best-effort revocation; clears cookie   |
| GET `/oauth/callback`   | Separate callback hook; currently rejects every request because no OAuth flow is pending |

Every route checks the exact `127.0.0.1:<port>` Host, rejecting alternatives such
as `localhost`. An Origin header must match exactly. Management requests reject
cross-site and same-site Fetch metadata; same-origin requests and direct navigation
are permitted. Every mutation requires an explicit exact Origin. Every `/api/`
request also requires the local cookie, with CSRF required for mutations. Login
requires `application/json` and a bounded 4 KiB body; JSON parser failures return
fixed errors without input fragments. Responses are no-store, nosniff and
no-referrer, with same-origin CSP and no framing. CORS is not enabled.

The OAuth callback is not a management authentication shortcut. Only its GET
navigation is exempt from the management Fetch-metadata restriction, retaining
the exact Host and any supplied Origin check. Phase 5 must validate a pending
flow's state, issuer, callback and PKCE before implementing exchange. Arbitrary
callback query values currently return a fixed error and never appear in logs.

Keep the CSRF challenge in frontend memory only. Send mutations with same-origin
credentials and `X-CSRF-Token`. Never use localStorage/sessionStorage for cookies,
CSRF, passwords or tokens. Neither context, status nor login response exposes a
central bearer token. Native session data is never transparently proxied to the
browser. Future management bridges must return explicit metadata DTOs.

## Integration seams and actual verification

`SessionManager::request` loads the selected native session, sends only to fixed
`/api/` paths on its origin and invalidates on 401. It returns a native session
with generation plus the private response to native callers; success and revision
409 are preserved, other errors are masked. `central::bounded_body` provides a
2 MiB response limit. The snapshot group must validate a complete snapshot and
recheck generation inside `StateStore::locked` before atomically installing it;
only that durable installation may renew freshness. `save_credential` accepts only typed cache/upstream kinds and also checks
generation and journals ownership before any Keychain insertion. It does not
replace assignment, binding-version, snapshot-lease or upstream-origin checks.

`ManagementAssets` accepts a compiled index HTML and an exact embedded `/assets/`
lookup table. It never reads browser-supplied filesystem paths. Phase 7 can supply
its production React bundle while preserving this router's cookie/security layer.
Phase 3 management bridges should mount only explicitly typed authenticated
handlers; OAuth callback validation belongs to the distinct callback route.

The nine focused CLI tests passed on actual macOS **27.0.1 (26A434), arm64**, with
Rust 1.95.0. They include actual default Keychain round-trip/replacement/deletion,
account and deployment isolation, logout cleanup, deterministic cleanup failure
and retry, in-flight login/logout, rejection versus outage, permission/symlink
boundaries, Host/Origin/Fetch/cookie/CSRF negatives, callback separation, malformed
input and captured log/browser-read/status redaction. A real CLI subprocess served
the development page and context over an ephemeral loopback TCP port with empty
stdout; a cold MCP subprocess also had empty stdout. Stand-in central HTTP
fixtures were used for focused native behavior; no PostgreSQL database was touched.

CLI rustfmt, all focused tests, Clippy with warnings denied and build passed.
Default-browser launch was implemented but not exercised by these tests
(`--no-open` was used). No older macOS version, Linux endpoint, actual React UI,
configuration lease, aggregation, install flow or completed OAuth flow is claimed.
