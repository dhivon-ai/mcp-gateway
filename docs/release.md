# MVP release evidence

Release: 0.1.0, verified on 2026-10-15. This is a locally built, unpublished MVP.
The approved scope and acceptance mapping are in [acceptance](acceptance.md).

## Artifacts

- Apple Silicon archive: `$GATEWAY_BUILD_CACHE/releases/mcp-gateway-0.1.0-macos-arm64.tar.gz`.
- SHA-256: `afbda4e96c29cda7d3102d08bfb36d116cb725522ff7841ae2543dfe7fdf7bfe`.
- Check file: the archive name plus `.sha256`; `shasum -a 256 -c` passed.
- Local service image: `mcp-gateway-server:0.1.0`, Linux arm64;
  image ID `sha256:82303d778e91c982cbac86f0aaf9576e8b9e2fc83ec6709b570f919648cd2bd1`.

The archive contains only the arm64 executable, installation README and
compatibility document. Mach-O inspection reports deployment target 27.0.1 and
system framework/library dependencies only. Both websites are embedded at compile
time; no Node/runtime asset directory is required for end users. Developer ID
signing/notarization and publication remain outside this release.

## Verified journey

A fresh Compose PostgreSQL/service deployment applied migrations, explicitly
bootstrapped an admin and served compiled administrator assets. Two users/groups,
role denial, default-deny catalog assignments, per-user write-only secret creation,
bearer binding and ciphertext inspection passed. Restart preserved deployment,
revision, opt-ins and decryptable credentials.

On the actual Mac, the extracted archive's immutable creating-binary copy logged
in to this container through an isolated loopback proxy for the private HTTP hop.
It served compiled local assets, explicitly installed `/usr/bin/true`, enabled the
assigned fixture and used only `mcp` for the agent. A Node child_process client
initialized MCP2025-11-25, discovered the exact stable alias, invoked the local
STDIO echo and proved confirmed disable blocks subsequent dispatch. Open-input
SIGTERM and creating-binary logout completed; native active identity, credential
accounts and pending-cleanup metadata were empty before removing private state.

A real `pg_dump -Fc` restored into a fresh second Compose database with separately
held keys. Copied sessions were deleted; the old session was rejected, a fresh
login preserved deployment/revision/settings, and the bearer value resolved.
An independent wrong key returned503; restoring the matching key recovered.
Captured service logs contained none of the fixture password, bearer value or
session tokens. Generated containers, networks, volumes and private fixture state
were removed. No real account, provider, signing identity or production data was
used. HTTPS termination is documented; a publicly issued certificate test is not
claimed for this isolated internal-hop rehearsal.

The extracted archive also runs the official rmcp one-entry client suite: local
STDIO plus remote none/bearer/OAuth calls, same-Mac updates, cold valid-cache offline
restart, reconnect, protocol-only stdout, signal/owned-descendant cleanup, idle
input and blocked output. Compiled local index/module delivery is asserted.
The cumulative browser journeys, reference comparison, keyboard and responsive
checks remain in [admin UI](admin-ui.md) and [local UI](local-ui.md).

Final local quality results: 81 workspace Rust tests passed (the native child
helper and eight database cases are intentionally separate), all eight explicit
PostgreSQL tests passed, and 20 frontend behavioral tests passed. Formatting,
Clippy with warnings denied, generated contracts, strict TypeScript, lint and
production builds passed. The final extracted archive's official-client suite
passed in34.50s; its native executable is byte-identical to the successful
combined container/Node/Keychain rehearsal.

## Verification commands

```sh
cargo test --locked --workspace
TEST_DATABASE_URL=postgres://gateway_test:isolated-fixture-password@127.0.0.1:18432/gateway_test cargo test --locked -p gateway-server -- --ignored
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo build --locked --workspace
bash scripts/check-contracts.sh
pnpm test
pnpm lint
pnpm typecheck
pnpm format:check
pnpm build
bash scripts/package-macos.sh
node scripts/verify-deployment.ts
```

Use the owned build/target/assets variables in [testing](testing.md). For archive
verification set GATEWAY_TEST_CLI to the extracted absolute executable; for the
combined Mac/container rehearsal also set GATEWAY_FIXTURE_BIN. Database suites
require the isolated test PostgreSQL and explicitly execute the ignored cases.
The cache child marker is invoked by its actual Mac parent test. Hosted CI is
configured for service/database/frontend and arm64 Mac checks plus the isolated
Compose rehearsal; no hosted run is claimed.

## Definition of Done

- [x] Central password identity/roles, default deny, additive grants, original-name
  tool policy and revision-checked user enablement are implemented and verified.
- [x] Mac Keychain/encrypted cache, complete-only startup/60s refresh, exclusive
  one-hour ceiling and online-only mutations fail closed under tested failures.
- [x] Write-only encrypted per-user vault, owned bearer selection and supported
  public-client OAuth consent/rotation/recovery are integrated.
- [x] One STDIO gateway aggregates permitted local and remote tools with current
  dispatch gates, bounded protocol/lifecycle and native pipes/Unix socket pairs.
- [x] React/strict TypeScript/shadcn admin and local websites support required
  loading/errors/conflicts/offline/expiry, keyboard and responsive journeys.
- [x] Service container/Compose/health/external-key/TLS guidance, arm64 archive,
  checksums, restart/dump/restore and the deployment-to-agent rehearsal exist.
- [x] Operator, user, agent, provider, recovery and compatibility runbooks describe
  tested behavior and explicit limits; production credentials were not required.

This document records product delivery evidence. Cadre's final clean-review and
human completion approval are separate lifecycle gates. See [compatibility](compatibility.md)
for actual platforms/providers/agents and the unvalidated or deferred surfaces.
