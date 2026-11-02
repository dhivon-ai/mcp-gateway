# Plan: Dhivon MCP Gateway rebrand, product website and documentation

- Track: `dhivon-brand-site-docs`
- Spec revision: 1
- Plan revision: 3
- Status: proposed

## Phase 1: Product identity and approved assets

- Phase dependencies: none

### Outcome

Establish the names and visual assets that every build, example and interface will consume.

- [x] T1.1 Rename product packages, binaries and runtime namespaces <!-- commit: 757d8b169cedc6578742c8a99e10fea45b24e1d1 -->
  - Task dependencies: none
  - Commit group: product-identifiers
  - Scope: Apply FR-001 across Cargo paths/imports, workspace metadata, CLI/help, endpoint storage and Keychain, protocol identification, cookies and frontend/backend contracts. Audit literal names, generic terms and standard identifiers separately.
  - Verification: Build both binaries, check help/version, run focused policy/auth/native isolation and renamed contract tests. Confirm no legacy reads or destructive cleanup.
  - Handoff: Identifier map, audit exceptions and tested fresh paths.
- [x] T1.2 Align deployment, packaging and developer tooling <!-- commit: 757d8b169cedc6578742c8a99e10fea45b24e1d1 -->
  - Task dependencies: T1.1
  - Commit group: product-identifiers
  - Scope: Update images, Compose configuration and database defaults, environment keys, shell/TS scripts, build.rs inputs, pnpm filters, CI, fixture namespaces, agent entry and current runbook commands as one rename. Generate lockfile changes with pinned tools.
  - Verification: Exercise fresh Compose config, renamed fixture commands, frozen installs, embedded build and archive naming. Confirm external tools still receive their standard inputs.
  - Handoff: Reproducible commands and owned build/output locations.
- [x] T1.3 Vendor the Dhivon brand kit and shared presentation tokens <!-- commit: bb057261035dadea4a830e5ef66e9a092bae6ce0 -->
  - Task dependencies: none
  - Commit group: brand-assets
  - Scope: Select the official SVG lockup/mark, compact and dark variants, favicon assets, local Manrope and OFL. Share assets and semantic theme decisions across site and management bundles without an absolute runtime reference.
  - Verification: Check provenance and license, mark geometry, minimum sizes and primary/control contrast. Verify both Rust asset packers accept the selected font/icon MIME types under existing bounds.
  - Handoff: Asset list, semantic color roles and documented usage.
- [x] T1.4 User Manual Verification <!-- commit: 3ca90793f24df65b376e2b3930f7c4babf248351 -->
  - Verification: Review rename coverage, executable names and brand samples against AC-001 and FR-003.

- Phase completion commit: `3ca90793f24df65b376e2b3930f7c4babf248351`

## Phase 2: Administrator and developer interfaces

- Phase dependencies: P1

### Outcome

Admins and developers see the same Dhivon identity while using their distinct workflows.

- [x] T2.1 Restyle shared UI and central administrator workflows <!-- commit: 03a0ae79e333cae95b3260ca8e3419df797d254b -->
  - Task dependencies: none
  - Commit group: admin-brand
  - Scope: Replace the old glyph, Geist theme, titles and chrome. Carry the new identity through sign-in, identities, catalog and access editors, tables, drawers, dialogs and validation feedback.
  - Verification: Use compiled admin assets at desktop/390px and keyboard-only. Exercise login, role denial, revision conflict, policy edits and focus return. Check asset delivery and console.
  - Handoff: Admin screenshots, states exercised and component/token decisions.
- [x] T2.2 Apply the brand to the Mac management UI <!-- commit: 2d2b662d1885b44b389687c545ebddb32a49b08a -->
  - Task dependencies: T2.1
  - Commit group: local-brand
  - Scope: Adapt local login, catalog, installation, status, enablement, secrets and OAuth flows, including pending, disabled and failure states. Preserve per-account reset and secret handling.
  - Verification: Exercise compiled local assets, keyboard and narrow view, explicit enable/disable, offline feedback and auth states. Check protected loopback behavior, bundled font and icon loading.
  - Handoff: Local screenshots, interaction evidence and retained security boundaries.
- [x] T2.3 User Manual Verification <!-- commit: d531666dc87e3c56fba248a4f7fb5b0846393b33 -->
  - Verification: Review both compiled experiences and an error state on each against AC-002.

- Phase completion commit: `d531666dc87e3c56fba248a4f7fb5b0846393b33`

## Phase 3: Product and operator documentation

- Phase dependencies: P1

### Outcome

A reader can find the guide for their role and complete a fresh deployment and agent setup.

