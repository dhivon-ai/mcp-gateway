# Technology Stack

## Product identity

Dhivon MCP Gateway is a Dhivon AI product. Project source is Apache-2.0 with
Copyright 2026 Dhivon AI; original third-party licenses remain intact. Current
Cargo/npm identities, delivery names and native namespaces use Dhivon. Setup and
archived provenance retain their original identifiers.

## Delivered workspace

Version 0.1.0 uses Rust 1.95.0 (edition 2024, declared rust-version 1.95), pinned in
`rust-toolchain.toml`, with Cargo.lock and workspace `unsafe_code = forbid`.

| Package | Responsibility |
| --- | --- |
| `crates/dhivon-mcp-gateway-core` | Public Serde/ts-rs contracts, pure policy/lease validation, shared authenticated encryption. |
| `crates/dhivon-mcp-gateway-server` | `dhivon-mcp-gateway-server`: central Axum APIs/admin assets, SQLx migrations, PostgreSQL identity/settings/vault. |
| `crates/dhivon-mcp-gateway-cli` | `dhivon-mcp-gateway`: protected loopback UI, Mac Keychain/cache/sync, installation, OAuth and STDIO aggregation. |
| `crates/dhivon-mcp-gateway-fixtures` | Controlled SDK STDIO/HTTP/OAuth services and interoperability tests. |
| `apps/web` | `@dhivon/mcp-gateway-web`: central/local React applications sharing strict TypeScript/shadcn primitives and generated wire types. |
| `apps/site` | `@dhivon/mcp-gateway-site`: independent static product/docs builder and preview; canonical repository Markdown through Marked 18.1.2. |
| `assets/brand` | Approved shared Soft Gate SVGs/variants, local Manrope TTF, semantic tokens and original full OFL. |

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
icons and locally bundled Manrope; `components.json` and pnpm-lock.yaml define
the component setup. Navy #102C4C, emerald #00AD78 and white form the approved brand palette;
semantic tokens provide readable actions/status. Vitest/Testing Library/jsdom,
ESLint/TypeScript-aware rules and Prettier provide checks. Application/configuration
source is strict TypeScript. The static site runs
Node TypeScript independently of React/Vite and uses its own build/type checks.
Verification/recovery scripts use Python3 stdlib fcntl for a kernel-held lock on
the validated Mac; Python is not a distributed client/service dependency.

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

`deploy/Dockerfile` builds dhivon-mcp-gateway-server:0.1.0 and embeds web assets
into a Linux arm64 Rust service on a nonroot Debian bookworm runtime. Compose supplies private PostgreSQL, external keys,
health and loopback/private HTTP behind HTTPS termination guidance. The PG image
is version/digest pinned; build base tags may change, so record actual image IDs.
`scripts/package-macos.sh` produces an unsigned arm64 embedded archive/checksum
on the validated Mac with deployment target 27.0.1. Product-defined build keys
are DHIVON_MCP_GATEWAY_BUILD_CACHE and DHIVON_MCP_GATEWAY_WEB_ASSETS;
CARGO_TARGET_DIR remains Cargo's standard override. Both outputs carry readable
Apache LICENSE/Dhivon AI NOTICE/full Manrope OFL. Static output publishes them too.
Release `strip=false` preserves
loadable proc-macro dylibs with the pinned toolchain on that Mac.

Local evidence: 83 workspace Rust tests, 11 explicit real-PG cases, 44 frontend tests,
quality/contracts/build checks, compiled browser flows and extracted all-auth MCP
client suite pass. Actual container→Mac→Node STDIO, Keychain cleanup, restart,
dump/fresh restore and wrong/matching-key recovery pass. CI configuration exists
for Linux/Mac/web/database/deployment; no hosted run is claimed. See `docs/testing.md`,
`docs/dhivon-release.md` and `docs/compatibility.md` for current commands/evidence
and validated limits; `docs/release.md` remains original MVP provenance.

The final static build/crawl covers 34 pages. Exact notice checks read the extracted
archive, actual runtime image and emitted site bytes. Docker HTTPS/nginx with a
controlled MCP/issuer and actual extracted Mac callback was verified in Safari;
public DNS/hosting and named external-provider registrations remain unverified.
The expiry/rejection resolver fix retains scoped fences and central refresh CAS.
The final cleanup/ownership suite passes 25/25, covering real Docker/native signal
cleanup plus registration uncertainty, exclusive lock/crash retry and journal
freshness. See scripts/rehearsal-lifecycle.ts, rehearsal-lock.ts, their process
tests and docs/https-oauth-rehearsal.md. These focused runs reuse earlier unaffected
Rust/UI/HTTPS evidence only while relevant source/artifact fingerprints match.

Current Mac archive SHA-256:
`e66ae212267e01f7af952db662c6da33edc284e8529e07bc40cb142134092799`.
Current extracted executable SHA-256:
`f36d5e229cf3fd2bf97ea780340b0304a605f6b848044755754ecee64c83d5d7`.
Current service image ID:
`sha256:7c6b562735589c7dbed0552c26a7d3f3fda20e6eb5fa773589c7364ae7ae0b07`.
Metadata archival/refresh does not rebuild or change these historical artifacts.
