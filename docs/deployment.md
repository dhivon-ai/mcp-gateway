# Set up a gateway

Run one gateway for your team. Once it’s running, you can sign in to the admin console and invite a developer.

## Before you start

Use a supported host with Docker, Docker Compose, Bash, Python 3.8 or later and a checkout of this repository. Run the commands as an ordinary user with Docker access. Choose the HTTPS address developers will use, such as `https://gateway.example.com`, and have your first administrator’s email and password ready.

Point the hostname at your host. Configure your HTTPS reverse proxy to forward that hostname to `127.0.0.1:8080`, preserving the Host header. For same-host Caddy, a site block looks like this:

```caddyfile
gateway.example.com {
    reverse_proxy 127.0.0.1:8080
}
```

Replace the hostname with yours. For automatic public certificates, the domain must resolve to your host and ports 80 and 443 must be reachable. The database stays on Docker’s private network.

## Configure once

From the repository root, run:

```sh
bash scripts/configure-gateway.sh
```

Enter the HTTPS address, first administrator’s email and a password of 12–128 bytes at the prompts. The password is hidden. If your domain is internationalized, enter its ASCII spelling (for example, `xn--fa-hia.example.test` for `faß.example.test`). Keep the **private deployment directory** printed by the command. It holds the keys and database credentials your installation needs.

If first-time configuration is interrupted, rerun the helper from the same checkout with the same config location. For an existing deployment, keep using its original checkout and `.env`. If the settings are missing, restore the matching `.env` and private directory from backup.

## Start the gateway

```sh
docker compose up -d --build --wait
```

Compose builds the server, starts PostgreSQL and creates your first administrator. On subsequent starts, use the same command and checkout. Compose reuses your existing account, data and keys.

Open your configured HTTPS address and sign in with the email and password you entered. A successful sign-in opens the admin console. [Give your team access](admin-ui.md) and send developers the [Mac guide](mac-install.md).

To check service availability, visit `/healthz` on the same HTTPS address. Expect HTTP 200. If startup fails, use `docker compose ps -a` and `docker compose logs --tail 50 setup dhivon` to identify the failing service.

## Backup and restore

Back up the database **and** the matching vault before upgrades. Follow [backup, restore and advanced deployment](operations.md) for the commands and checks.
