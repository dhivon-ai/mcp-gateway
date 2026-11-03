# Product

## Project context

Dhivon MCP Gateway is a product of Dhivon AI, licensed under Apache-2.0 with
Copyright 2026 Dhivon AI. The bundled Manrope font retains its original SIL OFL
copyright and full license. The repository began as a greenfield project on
2026-10-14; that setup classification remains historical provenance.

Four tracks are archived: `mac-mvp`, `dhivon-brand-site-docs`,
`single-key-endpoint-store` and `consumer-website`. The public guides are live at
https://mcp.gateway.dhivon.ai. Teams host their own gateway. The verified 0.1.0
Apple Silicon CLI archive is unsigned and distributed through a trusted channel
or built from source. No product track is active.

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
  One Keychain master item per initialized installation selects the encrypted
  local index. Sessions, upstream tokens and configuration are protected in private
  files. Logout fences the login and retains the installation master. Authenticated
  repair preserves refusal, expiry and lease evidence. Independent online sign-in
  can rebuild an unreadable identity. Ordinary central configuration is plaintext.
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
- The guide directory separates joining a team from hosting a gateway. Only
  explicitly selected guides and assets are published. Internal tickets belong in
  Cadre. Supported old links redirect; unpublished doc paths return 404.
- After preparing DNS, TLS and private administrator inputs, operators run
  `bash scripts/configure-gateway.sh` once, then `docker compose up -d --build --wait`
  from the root. The stack provisions private PostgreSQL, vault keys and the first
  admin. Mac developers can build the CLI and its embedded console with
  `bash scripts/package-macos.sh`. The extracted archive includes offline guides.

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
denial and no call replay. The hosted documentation site is public. The control-plane test used a controlled provider.

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
Keychain are trusted. See `docs/compatibility.md`, `docs/checksums.md` and `docs/consumer-setup-verification.md`;
`docs/release.md` retains original MVP evidence.

## Domain language

- Control plane: central identity, policy, settings and encrypted secret owner.
- Local gateway: endpoint installation/auth/process owner and agent STDIO server.
- Assignment: additive administrator grant; never installation or enablement.
- Enablement: centrally confirmed per-user opt-in, distinct from native readiness.
- Effective tool: original upstream tool passing every current authorization gate.
- Policy lease: protected bounded snapshot, never a perpetual offline identity.

## Recorded verification and current evidence

The endpoint review at `aab0881` covers a real Mac Keychain, concurrent first
fetch and refusal, cold restart, damaged index/marker and authenticated recovery.
The consumer guide review at `6ea16ed` covers the extracted Mac journey and the
root Compose stack, including restart, HTTPS sign-in, and credential recovery from
a restored database with its matching keys. The checksums in `docs/checksums.md`
identify the exact tested archives. `docs/dhivon-release.md` remains the record of
earlier artifacts and tests. Public Pages routing and desktop/mobile stylesheet
delivery were checked after those reviews at `ad1daed` and `4ce8326`.

## Future decisions

New tracks must explicitly scope deployment hardening/scale, real providers/agents,
extra platforms/browsers, signed distribution, SAML or SaaS. Preserve the delivered
security and synchronization contracts unless an approved revision changes them.
