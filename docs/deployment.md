# Self-hosted deployment and recovery

Run one enterprise per deployment. The service image embeds the compiled React
administrator website and SQLx PostgreSQL migrations. The Compose example binds
only loopback port 8080; PostgreSQL is private to its network. Terminate verified
HTTPS at a reverse proxy before exposing the enterprise origin. The Mac endpoint
and remote production MCPs require verified TLS.

Build with Docker/Compose from the repository root:

```sh
docker build -f deploy/Dockerfile -t mcp-gateway-server:0.1.0 .
```

Node 24.19.0/pnpm 11.4.0 and Rust 1.95.0 build stages use the committed lockfiles.
The runtime is Debian bookworm, an unprivileged UID, read-only root filesystem,
no capabilities and a read-only external key mount. PostgreSQL 18.6 is digest
pinned. Build base tags and Debian security packages can receive upstream
rebuilds; record the resulting image ID and pin an approved image digest in your
own deployment rather than assuming byte-identical builds.

Use a private deployment directory outside Git. The commands below use Bash and
an ordinary Unix operator account. The service UID/GID match the owner of its
0600 key file; do not use UID 0. Substitute your actual hostname before running.
No default account or password exists.

```sh
gateway_deploy="$HOME/.config/mcp-gateway-server"
install -d -m 700 "$gateway_deploy"
gateway_db_password=$(openssl rand -hex 32)
cat > "$gateway_deploy/deployment.env" <<EOF
POSTGRES_PASSWORD=$gateway_db_password
GATEWAY_PUBLIC_URL=https://gateway.example.com
GATEWAY_KEY_FILE=$gateway_deploy/vault-keys.json
GATEWAY_IMAGE=mcp-gateway-server:0.1.0
GATEWAY_PORT=8080
GATEWAY_UID=$(id -u)
GATEWAY_GID=$(id -g)
EOF
chmod 600 "$gateway_deploy/deployment.env"
unset gateway_db_password
docker run --rm --user "$(id -u):$(id -g)" \
  --mount "type=bind,source=$gateway_deploy,target=/keys" \
  mcp-gateway-server:0.1.0 generate-vault-key --output /keys/vault-keys.json
gateway_compose() {
  docker compose --env-file "$gateway_deploy/deployment.env" \
    -p mcp-gateway -f deploy/compose.yaml "$@"
}
gateway_compose up -d --wait postgres
read -r -s -p 'New administrator password: ' gateway_admin_password
printf '\n'
printf '%s' "$gateway_admin_password" | gateway_compose run --rm --no-deps -T \
  gateway bootstrap-admin --username admin --password-stdin
unset gateway_admin_password
gateway_compose up -d --wait gateway
```

Choose a password of at least 12 bytes. Bootstrap refuses a second first-admin
operation. Subsequent admins/users/password resets use the authenticated Users
screen. Persist and separately protect the database password/environment and
vault key file; never commit them or paste `compose config` output into tickets.
On Linux the mounted file must be readable only by the configured service UID.
For another deployment, use a distinct Compose project, key and environment.
Changing an existing PostgreSQL password env value alone does not rotate its
already initialized database password; perform a coordinated database credential
rotation with your operator tooling.

Point DNS at the host and configure your HTTPS proxy. `deploy/Caddyfile.example`
is a same-host Caddy example; replace its hostname to match GATEWAY_PUBLIC_URL.
Caddy can provision certificates automatically for a reachable domain. Use your
enterprise certificate/proxy configuration where appropriate. Preserve the
original Host (including nonstandard port) and Origin through the proxy; the
service rejects mismatches and cross-site browser requests. Protect the internal
HTTP hop with the loopback binding or an equivalent private network. Do not
expose it publicly or enable the hidden fixture HTTP exception.

```sh
curl --fail --silent --show-error https://gateway.example.com/healthz
gateway_compose ps
gateway_compose logs --tail 100 gateway
```

Health performs a database query; startup automatically applies migrations.
Compose waits for a healthy database before starting the service. Health alone
does not prove vault decryption; the recovery rehearsal below checks credential
resolution too. Docker sends SIGINT for graceful Rust shutdown. No MCP command
runs inside this central container. Its logs contain redacted events, never tool
payloads/password/token values. Login throttling uses actual peer IP and username;
clients behind a single reverse proxy share that peer's throttle bucket. Trusted
forwarded-IP interpretation and audit export/retention are later work.

