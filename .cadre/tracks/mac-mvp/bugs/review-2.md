# Review Bugs: mac-mvp @ 2026-10-15

- Review cycle: 2
- Status: approved under persisted policy-v2 Autonomous authority on exact promotion
- Reviewed commit range: `4991f838b32bce062fc9cdae6e1df9dd779831c3..b3ea6ab52ce58249946831d67cba30106cbd8f90`

## BUG-002: Valid administrator collections still exceed browser decoder bounds

- Finding ID: `catalog-decoder-contract-bounds` (recurring; preserve identity)
- Severity: P2
- Evidence: `apps/web/src/lib/decode.ts:52` defaults every array to10,000 rows; configuration assignments and `admin/read-data.ts` users/groups use that default. `gateway-server/src/identity.rs::list_users/list_groups` fetch all records and their create handlers have no total row limit. `configuration.rs::create_assignment` likewise has no total assignment cap; the1MiB catalog budget covers definitions only. Server group membership separately has an explicit10,000-member bound and must retain it. Secret store256 and MCP512 bounds are unrelated.
- Impact: A legitimate enterprise read set (e.g.500 users assigned21 MCPs) prevents every admin screen from loading after the10,001st assignment. More than10,000 users/groups fail similarly (FR-001/003/022, AC-001/011). This is the same backend/frontend contract mismatch as BUG-002; its name/environment cases were fixed but collection coverage was incomplete.
- Reproduction: Run actual TypeScript decoders on shaped user/group/assignment arrays at10,000 and10,001: each accepts10,000 and throws Invalid response contract at10,001. Source confirms these central response collections are unbounded; no production or secret data used.
- Approved remediation acceptance criteria: Accept all server-valid admin collection lengths without introducing a smaller arbitrary limit or tightening existing backend data. Retain actual bounded contracts and validate every element. Regress10,001+ valid collections, malformed late elements and over-limit actual bounded group membership/MCP/command/tool data. Verify revision-bracketed initial/follow-up reads remain coherent. Rebuild both embedded artifacts and validate current package/deployment journeys; no new performance/large-scale guarantee is claimed.
- Human approval: No new human approval claimed. Exact in-scope remediation binds to original policy-v2 Autonomous execution2026-10-14T09-21-15-303Z; final clean completion requires explicit human approval.

## Resolved prior findings

- BUG-001 `endpoint-metadata-capacity-corruption`: shared read/write validation and preflight128/129 regression preserve readable state/cache/credentials/logout; native/fullchecks verified.
- BUG-003 `admin-read-set-revision-race`: dependent configuration reads bracket parallel identity reads for initial and post-save loads; deterministic races and real service/browser409/reload proof pass.
- BUG-004 `catalog-command-round-trip`: JSON arrays/maps preserve literal runtime/installation values and policy-only saves; behavioral and actual browser/backend equality proof pass.
- First remediation execution2026-10-14T23-00-57-037Z is counted once for each attempted finding. Original discovery is not counted. Cumulative review reuses unchanged full83Rust/8realPG/34frontend/native/browser/deploy/restore evidence with source checks. No finding is waived.
