# Get started

Dhivon connects your coding agent to tools your team has approved. Choose the path that fits what you’re doing today.

- **[Connect to my team](mac-install.md)** — for developers with a gateway URL and account. Open the Mac console, enable an assigned tool, then connect your agent.
- **[Set up a gateway](deployment.md)** — for the person hosting the service. Start the server, sign in and give someone access.

## Try an echo tool

Echo sends your message back to you. It’s a small way to check the connection from your agent to a local tool. If your team already assigned you a tool, you can use that instead.

1. **Prepare the tool.** On the developer’s Mac, from the source checkout with Rust installed, run `cargo build --locked -p dhivon-mcp-gateway-fixtures`. The executable is `target/debug/dhivon-mcp-gateway-fixture` (or under your chosen Cargo target directory). Get its absolute path.
2. **Grant access.** In the admin console, add **Echo walkthrough** in **Catalog**. Choose STDIO (a local command), enter the executable’s absolute path, use no arguments or authentication, and allow the original tool name `echo`. Save, then assign it to the developer under **Access**.
3. **Enable it.** Sign in through the [Mac console](mac-install.md). Open **Manage** for Echo walkthrough and enable it. Expect **Enabled**.
4. **Try it.** [Connect your agent](agent-configuration.md), refresh its tool list and ask it to call the echo tool with `{"message":"Hello from Dhivon"}`. It should return **Hello from Dhivon**.
5. **Check access.** Disable Echo walkthrough in the console, wait for confirmation and refresh the agent connection. The tool disappears and a call to its previous name is denied.

If the tool is missing at step 3 or 4, use the [troubleshooting guide](troubleshooting.md).
