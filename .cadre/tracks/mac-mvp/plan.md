# Plan: macOS enterprise MCP Gateway MVP

- Track: `mac-mvp`
- Spec revision: 1
- Plan revision: 1
- Status: proposed

## Phase 1: Workspace and policy contracts

- Phase dependencies: none

### Outcome

Establish runnable Rust/frontend workspaces, pinned toolchains, domain/API contracts, and fixtures that encode the approved access and freshness rules.

- [ ] T1.1 Create the Rust and React/TypeScript workspace foundation
  - Task dependencies: none
  - Commit group: workspace-foundation
  - Scope: Initialize Cargo/pnpm packages, stable toolchains/lockfiles, strict TS and shadcn/Vite configuration, platform adapters, scripts, and root documentation. Record the dependency-supported deployment target/MSRV rather than assuming versions.
  - Verification: Rust/frontend build, formatter/linter/type checks, CLI/server help smoke checks.
  - Handoff: Package boundaries, pinned versions, commands, and platform constraints.
- [ ] T1.2 Define shared policy, configuration, and credential contracts
  - Task dependencies: T1.1
  - Commit group: policy-contracts
  - Scope: Define stable identities, additive assignments, allowlist/denylist, centrally authoritative enablement, secret references, cache lease, revisions, error/auth modes, and Rust/TS API consistency. Document schema generation/checks and OAuth registration/credential scope.
  - Verification: Contract serialization/validation tests and generated-type consistency check.
  - Handoff: Contract/schema version, invariant mapping to FRs, and API ownership.
- [ ] T1.3 Implement the pure policy and lease evaluators with boundary tests
  - Task dependencies: T1.2
  - Commit group: policy-contracts
  - Scope: Implement effective-tool authorization and freshness/identity predicates using injectable clock boundaries. Encode last-grant removal, 3,600-second equality expiry, no renewal from failed sync/token refresh, and known-session rejection.
  - Verification: Policy matrix plus 3,599/3,600/3,601 tests; deny paths prove no dispatch.
  - Handoff: Reusable evaluators and semantics every transport/UI must consume.
- [ ] T1.4 Establish fixture services and verification entry points
  - Task dependencies: T1.3
  - Commit group: quality-foundation
  - Scope: Create controlled STDIO/HTTP MCP and OAuth fixtures, PostgreSQL test lifecycle, redacted event conventions, and CI/dev entry points. Define safe owned build outputs and avoid production credential dependence.
  - Verification: Fixture startup/shutdown and initial CI command smoke checks.
  - Handoff: Fixture controls, test prerequisites, owned output paths, and supported test environments.
- [ ] T1.5 User Manual Verification
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: pending

## Phase 2: Central identity, catalog, enablement, and encrypted secrets

- Phase dependencies: P1

### Outcome

A persisted self-hosted control plane can authenticate users, administer the catalog/access policy, synchronize user settings, and protect user credentials.

- [ ] T2.1 Implement administrator bootstrap and username/password sessions
  - Task dependencies: none
  - Commit group: service-identity
  - Scope: Implement migration/role foundations, explicit first-admin bootstrap, users/groups/membership, Argon2id, opaque 12-hour sessions, throttling, password reset, account disablement, and role enforcement.
  - Verification: Integration tests for bootstrap, wrong credentials, role bypass, expiration and session invalidation.
  - Handoff: Auth endpoints, session semantics, migrations, and bootstrap command.
- [ ] T2.2 Verify identity isolation and document administration contracts
  - Task dependencies: T2.1
  - Commit group: service-identity
  - Scope: Complete auth boundary and cross-user tests; document administration error/role behavior and redaction. Ensure no default credentials or password values in responses/logs.
  - Verification: Auth suite, logs scan, Rust checks.
  - Handoff: Identity test evidence and session/role constraints for endpoint consumers.
- [ ] T2.3 Implement catalog, grants, tool policies, and user-level settings APIs
  - Task dependencies: T2.2
  - Commit group: service-configuration
  - Scope: Implement catalog CRUD and validation, additive grants, last-grant disablement, tool policy modes, bounded assigned-gateway tool-name inventory, versioned user enablement/bindings, and effective snapshots with validation/expiry times. Mutations use revisions/transactions; configuration remains plaintext centrally. Inventory observations cannot change grants or policy.
  - Verification: Database/API tests for policy matrix, revision conflict, delete/regrant, and snapshot freshness.
  - Handoff: Endpoint snapshot/mutation schema and server-authoritative state semantics.
