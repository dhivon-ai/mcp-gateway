# Connect your coding agent

You need the absolute path to the installed Mac CLI and one tool enabled in the [Mac console](mac-install.md). The agent launches the CLI and talks to it over standard input and output (STDIO).

## Add the connection

Open your agent’s MCP settings. Choose a local command, use the absolute path to `dhivon-mcp-gateway`, and pass one argument, `mcp`. In clients with an `mcpServers` JSON setting, the entry looks like this:

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

Replace the example path with yours. The CLI uses the login you selected in the console. Keep your URL, password and tokens out of the agent configuration.

## Make a first fixture call

Restart the agent connection and list its tools. Ask it to use a tool you enabled. The gateway gives tools unique names beginning with `gw_`, so select the one the agent discovered. In the [echo example](quickstart.md#try-an-echo-tool), send **Hello from Dhivon** and expect the same message back.

After switching accounts, restart the agent connection. If the tool is missing, check its assignment and enablement in [troubleshooting](troubleshooting.md).
