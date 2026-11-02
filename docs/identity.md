# Central identity API

The service has no default users or passwords. Set `DATABASE_URL` and
`DHIVON_MCP_GATEWAY_PUBLIC_URL` (the externally visible HTTPS origin), then run:

```sh
dhivon-mcp-gateway-server bootstrap-admin --email admin@example.com --name "Administrator" --password-stdin
```

Supply the password through standard input, using a password manager or a
terminal prompt; never place it in shell arguments/history. Bootstrap only works
while the deployment has no users. Password creation/reset requires 12–128 bytes.
Email is the only login identifier; Name is display-only. Emails are trimmed and
ASCII-case-folded, unique, and limited to254bytes. Supported addresses use an
unquoted ASCII dot-atom local part (up to64bytes) and hostname domain labels;
quoted addresses, Unicode mailboxes and domain literals are unsupported. Names
are trimmed Unicode, nonempty, at most200UTF-8 bytes and free of control characters;
duplicate display names are allowed. Group names are trimmed. Passwords use
salted Argon2id (19 MiB, two iterations, parallelism one).

Native clients POST email/password to `/api/endpoint/login`. The response is a
private endpoint session containing an opaque token, deployment ID, user and
absolute expiry. The Mac stores it in an encrypted file under its one Keychain master key. Browser clients first GET
`/api/context` to establish an HttpOnly pre-login cookie and obtain its CSRF
challenge, then POST `/api/auth/login` with `X-CSRF-Token`. Login returns context,
with the session token confined to an HttpOnly SameSite=Strict cookie (Secure on
HTTPS). Browser mutation requests require the session CSRF challenge.

Sessions expire after 12 hours and polling never extends them. Only SHA-256 token
hashes are stored centrally. Logout revokes one session; password reset and any
account name/email/role/active edit revoke all target sessions. Disabled accounts cannot log
in. The last active administrator cannot be demoted/disabled. Identity edits and
policy changes serialize under the deployment revision lock, rechecking the
administrator's live authority in the transaction.

Admin-only routes:

All user/group writes also require the deployment's `expectedRevision` from
`GET /api/admin/configuration`, including a JSON body on group deletion.
Stale identity/group changes return 409 without overwriting concurrent edits.

| Route                                | Behavior                                                |
| ------------------------------------ | ------------------------------------------------------- |
| GET/POST `/api/admin/users`          | List metadata / create with name, email, password, role |
| PUT `/api/admin/users/{id}`          | Set name/email/role/active; revoke target sessions      |
| PUT `/api/admin/users/{id}/password` | Reset password and revoke target sessions               |
| GET/POST `/api/admin/groups`         | List / create name and memberIds                        |
| PUT/DELETE `/api/admin/groups/{id}`  | Replace name/membership / delete                        |

Unauthorized requests return 401, role/CSRF/origin rejection 403, malformed input
400, duplicate email400; other duplicates/last-admin protection409, throttling 429. Public errors are typed
and redact JSON parser/database details. Responses never include password hashes.
Events carry actor/target IDs and action names, without credentials or payloads.
Login is bounded to five attempts per canonical email/minute and 100 per actual peer
IP/minute, with a bounded Argon2 verification pool. Forwarded IP headers are not
trusted; a reverse proxy shares a conservative peer-IP limit.

The configured Host/Origin must match exactly. HTTPS termination must retain the
public Host header. HTTP is available only via the explicit loopback fixture
switch and cannot target non-loopback hosts.

## Fresh identity setup

New Dhivon deployments use a fresh database and separate native state namespace.
Bootstrap requires `--email`, `--name` and `--password-stdin`. Creation/edit APIs
use distinct display name and email fields. Existing populated historical
username schemas are refused safely during schema initialization. The current
setup starts from the documented fresh deployment and email sign-in path.
Stable user IDs remain unchanged through ordinary profile edits.

An in-flight login rechecks its original email under the final row lock after
password verification. An email edit cannot let an already-looked-up old address
create a new session. Password reset/disable and last-admin protections remain.