- [ ] T2.4 Verify configuration sync consistency and migration durability
  - Task dependencies: T2.3
  - Commit group: service-configuration
  - Scope: Test concurrent clients/admin changes, rejected stale updates, restart durability, migrations, tombstones and complete unchanged refresh. Cover non-secret field validation and no unauthorized credential references.
  - Verification: Integration/concurrency/migration tests and serialization checks.
  - Handoff: Verified revision ordering, migration compatibility, and config refresh contract.
- [ ] T2.5 Implement encrypted per-user secret and managed credential storage
  - Task dependencies: T2.4
  - Commit group: service-secrets
  - Scope: Add write-only key/value secret APIs, stable IDs/versioning, reference-aware deletion, AEAD encryption with external keys/AAD owner binding, and scoped endpoint credential resolution. Add managed OAuth record/version/refresh-coordination APIs without exposing values through ordinary browser reads.
  - Verification: Encryption round-trip/tamper/owner/version tests, plaintext-column inspection, role/ownership tests, and referenced-delete tests.
  - Handoff: Key ownership, secret CRUD/resolution rules, OAuth record/lease contract, and key IDs.
- [ ] T2.6 Verify vault recovery, failure paths, and API documentation
  - Task dependencies: T2.5
  - Commit group: service-secrets
  - Scope: Exercise missing/wrong keys, cross-user reads, redacted output, credential-version conflicts and serialized refresh lease behavior; document backup/key rotation/restore and secret API use.
  - Verification: Vault integration suite, logs/API read scans, central fmt/Clippy/tests/build.
  - Handoff: Vault test evidence, key recovery prerequisites, and consumer credential rules.
- [ ] T2.7 User Manual Verification
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: pending

## Phase 3: Mac CLI identity, encrypted cache, and online configuration sync

- Phase dependencies: P2

### Outcome

The Apple Silicon endpoint authenticates, protects its local state, refreshes configuration, and enforces the one-hour limit and online-only writes.

- [ ] T3.1 Implement CLI management/session and macOS Keychain integration
  - Task dependencies: none
  - Commit group: endpoint-identity
  - Scope: Add ui/mcp/status/logout command structure, deployment/account scoping, browser opening, central login/session handling, Keychain key/credential adapter, user-only paths and cleanup. Keep STDIO startup independent from browser interaction.
  - Verification: Command/session tests plus actual Mac Keychain round-trip/logout/identity isolation.
  - Handoff: CLI contract, app paths, Keychain identities, and session handoff.
- [ ] T3.2 Verify endpoint identity changes and local management security
  - Task dependencies: T3.1
  - Commit group: endpoint-identity
  - Scope: Implement/test loopback Host/Origin/local auth/CSRF controls, secure browser-session establishment without capability URLs, OAuth callback separation, logout and known-session rejection. Ensure credentials never enter browser storage/read APIs.
  - Verification: Security request tests, redacted status/log scans, cross-login cache checks.
  - Handoff: Protected local API/session protocol and browser access boundaries.
- [ ] T3.3 Implement atomic encrypted snapshots and bounded refresh
  - Task dependencies: T3.2
  - Commit group: endpoint-cache
  - Scope: Implement AEAD cache format and authenticated timestamps/revisions, startup/60-second refresh with 10-second request deadline, success-only lease renewal, identity binding, Keychain credentials, atomic shared state and latest-snapshot dispatch access. Handle reconnect and unchanged configs.
  - Verification: Cache crypto/atomic-write/revision tests, fake network/clock refresh tests.
  - Handoff: Cache schema, refresh state machine, source of time, and shared-process snapshot contract.
- [ ] T3.4 Verify expiry, restart, clock changes, and recovery
  - Task dependencies: T3.3
  - Commit group: endpoint-cache
  - Scope: Test 3,599/exact-3,600/3,601 seconds, failed/partial responses, token refresh, sleep/wake, process restart, copied cache, missing key, corrupt ciphertext and rollback. Freshness uncertainty fails closed; no indefinite stale fallback.
  - Verification: Deterministic time/network tests and actual Mac restart/sleep-aware clock evidence.
  - Handoff: Freshness proof, offline limitations, and recovery behavior.
- [ ] T3.5 Implement online-only local config/install and vault API bridge
  - Task dependencies: T3.4
  - Commit group: endpoint-management
  - Scope: Bridge confirmed central enable/disable/binding/vault mutations using revisions, with no offline writes/queue. Implement assigned install execution/progress/timeout/cancellation and installed-but-disabled recovery. Publish same-Mac config atomically before acknowledging settings changes.
  - Verification: Mutation/install tests for conflicts, outage, expiry, revocation, command arguments, and central commit failure.
  - Handoff: Local endpoints/progress events, server-confirmed settings, and installation lifecycle.
