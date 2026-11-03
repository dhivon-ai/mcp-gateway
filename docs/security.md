# How access is protected

Administrators decide who can use each tool. Developers sign in and enable the tool servers they want. Both decisions must allow a call.

## Identity and permissions

Your email is your sign-in identity. A display name is only a label. Changing account details or resetting a password revokes affected sessions. Administrators can assign access without seeing users’ secret values.

## Credentials and data at rest

The server encrypts per-user secrets with its external vault key. On the Mac, one random master key in Keychain protects the gateway’s encrypted files, including the saved server URL, login state and credentials. Your login password is never saved on the Mac. [Manage secrets and vault keys](vault.md).

## Network and browser protection

Use the team’s verified HTTPS origin for the server. The Mac console runs on a separate loopback address in your browser. Your agent starts the local CLI and passes tool requests over standard input and output. Local tools run with your Mac permissions. Remote tools execute at their configured service.

## Policy leases and offline behavior

The gateway refreshes access at startup and normally every 60 seconds. With the server offline, a tool can keep working only while its last verified settings and credentials remain valid, for at most one hour. Settings changes and new sign-ins require a connection. The gateway checks access again before each call and never automatically repeats a tool call.

[Recover access on your Mac](endpoint-identity.md#upgrade-and-recovery) or [troubleshoot a missing tool](troubleshooting.md).
