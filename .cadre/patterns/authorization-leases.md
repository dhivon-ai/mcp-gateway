# Pattern: Authorization leases with identity and generation fences

## Use when

An endpoint executes centrally authorized work during bounded outages while local
processes and asynchronous requests share one selected account.

## Guidance

- Protect deployment, stable user/installation IDs, login generation, revision,
  validation and expiry as an authenticated authorization snapshot.
- Renew only after a complete authenticated response passes validation and durable
  publication. Heartbeats, failed/partial requests, reads, file rewrites and token
  refresh never renew policy. Charge request and persistence time against the lease.
- Enforce the earlier wall/session/token/native deadline; expire at equality.
  Retain protected receipt/rollback watermarks, a sleep-aware native clock and boot
  identity. Require online recovery when freshness cannot be established safely.
- Pin the original principal/generation before I/O and recheck under the persistence
  lock. Late login/sync results cannot replace a newer login/logout. Distinguish a
  known session rejection, which invalidates immediately, from an outage.
- Consume the latest shared snapshot before every list/call; fence completion and
  cancel/retire owned work on expiry or learned revocation. Never replay calls.
  Offline settings/install/secret/new OAuth writes are prohibited, without a queue.

## Tradeoffs and limits

This project uses a 3,600-second ceiling, 60-second polling and 10-second requests;
new designs must explicitly select and test bounds. The OS/Keychain is trusted.
Actual Mac restart and conservative clock simulations were verified; actual
suspend/reboot and other endpoints were not. Cancellation cannot undo side effects.

## Provenance

- Track: `mac-mvp`; clean completion approved in `42e27c246d7869a94ed67098eb3da4bc6a395532`.
- Phase/task: P1/T1.2–T1.3, P3/T3.3–T3.6, P4/T4.3–T4.4.
- Commits: `2be53f911eb3f3d89db4278541d3eff36736ff08`, `cd2d02ab4881ccbcb9a0981af9e083aaf504bbbc`, `3c0de2000a830cf788851658164d366b2585cb10`, `fa504880b4d89cd26df3661c6ddab9bf5c981a2d`.
- Source/evidence: `crates/gateway-core/src/policy.rs`; CLI `cache.rs`, `clock.rs`, `sync.rs`, `session.rs`, `runtime.rs`; cache/identity/gateway tests for exact expiry, replay, partial publication, stale generation and actual Mac subprocess restart.
- Archived at: 2026-10-15 (proposed archive batch).
