# Plan: Single Keychain master key for all local gateway state

- Track: `single-key-endpoint-store`
- Spec revision: 1
- Plan revision: 1
- Status: proposed

## Phase 1: Prove the single-key encrypted store

- Phase dependencies: none

### Outcome

One Keychain item can protect replay-resistant local files across real Mac processes and crashes. Gate integration on proof that existing readers cannot use a superseded key after a committed transition.

- [ ] T1.1 Implement the private versioned store, bootstrap/identity key separation, component encryption, atomic root-key transition, bounded locked reads and recovery boundaries. Inventory persistent state and distinguish reconstructible server data from local-only evidence.
  - Task dependencies: none
  - Commit group: single-key-store
- [ ] T1.2 Prove key count, permissions/redaction, full directory rollback denial, crash boundaries and stale-reader behavior with real isolated Keychain subprocesses; test ciphertext substitution and corrupt-record containment. Stop for design revision if the one-item protocol cannot preserve freshness.
  - Task dependencies: T1.1
  - Commit group: single-key-store
- [ ] T1.3 User Manual Verification

- Phase completion commit: pending

## Phase 2: Move endpoint state and recover existing installations

- Phase dependencies: P1

### Outcome

UI and STDIO use the same encrypted state. The server can repair authorized data and existing installations can cut over safely.

- [ ] T2.1 Integrate identity/session, cache and clocks, static/OAuth credential heads and recovery journals, install receipts, scoped logout and status. Retain current generation, lease, policy and rejection checks in every runtime entry point.
  - Task dependencies: none
  - Commit group: endpoint-cutover
- [ ] T2.2 Implement bounded refetch and rekey, login recovery, compatible migration and idempotent cleanup. Test concurrent callers, interruption, offline restart, old-format ACLs, unavailable Keychain, rejected/expired credentials, pending OAuth recovery, local-only receipts and isolation.
  - Task dependencies: T2.1
  - Commit group: endpoint-cutover
- [ ] T2.3 User Manual Verification

- Phase completion commit: pending

## Phase 3: Verify the packaged gateway and document recovery

- Phase dependencies: P2

### Outcome

A reproducible Mac rehearsal proves the storage boundary through public CLI entry points.

- [ ] T3.1 Run pinned formatting, Clippy, workspace build/tests, relevant real database, agent, identity, vault and OAuth regressions. Rehearse the extracted CLI in a generated scope for normal use, legacy upgrade, tamper, recovery, offline denial and logout. Check count and contents with synthetic data only; keep creating artifacts until verified cleanup.
  - Task dependencies: none
  - Commit group: verification-and-guides
- [ ] T3.2 Update canonical guides and verification harness assumptions; check rendered documentation and relevant repository checks. Record actual test environment, evidence and limits, map each acceptance criterion to evidence, and complete Definition of Done.
  - Task dependencies: T3.1
  - Commit group: verification-and-guides
- [ ] T3.3 User Manual Verification

- Phase completion commit: pending

## Phase 4: Track-level User Manual Verification

### Outcome

Verify AC-001 through AC-005 using the packaged Mac flow and recorded failure/upgrade evidence.

- [ ] T4.1 User Manual Verification

- Phase completion commit: pending

## Approval

Pending combined specification and plan approval.
