# Back up and recover a gateway

These steps apply to the root Compose deployment. Run commands as the deployment owner. Use a private backup location and keep vault key backups under separate access controls from database dumps.

## Back up

From the repository root, give the backup command an absolute directory outside the checkout on a filesystem that supports hard links. It creates the directory with mode 0700 if needed. If it already exists, it must belong to you and have mode 0700.

```sh
bash scripts/backup-gateway.sh "$HOME/dhivon-backups"
```

On success, the command prints the full path to a new owner-only dump. Save that path for recovery. A failed export leaves earlier dumps intact.

Separately back up the private directory printed during configuration, including `gateway/keys/vault-keys.json` and `gateway/keys/deployment-id`. Keep the matching key ring for every retained backup. A database dump alone cannot recover encrypted credentials.

## Restore to a fresh deployment

1. Choose a fresh checkout, private setup directory and Compose project on an isolated host or loopback port. Use a separate `XDG_CONFIG_HOME` if testing on the same host. Run the [configuration prompts](deployment.md#configure-once). Keep routing away from this project during recovery.
2. Start **only PostgreSQL** in the fresh project with `docker compose up -d --wait postgres`. This creates the new database password and physical cluster identity. Keep using the same project name and environment for every step.
3. Set `dump` to the full path of the backup you want to recover. Restore it and invalidate sessions copied from the backup.

   ```sh
   dump="/absolute/path/to/the/saved-backup.dump"
   docker compose exec -T postgres pg_restore -U dhivon -d dhivon --no-owner --exit-on-error < "$dump"
   docker compose exec -T postgres psql -U dhivon -d dhivon -v ON_ERROR_STOP=1 -c 'DELETE FROM sessions;'
   ```

4. In the new private directory create `gateway/keys/` with mode 0700. Copy the **matching backup** files `vault-keys.json` and `deployment-id` there with mode 0600. Copy the **new** `postgres/cluster-id` to `gateway/keys/cluster-id` with mode 0600. These files must belong to the deployment owner. Keep the new database password paired with its new volume.
5. Run `docker compose up -d --build --wait`. Sign in with a restored account. Verify its assignments and an authorized call with a saved credential before switching HTTPS routing. Recheck revocations made after the backup. Have endpoints sign out and sign in again after a rollback.

If the gateway refuses the key or database identity, check that the dump, logical deployment ID and vault key ring come from the same backup. Preserve the files and volume for recovery.

## Advanced deployment

The source repository also includes `deploy/compose.yaml` for operators supplying their own database password, vault key and bootstrap step. Follow `docs/manual-deployment.md` in the checkout for that configuration. When rotating keys retain prior keys for older backups. See [vault operations](vault.md).
