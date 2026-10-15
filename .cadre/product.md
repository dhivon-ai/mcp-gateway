# Product

## Project context

MCP Gateway began as a greenfield repository on 2026-10-14. The self-hosted MVP
`mac-mvp` is now implemented, cleanly reviewed, approved and archived under
`.cadre/archive/mac-mvp/`. This context describes the delivered baseline, not a
new feature commitment. Version 0.1.0 is locally built and unpublished.

## Users and purpose

Enterprise administrators manage which MCP servers and tools coding agents can
use. Developers sign in through a React/TypeScript/shadcn website served locally
by a Rust CLI and explicitly opt in to assigned MCPs. Coding agents use one local
STDIO gateway, aggregating permitted local STDIO and remote Streamable HTTP tools.
One enterprise runs each self-hosted control-plane deployment.

## Delivered behavior

- Administrators bootstrap explicitly, then manage users, groups/membership, MCP
  definitions, additive user/group/all-user assignments and original-name tool
  allowlists/denylists. New catalog definitions grant and enable nothing. Removing
  the last grant clears enablement; reassignment requires another explicit opt-in.
- Name is display-only. Unique canonical email/password is the sole login method
  in both websites and APIs. User creation requires name/email/password; profile
  edits preserve stable user IDs/ownership and revoke affected sessions. Passwords
  remain write-only and Argon2id-hashed; sessions are revocable and expire by 12h.
- Enablement and bearer-secret bindings are authoritative per user on the server.
  The endpoint owns local installation and process/auth availability. Changes are
  online, authenticated and revision-checked; conflicts require explicit reload.
- The Mac serves protected loopback management via `mcp-gateway ui --server <url>`.
  `mcp-gateway mcp` is the agent's sole STDIO entry; `status` is redacted and
  `logout` clears scoped local state, with online revocation when reachable.
- Each running process refreshes at startup/every 60s, with a 10s request bound.
  Only a complete validated, durably installed snapshot renews the lease. The
  authenticated encrypted local file is usable for at most 3,600s after validation,
  further bounded by session/token lifetime. Exact expiry fails closed. Failure,
  token refresh, restart or file copying cannot renew it; uncertainty requires sync.
- Offline configuration, secret, installation and new OAuth changes are rejected,
  without a queue. Logout remains available. Learned session rejection invalidates
  immediately; unavailable connectivity can use only the unexpired existing lease.
- Remote auth supports none, an owned per-user bearer secret, or the supported
  preregistered public-client OAuth profile. The central per-user key/value store
  encrypts secret values; users select their own stable secret IDs for bearer MCPs.
  OAuth credentials are separately managed/encrypted per user/MCP/installation.
  Native keys/tokens live in Keychain; ordinary central configuration is plaintext.
- Optional structured installation commands execute as the Mac user only after an
  explicit action. Failure never enables. A successful install with failed server
  confirmation can retry enable separately; a receipt is not current enablement.
- Discovery and dispatch enforce current assignment, opt-in, policy, identity,
  lease and auth. Stable aliases route original tools; only supported tool MCP
  capabilities are exposed. Calls are bounded and never automatically replayed.

## Compatibility and delivery boundaries

Apple Silicon only; archive target/minimum validated macOS 27.0.1 (26A434).
Self-hosted Linux arm64 service/PostgreSQL 18.6, actual Mac Keychain/process behavior,
in-app Chromium desktop/390px, official rmcp 3.5.1/MCP 2025-11-25 tools and controlled
OAuth fixtures are verified. Runbooks, Compose/health/TLS guidance, embedded arm64
archive/checksum, restart, fresh backup/restore and independent-key recovery exist.

The email identity change explicitly requires fresh prerelease databases; a
populated older username schema is refused without deleting data. No legacy
account/API/cache compatibility is promised. Names never authenticate.

SAML, SaaS multi-tenancy, Intel/Windows/Linux endpoints, signing/notarization/public
publication, legacy HTTP+SSE and non-tool MCP features remain later work. Do not
claim arbitrary provider/commercial-agent/Safari/Firefox/older-Mac compatibility,
hosted CI execution, actual machine sleep/reboot, HA, compliance certification or
enterprise-scale performance. Cancellation cannot undo side effects; the OS and
Keychain are trusted. See `docs/compatibility.md` and `docs/release.md`.

## Domain language

- Control plane: central identity, policy, settings and encrypted secret owner.
- Local gateway: endpoint installation/auth/process owner and agent STDIO server.
- Assignment: additive administrator grant; never installation or enablement.
- Enablement: centrally confirmed per-user opt-in, distinct from native readiness.
- Effective tool: original upstream tool passing every current authorization gate.
- Policy lease: protected bounded snapshot, never a perpetual offline identity.

## Future decisions

New tracks must explicitly scope deployment hardening/scale, real providers/agents,
extra platforms/browsers, signed distribution, SAML or SaaS. Preserve the delivered
security and synchronization contracts unless an approved revision changes them.
