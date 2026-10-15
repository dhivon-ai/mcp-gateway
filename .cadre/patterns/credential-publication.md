# Pattern: Recoverable credential publication and bounded metadata

## Use when

Protected credentials and ordinary metadata span separate stores, or a one-use
OAuth response must survive failure to persist it centrally.

## Guidance

- Scope credentials to stable deployment/user/installation/generation/MCP and
  runtime or binding version. Ordinary browser reads return metadata, never values.
  Administrative permission does not grant another user's secret-resolution access.
- Bind owner/record/version/key identity to authenticated ciphertext centrally;
  retain encryption keys separately from PostgreSQL. Native keys/tokens live in
  Keychain; transient secret buffers are zeroized.
- Journal exact owned accounts before insertion. Publish immutable protected heads
  using atomic metadata pointers and pending/retirement barriers. Partial publication
  denies old authorization until complete online recovery; retain cleanup journals.
- Apply identical metadata invariants before reads and durable writes. Preflight
  eventual head/pending/account capacity before persistent or Keychain effects.
  Rejection must preserve existing identity/status/sync/credentials and logout.
- Serialize OAuth refresh with a central lease/version CAS. Journal a complete
  issued response before save; recovery confirms/retries that same complete CAS,
  never another provider exchange. Preserve original native token lifetime anchors.
  Disconnect and cancellation fence late saves even before a record exists.
- Pin authenticated I/O to validated destinations and reject redirects/unrelated
  origins. Never forward a control-plane session as upstream authentication.

## Tradeoffs and limits

Immutable heads increase publication/cleanup work. The tested native limit is 128
MCP identities per login generation, including retired ones; logout/login clears
that generation. Do not loosen readers to hide growth. Immutable heads address
observed cross-process Keychain behavior on this Mac, not a universal OS claim.
Public-client OAuth fixture coverage is not universal provider compatibility.

## Provenance

- Track: `mac-mvp`; clean completion approved in `42e27c246d7869a94ed67098eb3da4bc6a395532`.
- Phase/task: P2/T2.5–T2.6, P3/T3.3, P5/T5.1–T5.5, P10/T10.1 (BUG-001).
- Commits: `ce051f249b8110858813339ced107dcb86fb1dec`, `cd2d02ab4881ccbcb9a0981af9e083aaf504bbbc`, `d4fb795a60fbb6f93a64575d88861f93ba8c0643`, `64a57dbd489202da80507c361152decd9bb1cae1`, `798b0fb0cc049842c81e28f6e7e36235ac8a71bf`.
- Source/evidence: CLI `store.rs`, `upstream_credentials.rs`, `oauth_pending.rs`, `oauth_control.rs`, server vault/core crypto; credential tests for 128/129, invalid writes, failed publication and token anchors; real-PG OAuth/vault recovery tests.
- Archived at: 2026-10-15 (proposed archive batch).
