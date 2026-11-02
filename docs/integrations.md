# MCP integration recipes

An administrator defines an MCP, its assignment and its original-name tool
policy. Each developer then explicitly enables the assigned MCP through local
management. The coding agent keeps the same
[single gateway entry](agent-configuration.md) as MCPs are added or removed.

## Local STDIO

In the administrator catalog, choose STDIO and supply an absolute executable
path, a JSON array of arguments and a JSON object of environment values. These
paths are resolved on the developer's Mac. For the local fixture walkthrough,
use the built `dhivon-mcp-gateway-fixture`, arguments `[]`, environment `{}`,
authentication None and an allowlist containing `echo`.

For a command such as `/absolute/path/to/mcp --workspace /absolute/project`, use
`/absolute/path/to/mcp` as the executable and
`["--workspace", "/absolute/project"]` as its argument array. This is a shape
example. Substitute the command and options documented by the actual MCP.
Optional installation is a separate structured command that runs only after
the developer confirms it. Put the working directory in its dedicated field
when required.

Keep upstream stdout reserved for MCP protocol messages. The process runs with
the developer's privileges and must satisfy its own runtime prerequisites.
Document required non-secret environment values for your team. The remote bearer
binding flow below applies to HTTP authentication. See
[gateway aggregation](gateway-aggregation.md) for process and protocol bounds.

## Remote Streamable HTTP without authentication

Choose Streamable HTTP and enter the MCP's exact HTTPS URL, such as
`https://mcp.example.com/mcp`, replacing the example with the real service.
Choose None for authentication, set the original tool allowlist, then assign
the definition. The developer refreshes and explicitly enables it.

Expect permitted tools to appear when discovery succeeds. A connection failure
appears as unavailable for this MCP while healthy MCPs remain usable. Production
connections require verified HTTPS. Redirects are rejected. The gateway forwards
the tested tools profile, described in [compatibility](compatibility.md).

## Remote Streamable HTTP with a bearer key

Configure the exact HTTPS URL and Bearer authentication in the administrator
catalog. Assign access and choose the allowed original tool names.

The developer creates a secret in the local UI's **Secrets** screen, opens the
MCP's **Manage** view, selects that owned secret and explicitly enables the MCP.
The browser displays secret metadata only. Confirm the provider accepts that
token for this exact MCP endpoint before sharing setup instructions.

The gateway resolves the user's selected key into native Keychain and sends it
only to the configured upstream. Expect an authentication status and discovered
permitted tools after synchronization. Replace an invalid key through the secret
flow, reload state and reconnect the affected MCP. See
[native credentials](remote-credentials.md) and [vault operations](vault.md) for
version checks, isolation and recovery.

## Remote Streamable HTTP with supported OAuth

Register a public native client with the provider. Configure authorization-code
and refresh grants, S256 PKCE, the required scopes and an exact allowed loopback
callback. Put the client ID and scopes in the administrator's OAuth definition,
alongside the MCP's exact HTTPS URL. Assign the MCP and its tool allowlist.

For a provider registered to port 19001, the developer starts management with
the following command, substituting the enterprise origin.

```sh
dhivon-mcp-gateway ui --server https://gateway.example.com --port 19001
```

Register `http://127.0.0.1:19001/oauth/callback` with that provider. The developer
chooses **Connect**, completes provider consent, reloads confirmed status and
explicitly enables the MCP. Expect a connected credential state and separately
confirmed enablement. The agent's `mcp` command uses the prepared credential.

Follow the full [OAuth profile and registration guide](oauth.md) before choosing
a provider. Controlled fixtures verify this profile. Record provider-specific
interoperability separately and confirm the discovery, issuer, resource and
refresh requirements for the intended deployment.

## Check access end to end

List tools through the agent gateway, invoke one permitted tool with safe input,
then disable the MCP and confirm the old alias is rejected. A newly discovered
tool remains unavailable under an allowlist until an administrator permits its
original name. Review a revision conflict by reloading current state before
saving. The [quickstart](quickstart.md) demonstrates these steps with a controlled
echo fixture.

For local HTTP/provider development only, use the explicitly isolated
[MCP and OAuth fixtures](testing.md#mcp-and-oauth-fixtures). They use public test
tokens and a loopback-only exception and are intended for the named fixture
workflow.
