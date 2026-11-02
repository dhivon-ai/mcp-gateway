# From deployment to your first tool call

This walkthrough gives an administrator and a developer a shared setup. The
administrator grants a developer access to a controlled echo MCP. The developer
enables it, connects one coding-agent entry and calls its permitted tool.
Disabling it then removes that access.

## Prerequisites

| Role                 | You need                                                                                                                                |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Operator             | A fresh deployment, Docker with Compose, PostgreSQL storage, an ordinary Unix account, a private vault-key location and an HTTPS origin |
| Administrator        | The bootstrapped administrator email and password, plus the developer's account details                                                 |
| Developer            | Apple Silicon on the validated macOS version, a trusted local CLI archive, the enterprise HTTPS origin and an assigned account          |
| Building from source | Rust 1.95.0, Node 24.19.0 and pnpm 11.4.0 as pinned in the repository                                                                   |

Check [compatibility](compatibility.md) for exact platform and protocol evidence.
The archive is locally built, unsigned and unpublished. Obtain it through your
operator's trusted process or build it with the [release instructions](testing.md#release-checks).

## Build the local archive when needed

On the validated Apple Silicon Mac, select the pinned Node 24.19.0, pnpm 11.4.0
and Rust 1.95.0 toolchains. Set an absolute owned build directory, substituting
the path below. During Cadre execution, use its returned build directory.

```sh
export DHIVON_MCP_GATEWAY_BUILD_CACHE=/absolute/owned/build
mkdir -p "$DHIVON_MCP_GATEWAY_BUILD_CACHE"
bash scripts/package-macos.sh
```

The packaging script builds the current frontend, sets
`DHIVON_MCP_GATEWAY_WEB_ASSETS` to its `web` output, and compiles the CLI with
those assets embedded. Expect a versioned Apple Silicon archive and SHA-256 file
in the build directory's `releases` folder. Keep each extracted executable
unchanged while it owns native credentials. Continue with
[archive verification and installation](mac-install.md).

## 1. Deploy and bootstrap

From the repository root, follow [deployment and recovery](deployment.md) to
build `dhivon-mcp-gateway-server:0.1.0`, create an external private vault key,
initialize fresh PostgreSQL, bootstrap your administrator and start the service.
Set `DHIVON_MCP_GATEWAY_PUBLIC_URL` to your real HTTPS origin and configure its
proxy before connecting a developer's Mac.

Check the health endpoint, substituting the origin below.

```sh
curl --fail --silent --show-error https://gateway.example.com/healthz
```

Expect HTTP 200. Open the same origin and sign in as the administrator. Health
confirms the service can query the database. The later tool call verifies the
developer path as well.

## 2. Create a developer and a controlled MCP

In the administrator's **Users** screen, create a user with a display name, email
and password. Use an ordinary user role for this walkthrough. Groups can collect
users who should receive the same assignment.

On the developer's Mac, build the controlled fixture from this checkout.

```sh
cargo build --locked -p dhivon-mcp-gateway-fixtures
```

The executable is under `$CARGO_TARGET_DIR/debug/` when that variable is set,
otherwise under `target/debug/`. Obtain its absolute path. This fixture is a
local development tool and should be used only in a controlled setup.

In **Catalog**, add an MCP named **Echo walkthrough** with the following fields.

| Field                | Value                                                                    |
| -------------------- | ------------------------------------------------------------------------ |
| Transport            | STDIO                                                                    |
| Executable           | The absolute path to `dhivon-mcp-gateway-fixture` on the developer's Mac |
| Arguments            | `[]`                                                                     |
| Environment          | `{}`                                                                     |
| Authentication       | None                                                                     |
| Installation command | Leave empty, since the executable was built explicitly                   |
| Tool policy          | Allowlist containing the original name `echo`                            |

Creating the catalog entry grants no access. In **Access**, assign the MCP to
the developer or a group containing that developer. Save and confirm the server
accepted the change. See [administrator management](admin-ui.md) and
[assignment semantics](configuration.md) for policy and revision details.

## 3. Sign in and explicitly enable it

Install the trusted archive using [Mac installation](mac-install.md). Verify the
checksum, then start local management with your enterprise origin.

```sh
dhivon-mcp-gateway ui --server https://gateway.example.com
```

Keep this command running. The browser opens a loopback page. Sign in with the
developer's email and password. **Echo walkthrough** should appear in the assigned
catalog. Open **Manage** and explicitly enable it. The executable is already
present, so this example needs no installer or remote credentials.

Expect server-confirmed **Enabled**. An assignment, an installed executable and a
completed login are each separate from enablement. If the entry is absent,
confirm the assignment and group membership, then refresh local state. If the
executable is unavailable, check its absolute path and execution permissions on
this Mac.

## 4. Connect one agent entry and call echo

For agents using `mcpServers` JSON, adapt this entry to the client's configuration
format. Substitute the absolute installed CLI path.

```json
{
  "mcpServers": {
    "dhivon-mcp-gateway": {
      "command": "/Users/your-name/.local/bin/dhivon-mcp-gateway",
      "args": ["mcp"]
    }
  }
}
```

Restart the agent's gateway connection. Ask it to list available tools and call
the tool described as **Echo a fixture message** with
`{"message":"Hello from Dhivon"}`. Expect the text **Hello from Dhivon** and a
structured result with the same message. The agent sees a stable `gw_` alias for
the upstream tool. Use the discovered name, while the administrator's policy
continues to use `echo`.

To exercise the same single-entry path without a commercial agent, use the
[runnable Node fixture client](agent-configuration.md#make-a-first-fixture-call).
It discovers the alias and invokes the tool through the gateway. The controlled
SDK client is the compatibility baseline. Agent-specific configuration examples
need validation in the selected commercial client.

## 5. Confirm that disablement removes access

In local management, disable **Echo walkthrough** and wait for server-confirmed
state. Ask the agent to list tools again. The echo tool should disappear. An
attempt to invoke its previous alias should be rejected by the gateway before
reaching the fixture. Restart discovery if your client retains an old tool list.

An administrator can also remove the final assignment. A running endpoint learns
central changes on synchronization, normally every 60 seconds. A failed network
request can use only its already valid bounded lease. Regranting access requires
another explicit enable action. See [lease behavior](endpoint-cache.md) for exact
expiry and outage semantics.

Enable again only if you want to continue using the fixture. When finished, stop
the agent connection and log out through local management or
`dhivon-mcp-gateway logout`. Logout clears this login's native credentials and
requests central session revocation when reachable.

## Continue with your real MCPs

Use [integration recipes](integrations.md) to substitute a real local STDIO or
remote Streamable HTTP MCP. For remote authentication, the user selects their own
bearer secret or completes the supported public-client OAuth flow. Follow
[local management](local-ui.md) for the explicit actions and readiness states.
