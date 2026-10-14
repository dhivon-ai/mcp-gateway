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
