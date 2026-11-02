# Pattern: Fail-closed rehearsal ownership and exclusive recovery

## Use when

A verification harness creates processes, native credentials and container state,
and interruption or a later recovery must clean only its own resources.
Use alongside isolated release verification; this is not a production supervisor.

## Guidance

- Generate private scopes and persist intent before effects. Retain the immutable
  creating executable, scoped native state and exact configuration hashes. Refuse
  production targets, changed inputs, active supervisors and unrelated resources.
- Register the actual ChildProcess handle and spawned PID immediately after spawn,
  before fallible identity lookup or persistence. Keep that handle on failure and
  stop/reap the known child with bounded escalation. Persist intent, spawned,
  exited and not_spawned states; missing ownership is uncertainty, not absence.
- A later process has no original handle: require exact PID/birth-command identity
  or confirmed absence before signaling. Unknown intent or mismatched identity
  retains recovery_required and denies native logout and stack removal. Do not
  weaken PID-reuse protection to force cleanup.
- Funnel SIGINT/SIGTERM and ordinary failures through one idempotent cleanup path.
  Drain owned writers before creating-binary logout; verify empty native account/
  cleanup metadata before removing central resources. Diagnostics are best effort
  and cannot skip teardown. Cleanup commands can create children too: drain newly
  registered handles before logout and after teardown, within per-child bounds.
- Serialize recovery before reading the journal and hold ownership through final
  evidence writes. A permanent, private regular-file inode with a kernel-held lock
  avoids stale-PID check/unlink/recreate races and empty-owner publication windows.
  Check inode identity; release only the descriptor actually owned, never unlink
  a successor's lock. Lock bytes are not authority. Missing lock support fails
  closed; process exit must allow a later owner to retry.
- Verify exact child/container/network/volume/credential outcomes independently.
  Preserve each failed attempt and the original failed verification outcome even
  after recovery succeeds. A deliberately unresolved no-effect fixture may retain
  its journal without live resources; report those facts separately.

## Tradeoffs and limits

The tested implementation is Node24 on an Apple Silicon Mac/local filesystem.
A bounded Python3 stdlib fcntl helper locks the shared open file description;
Node retains its descriptor after helper exit. Python is verification-only here.
Do not assume other OS/filesystems share those semantics without process tests.
The kernel lock prevents cooperative contenders, not malicious same-user changes.
SIGKILL cannot run cleanup: durable recovery is still required. This harness owns
specific direct children, not arbitrary escaped descendant trees. OS/browser
certificate trust remains separately authorized operator work.

The earlier P10 implementation passed its normal signal paths but still lost
handles during failed registration and raced on stale-lock replacement. P12 fixes
those gaps; the older successful tests do not establish the stronger invariants.
Keep completed historical reports unchanged and use the final regressions below.

## Provenance

- Track: `dhivon-brand-site-docs`; clean completion approved in
  `7249c26ed16da52cdcb3b9a848832e579ef7ab18`.
- Phase/task: P10/T10.1–T10.2 (BUG-008), P12/T12.1–T12.3 (BUG-009/010).
- Commits: `03764affe7f76d019a24e34c020ed68d40eb14cf`,
  `9aba60c2341b3154f70a27ad04422bb812feef4a`.
- Source/evidence: `scripts/rehearsal-lifecycle.ts`, `scripts/rehearsal-lock.ts`,
  `scripts/recover-https-oauth.ts`, `scripts/rehearsal-lifecycle.test.ts`,
  `scripts/rehearsal-ownership.test.ts` and their private fixture entry points;
  `docs/https-oauth-rehearsal.md`, `docs/dhivon-release.md` and review cycles2–4.
  Final combined process suite25/25 covers actual Docker/PostgreSQL/extracted-Mac
  interruption/cleanup and deterministic identity/write/diagnostic failures,
  ambiguous intent, concurrent contenders, empty/held-lock forced exits,
  successor-safe release, journal freshness, missing Python and partial bytes.
  It verifies preserved unrelated state; clean review reruns25/25 in161.11s.
- Archived at: 2026-11-02 (proposed archive batch).
- Supersession: none. Complements isolated-release-verification and
  owned-mcp-transports; does not replace the Rust gateway's process-group design.