- [ ] T3.6 Verify Mac endpoint failure cleanup and documented CLI behavior
  - Task dependencies: T3.5
  - Commit group: endpoint-management
  - Scope: Test all offline mutation rejection paths, interrupted installs, credentials absence, owned process cleanup and stale state prevention. Record CLI usage and diagnostics; run endpoint quality checks.
  - Verification: Mac endpoint integration suite, fmt/Clippy/tests/build, redaction checks.
  - Handoff: Endpoint verification evidence and UI/gateway integration guidance.
- [ ] T3.7 User Manual Verification
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: pending

## Phase 4: STDIO gateway and upstream tool aggregation

- Phase dependencies: P3

### Outcome

An agent uses one gateway to discover/invoke authorized local and remote tools under current cached policy.

- [ ] T4.1 Implement upstream adapters and deterministic tool routing
  - Task dependencies: none
  - Commit group: gateway-aggregation
  - Scope: Use pinned rmcp transports for child-process STDIO and Streamable HTTP; implement no-auth connection baseline, paginated discovery, schema/result preservation and collision-free deterministic namespacing. Advertise only implemented tool capabilities with 2025-11-25 baseline tests.
  - Verification: Fixture discovery/dispatch/schema/collision/pagination tests and protocol negotiation checks.
  - Handoff: Upstream interfaces, route mapping, capability/SDK constraints and auth hooks.
- [ ] T4.2 Verify aggregation against malformed and changing upstreams
  - Task dependencies: T4.1
  - Commit group: gateway-aggregation
  - Scope: Cover schema/list changes, duplicate names, unavailable/upstream-invalid cases and isolated failures. Report bounded tool-name observations while online and assigned, without credentials/results. Apply the shared policy evaluator on list and call; prove inventory cannot grant access and direct-call bypass is rejected.
  - Verification: Multi-upstream integration tests with call-recording fixtures.
  - Handoff: Aggregation evidence and upstream error/list-update semantics.
- [ ] T4.3 Implement fresh-snapshot dispatch and bounded gateway lifecycle
  - Task dependencies: T4.2
  - Commit group: gateway-runtime
  - Scope: Run refresh in STDIO mode, consume latest committed local snapshot for new calls, propagate tool-list changes, reject expired/disabled/unassigned tools, apply timeouts/concurrency/cancellation and terminate only owned upstreams. Keep logs on stderr.
  - Verification: Expiry/revocation/same-Mac mutation/timeout/cancel/concurrency/exit tests; stdout framing check.
  - Handoff: Runtime limits, refresh linkage, cleanup guarantees and cancellation boundaries.
- [ ] T4.4 Verify the one-entry-point agent journey and document configuration
  - Task dependencies: T4.3
  - Commit group: gateway-runtime
  - Scope: Exercise standard MCP client fixtures using only the CLI STDIO command. Cover cold offline start, stale cache recovery, no interactive protocol prompts, no side-effect retry and accurate unsupported capabilities. Document a generic agent configuration for manual verification.
  - Verification: End-to-end transport suite plus Rust quality checks and redacted diagnostics.
  - Handoff: Protocol/agent evidence, example configuration, and unsupported-feature matrix.
- [ ] T4.5 User Manual Verification
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: pending

## Phase 5: Bearer selection and OAuth remote connections

- Phase dependencies: P4

### Outcome

Assigned remote MCPs support no auth, user-selected bearer secrets, and verified OAuth connections with encrypted central credential storage.

- [ ] T5.1 Implement bearer secret binding and bounded endpoint credential resolution
  - Task dependencies: none
  - Commit group: remote-bearer
  - Scope: Wire the admin-required bearer mode, current-user key selector API, online binding, scoped secret delivery and Keychain caching into upstream transports. Validate endpoint/origin/resource binding and credential version changes during sync.
  - Verification: Bearer-auth fixture calls, replacement/delete/revocation, missing-key, cross-user and redirect tests.
  - Handoff: Bearer readiness/selection flow, secret version sync and request-header safeguards.
- [ ] T5.2 Verify bearer isolation and offline credential behavior
  - Task dependencies: T5.1
  - Commit group: remote-bearer
  - Scope: Prove no token leaks to ordinary UI reads/logs/unrelated origins, and valid cached credentials never bypass expired policy or enable offline edits. Handle remote 401 without silently changing credentials/mode.
  - Verification: Token/log/DB inspections and offline policy/token expiry integration tests.
  - Handoff: Bearer acceptance evidence and UI error cases.
