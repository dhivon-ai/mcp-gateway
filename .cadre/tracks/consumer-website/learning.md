# Incremental Learning: Simpler product guides and two-command setup

Root phases read the Pattern Seed. Later phases read learning from their declared predecessors. Append dated observations and evidence during delivery.

<!-- cadre:pattern-seed:start -->
## Pattern Seed

<!-- cadre:memory:start -->
{"schemaVersion":1,"specRevision":2,"planRevision":3,"patterns":[{"path":"patterns/canonical-product-guides.md","sha256":"2b6a6ed1e2585a7b89530c6475c47295b561393cbaf6a7ab4fcc53874c69e6a5"},{"path":"patterns/isolated-release-verification.md","sha256":"7759aa6f770fd16bbf94ddbb113d0bce6c2eae3a6a8bf2cae02fb382a30cfec6"},{"path":"patterns/rehearsal-recovery-ownership.md","sha256":"1e1084ae00877e10b9a7d8ce72e97bfcb2c1d36e3aaf55fa4d23baae0a749147"}]}
<!-- cadre:memory:end -->

- Spec revision: 2
- Plan revision: 3

| Pattern | Relevance | Constraints |
| --- | --- | --- |
| `patterns/canonical-product-guides.md` | Detail-page navigation and onboarding | One maintained guide per task, accurate availability, approved local assets, crawl guide links, verify keyboard/mobile experience, and check the existing homepage for regressions. |
| `patterns/isolated-release-verification.md` | Compose and native packaging | Test actual runtime image and extracted CLI, compare notices, verify restart/restore and trusted HTTPS with explicitly scoped fixtures. |
| `patterns/rehearsal-recovery-ownership.md` | Failure and recovery checks | Reuse owned fixtures and evidence, preserve unrelated resources and release creating-binary credentials before tearing down the stack. |
<!-- cadre:pattern-seed:end -->

## Phase 1: Simplify setup commands

Read the three seeded patterns. The root Compose setup and Mac archive are integrated at 6414c595. Configure once with bash scripts/configure-gateway.sh; start with docker compose up -d --build --wait. The validated local reverse proxy terminates HTTPS, PostgreSQL stays private, and restore requires the saved vault plus deployment ID. Restart and interrupted bootstrap preserve accounts and credentials. Mac packaging with no output variables produces a private dist/macos-XXXXXXXX/releases archive and checksum. Actual extracted CLI, echo call, disable denial and scoped native cleanup passed. Guides bundled in the archive require a final package rebuild if edited.

## Phase 2: Simplify the detail pages and verify the setup journeys

Read Phase 1 learning and checked both commands against the implemented entry points. Publish only listed pages/assets. The final site with 20 guides passed a synthetic unlisted-source regression, rendered link/fragment checks and byte-for-byte home screenshot comparisons at two widths. The guide contents are bundled in the archive, so the published verification checksum is kept in its own unbundled guide. See docs/consumer-setup-verification.md (a1b88eb) for the root-stack, native Mac, recovery and browser evidence.

## Phase 3: Track-level User Manual Verification

The user approved track-level verification at a1b88eb on 2026-11-03 after receiving the Chrome preview and the verification record in docs/consumer-setup-verification.md.

## Review cycle 1 handoff

All three seed patterns still apply. Include a Linux-native password-file permission test; test setup publication separately from database interruption. Test backup failure before restore and the distributed guide links outside the checkout. Measure the second default Mac build. An OrbStack bind mount and a site crawl alone did not exercise these cases.

## Phase 4: Repair operator setup and backup

At phase head 44704dd, fresh Linux permissions with distinct operator and database UIDs established what the Mac bind mount missed. Pass an inherited private FD into a pipe opened by the database user. Keep the database URL out of generated CLI help. Kernel-held directory locks and exclusive publication let the same checkout resume incomplete private setup without taking another deployment's state. Verified the Python regression cases under both Mac and Linux. The backup helper keeps completed 0600 dumps separate from temporary output; the documented restore recovered a saved bearer on a new cluster with matching keys. See docs/consumer-setup-verification.md.

## Phase 5: Repair repeat Mac builds and archive onboarding

At phase head 9e353dd, a checkout-local private Cargo target served two default builds with independent UI and archive directories. Test two different embedded UI builds, not just equal version strings. Follow links from README.md and COMPATIBILITY.md in an extracted archive outside the checkout and compare the generated guide bytes with the integrated source. Keep the published checksum in its own unbundled guide. All 37 links passed and the native agent call and disable denial were verified against the root Compose stack.

## Phase 6: Track-level User Manual Verification

The user approved the integrated product at be6570c after reviewing the seven remediation results. Regenerating the archive guides from that commit matched the extracted bytes. The final site crawl, desktop and mobile checks passed. See docs/consumer-setup-verification.md for environment and coverage.
