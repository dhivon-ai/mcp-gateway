# Incremental Learning: Single Keychain master key for all local gateway state

Root phases read this seed; later phases read each declared dependency phase's learning. Append verified handoffs without overwriting history.

<!-- cadre:pattern-seed:start -->
## Pattern Seed

<!-- cadre:memory:start -->
{"schemaVersion":1,"specRevision":1,"planRevision":1,"patterns":[{"path":"patterns/authorization-leases.md","sha256":"a949846db0786cbcc3b82b93a073f6a0427175fc8c02033d7fcc771db11360c8"},{"path":"patterns/credential-publication.md","sha256":"e7bc09e68d460cd5aa68cdf0298ba67b314ff894d318eb748153e10426d2be91"},{"path":"patterns/owned-mcp-transports.md","sha256":"d407bb97d2b46c96f9b109e6c18af6fcb769a2d4ed52051c4d4d6f8f808083a3"},{"path":"patterns/isolated-release-verification.md","sha256":"7759aa6f770fd16bbf94ddbb113d0bce6c2eae3a6a8bf2cae02fb382a30cfec6"},{"path":"patterns/rehearsal-recovery-ownership.md","sha256":"1e1084ae00877e10b9a7d8ce72e97bfcb2c1d36e3aaf55fa4d23baae0a749147"},{"path":"patterns/canonical-product-guides.md","sha256":"2b6a6ed1e2585a7b89530c6475c47295b561393cbaf6a7ab4fcc53874c69e6a5"}]}
<!-- cadre:memory:end -->

- Spec revision: 1
- Plan revision: 1

- `patterns/authorization-leases.md`: Preserve expiry, online-only renewal, pinned login and rollback across restarts.
- `patterns/credential-publication.md`: Replace per-secret Keychain placement. Preserve its publication/rejection barriers, capacity and complete-response CAS recovery.
- `patterns/owned-mcp-transports.md`: Recheck authorization at dispatch/completion, use exact destinations and never replay tool calls.
- `patterns/isolated-release-verification.md`: Use isolated old/new immutable binaries and real Mac checks through packaged entry points.
- `patterns/rehearsal-recovery-ownership.md`: Update cleanup evidence for one key, verify owned outcomes under lock and retain failed attempts.
- `patterns/canonical-product-guides.md`: Edit the canonical guides and report only the environments exercised.
<!-- cadre:pattern-seed:end -->

## Phase observations

Pending execution.
