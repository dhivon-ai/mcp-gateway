# Dhivon MCP Gateway documentation

Dhivon MCP Gateway is a self-hosted Dhivon AI product. Start with the
[quickstart](quickstart.md) for the path from deployment to a permitted tool call.

Read [why the gateway exists and how it works](architecture.md), review
[security and trust boundaries](security.md), or see the [product roadmap](roadmap.md).

## Operators

- [Deployment and recovery](deployment.md) covers image builds, private keys,
  PostgreSQL, administrator bootstrap, HTTPS, health, backup and fresh restore.
- [Identity](identity.md) explains accounts, email sign-in, roles and sessions.
- [Vault operations](vault.md) covers encrypted user secrets, external keys and rotation.
- [Compatibility](compatibility.md) lists the supported profile and tested limits.
- [Current Dhivon release evidence](dhivon-release.md) records the renamed artifacts.
- [Historical release evidence](release.md) and [acceptance coverage](acceptance.md) preserve
  the original MVP's dated verification records.

## Administrators

- [Administrator guide](admin-ui.md) walks through catalog, users, groups,
  assignments and tool policies.
- [Configuration reference](configuration.md) defines grants, enablement,
  revisions and policy synchronization.
- [Integration recipes](integrations.md) cover local commands and remote
  Streamable HTTP with no authentication, bearer keys or supported OAuth.
- [OAuth registration](oauth.md) specifies discovery, public clients, callback
  registration and refresh behavior.

- [Troubleshooting](troubleshooting.md) gives symptom, diagnosis, recovery and expected outcomes.

## Developers

- [Mac installation](mac-install.md) covers archive verification, login and the
  explicit install/enable sequence.
- [Local management](local-ui.md) explains catalog, secrets, OAuth, status and offline feedback.
- [Agent configuration](agent-configuration.md) provides the single STDIO entry
  and a runnable fixture client.
- [Native credentials](remote-credentials.md) explains readiness, ownership and recovery.

## Architecture and contributors

- [Architecture](architecture.md) explains component ownership and tool execution locations.
- [Contributing](contributing.md) covers toolchains, build output, checks and release evidence.

- [Wire and policy contracts](contracts.md) describes Rust-generated TypeScript and validation.
- [Gateway aggregation](gateway-aggregation.md) covers discovery, aliases,
  dispatch, limits and owned-process shutdown.
- [Endpoint identity](endpoint-identity.md), [policy cache](endpoint-cache.md) and
  [endpoint management](endpoint-management.md) detail the native boundaries.
- [Verification and fixtures](testing.md) provides development checks, isolated
  PostgreSQL, controlled MCP/OAuth services and release commands.

The guides use placeholders such as `gateway.example.com` and
`/absolute/path/to/...`. Substitute your own origin and executable paths before
running commands. Public fixture credentials are identified as test-only wherever
they appear.
