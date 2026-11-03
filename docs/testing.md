# Verification and fixtures

Set `DHIVON_MCP_GATEWAY_BUILD_CACHE` to the runtime-owned build path and `CARGO_TARGET_DIR`
to its `cargo` child before running checks during Cadre execution.

Run `bash scripts/check-contracts.sh` to compare generated TypeScript against the
Rust source. Run workspace Rust tests/Clippy/format and frontend test/lint/type
checks before integration. CI targets Linux for the service and an Apple Silicon
macOS runner for endpoint verification. The Mac runner label follows the
[GitHub hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

## Isolated PostgreSQL

```sh
docker compose -p dhivon-mcp-gateway-tests -f deploy/compose.test.yaml up -d --wait
```

The fixture database is `dhivon_mcp_gateway_test`, on loopback port 18432 with user
`dhivon_mcp_gateway_test` and password `isolated-fixture-password`. These are fixture values,
never production credentials. Database API integration tests use a separate
schema/database lifecycle and must not connect to production instances.
Remove only this owned fixture stack with its exact Compose project when done.

Run real database API tests explicitly (normal platform checks skip this external
prerequisite):

```sh
TEST_DATABASE_URL=postgres://dhivon_mcp_gateway_test:isolated-fixture-password@127.0.0.1:18432/dhivon_mcp_gateway_test cargo test -p dhivon-mcp-gateway-server -- --ignored
```

Each test creates/migrates/drops a uniquely named owned database. The harness
rejects non-loopback DSNs and any base database other than `dhivon_mcp_gateway_test`.

## MCP and OAuth fixtures

```sh
cargo run -p dhivon-mcp-gateway-fixtures --bin dhivon-mcp-gateway-fixture
cargo run -p dhivon-mcp-gateway-fixtures --bin dhivon-mcp-gateway-fixture -- --transport http --auth none
cargo run -p dhivon-mcp-gateway-fixtures --bin dhivon-mcp-gateway-fixture -- --transport http --auth bearer
cargo run -p dhivon-mcp-gateway-fixtures --bin dhivon-mcp-gateway-fixture -- --transport http --auth oauth
```

The STDIO fixture serves protocol traffic only. HTTP fixtures bind to loopback;
the bearer fixture uses the literal test token `fixture-token`. The tools are
paginated `echo` (message and optional delayMs) and `fail` (tool-level error).

The OAuth fixture exposes protected-resource/issuer metadata, an authorization
code endpoint and token endpoint. Its pre-registered public client ID is
`dhivon-mcp-gateway-fixture-client`; callback path is `/oauth/callback` on loopback. It
requires PKCE S256, state/resource, binds redirect URI, expires codes after five
minutes, issues 30-second access tokens, and rotates one-use refresh tokens.
It deliberately has no real identity provider and must never be deployed publicly.
HTTP fixtures are test-only exceptions; production adapters require verified TLS.

## Release checks

Project and font notices are checked in the actual outputs. Mac packaging checks
its archive; the deployment rehearsal checks the runtime image; website CI checks
the published files and licensing links. To check existing builds directly:

```sh
node scripts/check-notices.ts --archive /absolute/path/dhivon-mcp-gateway-0.1.0-macos-arm64.tar.gz
node scripts/check-notices.ts --image dhivon-mcp-gateway-server:0.1.0
node scripts/check-notices.ts --site "$DHIVON_MCP_GATEWAY_BUILD_CACHE/site"
```

These checks compare exact source bytes, including the full unchanged Manrope
OFL. See [license and ownership](licensing.md) for readable notice locations.

Build the container with `docker build -f deploy/Dockerfile -t dhivon-mcp-gateway-server:0.1.0 .`.
With an existing owned DHIVON_MCP_GATEWAY_BUILD_CACHE, run `node scripts/verify-deployment.ts`
using Node24 to rehearse two isolated Compose stacks and external-key recovery.
It accepts no production DSN/project/key and removes only generated stacks/volumes.
Failure retains its private recovery directory. See [deployment](deployment.md).

On the validated Apple Silicon Mac, run `bash scripts/package-macos.sh`. It builds
the compiled frontend and release target, embeds assets and creates a versioned
archive/checksum in a fresh `dist/macos-XXXXXXXX/releases` directory. Default builds
reuse a private Cargo target at `dist/.macos-cargo`. When you set
`DHIVON_MCP_GATEWAY_BUILD_CACHE`, the script uses its `releases/` and `cargo/`
subdirectories. An explicit `CARGO_TARGET_DIR` takes precedence. Builds keep Rust symbols
because the pinned Rust toolchain's Mach-O strip step can produce unloadable
proc-macro libraries on macOS27 ([upstream issue](https://github.com/rust-lang/rust/issues/157750)).
No signing, publication or existing agent configuration edits occur.

Extract into a new owned directory, then run the actual CLI suite with
`DHIVON_MCP_GATEWAY_TEST_CLI=/absolute/extracted/dhivon-mcp-gateway cargo test --locked -p dhivon-mcp-gateway-cli --test gateway`.
The creating binary is used for login/Keychain/logout, all four MCP auth modes,
embedded local index/module delivery and process/signal cleanup. Retain the
extracted binary until its Keychain fixture cleanup has completed. Without the
override, the same suite tests Cargo's development binary.

The full container-to-archive journey also runs with both `DHIVON_MCP_GATEWAY_TEST_CLI`
(extracted release executable) and `DHIVON_MCP_GATEWAY_FIXTURE_BIN` (built dhivon-mcp-gateway-fixture)
set while invoking `node scripts/verify-deployment.ts` on the Mac. It creates a
fixture-only loopback proxy for the internal control-plane hop, logs in through
the packaged local APIs, explicitly installs/enables, spawns the sole STDIO entry
through Node socket pairs, invokes the exact namespaced echo tool, then confirms
disable blocks the next dispatch. Its immutable creating-binary copy performs
logout and `fixture-retire --fixture-state-path <directory>`. The latter requires a signed-out installation, verifies the old namespace is empty, deletes the one fixture master, and confirms its absence before reporting retirement. The production `logout` retains its master and encrypted install receipts.

For a Docker-hosted HTTPS control plane, HTTPS MCP/OAuth fixture and actual native
loopback callback, follow the [HTTPS OAuth rehearsal](https-oauth-rehearsal.md).
It preserves certificate verification, requires explicit temporary localhost trust
and cleanup, and labels controlled-provider evidence separately from public hosting.

## Single-key endpoint storage

`cargo test --locked -p dhivon-mcp-gateway-cli --test master_key` exercises two live processes and a full file rollback. The CLI library’s `cutover` tests interrupt preparation, fencing and deletion, exercise a signed-out handoff, and compare concurrent recovery attempts. The `identity` and `upstream_credentials` suites check independent online sign-in, failed offline repair, authenticated bearer refetch, rejection and exact expiry. The PostgreSQL `remote_oauth` integration uses the encrypted store on macOS.

To reproduce the first-fetch refusal and damaged-index sign-in regressions on macOS, run:

```sh
cargo test --locked -p dhivon-mcp-gateway-cli --test upstream_credentials first_fetch_cannot_publish_after_an_authenticated_refusal
cargo test --locked -p dhivon-mcp-gateway-cli --test identity unreadable_index_requires_independent_online_sign_in
```

For a package upgrade rehearsal, also set `DHIVON_MCP_GATEWAY_TEST_V1_CLI` to an absolute path to the original v1 executable and run the extracted CLI gateway suite above. It creates a signed-out v1 installation, migrates it, checks Keychain item counts without reading secrets, damages a synthetic snapshot, repairs through the running UI and verifies fixture retirement.

The real locked-Keychain check is **manual**: macOS may show a password dialog. Run it only when prepared:
`cargo test --locked -p dhivon-mcp-gateway-cli --lib locked_isolated_keychain -- --ignored --nocapture`. The test prints its new isolated path and disposable password **before** creation. Its test-only password is `Dhivon-Test-Only-27!`. Choose Cancel or Deny to verify that a locked read fails; entering the password unlocks the fixture and changes that test condition. The test deletes its own Keychain. Routine tests do not create or lock a separate Keychain database.
