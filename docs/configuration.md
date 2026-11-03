# How tool access works

A tool appears in a coding agent when the administrator allows it, the developer enables its MCP server, and the local gateway has the credentials and executable it needs.

- **Catalog** describes how to reach a tool server. Adding an entry makes it available for assignment.
- **Assignments** grant a user, group or everyone access to that entry. Grants add together. Removing the final grant clears the user’s enablement.
- **Tool policy** uses the tool’s original upstream name. An allowlist exposes only the names you select. A denylist excludes the names you select. An empty allowlist exposes no tools.
- **Enablement** records the developer’s choice. Installing a local executable or connecting an OAuth account is a separate action.

Agents see unique tool names beginning with `gw_`. A saved name can’t bypass a later access change. After an administrator changes policy, a running gateway normally refreshes within 60 seconds. For an immediate check, refresh the local console or restart the agent connection.

If two people edit the same settings, reload the latest state before saving again. [Give someone access](admin-ui.md) or [try the echo walkthrough](quickstart.md#try-an-echo-tool).