- [ ] T5.3 Implement OAuth discovery and interactive public-client authorization
  - Task dependencies: T5.2
  - Commit group: remote-oauth
  - Scope: Use SDK-supported MCP auth with validated discovery, admin-supplied pre-registered public client ID, PKCE S256, state/issuer/resource checks and local callback. Connect/reconsent only in the management flow; cancel cleanly and reject unsupported provider requirements.
  - Verification: Controlled OAuth discovery/code flow tests, callback/state/issuer/resource negative tests and provider-error cases.
  - Handoff: OAuth registration/callback contract, supported standards profile and flow state machine.
- [ ] T5.4 Implement encrypted OAuth persistence and coordinated refresh
  - Task dependencies: T5.3
  - Commit group: remote-oauth
  - Scope: Persist access/refresh tokens centrally in managed scoped records, cache in Keychain, acquire refresh coordination/version guards, atomically store rotated credentials and recover after restart. No offline refresh or new OAuth connection when the control plane is unavailable.
  - Verification: Token rotation/concurrent refresh/cold-start/failed-persist/cancel/expiry/offline tests.
  - Handoff: Managed credential ownership/registration scope, refresh coordination and reauth states.
- [ ] T5.5 Verify remote auth integration and document provider setup
  - Task dependencies: T5.4
  - Commit group: remote-oauth
  - Scope: Integrate no-auth/bearer/OAuth tools into the gateway; test scope/token rejection and no central-session passthrough. Record pre-registration steps, supported provider profile, fixture prerequisites and recovery behavior.
  - Verification: Complete remote auth/security suite, stdout/redaction checks and Rust checks.
  - Handoff: Auth compatibility evidence, operator setup, and local UI connect/reconnect requirements.
- [ ] T5.6 User Manual Verification
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: pending

## Phase 6: Administrator React website

- Phase dependencies: P2

### Outcome

Administrators can manage users/groups, catalog, assignments and tool restrictions through the complete central website.

- [ ] T6.1 Design the admin workflows and shared shadcn UI conventions
  - Task dependencies: none
  - Commit group: admin-ui-design
  - Scope: Apply frontend builder/React/shadcn guidance to the approved information architecture. Define user/group/catalog/assignment/tool-policy journeys, validation/permission states, keyboard access and shared TypeScript API/UI primitives.
  - Verification: Review design against FR-022, current API schemas and accessibility requirements.
  - Handoff: Screen/component conventions, API client types and required UI states.
- [ ] T6.2 Implement central administrator screens and permission-aware actions
  - Task dependencies: T6.1
  - Commit group: admin-ui
  - Scope: Build login, users/groups/membership, MCP definitions/install/auth settings, additive assignments and tool allowlist/denylist screens with discovered-name selection and manual-entry fallback. Show effective policy clearly and keep credential values out of catalog and response displays.
  - Verification: Interaction tests for forms, errors, role denials and configuration edits.
  - Handoff: Admin UI routes, API integration and effective-access explanations.
- [ ] T6.3 Verify admin behavior, accessibility and production assets
  - Task dependencies: T6.2
  - Commit group: admin-ui
  - Scope: Exercise real central APIs with fixture data; verify keyboard/focus, long/empty/error/loading states, responsive layout and forbidden access. Build compiled assets served from Rust; run frontend quality checks.
  - Verification: Browser journeys/visual QA, component tests, format/lint/typecheck/build.
  - Handoff: Admin acceptance evidence, supported browsers and production asset integration.
- [ ] T6.4 User Manual Verification
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: pending

## Phase 7: Local user React website

- Phase dependencies: P3, P5

### Outcome

Mac users log in, manage permitted MCPs and encrypted server secrets, connect OAuth, and understand offline/freshness state.

- [ ] T7.1 Design local management, secret selection and OAuth user flows
  - Task dependencies: none
  - Commit group: local-ui-design
  - Scope: Apply frontend builder/React/shadcn guidance to login/catalog/install/enable/secret/OAuth flows, central-vs-endpoint state, one-hour countdown and offline read-only states. Reuse approved shared UI conventions without depending on an undeclared phase's learning.
  - Verification: Review flow coverage against FR-023 and protected local API/auth contract.
  - Handoff: User screen/error/loading conventions and bearer/OAuth interaction definitions.
