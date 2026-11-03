# Manage secrets and vault keys

## Save a bearer key

Open **Secrets** in the Mac console and add a name and value. In **Manage**, select that secret for the assigned bearer-authenticated MCP and enable it. Values are write-only. To change a value, replace it. To delete a key, first disable or remove its bindings.

The server encrypts secrets separately for each user. It can supply an authorized credential to a signed-in gateway. On the Mac, the gateway protects its copy in encrypted files with one Keychain master key. The browser and coding-agent configuration do not need the token.

## Protect the server vault

Back up the vault key ring separately from PostgreSQL. Losing the matching key makes stored secrets unreadable. Follow the [backup and restore procedure](operations.md) for the root Compose stack.

When rotating keys, keep the previous keys in the ring, add the new key and set it active. Restart with the combined key ring. In the root Compose deployment run `docker compose run --rm --no-deps dhivon rotate-vault` with that same configuration. Verify access with a saved credential before retiring old keys. Keep matching keys for every retained backup.
