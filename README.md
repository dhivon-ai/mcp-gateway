# Dhivon MCP Gateway

A product of **Dhivon AI** for teams that need to control which MCP servers and
tools their coding agents can use.

Each enterprise runs its own control plane. Administrators assign MCPs and tool
policies, developers explicitly enable their assigned MCPs on their Mac, and
each coding agent connects to one local STDIO gateway. The gateway checks current
access before exposing or calling a tool. Local tools run as the developer, and
remote tools run at their configured upstream service.

## Start with your role

| You want to                                             | Start here                                                                             |
| ------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Understand the setup and make a first tool call         | [Quickstart](docs/quickstart.md)                                                       |
| Deploy, bootstrap, back up or recover the service       | [Deployment and recovery](docs/deployment.md)                                          |
| Manage users, groups, assignments and tool restrictions | [Administrator guide](docs/admin-ui.md)                                                |
| Install the Mac application and connect an agent        | [Mac installation](docs/mac-install.md) and [agent setup](docs/agent-configuration.md) |
| Add a local or remote MCP, bearer key or OAuth client   | [Integration recipes](docs/integrations.md)                                            |
| Find the full reference and contributor checks          | [Documentation index](docs/index.md)                                                   |

The current endpoint targets Apple Silicon. Locally built archives are unsigned
and unpublished. Review [compatibility](docs/compatibility.md) before planning a
deployment. The [MVP release report](docs/release.md) records the original
delivery's artifact identity and evidence.

## Why a gateway

Configuring every upstream separately in every agent gives each developer another
place to maintain access settings and credentials. Dhivon MCP Gateway puts the
team's assignments and tool policy in one self-hosted control plane, while the
developer keeps an explicit choice about which assigned MCPs to enable.

An administrator grant makes an MCP available to a user. The developer's opt-in
makes it eligible for use on the agent connection. The gateway then checks tool
policy, authentication and a bounded authorization lease on discovery and calls.
Removing the last grant clears enablement, and access requires a new opt-in when
granted again. [Follow that complete example](docs/quickstart.md).

## Build and contribute

Use Rust 1.95.0, Node 24.19.0 and pnpm 11.4.0 from the committed toolchain files.
Run these commands from the repository root.

```sh
pnpm install --frozen-lockfile
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --workspace
pnpm typecheck
pnpm lint
pnpm test
pnpm format:check
pnpm build
cargo run --locked -p dhivon-mcp-gateway-server -- --help
cargo run --locked -p dhivon-mcp-gateway-cli -- --help
```

The Rust workspace separates domain contracts, the central service, the Mac
gateway and controlled test fixtures. The management frontend is in `apps/web`
and uses React, TypeScript, Vite, Tailwind and shadcn/ui. Both Rust binaries embed
its production assets.

During Cadre execution, use the runtime-provided owned
`DHIVON_MCP_GATEWAY_BUILD_CACHE` and its `cargo` child as `CARGO_TARGET_DIR`.
[Testing](docs/testing.md) covers generated contracts, explicit PostgreSQL cases,
native fixtures and release rehearsal. The historical MVP specification and plan
are preserved in `.cadre/archive/mac-mvp/`.

## Access and credential boundaries

Enablement is authoritative on the server. Native credentials and cache keys live
in macOS Keychain. The authenticated encrypted cache expires at most one hour
after a complete successful synchronization, further bounded by session and
credential lifetimes. Failed requests cannot renew it. Configuration and new
authentication changes require connectivity.

PostgreSQL stores central configuration and encrypted user secrets. Operators keep
the vault keys separately from the database. [Policy configuration](docs/configuration.md),
[native credentials](docs/remote-credentials.md) and [vault operations](docs/vault.md)
describe these contracts and recovery requirements.