- [x] T3.1 Write the role-based quickstart and integration guides <!-- commit: 3a927a3643fa8b435d8c197230ccef2361d5309c -->
  - Task dependencies: none
  - Commit group: getting-started
  - Scope: Rework the README and docs navigation around operator, administrator, developer and contributor entry points. Write the fresh setup through first tool call, expected results, agent example and transport/auth recipes. Repair stale links to archived evidence.
  - Verification: Check example syntax and options against the renamed binaries. Walk the documented commands using controlled fixtures, label substitutions and link all supporting guides.
  - Handoff: Canonical Markdown paths, quickstart walkthrough and integration examples.
- [x] T3.2 Complete the reference, troubleshooting and roadmap <!-- commit: 3c5d4f3b65fffa45674c054ee1a6c0ee7905cbbd -->
  - Task dependencies: T3.1
  - Commit group: operations-docs
  - Scope: Cover configuration, architecture, security/trust boundaries, deployment/recovery, compatibility, contribution and test setup. Give symptom, diagnosis, safe recovery and expected results for access, lease and credential failures. Organize the undated roadmap by evidence and approval status.
  - Verification: Cross-check technical claims against code, compatibility and release evidence. Verify every FR-006 topic and FR-007 category has a navigable source; preserve dated evidence.
  - Handoff: Guide index, roadmap sources and identified verification limits.
- [x] T3.3 User Manual Verification <!-- commit: 3c5d4f3b65fffa45674c054ee1a6c0ee7905cbbd -->
  - Verification: Follow the operator and developer routes and check roadmap status labels against AC-004 and AC-005.

- Phase completion commit: `3c5d4f3b65fffa45674c054ee1a6c0ee7905cbbd`

## Phase 4: Public product and documentation site

- Phase dependencies: P1, P3

### Outcome

Readers can understand why the gateway exists and browse the same setup and reference material that maintainers edit.

- [x] T4.1 Build the standalone public site and guide routes <!-- commit: 4a3ccc03f1334237204fc8ba23b328bf414241bc -->
  - Task dependencies: none
  - Commit group: public-site
  - Scope: Add a static site workspace/entry in this repository with scoped build and preview commands, local brand assets, accessible navigation and rendered canonical guides. Support direct links, anchors and page refresh. Keep deployment output separate from embedded admin/local bundles.
  - Verification: Build from the repo alone, serve the output without a backend, crawl links/assets and test nested URLs. Check the authenticated apps still open their intended root.
  - Handoff: Build/preview instructions, route map and static output location.
- [x] T4.2 Explain the problem, the control flow and what comes next <!-- commit: 4a3ccc03f1334237204fc8ba23b328bf414241bc -->
  - Task dependencies: T4.1
  - Commit group: public-site
  - Scope: Write and design the landing page and product sections. Show an admin granting one team a fixture MCP, a developer enabling it, an agent seeing and calling only permitted tools, then losing access after disable. Label policy ownership and where local/remote tools execute. Link to relevant guides, release limits and roadmap.
  - Verification: Review AC-003 from a first-time visitor's perspective. Test the primary paths at desktop and 390px, keyboard, zoom, contrast and console. Confirm unavailable download and support claims are absent.
  - Handoff: Page captures, content/evidence mapping and browser results.
- [x] T4.3 User Manual Verification <!-- commit: 4a3ccc03f1334237204fc8ba23b328bf414241bc -->
  - Verification: Read the homepage cold, explain the need in your own words and navigate to the first call and roadmap.

- Phase completion commit: `4a3ccc03f1334237204fc8ba23b328bf414241bc`

## Phase 5: Release rehearsal and Definition of Done

- Phase dependencies: P2, P3, P4

### Outcome

The renamed artifacts, documentation and all three web experiences agree on what a fresh user can do.

- [x] T5.1 Run integrated quality, content and browser checks <!-- commit: fe930285c9c18bafbab36a72635b600aba47c7b5 -->
  - Task dependencies: none
  - Commit group: release-verification
  - Scope: Audit tracked code/docs for stale identifiers using word boundaries and a reviewed exception list. Run Rust fmt, locked all-target Clippy, workspace tests/build and isolated ignored PostgreSQL tests; frozen pnpm install, format, lint, typecheck, tests/build, public site build and generated contracts. Exercise compiled UI and public navigation plus AC-002 accessibility checks.
  - Verification: Use only Cadre's returned owned build cache. Capture actual outcomes, links, accessibility checks and corrected failures. Rebuild after material fixes.
  - Handoff: Check results, actual environment and audited rename exceptions.
