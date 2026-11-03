# Mac sign-in and recovery

Use these instructions when upgrading an existing Mac installation or recovering a damaged local login.

## Native state and Keychain ownership

The gateway keeps **one generic-password item per initialized installation** in the default Mac Keychain. In production the service is `com.dhivon.mcp-gateway.endpoint.v1.master.v2`. Its secret is exactly 32 random bytes; its account is an opaque random UUID. The account name and master key change together when a new encrypted index commits. The account name never contains a user ID or server URL.

The private state directory is `~/Library/Application Support/com.dhivon.mcp-gateway`. Its `endpoint-v2/` directory contains an authenticated index and separate encrypted records. These hold the selected origin, installation and login generation, session, cache key and checkpoints, upstream credentials, pending saves, rejection markers, cleanup lists and install receipts. The policy payload remains in `snapshot.enc`. Directories use 0700 and files use 0600. Lock files, opaque record names, format headers and the v1 fence `identity.json` contain no credentials or user configuration. The gateway never persists the login password.

The index opens before an identity is known. A random seed and fresh record UUID derive separate AES-256-GCM keys with HKDF-SHA-256; the purpose is also authenticated. Account records include the origin, deployment and verified user in their scope and are pinned to the current login generation. Credential consumers additionally verify the installation, MCP, runtime binding and version. The user ID is a binding, not an encryption secret.

Under the process lock the gateway writes and syncs new ciphertext and the encrypted index, then updates the one Keychain item to select that index. Earlier files cannot grant access after logout, credential rejection or a newer clock watermark, even when a whole old directory is restored. This assumes the OS and Keychain remain trusted. It does not protect against restoration of **both** a prior Keychain and its matching files.

## Upgrade and recovery

Stop all old UI and agent processes and retain the creating executable before upgrading. Start the new CLI normally. It validates the old ownership list against the exact legacy service, verifies the encrypted copy, fences v1 readers and retires only known entries. Unknown entries or unavailable Keychain access stop the upgrade. Historical plaintext in external backups is unaffected.

If the creating executable is needed for Keychain access, first finish any pending OAuth save using that executable. Before cutover, run its `logout`, then open the new UI and sign in. If cutover already fenced the old executable and reports incomplete legacy cleanup, run the new executable's **`upgrade-handoff`**, then the original executable's **`logout`**, then restart the new version. This explicit handoff discards the staged login and unsaved OAuth responses before enabling a signed-out legacy cleanup journal. Reconnect affected providers; no provider exchange or install is repeated.

With a valid native login, a successful authenticated sync can replace damaged policy data; a fresh credential response can repair its damaged local token while preserving version, expiry and rejection evidence. A locked Keychain must be unlocked. If the master or authenticated identity is lost, run `dhivon-mcp-gateway ui --server <origin>` and sign in online. This also works when both the local format marker and the encrypted index are damaged. A failed sign-in preserves the existing master key. A successful sign-in rebuilds the store, and restoring the old directory cannot bring back the previous login. A new installation identity cannot inherit old install receipts or installation-bound OAuth grants. Verify local side effects yourself and reconnect as necessary. Missing or damaged local-only evidence cannot be fetched from the server.

Logout fences the current generation, removes its credentials and policy, and attempts central revocation. Offline cleanup still works. A signed-out installation keeps one master key to protect installation receipts and configuration. Known 401s invalidate only the matching generation.
