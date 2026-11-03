# Incremental Learning: Single Keychain master key for all local gateway state

Root phases read this seed; later phases read each declared dependency phase's learning. Append verified handoffs without overwriting history.

<!-- cadre:pattern-seed:start -->
## Pattern Seed

<!-- cadre:memory:start -->
{"schemaVersion":1,"specRevision":1,"planRevision":3,"patterns":[{"path":"patterns/authorization-leases.md","sha256":"a949846db0786cbcc3b82b93a073f6a0427175fc8c02033d7fcc771db11360c8"},{"path":"patterns/credential-publication.md","sha256":"e7bc09e68d460cd5aa68cdf0298ba67b314ff894d318eb748153e10426d2be91"},{"path":"patterns/owned-mcp-transports.md","sha256":"d407bb97d2b46c96f9b109e6c18af6fcb769a2d4ed52051c4d4d6f8f808083a3"},{"path":"patterns/isolated-release-verification.md","sha256":"7759aa6f770fd16bbf94ddbb113d0bce6c2eae3a6a8bf2cae02fb382a30cfec6"},{"path":"patterns/rehearsal-recovery-ownership.md","sha256":"1e1084ae00877e10b9a7d8ce72e97bfcb2c1d36e3aaf55fa4d23baae0a749147"},{"path":"patterns/canonical-product-guides.md","sha256":"2b6a6ed1e2585a7b89530c6475c47295b561393cbaf6a7ab4fcc53874c69e6a5"}]}
<!-- cadre:memory:end -->

- Spec revision: 1
- Plan revision: 3

- `patterns/authorization-leases.md`: Preserve expiry, online-only renewal, pinned login and rollback across restarts.
- `patterns/credential-publication.md`: Replace per-secret Keychain placement. Preserve its publication/rejection barriers, capacity and complete-response CAS recovery.
- `patterns/owned-mcp-transports.md`: Recheck authorization at dispatch/completion, use exact destinations and never replay tool calls.
- `patterns/isolated-release-verification.md`: Use isolated old/new immutable binaries and real Mac checks through packaged entry points.
- `patterns/rehearsal-recovery-ownership.md`: Update cleanup evidence for one key, verify owned outcomes under lock and retain failed attempts.
- `patterns/canonical-product-guides.md`: Edit the canonical guides and report only the environments exercised.
<!-- cadre:pattern-seed:end -->

## Phase observations


## Phase 1: Single-key store

At 2357e8a, T1.1/T1.2 proved that an in-place password update with a stable account can return retired bytes to a running reader. Atomically updating both the random account UUID and the 32-byte value selects the durably staged encrypted index. The native two-process replay test, record substitution and orphan recovery tests passed in a private real Keychain scope; CLI Clippy passed. Integration must preserve this protected index and exact scoped cleanup.

## Phase 2: Endpoint cutover

At dec44aa, authenticated configuration and recoverable secret bytes are independent of their expiry, revision and rejection evidence. Real Mac and PostgreSQL OAuth tests exercised the encrypted path. An interrupted migration must distinguish an unfinished copy from an old client's completed logout. Verify both legacy ownership and source before fencing; the explicit creator handoff invalidates all staged authority first. Locked Keychain reads can trigger macOS dialogs. The separately invoked lock test announces its disposable password; the test Keychain and search-list entry were confirmed absent after verification.

## Phase 3: Packaged verification

Verified on Apple Silicon macOS 27.0.1 (26A434), Rust 1.95.0, Node 24.19.0 and pnpm 11.4.0. Package SHA-256: beda2ba6861193834404d1305425269b2ed419edb0979eea1dd996fb4f04021a. Product verification commit: 43bae33. The locked workspace suite, formatting, Clippy, build, generated contracts, site and archive notice checks passed. All 11 real PostgreSQL tests passed; the OAuth consent/refresh/failure/disconnect test also ran against the new native store. The lifecycle/ownership suite passed 25/25 including its early HTTPS startup termination case with untrusted disposable test certificates. No changes to OS certificate trust.

