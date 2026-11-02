# Pattern: Immutable artifact rehearsals and isolated recovery evidence

## Use when

Native credentials, a self-hosted service and an agent process must work together,
including backup/restore and cleanup without touching real state.

## Guidance

- Rehearse the extracted archive and actual container. Record checksum, executable/
  source/assets/runbook identity, architecture, validated OS, image ID and actual
  protocol/browser/provider coverage, rather than inferring release behavior.
- Use generated private roots, separate Keychain namespaces, UUID databases and
  Compose projects. Destructive cleanup scripts must not accept production targets.
  Retain the immutable creating executable until logout confirms empty native
  identity/accounts/pending-cleanup journals.
- Verify admin→native login→explicit install/enable→one agent STDIO call→disable
  denial using the host's real descriptors. Installation receipts are not central
  enablement; retry enable separately without implicitly rerunning installation.
- Exercise restart, actual dump and fresh restore with separately retained keys.
  Invalidate copied sessions, prove wrong-key denial/matching-key recovery, and
  scan captured logs for synthetic secret/session values.
- Clean only exactly evidenced owned resources; retain recovery evidence on failure
  and verify resource absence after success. Preserve historical journals.
- Reuse passing evidence only when relevant source, inputs and verification policy
  are unchanged. Bookkeeping alone does not invalidate product checks; changed
  embedded assets still need a current artifact rehearsal.

## Distribution and native OAuth boundaries

- Verify emitted project LICENSE/NOTICE and bundled third-party copyright/full
  license text against canonical source bytes in the extracted archive, actual
  runtime image and static output. Manifest metadata or a font license URL alone
  does not demonstrate notice delivery. Keep original third-party terms intact;
  expose readable notice links and run checks at publication entry points.
- Distinguish a hosted HTTPS control plane from a native loopback OAuth callback.
  Exercise the actual client listener/port, browser redirect, PKCE/state/issuer
  checks, credential persistence, explicit enablement, refresh and disconnect.
  Keep production certificate and hostname verification enabled. Provider client
  registration and permitted loopback ports require separate compatibility proof.
- Keep temporary trust operator-owned, specifically authorized and separately
  reconciled. A working native trust check does not prove every browser accepts
  the certificate. A controlled Docker HTTPS/provider rehearsal proves its tested
  topology, not public DNS/hosting or named external-provider interoperability.
- For signal/crash recovery, follow [Rehearsal recovery ownership](rehearsal-recovery-ownership.md).
  Independent resource outcomes, not a successful finally block, establish cleanup.

## Tradeoffs and limits

An isolated HTTP internal-hop test does not validate public TLS. Controlled OAuth
and SDK clients do not prove every provider/commercial agent. Report unrun signing,
publication, hosted CI, other platforms/browsers and sleep/reboot checks honestly.
Test cleanup is not permission to delete live data.

## Provenance

- Track: `mac-mvp`; clean completion approved in `42e27c246d7869a94ed67098eb3da4bc6a395532`.
- Phase/task: P8/T8.2–T8.3, P12/T12.1, P14/T14.2, P16/T16.3.
- Commits: `4469f1aaba87f2f1d447900da28afc2a78bfe860`, `b3ea6ab52ce58249946831d67cba30106cbd8f90`, `52c288b70ad0d8839aea8b3f12952ecd3e0cb26d`, `941f7d0c45588a0dbde586941fc6d3d8c9f714b0`.
- Source/evidence: `scripts/package-macos.sh`, `scripts/verify-deployment.ts`, CLI gateway tests, release/acceptance/deployment/compatibility guides; final extracted all-auth client40.47s and container/Mac/Node/Keychain/restart/fresh-restore/key-recovery rehearsal.
- Archived at: 2026-10-15 (proposed archive batch).

## Additional provenance

- Track: `dhivon-brand-site-docs`; clean completion approved in
  `7249c26ed16da52cdcb3b9a848832e579ef7ab18`.
- Phase/task: P5/T5.1–T5.2, P7/T7.1–T7.2 (BUG-007), P8/T8.1 (BUG-006).
- Commits: `fe930285c9c18bafbab36a72635b600aba47c7b5`,
  `327d97881634dba4cb17ad6e6afb0b959e5d1e5a`,
  `3d6eecb60106ba74a5126ee0b4a277ccec3e85d9`.
- Current source/evidence: `scripts/check-notices.ts`, `scripts/package-macos.sh`,
  `deploy/Dockerfile`, `apps/site/build.ts`, `scripts/verify-https-oauth.ts`,
  `docs/https-oauth-rehearsal.md` and `docs/dhivon-release.md`. Exact-byte artifact
  checks include the old archive failing and rebuilt archive/image/site passing;
  the extracted Mac/Safari controlled HTTPS/native OAuth flow passes. Earlier
  mac-mvp paths/identities above remain historical provenance after the rename.
- Archived at: 2026-11-02 (proposed archive batch).
- Supersession: extends this pattern; no earlier pattern or history is retired.
