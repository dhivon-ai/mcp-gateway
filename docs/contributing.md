# Contributing

Start from the [architecture](architecture.md) and the existing module responsible
for the behavior you want to change. The Cargo workspace separates shared
contracts/policy, the service, the Mac endpoint and controlled fixtures. The
React management frontend shares primitives across administrator and local
interfaces. Read the approved Cadre specification and plan for tracked work.

## Toolchains and build output

Use the exact versions committed in `rust-toolchain.toml`, `.node-version` and
`package.json` with the existing lockfiles. The current pins are Rust 1.95.0,
Node 24.19.0 and pnpm 11.4.0. Mac-specific native and Keychain tests run on the
validated Apple Silicon Mac. Check [compatibility](compatibility.md) before
claiming another platform.

Use Cadre's returned owned build directory during execution. For independent
local development, substitute an absolute private build directory below.

```sh
export DHIVON_MCP_GATEWAY_BUILD_CACHE=/absolute/owned/build
export CARGO_TARGET_DIR="$DHIVON_MCP_GATEWAY_BUILD_CACHE/cargo"
mkdir -p "$DHIVON_MCP_GATEWAY_BUILD_CACHE"
pnpm install --frozen-lockfile
pnpm build
export DHIVON_MCP_GATEWAY_WEB_ASSETS="$DHIVON_MCP_GATEWAY_BUILD_CACHE/web"
cargo build --locked --workspace
```

The embedded asset variable must point at the current frontend output before
compiling either Rust binary. Installed users need neither Node nor a development
server. Shared brand files live in `assets/brand`, including the Manrope OFL.

## Check a change

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
pnpm format:check
pnpm lint
pnpm typecheck
pnpm test
pnpm build
bash scripts/check-contracts.sh
```

Public wire contracts originate in Rust and generate TypeScript. Keep runtime
response decoding aligned with backend bounds. Use the existing script to verify
generated output. Match the existing semantic UI components and theme tokens,
and verify keyboard behavior and desktop/390px layout for rendered changes.

Ordinary Rust tests skip the explicitly ignored PostgreSQL cases. Follow
[isolated database setup](testing.md#isolated-postgresql) and execute them
separately. The harness accepts only the named loopback fixture database and
creates unique databases for each case. Keep private keys and test credentials
out of source, browser storage and diagnostic output.

## Preserve behavior across surfaces

Assignments, opt-in and native readiness are distinct. Keep the same effective
policy gates on discovery and dispatch, generation/revision checks on writes,
complete-only lease renewal, and credential owner/version boundaries. Retain
structured argument arrays and environment maps losslessly. Report confirmed
writes separately from a failed follow-up synchronization.

Read the relevant [security contract](security.md) and detailed guide before
changing authentication, process ownership or caching. Cover the actual regression
and failure path with focused verification. Use typed errors and fixed redacted
messages at public boundaries. Rust forbids unsafe code and the frontend uses
strict TypeScript.

## Release and documentation

For release-affecting changes, use the
[artifact and deployment checks](testing.md#release-checks). Rehearse the actual
extracted archive and service image, retain an unchanged creating binary through
native fixture logout and record exact identities and scope. Use fresh private
fixture resources and preserve recovery journals when cleanup fails.

Update the canonical guide for any changed command, configuration or visible
workflow. Check relative links and anchors from the rendered site. Record
verification dates, artifact checksum, image ID, platform, client and provider
coverage separately from proposed work. Keep the [roadmap](roadmap.md) aligned
with approved scope and delivered evidence. Historical release reports retain
the original artifact identities they tested.
