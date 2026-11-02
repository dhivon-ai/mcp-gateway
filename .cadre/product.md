# Product

## Project context

Dhivon MCP Gateway is a product of Dhivon AI, licensed under Apache-2.0 with
Copyright 2026 Dhivon AI. The bundled Manrope font retains its original SIL OFL
copyright and full license. The repository began as a greenfield project on
2026-10-14; that setup classification remains historical provenance.

Both `mac-mvp` and `dhivon-brand-site-docs` are implemented, cleanly reviewed,
approved and archived under `.cadre/archive/`. Version 0.1.0 has locally verified
Mac/service artifacts, branded administrator and developer interfaces, a standalone
public product/documentation site and detailed role-based guides. The archive is
unsigned and unpublished. No active product track or approved follow-on exists.

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
- The Mac serves protected loopback management via `dhivon-mcp-gateway ui --server <url>`.
  `dhivon-mcp-gateway mcp` is the agent's sole STDIO entry; `status` is redacted and
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

## Delivered identity, website and documentation

- Fresh installations use `dhivon-mcp-gateway` (Mac),
  `dhivon-mcp-gateway-server` (service/image), Cargo packages under
  `crates/dhivon-mcp-gateway-*`, `@dhivon/mcp-gateway-web` and
  `@dhivon/mcp-gateway-site`. Product-defined environment variables use
  `DHIVON_MCP_GATEWAY_*`; native service/state namespaces use
  `com.dhivon.mcp-gateway`. Standard external variables and protocol identifiers
  keep their defined names. No old-name aliases, compatibility reads, data
  conversion or deletion of unrelated legacy resources are provided.
- The approved Soft Gate marks, navy/emerald/white palette and locally bundled
  Manrope are shared by the central admin, local Mac and public site. Navy and
  semantic colors carry readable actions/status; emerald is a brand accent.
- The independent static `apps/site` site explains the problem of configuring
  upstreams per agent, central policy ownership, explicit developer opt-in and
  local/remote execution. Its grant→enable→call→disable example links role-specific
  quickstart, administration, deployment/recovery and agent setup routes.
- Canonical `docs/*.md` guides render into public pages with deep links and local
  assets, without requiring product services or credentials. Documentation covers
  configuration, transports/auth, security/leases, troubleshooting, compatibility,
  contributing/testing, licensing and an undated roadmap separating delivered
  evidence, approved next work and candidate ideas.
- Project LICENSE/NOTICE and unchanged full Manrope OFL accompany the extracted
  Mac archive, runtime image and static output; publication checks compare bytes
  against source and expose readable notice paths.

## Delivered OAuth and rehearsal recovery follow-ups

The intended deployment is an HTTPS control plane with a local client on the
user's Mac. OAuth redirects to that client's actual
`http://127.0.0.1:{port}/oauth/callback`. Hosted service origin and native callback
serve different roles; a provider's public-client/loopback-port registration must
match the supported profile. Production TLS/hostname verification remains enabled.

The owner-selected Docker HTTPS/controlled-provider simulation verified extracted
Mac sign-in/sync, Safari redirect to the native listener, S256 PKCE/state/issuer
checks, protected persistence, explicit enablement, tool use, refresh and disconnect
with subsequent denial. The resolver refreshes a validated online expired record
through the existing lease/version CAS before attempting to publish its rejected
old binding. It preserves the rejection fence, generation/policy gates, offline
denial and no call replay. Public hosting and named external providers are unverified.

Rehearsal supervision and recovery persist generated private ownership and immutable
creating binaries/configuration. Child handles survive failed registration; unknown
intent fails closed. A permanent-inode kernel lock serializes recovery before journal
reads through final evidence writes. Writers stop before creating-binary logout;
verified native cleanup precedes exact stack removal. Failed attempts and unresolved
ownership remain truthful. Python3/fcntl is required by these verification/recovery
scripts on the validated Mac; distributed product binaries do not require Python.
Temporary certificate trust remains separately authorized operator work; the tested
CA trust/certificate was removed and the leaf subsequently failed validation.

## Compatibility and delivery boundaries

Apple Silicon only; archive target/minimum validated macOS 27.0.1 (26A434).
Self-hosted Linux arm64 service/PostgreSQL 18.6, actual Mac Keychain/process behavior,
in-app Chromium desktop/390px, official rmcp 3.5.1/MCP 2025-11-25 tools and controlled
OAuth fixtures are verified. Safari additionally completed the controlled HTTPS/native
OAuth redirect; this does not establish general Safari UI or provider compatibility.
Runbooks, Compose/health/TLS guidance, embedded arm64
archive/checksum, restart, fresh backup/restore and independent-key recovery exist.

The email identity change explicitly requires fresh prerelease databases; a
populated older username schema is refused without deleting data. No legacy
account/API/cache compatibility is promised. Names never authenticate.

SAML, SaaS multi-tenancy, Intel/Windows/Linux endpoints, signing/notarization/public
publication, legacy HTTP+SSE and non-tool MCP features remain later work. Do not
claim arbitrary provider/commercial-agent/browser/older-Mac compatibility,
hosted CI execution, actual machine sleep/reboot, HA, compliance certification or
enterprise-scale performance. Cancellation cannot undo side effects; the OS and
Keychain are trusted. See `docs/compatibility.md` and current `docs/dhivon-release.md`;
`docs/release.md` retains original MVP evidence.

## Domain language

- Control plane: central identity, policy, settings and encrypted secret owner.
- Local gateway: endpoint installation/auth/process owner and agent STDIO server.
- Assignment: additive administrator grant; never installation or enablement.
- Enablement: centrally confirmed per-user opt-in, distinct from native readiness.
- Effective tool: original upstream tool passing every current authorization gate.
- Policy lease: protected bounded snapshot, never a perpetual offline identity.

## Recorded verification and current evidence

`docs/dhivon-release.md` records the dated current archive/executable/image
identities and the successive rebrand, notice, HTTPS/OAuth and cleanup checks.
Evidence includes 83 Rust workspace tests, 11 explicitly run isolated PostgreSQL
cases, 44 frontend tests, compiled desktop/390px flows, the final 34-page link/anchor
crawl, extracted-client/all-auth and deployment/restart/fresh-restore/key-recovery
checks. The final review reran 25/25 cleanup/ownership process tests in 161.11s and
strict script/notice checks against unchanged product identities. These counts
belong to the documented runs; a metadata refresh adds no application test run.

The current archive SHA-256 is
`e66ae212267e01f7af952db662c6da33edc284e8529e07bc40cb142134092799`;
service image ID is
`sha256:7c6b562735589c7dbed0552c26a7d3f3fda20e6eb5fa773589c7364ae7ae0b07`.
The clean review is 7249c26 and archive is 6e75dfd. Original reports and archived
learning retain the historical names and artifact hashes they actually tested.

## Future decisions

New tracks must explicitly scope deployment hardening/scale, real providers/agents,
extra platforms/browsers, signed distribution, SAML or SaaS. Preserve the delivered
security and synchronization contracts unless an approved revision changes them.