In the HTTPS administrator website, create users and groups/membership, define
MCPs with structured commands or remote URLs/auth registration, then grant to a
user/group/all. New definitions have no assignments and an empty tool allowlist.
Grants add together; removing the last one clears user enablement. Regranting
requires a new opt-in. Tool policies use original names, selected from reported
metadata or entered manually. Denylist mode permits new names except those
excluded; allowlist mode permits selected names only. Administrator permissions
do not expose another user's secret values or execute endpoint commands.

Install the arm64 archive using [Mac installation](mac-install.md), create or
select owned secrets for bearer auth, complete OAuth public-client registration
using [OAuth setup](oauth.md), explicitly install/enable, then configure one
STDIO agent using [agent configuration](agent-configuration.md). Ordinary central
configuration remains plaintext; only secret material is application-encrypted.

## Backup and restore

Back up before upgrades. Store the matching vault key ring through a separate
restricted secret-management backup, outside PostgreSQL and the database archive.
Retain every key required by retained database backups. Test the restore before
changing live routing. Losing the matching keys makes encrypted values unusable.

```sh
umask 077
mkdir -p "$gateway_deploy/backups"
gateway_compose exec -T postgres pg_dump -U gateway -d gateway -Fc \
  > "$gateway_deploy/backups/database.dump"
```

Use a fresh Compose project/empty database for restoration, a separate loopback
port and the matching separately recovered key file. Do not restore over the
running deployment. For example, create another private `restore.env` from the
original settings with `GATEWAY_PORT=18080` and its recovered key path, then:

```sh
restore_compose() {
  docker compose --env-file "$gateway_deploy/restore.env" \
    -p mcp-gateway-restore -f deploy/compose.yaml "$@"
}
restore_compose up -d --wait postgres
restore_compose exec -T postgres pg_restore -U gateway -d gateway \
  --no-owner --exit-on-error < "$gateway_deploy/backups/database.dump"
# Invalidate sessions copied from the restore point before exposing the service.
restore_compose exec -T postgres psql -U gateway -d gateway -v ON_ERROR_STOP=1 \
  -c 'DELETE FROM sessions;'
restore_compose up -d --wait gateway
```

Verify health, admin/user login, assignments/tool restrictions, central enablement,
secret metadata and an authorized credential-backed tool before changing HTTPS
routing. The database retains deployment identity; earlier backups may contain
older policy revisions. Endpoints must logout/re-login after rollback rather
than bypassing protected revision checks. Review policy/account revocations made
after the restore point before reopening access. OAuth provider rotation may have
invalidated backed-up refresh tokens; reconnect those installations explicitly.
Do not blindly replay a failed tool call. A database-only restore with wrong keys
fails credential resolution with a redacted 503; restoring the correct key recovers.

Key rotation is detailed in [vault operations](vault.md): retain old keys, add a
new active key, restart with the combined ring, run `rotate-vault`, inspect key-ID
counts, and keep old backup keys for their retention period. Test recovery before
retiring old key material. Stop/start or `restart gateway` preserves PostgreSQL
state. Never use `down --volumes` on the real deployment unless intentionally
deleting its data; the automated rehearsal uses it only on generated owned stacks.

## Reproducible rehearsal and evidence

On the development Mac with the local image built, Node 24 and an existing owned
absolute build directory:

```sh
GATEWAY_BUILD_CACHE=/absolute/owned/build node scripts/verify-deployment.ts
```

The script creates two UUID Compose projects/volumes and synthetic users/secrets;
it accepts no production DSN/project/key. It checks compiled assets, bootstrap,
roles/default deny, enablement, central ciphertext and redacted logs; restarts the
service, dumps/restores into a fresh database, rejects an independent wrong key
and verifies matching-key recovery. Both stacks/volumes are removed. Failed
runs retain only their private generated recovery directory for diagnosis.
On the validated Mac, additionally set GATEWAY_TEST_CLI to an extracted archive
executable and GATEWAY_FIXTURE_BIN to the built gateway-fixture executable to run
the complete container→packaged login/install/enable→Node STDIO→disable journey.
Its local proxy is a fixture-only internal hop; creating-binary logout verifies
Keychain cleanup. The ordinary Linux CI rehearsal skips this Mac-only addition.
Traffic is the isolated internal HTTP proxy hop with the exact public Host;
this does not claim publicly issued TLS or a real identity provider test.

Verified release results, archive checksum and exact platform evidence are
recorded in [release evidence](release.md). Primary operational references:
[Compose health/start order](https://docs.docker.com/compose/how-tos/startup-order)
and [Caddy reverse proxy](https://caddyserver.com/docs/quick-starts/reverse-proxy).