- [x] T5.2 Rehearse a fresh branded release and record evidence <!-- commit: fe930285c9c18bafbab36a72635b600aba47c7b5 -->
  - Task dependencies: T5.1
  - Commit group: release-verification
  - Scope: Build the Linux arm64 image and Mac arm64 archive/checksum. Follow the public guides through deploy/bootstrap, assign, install/enable, one STDIO call and disable denial. Exercise the extracted all-auth suite, restart and isolated fresh restore/key recovery. Record the new release identity and update current docs. Check off FR/AC coverage and record a post-track Cadre refresh handoff for project metadata and any remaining checkout/hosting administration.
  - Verification: Retain creating binary until fixture Keychain cleanup. Verify checksums and final embedded UI assets, wrong/matching key outcomes, safe cleanup and log redaction. Recheck published guides, links and static build after evidence edits.
  - Handoff: Dated release/compatibility evidence, AC coverage and any outstanding blockers.
- [x] T5.3 User Manual Verification <!-- commit: fe930285c9c18bafbab36a72635b600aba47c7b5 -->
  - Verification: Inspect packaged Mac and container flows, published guide evidence and full Definition of Done against AC-001 through AC-006.

- Phase completion commit: `fe930285c9c18bafbab36a72635b600aba47c7b5`

## Phase 6: Track-level User Manual Verification

### Outcome

Verify the complete rebrand, both application surfaces, the public website, documentation and roadmap against the approved specification, packaged evidence and acceptance criteria.

- [x] T6.1 User Manual Verification <!-- commit: eae1965 -->
  - Verification: Review the fresh installation journey, denial after disable, public explanation of the gateway and roadmap with the final artifacts under the persisted execution approval mode.

- Phase completion commit: `eae1965`

## Phase 7: Project licensing and distribution copyright notices

- Phase dependencies: P6

### Outcome

BUG-007 is addressed in a later implementation, adding the owner-approved
Apache-2.0 Dhivon AI source licensing and original third-party notices to deliverables.

- [x] T7.1 Add project licensing and include notices in delivery outputs <!-- commit: 327d97881634dba4cb17ad6e6afb0b959e5d1e5a -->
  - Task dependencies: none
  - Commit group: distribution-notices
  - Scope: Implement the retained Apache-2.0/Dhivon AI licensing decision: official root LICENSE, copyright NOTICE, README attribution and Cargo/npm metadata. The earlier d16c28a source update was reverted before review transition. Include root LICENSE/NOTICE and full Manrope copyright/OFL notices in the Mac archive and container at human-readable locations. Publish accessible project license/ownership notices with the static site without relicensing third-party assets.
  - Verification: Compare emitted project and third-party license bytes with canonical source, inspect the extracted archive, runtime image and built static output, and check accessible notice URLs.
  - Handoff: Exact notice locations, source hashes and verified output manifests.
- [x] T7.2 Verify release notice coverage and refresh artifact evidence <!-- commit: 327d97881634dba4cb17ad6e6afb0b959e5d1e5a -->
  - Task dependencies: T7.1
  - Commit group: distribution-notices
  - Scope: Add focused artifact-notice checks to existing release verification entry points. Rebuild affected outputs and update current release identities while preserving historical evidence.
  - Verification: Run focused release/notice checks, formatter/type/build checks and affected browser delivery checks. Verify archive checksums and container notice files.
  - Handoff: Artifact/checksum/image identities, notice assertions and any remaining limits.
- [x] T7.3 User Manual Verification <!-- commit: 327d97881634dba4cb17ad6e6afb0b959e5d1e5a -->
  - Verification: Inspect Apache-2.0 Dhivon AI ownership and third-party notices in actual deliverables against BUG-007.

- Phase completion commit: `327d97881634dba4cb17ad6e6afb0b959e5d1e5a`

## Phase 8: Hosted control plane and native OAuth verification

- Phase dependencies: P6

### Outcome

BUG-006 has named deployment/provider evidence for the intended HTTPS control-plane
and local loopback callback topology. Implementation is deferred and requires
the actual operator-selected target, origin and provider configuration.

