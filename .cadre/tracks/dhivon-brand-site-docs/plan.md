# Plan: Dhivon MCP Gateway rebrand, product website and documentation

- Track: `dhivon-brand-site-docs`
- Spec revision: 1
- Plan revision: 1
- Status: proposed

## Phase 1: Product identity and approved assets

- Phase dependencies: none

### Outcome

Establish the names and visual assets that every build, example and interface will consume.

- [ ] T1.1 Rename product packages, binaries and runtime namespaces
  - Task dependencies: none
  - Commit group: product-identifiers
  - Scope: Apply FR-001 across Cargo paths/imports, workspace metadata, CLI/help, endpoint storage and Keychain, protocol identification, cookies and frontend/backend contracts. Audit literal names, generic terms and standard identifiers separately.
  - Verification: Build both binaries, check help/version, run focused policy/auth/native isolation and renamed contract tests. Confirm no legacy reads or destructive cleanup.
  - Handoff: Identifier map, audit exceptions and tested fresh paths.
- [ ] T1.2 Align deployment, packaging and developer tooling
  - Task dependencies: T1.1
  - Commit group: product-identifiers
  - Scope: Update images, Compose configuration and database defaults, environment keys, shell/TS scripts, build.rs inputs, pnpm filters, CI, fixture namespaces, agent entry and current runbook commands as one rename. Generate lockfile changes with pinned tools.
  - Verification: Exercise fresh Compose config, renamed fixture commands, frozen installs, embedded build and archive naming. Confirm external tools still receive their standard inputs.
  - Handoff: Reproducible commands and owned build/output locations.
- [ ] T1.3 Vendor the Dhivon brand kit and shared presentation tokens
  - Task dependencies: none
  - Commit group: brand-assets
  - Scope: Select the official SVG lockup/mark, compact and dark variants, favicon assets, local Manrope and OFL. Share assets and semantic theme decisions across site and management bundles without an absolute runtime reference.
  - Verification: Check provenance and license, mark geometry, minimum sizes and primary/control contrast. Verify both Rust asset packers accept the selected font/icon MIME types under existing bounds.
  - Handoff: Asset list, semantic color roles and documented usage.
- [ ] T1.4 User Manual Verification
  - Verification: Review rename coverage, executable names and brand samples against AC-001 and FR-003.

- Phase completion commit: pending

## Phase 2: Administrator and developer interfaces

- Phase dependencies: P1

### Outcome

Admins and developers see the same Dhivon identity while using their distinct workflows.

- [ ] T2.1 Restyle shared UI and central administrator workflows
  - Task dependencies: none
  - Commit group: admin-brand
  - Scope: Replace the old glyph, Geist theme, titles and chrome. Carry the new identity through sign-in, identities, catalog and access editors, tables, drawers, dialogs and validation feedback.
  - Verification: Use compiled admin assets at desktop/390px and keyboard-only. Exercise login, role denial, revision conflict, policy edits and focus return. Check asset delivery and console.
  - Handoff: Admin screenshots, states exercised and component/token decisions.
- [ ] T2.2 Apply the brand to the Mac management UI
  - Task dependencies: T2.1
  - Commit group: local-brand
  - Scope: Adapt local login, catalog, installation, status, enablement, secrets and OAuth flows, including pending, disabled and failure states. Preserve per-account reset and secret handling.
  - Verification: Exercise compiled local assets, keyboard and narrow view, explicit enable/disable, offline feedback and auth states. Check protected loopback behavior, bundled font and icon loading.
  - Handoff: Local screenshots, interaction evidence and retained security boundaries.
- [ ] T2.3 User Manual Verification
  - Verification: Review both compiled experiences and an error state on each against AC-002.

- Phase completion commit: pending

## Phase 3: Product and operator documentation

- Phase dependencies: P1

### Outcome

A reader can find the guide for their role and complete a fresh deployment and agent setup.

- [ ] T3.1 Write the role-based quickstart and integration guides
  - Task dependencies: none
  - Commit group: getting-started
  - Scope: Rework the README and docs navigation around operator, administrator, developer and contributor entry points. Write the fresh setup through first tool call, expected results, agent example and transport/auth recipes. Repair stale links to archived evidence.
  - Verification: Check example syntax and options against the renamed binaries. Walk the documented commands using controlled fixtures, label substitutions and link all supporting guides.
  - Handoff: Canonical Markdown paths, quickstart walkthrough and integration examples.
