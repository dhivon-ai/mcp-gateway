# Connect an OAuth provider

Use this guide when a remote MCP server requires sign-in through its provider. You need the remote HTTPS MCP URL, the provider’s public client ID, permitted scopes and a registered loopback callback.

## Register the callback

The provider sends the developer back to the Mac running the console. For a provider that requires a fixed port, register `http://127.0.0.1:8765/oauth/callback` with the public client (substitute an allowed port), and start the Mac console with the same port:

```sh
dhivon-mcp-gateway ui --server https://gateway.example.com --port 8765
```

The URL passed to `--server` is the team’s HTTPS address. The loopback URL is the provider’s callback. Check the provider’s registration rules for matching hosts, paths and ports.

## Connect the tool

1. The administrator creates a Streamable HTTP catalog entry for the remote URL, selects OAuth and enters the registered public client ID and scopes. Assign the entry to the developer.
2. The developer opens **Manage → Connect**, signs in at the provider and returns to the console.
3. Refresh the entry, enable it, then discover its tools from the agent. To remove access, disconnect the provider in **Manage**.

The supported profile uses discovery, a public client and authorization code with S256 PKCE. Check the provider’s support before setup. A failed or cancelled consent does not enable a tool. [Troubleshoot OAuth](troubleshooting.md#oauth-consent-or-refresh-failed).