- [x] T8.1 Verify the approved hosted origin and real native OAuth flow <!-- commit: 3d6eecb60106ba74a5126ee0b4a277ccec3e85d9 -->
  - Task dependencies: none
  - Commit group: hosted-oauth-evidence
  - Scope: Obtain the actual operator-approved hosting target, HTTPS origin and supported provider registration before deployment or external authentication work. Verify the hosted control plane with a local client and its actual loopback callback port. Clarify the OAuth guide with the hosted-control-plane/native-callback distinction and provider/port requirements. Record registration constraints, code/state/PKCE checks, persisted credentials, explicit enablement, tool use, refresh and disconnect. Keep credentials out of repository evidence.
  - Verification: Validate DNS/certificate and control-plane health/authenticated sync from the developer machine. Complete provider consent in that machine's browser and verify callback, tool call and disconnect using the configured deployment. If external prerequisites are unavailable, preserve the precise blocker and make no compatibility claim.
  - Handoff: Approved origin/provider profile, tested versions/ports, redacted results and cleanup ownership.
- [x] T8.2 User Manual Verification <!-- commit: 3d6eecb60106ba74a5126ee0b4a277ccec3e85d9 -->
  - Verification: Review the actual HTTPS deployment and native loopback OAuth evidence against BUG-006.

- Phase completion commit: `3d6eecb60106ba74a5126ee0b4a277ccec3e85d9`

## Phase 9: Track-level User Manual Verification

### Outcome

The owner verifies the original completed delivery plus the later BUG-006 and
BUG-007 remediation outcomes and recorded limits.

- [x] T9.1 User Manual Verification <!-- commit: 3d6eecb60106ba74a5126ee0b4a277ccec3e85d9 -->
  - Verification: Review final artifact notices, deployed HTTPS/native OAuth evidence and retained original acceptance coverage under the selected future execution mode.

- Phase completion commit: `3d6eecb60106ba74a5126ee0b4a277ccec3e85d9`

## Phase 10: Interruption-safe HTTPS rehearsal cleanup

- Phase dependencies: P9

### Outcome

BUG-008 is resolved without changing product OAuth behavior or broadening trust.
The rehearsal cleans exactly its own resources or retains actionable recovery.

- [x] T10.1 Supervise interruption and persist rehearsal ownership <!-- commit: 03764affe7f76d019a24e34c020ed68d40eb14cf -->
  - Task dependencies: none
  - Commit group: rehearsal-cleanup
  - Scope: Persist generated Compose identity/config, child ownership and immutable creating CLI/state paths before effects. Funnel SIGINT/SIGTERM and ordinary failures into one idempotent bounded cleanup path. Attempt owned process/native cleanup even when diagnostics fail. Verify creating-binary logout before discarding credential ownership; retain exact recovery state and failed cleanup steps when necessary. Keep certificate-trust creation/removal under existing explicit operator ownership and update the runbook.
  - Verification: Interrupt generated rehearsal resources and confirm only owned resources are affected. Check repeated cleanup, diagnostic failure and retained-recovery paths; preserve protocol/policy, TLS validation and manual trust boundaries.
  - Handoff: Ownership/recovery schema, cleanup ordering, exact resource checks and residual operator steps.
- [x] T10.2 Verify cleanup on signals, failures and normal completion <!-- commit: 03764affe7f76d019a24e34c020ed68d40eb14cf -->
  - Task dependencies: T10.1
  - Commit group: rehearsal-cleanup
  - Scope: Add focused process-level checks for SIGINT/SIGTERM after resource startup and after native login, normal success, diagnostic failure and incomplete cleanup. Use private controlled fixtures; never accept production deployment or credential targets. Record actual Docker/native cleanup evidence and unchanged product artifact identities.
  - Verification: Demonstrate the original signal leak is prevented. Assert child/container/network/volume and credential cleanup, repeated teardown safety, truthful results and preserved evidence on failure. Run strict TypeScript/format and affected rehearsal checks; reuse unchanged product/runtime checks only with matching inputs.
  - Handoff: Reproduction before/after, actual cleanup/recovery assertions and supported limits.
- [x] T10.3 User Manual Verification <!-- commit: 03764affe7f76d019a24e34c020ed68d40eb14cf -->
  - Verification: Inspect interruption cleanup and retained recovery against BUG-008, including separation of manual certificate trust.

- Phase completion commit: `03764affe7f76d019a24e34c020ed68d40eb14cf`

## Phase 11: Track-level User Manual Verification

### Outcome

The owner verifies BUG-008 remediation while retaining completed licensing
and HTTPS/native OAuth evidence and its external-provider limits.

- [x] T11.1 User Manual Verification <!-- commit: 5caf554972acc253c62ccb968afc770e10ebc740 -->
  - Verification: Review interrupted/failed/successful rehearsal outcomes and recovery instructions, existing product artifacts and retained prior acceptance evidence under the selected execution mode.

- Phase completion commit: `5caf554972acc253c62ccb968afc770e10ebc740`

## Approval

Pending combined specification and plan approval.
