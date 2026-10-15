# Review Bugs: mac-mvp @ 2026-10-15

- Review cycle: 1
- Status: approved under persisted policy-v2 Autonomous authority on exact promotion
- Reviewed commit range: `4991f838b32bce062fc9cdae6e1df9dd779831c3..4469f1aaba87f2f1d447900da28afc2a78bfe860`

## BUG-001: Credential capacity can corrupt shared endpoint metadata

- Finding ID: `endpoint-metadata-capacity-corruption`
- Severity: P2
- Evidence: `crates/gateway-cli/src/store.rs:181` rejects over 128 upstream heads on reads, while `StateStore::write` at line 273 does not enforce those same invariants. `upstream_credentials.rs::replace` inserts another head without capacity preflight. The isolated actual native publisher probe fails after 129 sequential MCPs with one active/assigned MCP at a time: `identity.json` becomes unreadable.
- Impact: Normal churn accumulates retired MCP heads and blocks shared status, sync and logout; this violates failure isolation and recoverability (NFR-008, FR-016).
- Reproduction: In a single generation, repeatedly install a fresh valid snapshot with one newly identified enabled bearer MCP and publish its version-1 credential; repeat 129 times. Each publication reports success but the final store read returns Invalid. The scratch regression ran to the expected failure in 30.49 seconds; no production state or real Keychain was used.
- Approved remediation acceptance criteria: Every durable metadata write must remain readable under the same bounded invariants. Capacity rejection or safe scoped retirement must leave existing identity/cache/credentials usable and cleanup possible. Verify 128/129 sequential scopes, allocation failure, cleanup and unrelated authorization; preserve protected publication/rollback fences and native token lifetime.
- Human approval: No new human approval claimed. Exact in-scope remediation is bound to original policy-v2 Autonomous execution `2026-10-14T09-21-15-303Z`; final clean completion still requires explicit human approval.

## BUG-002: Valid catalog data breaks browser decoders

- Finding ID: `catalog-decoder-contract-bounds`
- Severity: P2
- Evidence: `gateway-core/src/policy.rs:92` permits a 200-byte name and CommandSpec validation has no 128-byte environment-key ceiling. `apps/web/src/lib/decode.ts:80,112` and `local/contracts.ts::mcp` reject these valid values at 128 characters; catalog name input also caps at 128. Actual core validation accepts both counterexamples; actual TypeScript decoders reject all three admin/local cases.
- Impact: One backend-valid entry prevents entire admin or local catalog loading, blocking normal management (FR-003, FR-022/023, AC-011).
- Reproduction: Decode an otherwise valid 200-ASCII-byte named MCP in both websites, or an admin STDIO command with a 129-byte environment key. All reject with Invalid response contract. Independent Rust test `backend_catalog_contract_accepts_frontend_counterexamples` passes on the same data.
- Approved remediation acceptance criteria: Both websites accept backend-valid bounded catalog values and permit normal management, including 200-byte names and long environment keys. Align form/decoder bounds without tightening existing server data; retain bounded input/response validation and strict TypeScript. Test valid and invalid boundaries through real contracts and visible UI behavior.
- Human approval: Persisted original policy-v2 Autonomous authority; no new human approval claimed.

## BUG-003: Mixed admin read revisions can resurrect removed members

- Finding ID: `admin-read-set-revision-race`
- Severity: P2
- Evidence: `apps/web/src/admin/app.tsx:164` and `refreshData` combine independent configuration/users/groups reads with Promise.all. `admin/identities.tsx:313` freezes `data.config.revision` independently of the initial roster/role values. Real PostgreSQL/API probe observed old group member count 2, concurrent revocation to 0, newer config revision 12, then a UI-shaped rename accepted with the stale roster and restored both members.
- Impact: An administrator intending only a name/role-field change can silently restore another administrator's revoked membership/account state despite revision CAS, violating NFR-004.
- Reproduction: During a load, let GET groups finish first. Another admin removes its members and commits. Let GET configuration then return the newer revision. Open the old group, rename it, and save its original memberIds with the newly loaded revision. The server accepts the write and re-adds removed members. The same read shape applies to user roles/active state and to post-write refresh.
- Approved remediation acceptance criteria: All editor initial values must belong to the revision used for writes. Use one atomic or verified consistent read set in initial and follow-up loading; concurrent changes produce coherent data or explicit reload/conflict, with no automatic write retry. Deterministic tests must interleave revoked membership/changed account data and prove no silent restoration; confirmed writes followed by failed reads remain clearly distinguished.
- Human approval: Persisted original policy-v2 Autonomous authority; no new human approval claimed.

## BUG-004: Catalog editing changes literal structured command arguments

- Finding ID: `catalog-command-round-trip`
- Severity: P2
- Evidence: `apps/web/src/admin/catalog.tsx:49` joins arguments/environment entries by newline and `parseCommand` splits them at line 65. Every save, including Save policy, rebuilds both commands from this representation. Actual source functions turn a single multiline Python -c argument into two separate arguments; a multiline environment value prevents an unrelated save. A single empty argument also becomes no arguments.
- Impact: Saving tool policy or an unrelated catalog field can change a working runtime/installation command, or make a valid entry uneditable (FR-003/022, AC-011).
- Reproduction: Open a backend-valid command with args `[-c, "print(first)\nprint(second)"]` and save without command edits. The draft/parse round trip splits the second argument. An env value containing a newline throws Environment entries must use KEY=value; valid empty-string argument `[""]` round-trips to `[]`.
- Approved remediation acceptance criteria: Runtime and installation argument arrays/environment maps must round-trip exactly, including empty strings, literal newlines and special characters, during unchanged, policy-only and unrelated saves. Explicit command edits must use a clearly described lossless structured representation with validation. Cover both commands and user-visible save payloads; preserve shell-free execution semantics and accessible forms.
- Human approval: Persisted original policy-v2 Autonomous authority; no new human approval claimed.
