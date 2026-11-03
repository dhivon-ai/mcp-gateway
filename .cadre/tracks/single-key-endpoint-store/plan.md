# Plan: Single Keychain master key for all local gateway state

- Track: `single-key-endpoint-store`
- Spec revision: 1
- Plan revision: 3
- Status: proposed

## Phase 1: Prove the single-key encrypted store

- Phase dependencies: none

### Outcome

One Keychain item can protect replay-resistant local files across real Mac processes and crashes. Gate integration on proof that existing readers cannot use a superseded key after a committed transition.

- [x] T1.1 Implement the private versioned store, bootstrap/identity key separation, component encryption, atomic root-key transition, bounded locked reads and recovery boundaries. Inventory persistent state and distinguish reconstructible server data from local-only evidence. <!-- commit: 2357e8a5f153046d5be65f108027a004988b254a -->
  - Task dependencies: none
  - Commit group: single-key-store
- [x] T1.2 Prove key count, permissions/redaction, full directory rollback denial, crash boundaries and stale-reader behavior with real isolated Keychain subprocesses; test ciphertext substitution and corrupt-record containment. Stop for design revision if the one-item protocol cannot preserve freshness. <!-- commit: 2357e8a5f153046d5be65f108027a004988b254a -->
  - Task dependencies: T1.1
  - Commit group: single-key-store
- [x] T1.3 User Manual Verification <!-- commit: 2357e8a5f153046d5be65f108027a004988b254a -->

- Phase completion commit: `2357e8a5f153046d5be65f108027a004988b254a`

## Phase 2: Move endpoint state and recover existing installations

- Phase dependencies: P1

### Outcome

UI and STDIO use the same encrypted state. The server can repair authorized data and existing installations can cut over safely.

- [x] T2.1 Integrate identity/session, cache and clocks, static/OAuth credential heads and recovery journals, install receipts, scoped logout and status. Retain current generation, lease, policy and rejection checks in every runtime entry point. <!-- commit: dec44aa3e23005dbccecc2eca15373615a1a1f44 -->
  - Task dependencies: none
  - Commit group: endpoint-cutover
- [x] T2.2 Implement bounded refetch and rekey, login recovery, compatible migration and idempotent cleanup. Test concurrent callers, interruption, offline restart, old-format ACLs, unavailable Keychain, rejected/expired credentials, pending OAuth recovery, local-only receipts and isolation. <!-- commit: dec44aa3e23005dbccecc2eca15373615a1a1f44 -->
  - Task dependencies: T2.1
  - Commit group: endpoint-cutover
- [x] T2.3 User Manual Verification <!-- commit: dec44aa3e23005dbccecc2eca15373615a1a1f44 -->

- Phase completion commit: `dec44aa3e23005dbccecc2eca15373615a1a1f44`

## Phase 3: Verify the packaged gateway and document recovery

- Phase dependencies: P2

### Outcome

A reproducible Mac rehearsal proves the storage boundary through public CLI entry points.

- [x] T3.1 Run pinned formatting, Clippy, workspace build/tests, relevant real database, agent, identity, vault and OAuth regressions. Rehearse the extracted CLI in a generated scope for normal use, legacy upgrade, tamper, recovery, offline denial and logout. Check count and contents with synthetic data only; keep creating artifacts until verified cleanup. <!-- commit: 43bae33639e41bd0154486fc3436df6945c092be -->
  - Task dependencies: none
  - Commit group: verification-and-guides
- [x] T3.2 Update canonical guides and verification harness assumptions; check rendered documentation and relevant repository checks. Record actual test environment, evidence and limits, map each acceptance criterion to evidence, and complete Definition of Done. <!-- commit: 43bae33639e41bd0154486fc3436df6945c092be -->
  - Task dependencies: T3.1
  - Commit group: verification-and-guides
- [x] T3.3 User Manual Verification <!-- commit: 43bae33639e41bd0154486fc3436df6945c092be -->

- Phase completion commit: `43bae33639e41bd0154486fc3436df6945c092be`

## Phase 4: Track-level User Manual Verification

### Outcome

Verify AC-001 through AC-005 using the packaged Mac flow and recorded failure/upgrade evidence.

- [x] T4.1 User Manual Verification <!-- commit: 43bae33639e41bd0154486fc3436df6945c092be -->

- Phase completion commit: `43bae33639e41bd0154486fc3436df6945c092be`

## Phase 5: Protect credential refusals and restore sign-in on fresh installations

- Phase dependencies: P4

### Outcome

A known server refusal remains effective when damaged credential files are restored. A fresh-format installation can open the console after losing its local files without trusting those files to authenticate. Verification uses only the current storage format; no migration or compatibility work is requested.

- [x] T5.1 Persist a scoped denial even if the bearer head and its recovery evidence are both unreadable. Bind it to the current identity, generation, MCP and binding, preserve unrelated grants, and require fresh authorized resolution before reuse. Test a real authenticated 403 followed by restoration of both files, an outage and process restart; the old bearer must stay unavailable. Also verify a delayed denial cannot affect a new identity or binding. <!-- commit: cf7fcdf5721f93030043b5f0f310ad407197758b -->
  - Task dependencies: none
  - Commit group: durable-credential-denial
- [x] T5.2 Let a current-format console reach independent sign-in when the master is present but the private state directory is lost. When the encrypted index and identity are healthy, handle a damaged ordinary format marker without discarding them. Preserve ownership and symlink checks, deny offline access to unverified data and do not replace an inaccessible Keychain item. Test the packaged UI at an explicitly supplied server, failed and successful authentication after loss, and continued use of healthy encrypted state after marker damage. <!-- commit: 77a5d397866b580d33e800938ef467f4b22d4242 -->
  - Task dependencies: none
  - Commit group: current-store-recovery
- [x] T5.3 User Manual Verification <!-- commit: 519aa1973ced5137070d630c995cd446ee0504c0 -->

- Phase completion commit: `519aa1973ced5137070d630c995cd446ee0504c0`

## Phase 6: Track-level User Manual Verification

### Outcome

Verify BUG-001 and BUG-002 on the current Mac package, including the scoped denial after disk restore and recovery through the loopback console. Record targeted regression results, fixture cleanup, and any remaining limits.

- [x] T6.1 User Manual Verification <!-- commit: 519aa1973ced5137070d630c995cd446ee0504c0 -->

- Phase completion commit: `519aa1973ced5137070d630c995cd446ee0504c0`

## Phase 7: Close first-fetch and damaged-directory recovery gaps

- Phase dependencies: P6

- [ ] T7.1 Record refusals for an MCP before its first credential is cached. Reproduce the overlapping authenticated 200 and 403, deny late publication and offline reuse after restart, then verify a new authorized pull and an unrelated grant.
  - Task dependencies: none
  - Commit group: first-fetch-refusal
- [ ] T7.2 Allow independent sign-in when the current encrypted index and ordinary marker are both damaged. Rehearse the packaged UI with empty, malformed and oversized markers, a failed login, a successful rebuild and denial of replay. Keep unsafe filesystem and Keychain access fail-closed.
  - Task dependencies: none
  - Commit group: damaged-bootstrap-recovery
- [ ] T7.3 User Manual Verification

- Phase completion commit: pending

## Phase 8: Track-level User Manual Verification

- [ ] T8.1 User Manual Verification

- Phase completion commit: pending

## Approval

Pending combined specification and plan approval.
