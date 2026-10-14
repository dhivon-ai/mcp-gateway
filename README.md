# Enterprise MCP Gateway

A self-hosted control plane and Apple Silicon macOS gateway for coding agents.
Administrators manage a default-deny MCP catalog; users choose enabled MCPs through
a locally served website. The coding agent connects to one STDIO gateway.

## Development

Rust is pinned in `rust-toolchain.toml`, Node LTS in `.node-version`, and pnpm in
`package.json`. The control plane and Mac CLI are separate Rust binaries; policy
and API contracts belong in `gateway-core`. The React/TypeScript website is in
`apps/web` and uses Vite, Tailwind and shadcn/ui. Both central and local sites use
the compiled frontend with role-specific routing.

```sh
pnpm install --frozen-lockfile
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
pnpm typecheck
pnpm lint
pnpm test
pnpm format:check
pnpm build
cargo run -p gateway-server -- --help
cargo run -p gateway-cli -- --help
```

During Cadre execution, set `CARGO_TARGET_DIR` and `GATEWAY_BUILD_CACHE` to the
owned build cache returned by the runtime; frontend builds honor that location.
`node_modules` contains dependencies only. No secrets belong in repository files.

## Boundaries

Per-user enablement is authoritative on the server. The Mac configuration file is
encrypted with a Keychain-held key and expires at most one hour after successful
authenticated synchronization. Failed requests cannot renew it. Offline changes
are rejected. Central configuration is ordinary PostgreSQL data; central user
secrets and OAuth tokens are encrypted separately with an operator-held key.

The current approved delivery specification and plan are in
`.cadre/tracks/mac-mvp/`. Deployment and user runbooks are added with delivery.

See [wire/policy contracts](docs/contracts.md) and [fixture/testing setup](docs/testing.md)
for isolated PostgreSQL, MCP/OAuth fixtures, and generated-contract verification.
