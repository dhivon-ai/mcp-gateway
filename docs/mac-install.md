# Apple Silicon installation

This MVP archive targets Apple Silicon and is validated on macOS 27.0.1 (26A434).
It contains one Rust executable with both local React UI assets and STDIO gateway;
Node, Rust and a frontend development server are not needed on an end-user Mac.
Distribution is not Developer ID signed/notarized or published. Use the locally
built trusted artifact; follow your enterprise's policy for unsigned software.

Verify the supplied checksum before extracting:

```sh
shasum -a 256 -c dhivon-mcp-gateway-0.1.0-macos-arm64.tar.gz.sha256
tar -xzf dhivon-mcp-gateway-0.1.0-macos-arm64.tar.gz
mkdir -p "$HOME/.local/bin"
cp dhivon-mcp-gateway-0.1.0-macos-arm64/dhivon-mcp-gateway "$HOME/.local/bin/dhivon-mcp-gateway"
"$HOME/.local/bin/dhivon-mcp-gateway" --version
"$HOME/.local/bin/dhivon-mcp-gateway" ui --server https://gateway.example.com
```

Use the actual enterprise HTTPS origin. The CLI opens a loopback page in your
browser; keep the command running while using management. Log in with the email and password
created by an administrator. Name is display-only. The operating system may ask for Keychain access.
Passwords are not saved locally. For a provider requiring a registered callback,
use `ui --server <origin> --port <registered-port>`; register
`http://127.0.0.1:<registered-port>/oauth/callback` with the OAuth public client.

The assigned catalog starts disabled. Manage an MCP to inspect its original tool
policy and native availability. Installation is explicit and uses your user
privileges. Choose Install, review the administrator's command summary and
confirm; enable separately or explicitly choose enable after successful install.
Failed installation never performs automatic enablement. Successful local work
with an unconfirmed server write stays recoverable: reload current settings,
then explicitly enable, without rerunning the installer. Another client's
confirmed enablement may differ from an earlier installation receipt.

For bearer MCPs, create a key/value in Secrets and select your own key in Manage.
Values are write-only; replace them rather than trying to reveal them. Rename
preserves the stable binding ID. Remove enabled references before deleting a key.
For OAuth, choose Connect, complete the provider's consent, reload current state,
then explicitly enable. Connected, native credential readiness and central
Enabled are separate states. Disconnect affects this Mac installation's tokens
and disables the user's MCP; other endpoints learn the confirmed setting on sync.

Configure your coding agent with one absolute executable and the argument `mcp`.
For agents using JSON `mcpServers`, adapt the enclosing configuration:

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

Use the absolute path from your installation. Never add passwords/tokens to the
agent entry. Restart its gateway process after switching account/deployment.
Only tools are forwarded; policies refer to original names, while the agent sees
stable `gw_` aliases. Named commercial agents are not independently validated;
the official rmcp 3.5.1 client is tested with this single command.

Each running process refreshes at startup and every 60 seconds, with a 10-second
request deadline. Only a complete durable authenticated snapshot renews the
lease. During an outage, eligible tools can use the last encrypted cache for at
most 3600 seconds from validation, further limited by session/token expiry.
Exact expiry fails closed and cancels/closes owned work where possible. Every
configuration, secret, installation and new OAuth change requires connectivity;
there is no offline queue. Status shows the fixed validation/expiry and remaining
time. A failed refresh or token refresh cannot extend it.

```sh
"$HOME/.local/bin/dhivon-mcp-gateway" status
"$HOME/.local/bin/dhivon-mcp-gateway" logout
```

Logout clears local cache/Keychain entries and revokes the central session when
reachable. It remains available offline. Exit the UI with Ctrl-C. The agent owns
its STDIO process; EOF/SIGTERM also requests cleanup. If a session is rejected,
log in again. If clock/cache integrity is uncertain, synchronize online. The
local browser authorization cookie lasts one hour; reload the page to obtain a
new local cookie. This does not extend the policy lease. Troubleshooting and
operator/provider details are in the repository's deployment, local UI, OAuth and
agent configuration guides.

A login generation tracks up to 128 credential-bearing MCP identities, including
formerly assigned ones. If a new MCP reports local credential capacity, log out
and sign in again online to clear retired local credentials. Existing credentials,
status, synchronization and logout remain available at this boundary.
