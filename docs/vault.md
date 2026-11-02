# Central secret store and managed OAuth records

Configuration stays ordinary PostgreSQL data. Secret values and OAuth access/
refresh tokens use AES-256-GCM with fresh 96-bit random nonces and a versioned
AEAD format. Associated data binds record kind, owner, stable secret ID (or MCP
and installation ID), version, and key ID. Keys live in an external file, never
PostgreSQL. Browser read APIs return metadata only. Native credential DTOs have
no Debug implementation and no generated browser bindings.

Create a key file in a private directory outside the database/backups:

```sh
dhivon-mcp-gateway-server generate-vault-key --output /private/path/vault-keys.json
```

The command creates a new 0600 file and prints its path, never its value. It
refuses to overwrite an existing file. Set `DHIVON_MCP_GATEWAY_VAULT_KEY_FILE` to that file,
alongside `DATABASE_URL` and `DHIVON_MCP_GATEWAY_PUBLIC_URL`. Startup rejects missing,
malformed, symlinked, or group/world-accessible key files. Container mounts must
retain owner-only permissions and be readable by the service UID.

The key file format is `{ "activeKeyId": "key-id", "keys": { "key-id":
"base64-encoded-32-byte-key" } }`. It supports at most 16 versioned keys. Store it
with operator-controlled secret management; do not put real keys in this repo,
shell history, logs, database dumps or browser storage.

User secret routes:

| Route                       | Behavior                                                   |
| --------------------------- | ---------------------------------------------------------- |
| GET `/api/secrets`          | Current user's key/ID/version/update time only             |
| POST `/api/secrets`         | Create key/value, expectedVersion null                     |
| PUT `/api/secrets/{id}`     | Replace value using current key and expectedVersion        |
| PUT `/api/secrets/{id}/key` | Rename key with expectedVersion; reencrypt for new version |
| DELETE `/api/secrets/{id}`  | Version checked; reject while enabled MCPs reference it    |

There is no read-value route. Ordinary admin permissions do not grant another
user's secret access. Keys are trimmed, 1–128 bytes without controls; values are
1–16,384 bytes; each user can store 256 keys. IDs remain stable across replace and
rename; versions increase. Cross-owner bindings are rejected both in API logic
and by a composite database foreign key. Rotation/replacement never changes
enablement automatically. Replacing a bound value updates the reference version
in the next confirmed snapshot.

Native clients POST `/api/endpoint/credentials/{mcpId}` with expectedRevision and
installationId. Only bearer-authenticated sessions may resolve credentials, and
only for currently assigned, enabled MCPs. Browser cookies are rejected even with
valid CSRF. The response contains the selected bearer token/version or the scoped
managed OAuth record. It is private native traffic over verified TLS; the local
bridge must put it in Keychain and never forward this response to its browser UI.

OAuth records use `(user, MCP, installation)` identity, separate from editable
key/value secrets. Native PUT `/api/endpoint/oauth/{mcpId}` requires current
revision, installationId, expectedVersion (null for first connection), nullable
refreshLease and validated credentials. Client ID/resource/scopes must match the
current admin definition. Auth/transport changes or last-grant removal delete
affected OAuth records. A native disconnect deletes this installation's record
and clears user-level MCP enablement. Public status returns connection/version/
expiry metadata only.

Refresh coordination uses POST `/api/endpoint/oauth/{mcpId}/lease` with
installationId/expectedVersion. A 60-second exclusive lease serializes concurrent
refreshes for that record. No upstream call occurs under a database lock. Persist
rotated tokens with the lease ID, unchanged expected credential version, and
current configuration revision. Persistence atomically increases version and
clears the lease; stale/expired/wrong leases fail. DELETE of the lease with its
holder ID releases coordination without changing tokens. Failed or canceled
refreshes cannot store partial credentials. The full discovery/PKCE/provider
flow is implemented by the Mac endpoint, separate from this storage API.

For key rotation, generate a second private file, securely merge the new key into
the existing key map and select its ID as active. Keep all old keys. Restart the
service with the combined file, then run `dhivon-mcp-gateway-server rotate-vault` using
the same environment/file. It reencrypts records in bounded transactions without
changing values or credential versions. Inspect key-ID counts in both vault
tables before retiring old keys. Old database backups still need their matching
old keys; retain them separately for the backup retention period.

Back up PostgreSQL and protect the key file separately. Restore both the database
and the matching key set, with owner-only file permissions, before starting the
service. Missing/wrong keys or modified ciphertext return a redacted 503 instead
of plaintext. Losing the matching key makes encrypted records unrecoverable;
users must replace generic values or reconnect OAuth. Release verification covers
an actual database dump/restore in addition to these API/crypto checks.
