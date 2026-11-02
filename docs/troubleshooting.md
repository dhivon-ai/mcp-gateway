# Troubleshooting

Start with the local **Status** screen or the redacted CLI status command.

```sh
dhivon-mcp-gateway status
```

Record the problem's time, deployed version, MCP name, current revision and fixed
error category. Share redacted diagnostics and expected behavior. Keep passwords,
secret values, tokens, tool arguments and results out of tickets and logs.

## An assigned MCP is missing

**Diagnose.** Confirm the developer signed in to the intended enterprise/account.
Ask an administrator to check that the definition is active and assigned directly,
through current group membership or to all users. A catalog entry alone grants
nothing. Check the local refresh result and revision.

**Recover.** Correct the intended assignment centrally, then refresh local
management online. If the final grant was removed and later restored, explicitly
enable the MCP again.

**Expect.** The MCP appears in the assigned catalog. Its native readiness and
central enablement remain visible as separate states. Follow the
[quickstart](quickstart.md) to test a permitted tool.

## The MCP is enabled but its tool is absent or denied

**Diagnose.** Check the original tool name in the administrator's allowlist or
denylist. An empty allowlist blocks every tool. Compare native authentication and
executable readiness, then inspect discovery status. The agent sees a generated
alias while policy uses the original upstream name.

**Recover.** Have the administrator permit the intended original name, refresh
local state and restart discovery in the agent. After confirmed disablement or
revocation, treat a retained alias as stale.

**Expect.** Only currently eligible tools appear. Direct calls to denied or old
aliases fail before upstream dispatch. See [configuration](configuration.md) and
[aggregation](gateway-aggregation.md).

## Offline or expired policy lease

**Diagnose.** Inspect validation time, expiry and remaining time in Status. A
failed refresh does not reset these values. The lease may end earlier than an
hour because a session or credential expires first. A cache-integrity or unknown
clock diagnostic also requires online recovery.

**Recover.** Restore access to the configured enterprise HTTPS origin and request
a complete refresh. If the session was rejected or expired, sign in again and
restart the agent's gateway process for the new login generation.

**Expect.** A complete successful sync reports current authorization and its new
validation/expiry. Eligible tools become usable. Settings/install/secret/new OAuth
changes remain unavailable until online confirmation. See
[policy cache](endpoint-cache.md) and [security](security.md#policy-leases-and-offline-behavior).

## Bearer credentials are unavailable or rejected

**Diagnose.** Check the MCP's configured auth mode, selected owned secret and
native status category such as `auth_required`, `auth_rejected` or `token_expired`.
Confirm the upstream accepts that token at the exact configured URL. A foreign
secret, stale binding or damaged credential record cannot fall back to another token.

**Recover.** Replace the value through Secrets if needed, select the intended
owned key in Manage, refresh online and review enablement. Reconnect the affected
MCP. For the local credential-capacity diagnostic, online logout/sign-in starts a
fresh generation and clears retired scoped entries.

**Expect.** Current binding metadata and native authentication readiness agree,
and a safe permitted call succeeds. Secret values remain write-only. See
[native credentials](remote-credentials.md).

## OAuth consent or refresh failed

**Diagnose.** Compare the provider's public-client registration, scopes, resource,
issuer and exact loopback callback with the definition and management port.
Inspect confirmed connection state after an interrupted response. An issued
provider token may already have been persisted even when the browser lost its
response.

**Recover.** Reload confirmed state. Follow the existing recovery status before
starting another consent. When reconnection is required, use Connect explicitly
with the registered port and complete provider consent. Review enablement after
connection succeeds.

**Expect.** Connected credential metadata and server-confirmed enablement appear
separately. Refresh cannot extend the policy lease. See [OAuth setup](oauth.md)
for the supported profile and complete-response recovery.

## Installation failed or enablement was not confirmed

**Diagnose.** Read fixed progress for missing executable, working directory,
permission, cancellation or timeout. An installer may have made partial local
changes. Distinguish failed installation from successful installation followed
by a failed central enable response.

**Recover.** Repair prerequisites and review existing local effects before
explicitly reinstalling. When installation succeeded, refresh confirmed state
and retry enablement separately without rerunning the installer.

**Expect.** Installation and confirmed enablement have separate outcomes. Failed
or interrupted installation never silently enables the MCP. See
[endpoint management](endpoint-management.md#installation-and-recovery).

## A save reports a revision conflict

**Diagnose.** Another administrator or endpoint changed the deployment revision.
The editor keeps the revision it opened with. A failed follow-up read can also
require reloading after a confirmed write.

**Recover.** Use **Reload current state**, compare current values with the intended
change and submit again only after review. Do not repeatedly resend a write or
installer action with guessed revisions.

**Expect.** The next save uses a current coherent state and reports confirmation.
See [administrator editing](admin-ui.md).

## The website or service will not start

**Diagnose.** Check the exact service and origin in the private deployment
configuration. Use the `gateway_compose` helper defined in the
[deployment guide](deployment.md), from the repository root.

```sh
gateway_compose ps
gateway_compose logs --tail 100 dhivon
```

Confirm PostgreSQL is healthy, the configured UID can read the private vault key,
and the proxy preserves the public Host/Origin. An asset-bundle diagnostic means
the Rust executable was compiled without the current production frontend.

**Recover.** Correct the indicated configuration or rebuild the embedded artifact.
For a restored database with credential-resolution 503, restore the matching key
set with owner-only permissions and run the recovery verification. Keep the
database and key backups available throughout recovery.

**Expect.** Health returns 200, the branded sign-in page loads, and an authorized
credential-backed call succeeds. Health alone does not establish vault recovery.
See [deployment](deployment.md) and [vault operations](vault.md).
