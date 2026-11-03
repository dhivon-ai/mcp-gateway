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
Native record key derivation uses hkdf=0.13.0 (HKDF-SHA-256). OAuth uses oauth2=5.0.0. Mac adapters use security-framework=3.7.0, nix=0.31.3,
rustix=1.1.5, sysctl=0.7.1 and libproc=0.14.11. Preserve explicit bounds, exact
destinations, no-replay SDK settings and safe nonblocking descriptor adapters.

Node 24.19.0 LTS (`.node-version`) and pnpm 11.4.0 (`package.json`) build the website;
end users need neither. React 19.3.0, TypeScript 6.0.3, Vite 8.3.4 and Tailwind 4.3.3
are pinned. shadcn/Radix uses the radix-nova style, semantic CSS variables, Lucide
icons and locally bundled Manrope; `components.json` and pnpm-lock.yaml define
the component setup. Navy #102C4C, emerald #00AD78 and white form the approved brand palette;
semantic tokens provide readable actions/status. Vitest/Testing Library/jsdom,
ESLint/TypeScript-aware rules and Prettier provide checks. Protected web application and configuration source is strict TypeScript. The static site runs
Node TypeScript independently of React/Vite and uses its own build/type checks.
First-time operator configuration uses Bash and Python 3.8+ standard library on the host.
Verification/recovery scripts also use Python3 stdlib fcntl for a kernel-held lock on
the validated Mac; Python is not a distributed client/service dependency.

## Data, security and protocol

PostgreSQL 18.6 stores ordinary config, stable IDs, global revisions, per-user
settings and write-only encrypted credentials. Immutable forward SQLx migrations
include explicit safe refusal of populated pre-email identity schemas. Argon2id,
opaque 12h sessions, roles, CSRF/Host/Origin checks and canonical email implement auth.
Server vault keys are external private files, separately backed up from PostgreSQL.

One 32-byte Mac Keychain master item selects the encrypted endpoint index.
Purpose-separated keys protect records on disk, including sessions, credentials,
refusals and install receipts. Account records bind the authenticated origin and
identity. The encrypted policy snapshot retains its own lease checks. See
`docs/endpoint-identity.md` for ownership, key transitions and recovery limits. Darwin continuous
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

The root Compose project builds the server, provisions the first administrator and
keeps PostgreSQL private. See `docs/deployment.md` for DNS/TLS and setup, and
`docs/operations.md` for private backups and restore. The default Mac packaging
command reuses a private Cargo target and prints a distinct archive and checksum.
`docs/checksums.md` identifies the tested archive with offline guides (SHA-256
`8d40699ffb6480488edad6460fd57c2d566112596371145f81ffe7db1aac4383`).

The existing Cloudflare Pages project `dhivon-mcp-gateway` builds GitHub main
with `pnpm site:build` and publishes `apps/site/dist`. `apps/site/publication.ts`
selects content. The build emits versioned CSS, redirects and a route manifest;
`functions/docs/[[path]].js` handles unpublished docs before Pages can return a
retained asset. Public documentation hosting is separate from the team's service.

For evidence use `docs/consumer-setup-verification.md`, the two November 3
archived tracks and the seven-pattern catalog. Earlier service image, executable,
browser and controlled-provider results belong to the specific runs in
`docs/dhivon-release.md` and `docs/https-oauth-rehearsal.md`. See
`docs/compatibility.md` for supported environments.
