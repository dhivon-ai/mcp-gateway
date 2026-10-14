# Wire and policy contracts

`gateway-core` owns the Rust/JSON contracts. Public fields are camelCase; tagged
transport/auth/grant modes use snake_case kind tags. Secrets have no `Debug`
implementation and endpoint credential types are excluded from frontend exports.
Browser secret APIs accept writes and return metadata only.

Generate the frontend bindings with:

```sh
cargo run -p gateway-core --example export_contracts -- apps/web/src/generated
```

During execution use the runtime-owned Cargo build cache. Committed generated
TypeScript files are checked against fresh generation in CI. Timestamps are Unix
seconds represented as JSON numbers, restricted to the year-9999 boundary; public
revisions are u32. No JSON bigint encoding is used.

Central snapshots contain only effectively assigned MCP definitions, plus their
server-authoritative enablement and credential references. User-disabled MCPs
remain visible but are never dispatchable; administrator-inactive entries are
omitted. Last-grant removal clears
enablement centrally; re-granting does not opt the user back in. Tool policy checks
use original upstream names; agent-facing namespace routing is a transport concern.

Freshness requires schema/identity integrity, an active principal, and a positive
lease no longer than 3,600 seconds that does not outlive the central session. At
`now >= expiresAt` it is invalid. A backward clock or malformed cache is rejected.
The endpoint must additionally enforce monotonic elapsed time across sleep and
preserve evidence through restart. Reading or refreshing a credential cannot
alter a configuration timestamp; only a validated server snapshot can do that.

OAuth records are scoped to user, MCP and native-client registration/installation;
the initial provider profile uses a pre-registered public client ID and PKCE.
Tokens are encrypted centrally, transported through endpoint-only credential
APIs, and cached in Keychain. Central login sessions are never upstream tokens.