- [ ] T3.2 Complete the reference, troubleshooting and roadmap
  - Task dependencies: T3.1
  - Commit group: operations-docs
  - Scope: Cover configuration, architecture, security/trust boundaries, deployment/recovery, compatibility, contribution and test setup. Give symptom, diagnosis, safe recovery and expected results for access, lease and credential failures. Organize the undated roadmap by evidence and approval status.
  - Verification: Cross-check technical claims against code, compatibility and release evidence. Verify every FR-006 topic and FR-007 category has a navigable source; preserve dated evidence.
  - Handoff: Guide index, roadmap sources and identified verification limits.
- [ ] T3.3 User Manual Verification
  - Verification: Follow the operator and developer routes and check roadmap status labels against AC-004 and AC-005.

- Phase completion commit: pending

## Phase 4: Public product and documentation site

- Phase dependencies: P1, P3

### Outcome

Readers can understand why the gateway exists and browse the same setup and reference material that maintainers edit.

- [ ] T4.1 Build the standalone public site and guide routes
  - Task dependencies: none
  - Commit group: public-site
  - Scope: Add a static site workspace/entry in this repository with scoped build and preview commands, local brand assets, accessible navigation and rendered canonical guides. Support direct links, anchors and page refresh. Keep deployment output separate from embedded admin/local bundles.
  - Verification: Build from the repo alone, serve the output without a backend, crawl links/assets and test nested URLs. Check the authenticated apps still open their intended root.
  - Handoff: Build/preview instructions, route map and static output location.
- [ ] T4.2 Explain the problem, the control flow and what comes next
  - Task dependencies: T4.1
  - Commit group: public-site
  - Scope: Write and design the landing page and product sections. Show an admin granting one team a fixture MCP, a developer enabling it, an agent seeing and calling only permitted tools, then losing access after disable. Label policy ownership and where local/remote tools execute. Link to relevant guides, release limits and roadmap.
  - Verification: Review AC-003 from a first-time visitor's perspective. Test the primary paths at desktop and 390px, keyboard, zoom, contrast and console. Confirm unavailable download and support claims are absent.
  - Handoff: Page captures, content/evidence mapping and browser results.
- [ ] T4.3 User Manual Verification
  - Verification: Read the homepage cold, explain the need in your own words and navigate to the first call and roadmap.

- Phase completion commit: pending

## Phase 5: Release rehearsal and Definition of Done

- Phase dependencies: P2, P3, P4

### Outcome

The renamed artifacts, documentation and all three web experiences agree on what a fresh user can do.

- [ ] T5.1 Run integrated quality, content and browser checks
  - Task dependencies: none
  - Commit group: release-verification
  - Scope: Audit tracked code/docs for stale identifiers using word boundaries and a reviewed exception list. Run Rust fmt, locked all-target Clippy, workspace tests/build and isolated ignored PostgreSQL tests; frozen pnpm install, format, lint, typecheck, tests/build, public site build and generated contracts. Exercise compiled UI and public navigation plus AC-002 accessibility checks.
  - Verification: Use only Cadre's returned owned build cache. Capture actual outcomes, links, accessibility checks and corrected failures. Rebuild after material fixes.
  - Handoff: Check results, actual environment and audited rename exceptions.
- [ ] T5.2 Rehearse a fresh branded release and record evidence
  - Task dependencies: T5.1
  - Commit group: release-verification
  - Scope: Build the Linux arm64 image and Mac arm64 archive/checksum. Follow the public guides through deploy/bootstrap, assign, install/enable, one STDIO call and disable denial. Exercise the extracted all-auth suite, restart and isolated fresh restore/key recovery. Record the new release identity and update current docs. Check off FR/AC coverage and record a post-track Cadre refresh handoff for project metadata and any remaining checkout/hosting administration.
  - Verification: Retain creating binary until fixture Keychain cleanup. Verify checksums and final embedded UI assets, wrong/matching key outcomes, safe cleanup and log redaction. Recheck published guides, links and static build after evidence edits.
  - Handoff: Dated release/compatibility evidence, AC coverage and any outstanding blockers.
- [ ] T5.3 User Manual Verification
  - Verification: Inspect packaged Mac and container flows, published guide evidence and full Definition of Done against AC-001 through AC-006.

- Phase completion commit: pending

## Phase 6: Track-level User Manual Verification

### Outcome

Verify the complete rebrand, both application surfaces, the public website, documentation and roadmap against the approved specification, packaged evidence and acceptance criteria.

- [ ] T6.1 User Manual Verification
  - Verification: Review the fresh installation journey, denial after disable, public explanation of the gateway and roadmap with the final artifacts under the persisted execution approval mode.

- Phase completion commit: pending

## Approval

Pending combined specification and plan approval.
