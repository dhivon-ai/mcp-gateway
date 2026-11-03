# Give your team access

Sign in with an administrator account at your gateway’s HTTPS address. To let a developer use a tool, create their account, choose the tool and grant access.

1. In **Users**, create an account with a display name, email and password. The email is the sign-in identifier. Use **Groups** if several people need the same tools.
2. In **Catalog**, add the MCP server that provides the tool. Set its [connection and authentication](integrations.md). For a local command, use a path that exists on the developer’s Mac.
3. In the tool policy, allow the original tool names the team needs. An empty allowlist allows none.
4. In **Access**, assign the MCP to the user or group and save. Give the developer the gateway URL and the [Mac setup guide](mac-install.md).

The developer should see the entry after signing in. They choose when to install or enable it. Removing the last assignment clears their enablement. Adding the assignment again requires them to enable it again.

## Manage accounts

Use **Users** to change a name or email, reset a password, change a role or deactivate an account. Affected sessions are revoked after account or password changes. Passwords and user secret values are write-only.

If another administrator saved a change first, use **Reload current state**, review the current values and reapply your edit. See [configuration](configuration.md) for assignment and tool rules.
