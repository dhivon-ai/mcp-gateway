# Troubleshooting

## An assigned MCP is missing

Check that you signed in to the right team URL and account. Ask an administrator to check **Access** and group membership. Refresh the Mac console. A new catalog entry needs an assignment before you can see it.

## The MCP is enabled but its tool is absent or denied

Check the tool’s original name against the administrator’s allowlist. For a local tool, verify its executable path on your Mac. For a remote tool, check its authentication in **Manage**. Refresh the agent’s tool list or restart its connection. Try the [echo walkthrough](quickstart.md#try-an-echo-tool) to check the path end to end.

## Offline or expired policy lease

Reconnect to the team’s HTTPS server and refresh the console. An expired session may require signing in again. Settings and installations need an online connection. [Read about offline access](security.md#policy-leases-and-offline-behavior).

## Bearer credentials are unavailable or rejected

In **Secrets**, replace an expired API key and confirm that **Manage** points to the correct secret. Refresh online. If Keychain is locked, unlock it when macOS prompts. For damaged local state, follow [Mac recovery](endpoint-identity.md#upgrade-and-recovery).

## OAuth consent or refresh failed

Check that the remote URL, client ID, scopes and callback port match the provider registration. Start the console with the registered port, reconnect through **Manage** and explicitly enable the entry. [OAuth instructions](oauth.md).

## Installation failed or enablement was not confirmed

Check the command path and permissions on the developer’s Mac. If installation succeeded, reload and explicitly enable the entry before trying an installation again.

## A save reports a revision conflict

Choose **Reload current state**, check what changed, then reapply your edit. [Administrator guide](admin-ui.md).

## The website or service will not start

From the deployment checkout, run `docker compose ps -a` and `docker compose logs --tail 50 setup dhivon`. Confirm the HTTPS address and proxy target. For a database or key mismatch, preserve the private setup directory and volume, then follow [recovery](operations.md). If the server is healthy but Mac sign-in fails, use the exact HTTPS origin supplied by your administrator.
