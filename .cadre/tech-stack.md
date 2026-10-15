# Technology Stack

## Delivered workspace

Version 0.1.0 uses Rust 1.95.0 (edition 2024, declared rust-version 1.95), pinned in
`rust-toolchain.toml`, with Cargo.lock and workspace `unsafe_code = forbid`.

| Package | Responsibility |
| --- | --- |
| `crates/gateway-core` | Public Serde/ts-rs contracts, pure policy/lease validation, shared authenticated encryption. |
| `crates/gateway-server` | `mcp-gateway-server`: central Axum APIs/admin assets, SQLx migrations, PostgreSQL identity/settings/vault. |
| `crates/gateway-cli` | `mcp-gateway`: protected loopback UI, Mac Keychain/cache/sync, installation, OAuth and STDIO aggregation. |
| `crates/gateway-fixtures` | Controlled SDK STDIO/HTTP/OAuth services and interoperability tests. |
| `apps/web` | Separate central/local React applications sharing strict TypeScript/shadcn primitives and generated wire types. |

## Pinned libraries and build tools

Cargo manifests/lockfile govern exact versions and features. Current dependencies
include Tokio 1.53.3, Axum 0.8.9, SQLx 0.9.0 (PostgreSQL/rustls/migrate), official
rmcp=3.5.1 (client/server/STDIO/child/StreamableHTTP/auth), reqwest 0.13.5 with rustls,
clap 4.6.7, Serde/tracing, ts-rs 12.0.1, Argon2 0.6.0 and AES-GCM 0.11.1.
OAuth uses oauth2=5.0.0. Mac adapters use security-framework=3.7.0, nix=0.31.3,
rustix=1.1.5, sysctl=0.7.1 and libproc=0.14.11. Preserve explicit bounds, exact
destinations, no-replay SDK settings and safe nonblocking descriptor adapters.

Node 24.19.0 LTS (`.node-version`) and pnpm 11.4.0 (`package.json`) build the website;
end users need neither. React 19.3.0, TypeScript 6.0.3, Vite 8.3.4 and Tailwind 4.3.3
are pinned. shadcn/Radix uses the radix-nova style, semantic CSS variables, Lucide
icons and Geist font; `components.json` and pnpm-lock.yaml define the component
setup. Vitest/Testing Library/jsdom, ESLint/TypeScript-aware rules and Prettier
provide checks. Application/configuration source is TypeScript, not JavaScript.

## Data, security and protocol

PostgreSQL 18.6 stores ordinary config, stable IDs, global revisions, per-user
settings and write-only encrypted credentials. Immutable forward SQLx migrations
include explicit safe refusal of populated pre-email identity schemas. Argon2id,
opaque 12h sessions, roles, CSRF/Host/Origin checks and canonical email implement auth.
Server vault keys are external private files, separately backed up from PostgreSQL.

Native sessions/upstream tokens/cache keys use Mac Keychain. Private user-scoped
files hold authenticated encrypted snapshots and ordinary bounded metadata,
atomic publication/cleanup journals and installation receipts. Darwin continuous
clock/boot identity and protected watermarks bound the one-hour lease across
same-boot restart and conservatively reject uncertainty. No offline mutation queue.

Agent transport is STDIO over checked nonblocking FIFO or connected Unix stream
pairs. Upstreams use local child STDIO or Streamable HTTP with none/bearer/public
OAuth. Supported MCP profile is tools-only 2025-11-25; typed/raw SDK seams enforce
capability and frame/pagination/resource limits. No legacy SSE/non-tool proxy.

## Delivery and evidence

`deploy/Dockerfile` embeds web assets into a Linux arm64 Rust service on a nonroot
Debian bookworm runtime. Compose supplies private PostgreSQL, external keys,
health and loopback/private HTTP behind HTTPS termination guidance. The PG image
is version/digest pinned; build base tags may change, so record actual image IDs.
`scripts/package-macos.sh` produces an unsigned arm64 embedded archive/checksum
on the validated Mac with deployment target 27.0.1. Release `strip=false` preserves
loadable proc-macro dylibs with the pinned toolchain on that Mac.

Local evidence: 83 workspace Rust tests, 11 explicit real-PG cases, 44 frontend tests,
quality/contracts/build checks, compiled browser flows and extracted all-auth MCP
client suite pass. Actual container→Mac→Node STDIO, Keychain cleanup, restart,
dump/fresh restore and wrong/matching-key recovery pass. CI configuration exists
for Linux/Mac/web/database/deployment; no hosted run is claimed. See `docs/testing.md`,
`docs/release.md` and `docs/compatibility.md` for commands and validated limits.
