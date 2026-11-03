# Native endpoint identity and local browser boundary

This document retains the Phase 3 identity-slice verification below. The complete
MVP now serves the local React UI, synchronizes encrypted snapshots and runs MCP
aggregation; see [local UI](local-ui.md), [cache](endpoint-cache.md) and
[agent configuration](agent-configuration.md) for current behavior. Sign-in uses
email/password only; Name is display-only. Current status returns separate
`email` and `name` metadata. Older username accounts/API/cache are unsupported;
see [identity deployment compatibility](identity.md#fresh-identity-setup).

The local Rust process owns the browser session, native login and encrypted endpoint storage. An authenticated session alone does not authorize tools or establish a configuration lease.

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

`mcp` uses the configured origin from the authenticated store. It refuses startup without usable policy and never prompts on stdin. `status` reports identity and expiry without returning tokens, passwords, cookies or CSRF values.

## Native state and Keychain ownership

The gateway keeps **one generic-password item per initialized installation** in the default Mac Keychain. In production the service is `com.dhivon.mcp-gateway.endpoint.v1.master.v2`. Its secret is exactly 32 random bytes; its account is an opaque random UUID. The account name and master key change together when a new encrypted index commits. The account name never contains a user ID or server URL.

The private state directory is `~/Library/Application Support/com.dhivon.mcp-gateway`. Its `endpoint-v2/` directory contains an authenticated index and separate encrypted records. These hold the selected origin, installation and login generation, session, cache key and checkpoints, upstream credentials, pending saves, rejection markers, cleanup lists and install receipts. The policy payload remains in `snapshot.enc`. Directories use 0700 and files use 0600. Lock files, opaque record names, format headers and the v1 fence `identity.json` contain no credentials or user configuration. The gateway never persists the login password.

The index opens before an identity is known. A random seed and fresh record UUID derive separate AES-256-GCM keys with HKDF-SHA-256; the purpose is also authenticated. Account records include the origin, deployment and verified user in their scope and are pinned to the current login generation. Credential consumers additionally verify the installation, MCP, runtime binding and version. The user ID is a binding, not an encryption secret.

Under the process lock the gateway writes and syncs new ciphertext and the encrypted index, then updates the one Keychain item to select that index. Earlier files cannot grant access after logout, credential rejection or a newer clock watermark, even when a whole old directory is restored. This assumes the OS and Keychain remain trusted. It does not protect against restoration of **both** a prior Keychain and its matching files.

## Upgrade and recovery

Stop all old UI and agent processes and retain the creating executable before upgrading. Start the new CLI normally. It validates the old ownership list against the exact legacy service, verifies the encrypted copy, fences v1 readers and retires only known entries. Unknown entries or unavailable Keychain access stop the upgrade. Historical plaintext in external backups is unaffected.

If the creating executable is needed for Keychain access, first finish any pending OAuth save using that executable. Before cutover, run its `logout`, then open the new UI and sign in. If cutover already fenced the old executable and reports incomplete legacy cleanup, run the new executable's **`upgrade-handoff`**, then the original executable's **`logout`**, then restart the new version. This explicit handoff discards the staged login and unsaved OAuth responses before enabling a signed-out legacy cleanup journal. Reconnect affected providers; no provider exchange or install is repeated.

With a valid native login, a successful authenticated sync can replace damaged policy data; a fresh credential response can repair its damaged local token while preserving version, expiry and rejection evidence. A locked Keychain must be unlocked. If the master or authenticated identity is lost, run `dhivon-mcp-gateway ui --server <origin>` and sign in online. This also works when both the local format marker and the encrypted index are damaged. A failed sign-in preserves the existing master key. A successful sign-in rebuilds the store, and restoring the old directory cannot bring back the previous login. A new installation identity cannot inherit old install receipts or installation-bound OAuth grants. Verify local side effects yourself and reconnect as necessary. Missing or damaged local-only evidence cannot be fetched from the server.

Logout fences the current generation, removes its credentials and policy, and attempts central revocation. Offline cleanup still works. A signed-out installation keeps one master key to protect installation receipts and configuration. Known 401s invalidate only the matching generation.

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

## Historical identity-slice verification

The following notes record the original identity slice. For the current encrypted endpoint integration and packaged tests, see [testing](testing.md#single-key-endpoint-storage).

`SessionManager::request` loads the selected native session, sends only to fixed
`/api/` paths on its origin and invalidates on 401. It returns a native session
with generation plus the private response to native callers; success and revision
409 are preserved, other errors are masked. `central::bounded_body` provides a
2 MiB response limit. The snapshot group must validate a complete snapshot and
recheck generation inside `StateStore::locked` before atomically installing it;
only that durable installation may renew freshness. `save_credential` accepts only typed cache/upstream kinds and also checks
generation and journals ownership before credential publication. It does not
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
