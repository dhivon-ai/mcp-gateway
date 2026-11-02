# Review Bugs: dhivon-brand-site-docs — cycle 3

- Review cycle: 3.
- Status: proposed; awaiting human approval of this complete report and remediation.
- Reviewed execution: `2026-11-02T09-16-20-231Z`, Track mode, spec 1 / plan 3.
- Reviewed commit range: `04656bde24e9b025d8183cfbc47e52540a9bb9a1..5caf554972acc253c62ccb968afc770e10ebc740`.
- Current HEAD: `193f4ba7202f06ba457ffc738e18e57b11db8007`, implementation completion bookkeeping only after the reviewed HEAD.
- Review date: 2026-11-02.

## Coverage and verification

Reviewed the complete execution range, all seven changed product files, current
specification/plan/state/learning/dependency context, prior findings and affected
supervision, Docker, native-logout, documentation and test paths. Reused unchanged
workflow, patterns/styleguides and earlier product context after Git verification.
P1–P11 completion markers, task identities and historical provenance are preserved.

The existing actual Docker/Mac suite passes 11/11 again in 149.99 seconds. It
covers SIGINT/SIGTERM before and after actual native login, the original HTTPS
entry point's early interruption, normal and ordinary failures, diagnostic failure,
incomplete native/stack cleanup and repeated recovery. Strict script TypeScript,
format and actual archive/image/site project/Manrope notice comparisons pass.
Product Rust/UI/license inputs are unchanged; prior HTTPS/OAuth and artifact
identities remain applicable, with their documented public-provider limits.
BUG-008's original signal leak is prevented in those exercised paths. No new
production OAuth or licensing defect was confirmed.

Two additional process-level probes expose the following gaps. Evidence lives in
`review3-llrmvais/` under the completed execution-owned build cache. They use exact
generated private scopes. No production deployment/credential target or TLS trust
is accepted or changed. All 13 review scopes have no remaining generated Docker
containers/networks/volumes. The exact probe child and recovery processes were
reconciled after evidence capture.

## BUG-009: Failed child identity registration can falsely report clean cleanup

- Severity: P2.
- Evidence: `scripts/rehearsal-lifecycle.ts:278-284` spawns before calling fallible
  `identity(p.pid)` and only then registers the actual handle. If lookup throws,
  the persisted intent still has `owner: null`. `:372-375` treats that missing
  owner/identity as successful absence. The aggregate cleanup then reports clean
  and permits native logout and stack removal.
- Impact: a live owned fixture/native child can escape shutdown while the result
  claims all children were removed. A native writer could remain during logout;
  that credential race is a source-based implication, not a claim that this probe
  logged in or observed a leaked credential.
- Reproduction: in a generated private Rehearsal, use a process-local PATH wrapper
  that fails exactly the first spawned child's `ps` identity lookup while leaving
  supervisor lookup intact. Spawn a private idle Node child, catch the expected
  registration failure, restore the real PATH, and invoke ordinary cleanup.
- Observed: `identityCaptureThrew: true`, `owner: null`,
  `cleanupReturnedClean: true`, `ownedChildAlive: true`. The retained result says
  `cleanupStatus: clean` and the children step is successful. The reviewer verifies
  the exact generated script/PID, terminates only that child and confirms absence.
- Evidence artifacts: `identity-probe.ts`, `identity-probe-result.json` and its
  generated `ownership.json`/`result.json`.
- Proposed remediation acceptance criteria:
  1. Own the real child handle/PID before fallible identity capture or persistence.
     Registration failure must stop/reap that known child or remain explicitly
     unresolved; no missing owner/intention may be assumed absent during recovery.
  2. Never grant native cleanup authority or a clean result until owned writers
     are confirmed stopped; preserve PID-reuse protection and exact recovery data.
  3. Add deterministic process regressions for identity lookup and post-spawn
     persistence failure and interrupted registration. Verify actual absence or
     truthful `recovery_required`, including preservation of unrelated processes.
- Human approval: pending for this exact report and plan 4 proposal.

## BUG-010: Recovery lock reclamation can replace a live owner and strand retries

- Severity: P2.
- Evidence: `scripts/rehearsal-lifecycle.ts:109-121` reads a lock, checks the old
  owner's liveness, then unconditionally unlinks the path and recreates it. It
  does not verify that the path still names that inspected stale lock. `:121-124`
  also creates an empty lock before obtaining/writing the new owner. Release at
  `:558` unlinks the path without proving current ownership.
- Impact: two recoveries can both enter cleanup for the same namespace, allowing
  overlapping native logout, Compose teardown and writes to the shared fixed
  `.tmp` journal path. Separately, a crash during acquisition leaves a zero-byte
  lock that prevents the documented recovery command from retrying. Concurrent
  corruption was not induced; the confirmed result is simultaneous ownership.
- Reproduction, concurrency: create a generated scope in a short-lived creator so
  its lock is stale. Start two recovery processes. A process-local `ps` barrier
  makes both inspect that same stale owner. Release A until it has acquired its
  replacement lock, then release B using its earlier stale observation.
- Observed: both recovery constructors succeed while both processes are alive.
  Lock ownership changes from PID 29825 to PID 29826, replacing A's live lock.
  The reviewer ends only B, allows A to clean its exact generated scope and confirms
  both probe processes exited. No native login or container startup is needed.
- Reproduction, crash: pause owner lookup immediately after exclusive creation
  opens the new lock, force-stop that exact generated recovery process, and invoke
  the normal recovery CLI with real PATH. The lock is zero bytes; retry exits 1
  with `SyntaxError` before cleanup. After saving the empty bytes/error, the
  reviewer reconciles only the exact empty probe lock and verifies cleanup exits 0.
- Evidence artifacts: `lock-probe.py`, `lock-probe-result.json`, private recovery
  logs, `empty-lock-probe.py`, `empty-lock-probe-result.json`, preserved empty-lock
  bytes and retry error log.
- Proposed remediation acceptance criteria:
  1. Make acquisition/stale reclamation exclusive across concurrent processes.
     A contender cannot unlink a successor's live lock based on an old observation.
  2. Make interrupted lock publication recoverable without deleting a live or
     unrelated owner's lock; release only ownership still held by this recovery.
  3. Add deterministic simultaneous-stale-contender, interrupted/partial-lock and
     old-owner-release regressions. Assert at most one cleanup owner, safe retry,
     intact journals and unchanged unrelated resources and credential namespaces.
- Human approval: pending for this exact report and plan 4 proposal.

## Proposed disposition

Approve both findings in this one review cycle and the exact staged plan 4,
reassessed learning, dependency context and proposed changes-requested state.
Append P12 for cohesive ownership/lock remediation and tests, then final P13 for
Track-level User Manual Verification. Preserve completed P1–P11 and spec revision 1.
The canonical track remains `ready_for_review` until approval. No product fixes,
new implementation execution or canonical review transition have been made.
