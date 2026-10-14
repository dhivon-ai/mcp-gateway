# Authoritative configuration

`GET /api/admin/configuration` returns the catalog, assignments, latest observed
tool names, and a deployment-wide revision. Administrator writes to
`/api/admin/mcps` and `/api/admin/assignments` require `expectedRevision`; deletion
also takes a JSON revision body. Catalog creation supplies a stable UUID in the
definition. Edit paths and definition IDs must match. Assignment IDs are created
by the service. Writes validate remote HTTPS, structured commands, authentication
combinations, target existence, and tool-name bounds. A deployment supports up to
512 catalog entries and 1 MiB of serialized definitions in this MVP, leaving
bounded space for snapshot identity, enablement and secret references.

Creating an MCP creates no grant or enablement. User, group and all-user grants
combine additively. Empty allowlists expose no tools. Original upstream names are
the policy identifiers. Removing the last grant, removing group membership,
deleting a group, or disabling an MCP deletes affected opt-ins transactionally.
Restoring access requires a new explicit enable action. Changing the upstream
transport or authentication mode clears opt-ins and bindings, preventing an old
credential selection from following a changed endpoint. Policy/name edits retain
otherwise permitted enablement.

`GET /api/config` returns the authenticated user's complete assigned active
catalog, enablement, secret metadata references, revision, deployment/user IDs,
server validation time, and absolute expiry. It contains no secret values.
Expiry is the earlier of validation time +3,600 seconds and session expiry.
Every successful complete refresh issues a new lease, even when the revision is
unchanged. Endpoint code must validate and durably install it before freshness
is renewed; connecting or fetching credentials is insufficient.

`PUT /api/settings/{mcpId}` accepts `expectedRevision`, `enabled`, and nullable
`secretId`. It changes only the authenticated user's state and requires current
assignment. Bearer enablement requires an owned secret; other auth modes reject
bearer bindings. Disabling removes the binding. Successful mutations return a
complete confirmed snapshot. Revision conflicts return 409 with the current
revision **and current snapshot**, for explicit reconsideration. No blind retry
or deferred write is allowed. Revisions are deployment-wide, so an unrelated
user's change can also require a refetch in this MVP.

Transactions serialize revision changes and revalidate live sessions within the
same authority lock. No hashing or upstream network operation runs while that
lock is held. Configuration and enablement are ordinary PostgreSQL data; secret
references have a composite ownership foreign key. Migrations preserve stable
deployment identity, revisions and opt-ins across service restarts.

Assigned native endpoints may POST `/api/endpoint/inventory` with `mcpId` and
bounded original `names`. Reports are deduplicated and contain no schemas,
arguments, results or credentials. The admin read returns one latest bounded
observation per MCP. Reports do not increment the policy revision, enable a
server, create grants, or alter tool policy. Unknown/unassigned reports are
rejected. Names remain untrusted metadata; UI rendering must use text nodes.
