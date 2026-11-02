# Review Bugs: dhivon-brand-site-docs — cycle 2

- Review cycle: 2.
- Status: proposed; awaiting human approval of this report and remediation plan.
- Reviewed execution: `2026-11-02T08-03-09-159Z`, Track mode, spec 1 / plan 2.
- Reviewed range: `69ba2958579a4f592d2814f49f6afc5104d51754..2c272be1bc9e0af5225ec20bf2bb875178d865bc`.
- Current HEAD: `9f5bf2288950bcf548b9034b42f26c14db7eb354`, completion bookkeeping only after the reviewed HEAD.
- Review date: 2026-11-02.

## Coverage and prior findings

Reviewed all 27 changed product files, the completed execution, current plan,
prior findings and relevant credential, policy, packaging, fixture and publication
callers. Reused unchanged scope, workflow, patterns and styleguides after Git
verification. Original whole-track review coverage remains historical evidence;
this cycle covers the complete remediation range, not just its last fix.

BUG-007 is resolved for its approved project/Manrope notice requirements. The
actual final archive, nonroot service image and static output again pass exact
source-byte checks for LICENSE, NOTICE and full unchanged OFL. Cargo/npm ownership
metadata and publication paths agree.

BUG-006 is addressed for the owner's explicitly selected Docker HTTPS simulation.
Evidence covers HTTPS sign-in/sync, Safari redirect to the actual native port,
PKCE/state/issuer negatives, persistence, explicit enablement/tool use, version
1→2 refresh after an old-token rejection, disconnect and denial. Public hosting
and named external-provider compatibility remain documented, unverified limits.

The resolver change preserves validated scope, full online policy, generation
checks, refresh lease/version CAS, rejection of the old binding, offline denial
and no tool replay. No additional runtime defect was confirmed. Re-ran the
credential publication/rejection suite: 8 passed. Reused unchanged recorded
83-test workspace, extracted-client and HTTPS evidence against verified current
product identities. Git confirms no product drift after the reviewed HEAD.

## BUG-008: Interrupted HTTPS rehearsal skips resource cleanup

- Severity: P2.
- Evidence: `scripts/verify-https-oauth.ts:515-537` holds teardown exclusively in
  a top-level `finally`. The script has no SIGINT/SIGTERM handlers. Its result
  and ownership summary is written only after teardown, at `:543-559`.
- Impact: stopping the interactive rehearsal can leave generated Docker
  containers, network/volume resources and an incomplete recovery record.
  By inspection, interruption after native login also skips the creating CLI's
  logout/Keychain cleanup; this probe intentionally stopped before native login.
- Reproduction: run the actual script with owned cache/package/fixture inputs.
  Wait for its generated PostgreSQL container to start, send SIGTERM to the Node
  PID, then inspect Docker resources using that exact generated project's
  `com.docker.compose.project` label and check its private `result.json`.
- Observed result: Node exited on SIGTERM (Python return code -15). One generated
  PostgreSQL container and one volume remained. No `result.json` was written.
  No native identity had been created. The reviewer then removed only the exact
  probe resources and confirmed container/network/volume absence. No certificate
  trust or native credentials were created for this probe.
- Probe evidence: `review2-interruption/result.json` and private probe/recovery
  logs in the existing execution-owned build cache.
- Proposed remediation acceptance criteria:
  1. Persist exact generated resource/process ownership and immutable creating
     CLI/state paths before effects, enabling deterministic interruption recovery.
  2. Route SIGINT/SIGTERM through one idempotent bounded cleanup path. Stop owned
     children, perform creating-binary logout and verify native cleanup, then
     remove only generated Compose resources. Retain precise recovery evidence
     when cleanup cannot finish. Keep manual TLS trust ownership separate.
  3. Make diagnostics best effort: failure at line 517 must not prevent
     independent native/process cleanup attempts.
  4. Verify interruption after resource startup and after native login, normal
     success, and cleanup/diagnostic failures. Assert actual resource/credential
     outcomes and truthful recovery results without changing product OAuth policy.
- Human approval: pending for this exact report, plan 3, reassessed learning,
  dependency context and proposed changes-requested state.

## Proposed disposition

Record cycle 2 as changes requested. Preserve the completed execution and every
completed phase, including former final verification phases P6/P9. Append P10
for interruption-safe rehearsal cleanup and focused verification, then P11 for
final track-level user verification. Canonical state remains `ready_for_review`
until approval. No remediation or new execution was started during this review.
