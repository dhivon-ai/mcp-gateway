# Verification and fixtures

Set `GATEWAY_BUILD_CACHE` to the runtime-owned build path and `CARGO_TARGET_DIR`
to its `cargo` child before running checks during Cadre execution.

Run `bash scripts/check-contracts.sh` to compare generated TypeScript against the
Rust source. Run workspace Rust tests/Clippy/format and frontend test/lint/type
checks before integration. CI targets Linux for the service and an Apple Silicon
macOS runner for endpoint verification. The Mac runner label follows the
[GitHub hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

## Isolated PostgreSQL

```sh
docker compose -p cadre-gateway-tests -f deploy/compose.test.yaml up -d --wait
```

The fixture database is `gateway_test`, on loopback port 18432 with user
`gateway_test` and password `isolated-fixture-password`. These are fixture values,
never production credentials. Database API integration tests use a separate
schema/database lifecycle and must not connect to production instances.
Remove only this owned fixture stack with its exact Compose project when done.

Run real database API tests explicitly (normal platform checks skip this external
prerequisite):

```sh
TEST_DATABASE_URL=postgres://gateway_test:isolated-fixture-password@127.0.0.1:18432/gateway_test cargo test -p gateway-server -- --ignored
```

Each test creates/migrates/drops a uniquely named owned database. The harness
rejects non-loopback DSNs and any base database other than `gateway_test`.

## MCP and OAuth fixtures

```sh
cargo run -p gateway-fixtures --bin gateway-fixture
cargo run -p gateway-fixtures --bin gateway-fixture -- --transport http --auth none
cargo run -p gateway-fixtures --bin gateway-fixture -- --transport http --auth bearer
cargo run -p gateway-fixtures --bin gateway-fixture -- --transport http --auth oauth
```

The STDIO fixture serves protocol traffic only. HTTP fixtures bind to loopback;
the bearer fixture uses the literal test token `fixture-token`. The tools are
paginated `echo` (message and optional delayMs) and `fail` (tool-level error).

The OAuth fixture exposes protected-resource/issuer metadata, an authorization
code endpoint and token endpoint. Its pre-registered public client ID is
`gateway-fixture-client`; callback path is `/oauth/callback` on loopback. It
requires PKCE S256, state/resource, binds redirect URI, expires codes after five
minutes, issues 30-second access tokens, and rotates one-use refresh tokens.
It deliberately has no real identity provider and must never be deployed publicly.
HTTP fixtures are test-only exceptions; production adapters require verified TLS.

## Release checks

Build the container with `docker build -f deploy/Dockerfile -t mcp-gateway-server:0.1.0 .`.
With an existing owned GATEWAY_BUILD_CACHE, run `node scripts/verify-deployment.ts`
using Node24 to rehearse two isolated Compose stacks and external-key recovery.
It accepts no production DSN/project/key and removes only generated stacks/volumes.
Failure retains its private recovery directory. See [deployment](deployment.md).

On the validated Apple Silicon Mac, run `bash scripts/package-macos.sh`. It builds
the compiled frontend and release target, embeds assets and creates a versioned
archive/checksum under `$GATEWAY_BUILD_CACHE/releases`. Builds keep Rust symbols
because the pinned Rust toolchain's Mach-O strip step can produce unloadable
proc-macro libraries on macOS27 ([upstream issue](https://github.com/rust-lang/rust/issues/157750)).
No signing, publication or existing agent configuration edits occur.

Extract into a new owned directory, then run the actual CLI suite with
`GATEWAY_TEST_CLI=/absolute/extracted/mcp-gateway cargo test --locked -p gateway-cli --test gateway`.
The creating binary is used for login/Keychain/logout, all four MCP auth modes,
embedded local index/module delivery and process/signal cleanup. Retain the
extracted binary until its Keychain fixture cleanup has completed. Without the
override, the same suite tests Cargo's development binary.

The full container-to-archive journey also runs with both `GATEWAY_TEST_CLI`
(extracted release executable) and `GATEWAY_FIXTURE_BIN` (built gateway-fixture)
set while invoking `node scripts/verify-deployment.ts` on the Mac. It creates a
fixture-only loopback proxy for the internal control-plane hop, logs in through
the packaged local APIs, explicitly installs/enables, spawns the sole STDIO entry
through Node socket pairs, invokes the exact namespaced echo tool, then confirms
disable blocks the next dispatch. Its immutable creating-binary copy performs
Keychain logout and verifies empty cleanup metadata before removing private state.