- AC-001: Extracted Rust CLI exercised simultaneous UI/STDIO, real bearer and OAuth calls, with metadata-only counts (one master, zero legacy; zero after fixture retirement). Recursive fixture scan found no known bearer, tokens, MCP name or server URL. Container-to-package journey installed, enabled, called, then denied after disablement.
- AC-002: Live two-process full-directory rollback test; scope/ciphertext substitution, stale generation, exact expiry and clock regressions; extracted cold offline restart.
- AC-003: Damaged bearer denied offline, repaired by one authenticated response, still subject to rejection and original lease. The running extracted UI repaired a truncated policy. Failed offline sign-in left the damaged identity untouched; successful new sign-in restored it. Missing master, stale rebuild witness and isolated locked-Keychain read denial were exercised.
- AC-004: Old executable built from the planning baseline produced the signed-out installation for the package upgrade. Real-Keychain cutoff tests retained an existing pending-save byte sequence, interrupted copy/fence/retirement, ran signed-out creator handoff, and rejected old logout replay. Standalone OAuth/database regression exercised actual issued responses and CAS recovery. Live readers cannot consume a prepared import. Physical power loss, public OAuth providers and restoring the OS Keychain itself were not simulated.
- AC-005: Canonical guides, package/archive and static site verified. The test-only isolated Keychain was removed and its search-list absence checked; future manual tests announce the disposable password before creating it.

## Phase 4: Track verification

The user approved the AC-001 through AC-005 verification evidence for 43bae33. At handoff, product HEAD was unchanged and the packaged archive matched its recorded SHA-256.

## Review cycle 1: Fresh-install recovery

Retain the same six patterns and hashes. Apply credential-publication and authorization-leases to the refusal itself even when both the credential and its recovery evidence are unreadable. Exercise missing private files and damaged format markers through the packaged UI with an explicit server. Review scope is current-format installations only, per the owner; legacy migration is not a remediation task. Keep the completed historical verification intact.

## Phase 5: Current-format refusal and sign-in recovery

At integrated product commit `519aa1973ced5137070d630c995cd446ee0504c0`, the encrypted index can record a control-plane refusal without opening either the selected credential file or its recovery evidence. Each refusal has a new identifier. An already-running request checks the decision it observed before it may publish. An authorized response still checks policy, version, fingerprint, provider rejection and expiry. If neither credential nor recovery evidence authenticates, the gateway denies use until sufficient evidence is available or the user signs in independently.

The packaged Mac gateway encountered an actual authenticated 403 with two damaged bearer files. Restoring those exact files during a 503 outage and starting a fresh STDIO process kept the bearer unavailable while another MCP worked. Separate native tests exercised repeated refusals, stale successes, independent MCPs, provider rejection, and repair from a fresh response using the surviving evidence.

After losing the private directory with the master still present, the packaged loopback console served its sign-in page. A rejected password preserved the original master. Successful independent login rebuilt the store, and restoring the earlier directory did not restore the previous login. Damage confined to the private format marker preserved a healthy session. The native identity tests also rejected symlinks and unsafe permissions. Recovery fixtures were retired with their creating executable; no isolated Keychain database was created.

The Apple Silicon archive SHA-256 is `9c90fbe4bc83b342d8889306b3d3714acd13481a9944c375189489a4353929ad`. Archive notices and checksum, formatting, CLI Clippy, and the PostgreSQL-backed consent, refresh and disconnect test passed. The current-format package checks use synthetic callers and a loopback control plane.

## Phase 6: Track verification

The owner approved final verification for integrated commit `519aa1973ced5137070d630c995cd446ee0504c0`. The 69 CLI tests and PostgreSQL OAuth regression passed. The extracted Mac package exercised both review findings, and its archive checksum was verified again at approval.

## Review cycle 2

Retain the six approved patterns. Test the first authorization decision before any credential has been stored, including an earlier response arriving after a refusal. Packaged recovery coverage also needs a present but unrecognizable nonsecret marker together with an unreadable encrypted index.
