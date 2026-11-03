# Manage tools on your Mac

[Open the Mac console](mac-install.md#open-the-console) and sign in. **MCPs** lists the tool servers assigned to you.

- **Manage** an entry to see its tools and whether it needs an installation. Review the command before confirming: it runs with your Mac permissions.
- For a bearer token, create a key under **Secrets** and select it in **Manage**. Secret values can be replaced but cannot be read back.
- For OAuth, choose **Connect**, complete the provider’s consent screen, then return and enable the entry.
- Choose **Enable** and wait for confirmation. Your [coding agent](agent-configuration.md) can then discover permitted tools. **Disable** removes access.

Use **Status** to see the connected server and last successful refresh. Offline settings are read-only. If installation succeeded but enabling failed, reload and retry enablement separately. See [troubleshooting](troubleshooting.md) for recovery.