- [ ] T7.2 Implement local user screens and confirmed-state actions
  - Task dependencies: T7.1
  - Commit group: local-ui
  - Scope: Build login, assigned catalog, progress/results, server-confirmed enable/disable, per-user write-only secret CRUD/key selection, OAuth connect/reconnect/disconnect, diagnostics and cache expiry/offline status. Never optimistically claim successful enablement or expose token values.
  - Verification: User interaction tests for install/server failure, secret binding, auth readiness and offline rejection.
  - Handoff: Local UI routes, protected API integration, redacted status and confirmed-state behavior.
- [ ] T7.3 Verify local browser journeys and embedded production build
  - Task dependencies: T7.2
  - Commit group: local-ui
  - Scope: Exercise both auth modes, config conflicts, identity change/logout, expired cache, offline controls, keyboard/focus and actionable errors. Embed compiled assets in the arm64 CLI and run frontend checks.
  - Verification: Browser E2E/visual/accessibility QA, component tests and format/lint/typecheck/build.
  - Handoff: Local UI acceptance evidence and browser/platform asset requirements.
- [ ] T7.4 User Manual Verification
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: pending

## Phase 8: Complete MVP integration, deployment and Mac packaging

- Phase dependencies: P4, P5, P6, P7

### Outcome

A verified self-hosted deployment and arm64 CLI package support the entire administrator-to-user-to-agent journey.

- [ ] T8.1 Execute end-to-end policy, sync, vault and OAuth acceptance tests
  - Task dependencies: none
  - Commit group: mvp-verification
  - Scope: Run AC-001 through AC-012 with actual backend/UI/CLI plus controlled upstream/OAuth fixtures, deterministic one-hour expiry and actual Mac Keychain/process checks. Cover concurrent clients, redacted logs, same-Mac settings and lost connectivity. Resolve implementation defects within approved scope.
  - Verification: Acceptance matrix with test evidence, full applicable Rust/frontend checks, production asset/browser smoke.
  - Handoff: Cumulative acceptance evidence, platform/provider prerequisites, and unresolved limitations if any.
- [ ] T8.2 Package self-hosted service and Apple Silicon endpoint artifacts
  - Task dependencies: T8.1
  - Commit group: mvp-release
  - Scope: Produce service container/Compose/PostgreSQL migration/health setup with external vault keys and HTTPS termination instructions; build arm64 CLI with embedded UI/assets/checksum. Verify restart durability and backup/restore with separate encrypted-secret key recovery. Keep generated outputs in owned build paths.
  - Verification: Clean deployment/migration/health/backup-restore smoke; archive checksum and Mac startup tests.
  - Handoff: Artifact locations, exact tested toolchain/OS targets, container setup and key-recovery procedures.
- [ ] T8.3 Complete runbooks, compatibility evidence and Definition of Done
  - Task dependencies: T8.2
  - Commit group: mvp-release
  - Scope: Document bootstrap/admin grants/tool policy, Mac install/login/UI, secrets/bearer/OAuth pre-registration, agent STDIO setup, polling/one-hour semantics, offline read-only behavior, logout/diagnostics, key rotation and recovery. Record actual tested agents/protocols/browsers/macOS minimum. Run final quality checks and rehearsal of the full manual journey; do not publish/sign or alter existing user agent configs.
  - Verification: Documented commands on clean deployment/artifact, all checks, final AC evidence and no unowned process/secret output.
  - Handoff: Release/runbook and verification summary sufficient for track-level human verification.
- [ ] T8.4 User Manual Verification
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: pending

## Phase 9: Track-level User Manual Verification

### Outcome

The human verifies the complete track against the approved specification and acceptance criteria: self-hosted admin setup, Apple Silicon local UI/login, install/enable synchronization, encrypted cache/vault, one-hour outage expiry, bearer/OAuth connections, and one STDIO agent entry point.

- [ ] T9.1 User Manual Verification

- Phase completion commit: pending

## Approval

Pending human approval of this specification/plan/learning/state/dependency-context proposal in one combined receipt-backed planning commit. Implementation approval mode is selected by the implement workflow; this planning operation does not claim task execution or verification.

<!-- Regular phases declare dependencies explicitly. Regular tasks declare same-phase dependencies explicitly. -->
<!-- Phase User Manual Verification implicitly depends on every sibling task. -->
<!-- Track-level User Manual Verification implicitly depends on every preceding phase. -->
<!-- For a completed task, append a commit marker containing the full hexadecimal SHA. -->

<!-- Commit groups stay within one phase; preserve task evidence and reject cycles after collapsing groups. -->
