# Plan: macOS enterprise MCP Gateway MVP

- Track: `mac-mvp`
- Spec revision: 2
- Plan revision: 4
- Status: proposed

## Phase 1: Workspace and policy contracts

- Phase dependencies: none

### Outcome

Establish runnable Rust/frontend workspaces, pinned toolchains, domain/API contracts, and fixtures that encode the approved access and freshness rules.

- [x] T1.1 Create the Rust and React/TypeScript workspace foundation <!-- commit: 926e9a41d6d5d01dec79dcef869e91709d054136 -->
  - Task dependencies: none
  - Commit group: workspace-foundation
  - Scope: Initialize Cargo/pnpm packages, stable toolchains/lockfiles, strict TS and shadcn/Vite configuration, platform adapters, scripts, and root documentation. Record the dependency-supported deployment target/MSRV rather than assuming versions.
  - Verification: Rust/frontend build, formatter/linter/type checks, CLI/server help smoke checks.
  - Handoff: Package boundaries, pinned versions, commands, and platform constraints.
- [x] T1.2 Define shared policy, configuration, and credential contracts <!-- commit: 2be53f911eb3f3d89db4278541d3eff36736ff08 -->
  - Task dependencies: T1.1
  - Commit group: policy-contracts
  - Scope: Define stable identities, additive assignments, allowlist/denylist, centrally authoritative enablement, secret references, cache lease, revisions, error/auth modes, and Rust/TS API consistency. Document schema generation/checks and OAuth registration/credential scope.
  - Verification: Contract serialization/validation tests and generated-type consistency check.
  - Handoff: Contract/schema version, invariant mapping to FRs, and API ownership.
- [x] T1.3 Implement the pure policy and lease evaluators with boundary tests <!-- commit: 2be53f911eb3f3d89db4278541d3eff36736ff08 -->
  - Task dependencies: T1.2
  - Commit group: policy-contracts
  - Scope: Implement effective-tool authorization and freshness/identity predicates using injectable clock boundaries. Encode last-grant removal, 3,600-second equality expiry, no renewal from failed sync/token refresh, and known-session rejection.
  - Verification: Policy matrix plus 3,599/3,600/3,601 tests; deny paths prove no dispatch.
  - Handoff: Reusable evaluators and semantics every transport/UI must consume.
- [x] T1.4 Establish fixture services and verification entry points <!-- commit: 187754250b4ff432715796ae512f4121fb2b24db -->
  - Task dependencies: T1.3
  - Commit group: quality-foundation
  - Scope: Create controlled STDIO/HTTP MCP and OAuth fixtures, PostgreSQL test lifecycle, redacted event conventions, and CI/dev entry points. Define safe owned build outputs and avoid production credential dependence.
  - Verification: Fixture startup/shutdown and initial CI command smoke checks.
  - Handoff: Fixture controls, test prerequisites, owned output paths, and supported test environments.
- [x] T1.5 User Manual Verification <!-- commit: 187754250b4ff432715796ae512f4121fb2b24db -->
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: `187754250b4ff432715796ae512f4121fb2b24db`

## Phase 2: Central identity, catalog, enablement, and encrypted secrets

- Phase dependencies: P1

### Outcome

A persisted self-hosted control plane can authenticate users, administer the catalog/access policy, synchronize user settings, and protect user credentials.

- [x] T2.1 Implement administrator bootstrap and username/password sessions <!-- commit: 37911cbc3912e2baca39159c5406235d41be9842 -->
  - Task dependencies: none
  - Commit group: service-identity
  - Scope: Implement migration/role foundations, explicit first-admin bootstrap, users/groups/membership, Argon2id, opaque 12-hour sessions, throttling, password reset, account disablement, and role enforcement.
  - Verification: Integration tests for bootstrap, wrong credentials, role bypass, expiration and session invalidation.
  - Handoff: Auth endpoints, session semantics, migrations, and bootstrap command.
- [x] T2.2 Verify identity isolation and document administration contracts <!-- commit: 37911cbc3912e2baca39159c5406235d41be9842 -->
  - Task dependencies: T2.1
  - Commit group: service-identity
  - Scope: Complete auth boundary and cross-user tests; document administration error/role behavior and redaction. Ensure no default credentials or password values in responses/logs.
  - Verification: Auth suite, logs scan, Rust checks.
  - Handoff: Identity test evidence and session/role constraints for endpoint consumers.
- [x] T2.3 Implement catalog, grants, tool policies, and user-level settings APIs <!-- commit: 8c407380eb3bc0c2fcac6b1310067b5812fea154 -->
  - Task dependencies: T2.2
  - Commit group: service-configuration
  - Scope: Implement catalog CRUD and validation, additive grants, last-grant disablement, tool policy modes, bounded assigned-gateway tool-name inventory, versioned user enablement/bindings, and effective snapshots with validation/expiry times. Mutations use revisions/transactions; configuration remains plaintext centrally. Inventory observations cannot change grants or policy.
  - Verification: Database/API tests for policy matrix, revision conflict, delete/regrant, and snapshot freshness.
  - Handoff: Endpoint snapshot/mutation schema and server-authoritative state semantics.
- [x] T2.4 Verify configuration sync consistency and migration durability <!-- commit: 8c407380eb3bc0c2fcac6b1310067b5812fea154 -->
  - Task dependencies: T2.3
  - Commit group: service-configuration
  - Scope: Test concurrent clients/admin changes, rejected stale updates, restart durability, migrations, tombstones and complete unchanged refresh. Cover non-secret field validation and no unauthorized credential references.
  - Verification: Integration/concurrency/migration tests and serialization checks.
  - Handoff: Verified revision ordering, migration compatibility, and config refresh contract.
- [x] T2.5 Implement encrypted per-user secret and managed credential storage <!-- commit: ce051f249b8110858813339ced107dcb86fb1dec -->
  - Task dependencies: T2.4
  - Commit group: service-secrets
  - Scope: Add write-only key/value secret APIs, stable IDs/versioning, reference-aware deletion, AEAD encryption with external keys/AAD owner binding, and scoped endpoint credential resolution. Add managed OAuth record/version/refresh-coordination APIs without exposing values through ordinary browser reads.
  - Verification: Encryption round-trip/tamper/owner/version tests, plaintext-column inspection, role/ownership tests, and referenced-delete tests.
  - Handoff: Key ownership, secret CRUD/resolution rules, OAuth record/lease contract, and key IDs.
- [x] T2.6 Verify vault recovery, failure paths, and API documentation <!-- commit: ce051f249b8110858813339ced107dcb86fb1dec -->
  - Task dependencies: T2.5
  - Commit group: service-secrets
  - Scope: Exercise missing/wrong keys, cross-user reads, redacted output, credential-version conflicts and serialized refresh lease behavior; document backup/key rotation/restore and secret API use.
  - Verification: Vault integration suite, logs/API read scans, central fmt/Clippy/tests/build.
  - Handoff: Vault test evidence, key recovery prerequisites, and consumer credential rules.
- [x] T2.7 User Manual Verification <!-- commit: ce051f249b8110858813339ced107dcb86fb1dec -->
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: `ce051f249b8110858813339ced107dcb86fb1dec`

## Phase 3: Mac CLI identity, encrypted cache, and online configuration sync

- Phase dependencies: P2

### Outcome

The Apple Silicon endpoint authenticates, protects its local state, refreshes configuration, and enforces the one-hour limit and online-only writes.

- [x] T3.1 Implement CLI management/session and macOS Keychain integration <!-- commit: 723ea7a600bf6797a91bc6d258bf58462ca3d71c -->
  - Task dependencies: none
  - Commit group: endpoint-identity
  - Scope: Add ui/mcp/status/logout command structure, deployment/account scoping, browser opening, central login/session handling, Keychain key/credential adapter, user-only paths and cleanup. Keep STDIO startup independent from browser interaction.
  - Verification: Command/session tests plus actual Mac Keychain round-trip/logout/identity isolation.
  - Handoff: CLI contract, app paths, Keychain identities, and session handoff.
- [x] T3.2 Verify endpoint identity changes and local management security <!-- commit: 723ea7a600bf6797a91bc6d258bf58462ca3d71c -->
  - Task dependencies: T3.1
  - Commit group: endpoint-identity
  - Scope: Implement/test loopback Host/Origin/local auth/CSRF controls, secure browser-session establishment without capability URLs, OAuth callback separation, logout and known-session rejection. Ensure credentials never enter browser storage/read APIs.
  - Verification: Security request tests, redacted status/log scans, cross-login cache checks.
  - Handoff: Protected local API/session protocol and browser access boundaries.
- [x] T3.3 Implement atomic encrypted snapshots and bounded refresh <!-- commit: cd2d02ab4881ccbcb9a0981af9e083aaf504bbbc -->
  - Task dependencies: T3.2
  - Commit group: endpoint-cache
  - Scope: Implement AEAD cache format and authenticated timestamps/revisions, startup/60-second refresh with 10-second request deadline, success-only lease renewal, identity binding, Keychain credentials, atomic shared state and latest-snapshot dispatch access. Handle reconnect and unchanged configs.
  - Verification: Cache crypto/atomic-write/revision tests, fake network/clock refresh tests.
  - Handoff: Cache schema, refresh state machine, source of time, and shared-process snapshot contract.
- [x] T3.4 Verify expiry, restart, clock changes, and recovery <!-- commit: cd2d02ab4881ccbcb9a0981af9e083aaf504bbbc -->
  - Task dependencies: T3.3
  - Commit group: endpoint-cache
  - Scope: Test 3,599/exact-3,600/3,601 seconds, failed/partial responses, token refresh, sleep/wake, process restart, copied cache, missing key, corrupt ciphertext and rollback. Freshness uncertainty fails closed; no indefinite stale fallback.
  - Verification: Deterministic time/network tests and actual Mac restart/sleep-aware clock evidence.
  - Handoff: Freshness proof, offline limitations, and recovery behavior.
- [x] T3.5 Implement online-only local config/install and vault API bridge <!-- commit: 3c0de2000a830cf788851658164d366b2585cb10 -->
  - Task dependencies: T3.4
  - Commit group: endpoint-management
  - Scope: Bridge confirmed central enable/disable/binding/vault mutations using revisions, with no offline writes/queue. Implement assigned install execution/progress/timeout/cancellation and installed-but-disabled recovery. Publish same-Mac config atomically before acknowledging settings changes.
  - Verification: Mutation/install tests for conflicts, outage, expiry, revocation, command arguments, and central commit failure.
  - Handoff: Local endpoints/progress events, server-confirmed settings, and installation lifecycle.
- [x] T3.6 Verify Mac endpoint failure cleanup and documented CLI behavior <!-- commit: 3c0de2000a830cf788851658164d366b2585cb10 -->
  - Task dependencies: T3.5
  - Commit group: endpoint-management
  - Scope: Test all offline mutation rejection paths, interrupted installs, credentials absence, owned process cleanup and stale state prevention. Record CLI usage and diagnostics; run endpoint quality checks.
  - Verification: Mac endpoint integration suite, fmt/Clippy/tests/build, redaction checks.
  - Handoff: Endpoint verification evidence and UI/gateway integration guidance.
- [x] T3.7 User Manual Verification <!-- commit: 3c0de2000a830cf788851658164d366b2585cb10 -->
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: `e23a8e25675117892d381cb4fdc5334875330e13`

## Phase 4: STDIO gateway and upstream tool aggregation

- Phase dependencies: P3

### Outcome

An agent uses one gateway to discover/invoke authorized local and remote tools under current cached policy.

- [x] T4.1 Implement upstream adapters and deterministic tool routing <!-- commit: 38a858e23fde380bf37a7658367715e96cd333e2 -->
  - Task dependencies: none
  - Commit group: gateway-aggregation
  - Scope: Use pinned rmcp transports for child-process STDIO and Streamable HTTP; implement no-auth connection baseline, paginated discovery, schema/result preservation and collision-free deterministic namespacing. Advertise only implemented tool capabilities with 2025-11-25 baseline tests.
  - Verification: Fixture discovery/dispatch/schema/collision/pagination tests and protocol negotiation checks.
  - Handoff: Upstream interfaces, route mapping, capability/SDK constraints and auth hooks.
- [x] T4.2 Verify aggregation against malformed and changing upstreams <!-- commit: 38a858e23fde380bf37a7658367715e96cd333e2 -->
  - Task dependencies: T4.1
  - Commit group: gateway-aggregation
  - Scope: Cover schema/list changes, duplicate names, unavailable/upstream-invalid cases and isolated failures. Report bounded tool-name observations while online and assigned, without credentials/results. Apply the shared policy evaluator on list and call; prove inventory cannot grant access and direct-call bypass is rejected.
  - Verification: Multi-upstream integration tests with call-recording fixtures.
  - Handoff: Aggregation evidence and upstream error/list-update semantics.
- [x] T4.3 Implement fresh-snapshot dispatch and bounded gateway lifecycle <!-- commit: fa504880b4d89cd26df3661c6ddab9bf5c981a2d -->
  - Task dependencies: T4.2
  - Commit group: gateway-runtime
  - Scope: Run refresh in STDIO mode, consume latest committed local snapshot for new calls, propagate tool-list changes, reject expired/disabled/unassigned tools, apply timeouts/concurrency/cancellation and terminate only owned upstreams. Keep logs on stderr.
  - Verification: Expiry/revocation/same-Mac mutation/timeout/cancel/concurrency/exit tests; stdout framing check.
  - Handoff: Runtime limits, refresh linkage, cleanup guarantees and cancellation boundaries.
- [x] T4.4 Verify the one-entry-point agent journey and document configuration <!-- commit: fa504880b4d89cd26df3661c6ddab9bf5c981a2d -->
  - Task dependencies: T4.3
  - Commit group: gateway-runtime
  - Scope: Exercise standard MCP client fixtures using only the CLI STDIO command. Cover cold offline start, stale cache recovery, no interactive protocol prompts, no side-effect retry and accurate unsupported capabilities. Document a generic agent configuration for manual verification.
  - Verification: End-to-end transport suite plus Rust quality checks and redacted diagnostics.
  - Handoff: Protocol/agent evidence, example configuration, and unsupported-feature matrix.
- [x] T4.5 User Manual Verification <!-- commit: fa504880b4d89cd26df3661c6ddab9bf5c981a2d -->
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: `fa504880b4d89cd26df3661c6ddab9bf5c981a2d`

## Phase 5: Bearer selection and OAuth remote connections

- Phase dependencies: P4

### Outcome

Assigned remote MCPs support no auth, user-selected bearer secrets, and verified OAuth connections with encrypted central credential storage.

- [x] T5.1 Implement bearer secret binding and bounded endpoint credential resolution <!-- commit: d4fb795a60fbb6f93a64575d88861f93ba8c0643 -->
  - Task dependencies: none
  - Commit group: remote-bearer
  - Scope: Wire the admin-required bearer mode, current-user key selector API, online binding, scoped secret delivery and Keychain caching into upstream transports. Validate endpoint/origin/resource binding and credential version changes during sync.
  - Verification: Bearer-auth fixture calls, replacement/delete/revocation, missing-key, cross-user and redirect tests.
  - Handoff: Bearer readiness/selection flow, secret version sync and request-header safeguards.
- [x] T5.2 Verify bearer isolation and offline credential behavior <!-- commit: d4fb795a60fbb6f93a64575d88861f93ba8c0643 -->
  - Task dependencies: T5.1
  - Commit group: remote-bearer
  - Scope: Prove no token leaks to ordinary UI reads/logs/unrelated origins, and valid cached credentials never bypass expired policy or enable offline edits. Handle remote 401 without silently changing credentials/mode.
  - Verification: Token/log/DB inspections and offline policy/token expiry integration tests.
  - Handoff: Bearer acceptance evidence and UI error cases.
- [x] T5.3 Implement OAuth discovery and interactive public-client authorization <!-- commit: 64a57dbd489202da80507c361152decd9bb1cae1 -->
  - Task dependencies: T5.2
  - Commit group: remote-oauth
  - Scope: Use SDK-supported MCP auth with validated discovery, admin-supplied pre-registered public client ID, PKCE S256, state/issuer/resource checks and local callback. Connect/reconsent only in the management flow; cancel cleanly and reject unsupported provider requirements.
  - Verification: Controlled OAuth discovery/code flow tests, callback/state/issuer/resource negative tests and provider-error cases.
  - Handoff: OAuth registration/callback contract, supported standards profile and flow state machine.
- [x] T5.4 Implement encrypted OAuth persistence and coordinated refresh <!-- commit: 64a57dbd489202da80507c361152decd9bb1cae1 -->
  - Task dependencies: T5.3
  - Commit group: remote-oauth
  - Scope: Persist access/refresh tokens centrally in managed scoped records, cache in Keychain, acquire refresh coordination/version guards, atomically store rotated credentials and recover after restart. No offline refresh or new OAuth connection when the control plane is unavailable.
  - Verification: Token rotation/concurrent refresh/cold-start/failed-persist/cancel/expiry/offline tests.
  - Handoff: Managed credential ownership/registration scope, refresh coordination and reauth states.
- [x] T5.5 Verify remote auth integration and document provider setup <!-- commit: 64a57dbd489202da80507c361152decd9bb1cae1 -->
  - Task dependencies: T5.4
  - Commit group: remote-oauth
  - Scope: Integrate no-auth/bearer/OAuth tools into the gateway; test scope/token rejection and no central-session passthrough. Record pre-registration steps, supported provider profile, fixture prerequisites and recovery behavior.
  - Verification: Complete remote auth/security suite, stdout/redaction checks and Rust checks.
  - Handoff: Auth compatibility evidence, operator setup, and local UI connect/reconnect requirements.
- [x] T5.6 User Manual Verification <!-- commit: 64a57dbd489202da80507c361152decd9bb1cae1 -->
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: `64a57dbd489202da80507c361152decd9bb1cae1`

## Phase 6: Administrator React website

- Phase dependencies: P2

### Outcome

Administrators can manage users/groups, catalog, assignments and tool restrictions through the complete central website.

- [x] T6.1 Design the admin workflows and shared shadcn UI conventions <!-- commit: 59a60a018461b0506f183305aa9a9db8e459962c -->
  - Task dependencies: none
  - Commit group: admin-ui-design
  - Scope: Apply frontend builder/React/shadcn guidance to the approved information architecture. Define user/group/catalog/assignment/tool-policy journeys, validation/permission states, keyboard access and shared TypeScript API/UI primitives.
  - Verification: Review design against FR-022, current API schemas and accessibility requirements.
  - Handoff: Screen/component conventions, API client types and required UI states.
- [x] T6.2 Implement central administrator screens and permission-aware actions <!-- commit: ee84d90ead2431af8cee1483bccb48e52a3b9fe9 -->
  - Task dependencies: T6.1
  - Commit group: admin-ui
  - Scope: Build login, users/groups/membership, MCP definitions/install/auth settings, additive assignments and tool allowlist/denylist screens with discovered-name selection and manual-entry fallback. Show effective policy clearly and keep credential values out of catalog and response displays.
  - Verification: Interaction tests for forms, errors, role denials and configuration edits.
  - Handoff: Admin UI routes, API integration and effective-access explanations.
- [x] T6.3 Verify admin behavior, accessibility and production assets <!-- commit: ee84d90ead2431af8cee1483bccb48e52a3b9fe9 -->
  - Task dependencies: T6.2
  - Commit group: admin-ui
  - Scope: Exercise real central APIs with fixture data; verify keyboard/focus, long/empty/error/loading states, responsive layout and forbidden access. Build compiled assets served from Rust; run frontend quality checks.
  - Verification: Browser journeys/visual QA, component tests, format/lint/typecheck/build.
  - Handoff: Admin acceptance evidence, supported browsers and production asset integration.
- [x] T6.4 User Manual Verification <!-- commit: ee84d90ead2431af8cee1483bccb48e52a3b9fe9 -->
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: `ee84d90ead2431af8cee1483bccb48e52a3b9fe9`

## Phase 7: Local user React website

- Phase dependencies: P3, P5

### Outcome

Mac users log in, manage permitted MCPs and encrypted server secrets, connect OAuth, and understand offline/freshness state.

- [x] T7.1 Design local management, secret selection and OAuth user flows <!-- commit: 6a0d0e951ba5f59f7b8db685bf4dbe7ac0ddc558 -->
  - Task dependencies: none
  - Commit group: local-ui-design
  - Scope: Apply frontend builder/React/shadcn guidance to login/catalog/install/enable/secret/OAuth flows, central-vs-endpoint state, one-hour countdown and offline read-only states. Reuse approved shared UI conventions without depending on an undeclared phase's learning.
  - Verification: Review flow coverage against FR-023 and protected local API/auth contract.
  - Handoff: User screen/error/loading conventions and bearer/OAuth interaction definitions.
- [x] T7.2 Implement local user screens and confirmed-state actions <!-- commit: 828be9a37bd0fc518011e88774e2e82153a5b58f -->
  - Task dependencies: T7.1
  - Commit group: local-ui
  - Scope: Build login, assigned catalog, progress/results, server-confirmed enable/disable, per-user write-only secret CRUD/key selection, OAuth connect/reconnect/disconnect, diagnostics and cache expiry/offline status. Never optimistically claim successful enablement or expose token values.
  - Verification: User interaction tests for install/server failure, secret binding, auth readiness and offline rejection.
  - Handoff: Local UI routes, protected API integration, redacted status and confirmed-state behavior.
- [x] T7.3 Verify local browser journeys and embedded production build <!-- commit: 828be9a37bd0fc518011e88774e2e82153a5b58f -->
  - Task dependencies: T7.2
  - Commit group: local-ui
  - Scope: Exercise both auth modes, config conflicts, identity change/logout, expired cache, offline controls, keyboard/focus and actionable errors. Embed compiled assets in the arm64 CLI and run frontend checks.
  - Verification: Browser E2E/visual/accessibility QA, component tests and format/lint/typecheck/build.
  - Handoff: Local UI acceptance evidence and browser/platform asset requirements.
- [x] T7.4 User Manual Verification <!-- commit: 828be9a37bd0fc518011e88774e2e82153a5b58f -->
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: `828be9a37bd0fc518011e88774e2e82153a5b58f`

## Phase 8: Complete MVP integration, deployment and Mac packaging

- Phase dependencies: P4, P5, P6, P7

### Outcome

A verified self-hosted deployment and arm64 CLI package support the entire administrator-to-user-to-agent journey.

- [x] T8.1 Execute end-to-end policy, sync, vault and OAuth acceptance tests <!-- commit: bb74928cfc0e96ee682b998d75da5a7195b691a9 -->
  - Task dependencies: none
  - Commit group: mvp-verification
  - Scope: Run AC-001 through AC-012 with actual backend/UI/CLI plus controlled upstream/OAuth fixtures, deterministic one-hour expiry and actual Mac Keychain/process checks. Cover concurrent clients, redacted logs, same-Mac settings and lost connectivity. Resolve implementation defects within approved scope.
  - Verification: Acceptance matrix with test evidence, full applicable Rust/frontend checks, production asset/browser smoke.
  - Handoff: Cumulative acceptance evidence, platform/provider prerequisites, and unresolved limitations if any.
- [x] T8.2 Package self-hosted service and Apple Silicon endpoint artifacts <!-- commit: 4469f1aaba87f2f1d447900da28afc2a78bfe860 -->
  - Task dependencies: T8.1
  - Commit group: mvp-release
  - Scope: Produce service container/Compose/PostgreSQL migration/health setup with external vault keys and HTTPS termination instructions; build arm64 CLI with embedded UI/assets/checksum. Verify restart durability and backup/restore with separate encrypted-secret key recovery. Keep generated outputs in owned build paths.
  - Verification: Clean deployment/migration/health/backup-restore smoke; archive checksum and Mac startup tests.
  - Handoff: Artifact locations, exact tested toolchain/OS targets, container setup and key-recovery procedures.
- [x] T8.3 Complete runbooks, compatibility evidence and Definition of Done <!-- commit: 4469f1aaba87f2f1d447900da28afc2a78bfe860 -->
  - Task dependencies: T8.2
  - Commit group: mvp-release
  - Scope: Document bootstrap/admin grants/tool policy, Mac install/login/UI, secrets/bearer/OAuth pre-registration, agent STDIO setup, polling/one-hour semantics, offline read-only behavior, logout/diagnostics, key rotation and recovery. Record actual tested agents/protocols/browsers/macOS minimum. Run final quality checks and rehearsal of the full manual journey; do not publish/sign or alter existing user agent configs.
  - Verification: Documented commands on clean deployment/artifact, all checks, final AC evidence and no unowned process/secret output.
  - Handoff: Release/runbook and verification summary sufficient for track-level human verification.
- [x] T8.4 User Manual Verification <!-- commit: 4469f1aaba87f2f1d447900da28afc2a78bfe860 -->
  - Verification: Check this phase's outcome and recorded evidence against its specification requirements under the persisted execution approval mode.

- Phase completion commit: `4469f1aaba87f2f1d447900da28afc2a78bfe860`

## Phase 9: Track-level User Manual Verification

### Outcome

The human verifies the complete track against the approved specification and acceptance criteria: self-hosted admin setup, Apple Silicon local UI/login, install/enable synchronization, encrypted cache/vault, one-hour outage expiry, bearer/OAuth connections, and one STDIO agent entry point.

- [x] T9.1 User Manual Verification <!-- commit: 4469f1aaba87f2f1d447900da28afc2a78bfe860 -->

- Phase completion commit: `4469f1aaba87f2f1d447900da28afc2a78bfe860`

## Phase 10: Bounded endpoint credential metadata

- Phase dependencies: P9

### Outcome

Credential capacity failures remain local to the affected MCP and cannot make the shared identity unreadable or prevent cleanup (BUG-001).

- [x] T10.1 Prevent metadata publication beyond readable invariants <!-- commit: 798b0fb0cc049842c81e28f6e7e36235ac8a71bf -->
  - Task dependencies: none
  - Commit group: endpoint-metadata-remediation
  - Scope: Apply one consistent validation contract before durable metadata writes and reads. Preflight bounded credential-head/publication allocation; reject capacity safely or retire obsolete scope with protected cleanup. Preserve existing rollback/publication fences, token anchors, journal recoverability, and bounded file/account limits. No partial capacity failure may corrupt the identity shared by unrelated MCPs.
  - Verification: Reproduce the 128/129 sequential MCP boundary with one active MCP at a time; failed allocation leaves status, cache/sync, existing credentials and logout usable. Test bounded write/read parity and failed-publication cleanup with native credential/cache/session regressions.
  - Handoff: Capacity behavior, protected account/cleanup invariants, actual regression evidence and operator recovery guidance.
- [x] T10.2 User Manual Verification <!-- commit: 798b0fb0cc049842c81e28f6e7e36235ac8a71bf -->
  - Verification: Verify BUG-001 acceptance criteria and original identity, lease, isolation and cleanup requirements under persisted Autonomous authority; record actual checks separately from authorization.

- Phase completion commit: `798b0fb0cc049842c81e28f6e7e36235ac8a71bf`

## Phase 11: Consistent administrator state and lossless catalog editing

- Phase dependencies: P9

### Outcome

Both websites accept valid bounded catalog data, administrator edits preserve structured commands, and revision-checked saves use a consistent read set (BUG-002, BUG-003, BUG-004).

- [x] T11.1 Align catalog runtime decoders and preserve structured command values <!-- commit: 829c6cd9ac24975bca68052861e8b3777042615f -->
  - Task dependencies: none
  - Commit group: browser-contract-remediation
  - Scope: Align runtime decoder and form limits with the current valid server contract without tightening existing server data or weakening strict TypeScript. Support valid 200-byte MCP names and environment keys within the bounded catalog budget. Make argument/environment editing lossless for empty strings, embedded newlines and literal characters in runtime and installation commands, including tool-policy-only or unrelated saves. Use clearly described structured input where needed; do not silently reinterpret values.
  - Verification: Actual Rust-valid boundary fixtures decode in both websites; invalid over-limit input still fails. Behavioral tests load/manage long names and environment keys, and assert unchanged and explicit edited command arrays/maps survive save exactly. Frontend tests, format/lint/typecheck/build and targeted compiled-browser keyboard/responsive checks pass.
  - Handoff: Shared contract bounds, command editing semantics and compatibility/validation evidence.
- [x] T11.2 Bind all admin editor data to a consistent revision <!-- commit: 829c6cd9ac24975bca68052861e8b3777042615f -->
  - Task dependencies: T11.1
  - Commit group: browser-contract-remediation
  - Scope: Replace independently combined admin configuration/users/groups reads with an atomic or revision-verified read set for initial loading and post-write reload. A concurrent write must yield a coherent view or explicit conflict/reload; never attach a newer revision to stale role/active/member data. Preserve frozen editor revisions and no automatic mutation replay.
  - Verification: Deterministically interleave membership removal/account changes with initial and follow-up reads; no stale roster or role can be saved under a newer revision. Verify conflict feedback/reload, one confirmed mutation/no replay and failed follow-up read handling through user-visible tests and real service/browser smoke.
  - Handoff: Read consistency and failure behavior, controlled race proof, resulting API/UI contract and evidence.
- [x] T11.3 User Manual Verification <!-- commit: 829c6cd9ac24975bca68052861e8b3777042615f -->
  - Verification: Verify BUG-002 through BUG-004, NFR-004/005 and AC-011 using actual tests/browser evidence under persisted Autonomous authority.

- Phase completion commit: `829c6cd9ac24975bca68052861e8b3777042615f`

## Phase 12: Rebuilt and verified MVP artifacts

- Phase dependencies: P10, P11

### Outcome

Current container and Apple Silicon archive include all reviewed fixes, and runbooks identify the actual verified artifacts.

- [x] T12.1 Rebuild release artifacts and run cumulative acceptance checks <!-- commit: b3ea6ab52ce58249946831d67cba30106cbd8f90 -->
  - Task dependencies: none
  - Commit group: reviewed-mvp-release
  - Scope: Rebuild embedded websites, self-hosted image and arm64 archive/checksum from the fixed source. Update release/acceptance and affected UI/native documentation. Re-run applicable workspace/frontend/contract/real-Postgres checks, packaged Mac/Keychain/STDIO and disposable deployment-to-agent/restart/backup/restore/key recovery journey. Reuse unchanged earlier evidence only with source/input/policy identity verified. Clean owned test resources and retain honest compatibility limits.
  - Verification: Full relevant quality checks and all four regression witnesses pass; final archive checksum/source identity, compiled browser flow, actual Mac one-entry client, Node socketpair launch, Compose health/restart/fresh restore and encrypted-secret key recovery pass. No production credentials/publication/signing or existing agent configuration mutation.
  - Handoff: Exact commits/artifact checksums, acceptance evidence, remaining documented compatibility limits and cleanup witness.
- [x] T12.2 User Manual Verification <!-- commit: b3ea6ab52ce58249946831d67cba30106cbd8f90 -->
  - Verification: Verify all original AC-001 through AC-012 and four finding criteria against current integrated source/artifacts; record automated verification without claiming human approval.

- Phase completion commit: `b3ea6ab52ce58249946831d67cba30106cbd8f90`

## Phase 13: Track-level User Manual Verification

### Outcome

Verify the complete integrated MVP and remediation against the unchanged approved specification before cumulative autonomous review and the final explicit human completion decision.

- [x] T13.1 User Manual Verification <!-- commit: b3ea6ab52ce58249946831d67cba30106cbd8f90 -->

- Phase completion commit: `b3ea6ab52ce58249946831d67cba30106cbd8f90`

## Phase 14: Complete administrator collection contract

- Phase dependencies: P13

### Outcome

Valid administrator collections remain loadable beyond10,000 records while actual server-defined field and collection limits stay enforced (recurring BUG-002).

- [x] T14.1 Align collection decoding with backend contracts <!-- commit: c96f9c4abdc368331689f122f5abb301a9b62625 -->
  - Task dependencies: none
  - Commit group: admin-collection-contract
  - Scope: Remove the arbitrary generic browser row cap from server-unbounded user/group/assignment collections. Keep explicit limits matching actually bounded contracts, including10,000 group members,512 MCPs and bounded commands/tools. Validate every element and preserve strict TypeScript, revision-bracketed reads and safe error handling. Add regression coverage for large valid read sets and malformed/over-limit bounded data; do not tighten valid backend data or expand scope into pagination/performance promises.
  - Verification: Reproduce10,000/10,001 on current decoders, then accept valid larger assignment/user/group collections. Verify malformed late elements and actual bounded fields still reject. Confirm central collection constraints with source and real isolated PostgreSQL/API data; frontend behavioral/read-helper tests, strictTS/lint/format/build/contracts pass.
  - Handoff: Exact aligned contracts, source/API/decoder evidence, preserved security/concurrency checks.
- [x] T14.2 Rebuild current artifacts and verify changed embedded assets <!-- commit: 52c288b70ad0d8839aea8b3f12952ecd3e0cb26d -->
  - Task dependencies: T14.1
  - Commit group: collection-release
  - Scope: Rebuild current service container and Apple Silicon embedded archive/checksum. Update acceptance/release documentation with current artifact identities and regression results. Run actual packaged Mac/client and disposable deployment-to-agent smoke for the changed embedded build; reuse unchanged Rust/security/database/restore and compiled-browser layout evidence only after source/input/policy checks. Clean owned fixtures and retain documented compatibility limits.
  - Verification: Currentfrontend/embedded builds, archive checksum/source identity, actual packagedMac STDIO/Keychain and freshcontainer/Node one-entry journey pass. Existing83Rust/8PG/fullrestore checks may be reused only for unchanged relevant source; no publication/signing or production mutations.
  - Handoff: Exact artifact hashes and validation/cleanup evidence.
- [x] T14.3 User Manual Verification <!-- commit: 52c288b70ad0d8839aea8b3f12952ecd3e0cb26d -->
  - Verification: Verify recurringBUG-002 and unchanged originalAC001–012 under persisted Autonomous authority; distinguish automated verification from human approval.

- Phase completion commit: `52c288b70ad0d8839aea8b3f12952ecd3e0cb26d`

## Phase 15: Track-level User Manual Verification

### Outcome

Verify the complete canonical MVP and all remediation against unchanged scope before cumulative review and one explicit human completion decision.

- [x] T15.1 User Manual Verification <!-- commit: 52c288b70ad0d8839aea8b3f12952ecd3e0cb26d -->

- Phase completion commit: `52c288b70ad0d8839aea8b3f12952ecd3e0cb26d`

## Phase 16: Email identity and display names

- Phase dependencies: P15

### Outcome

User management requires display name/email/password and email is the sole sign-in identity (BUG-005, FR-025/AC-013).

- [ ] T16.1 Implement email/name persistence, contracts and authentication
  - Task dependencies: none
  - Commit group: email-profile-identity
  - Scope: Add immutable forward migration for fresh pre-release identity schema with explicit safe rejection of populated old accounts. Replace username wire/storage/login fields with canonical email and separate display name. Validate/uniquely constrain normalized email and bounded display names; bootstrap/create/edit/login/session/native status all carry consistent fields. Preserve role/revision/CSRF/last-admin checks, password secrecy, session revocation and stable identity ownership. Update fixtures/code generation; no legacy login fallback.
  - Verification: RealPG identity/migration/CRUD/normalized login/duplicate concurrent writes/name edits/stale revision/role/password reset/session revocation and ownership tests; current Rust/workspace/native/security/contracts checks pass.
  - Handoff: Explicit identity/email normalization and deployment compatibility contract, migration and API/native evidence.
- [ ] T16.2 Implement and verify both email-based websites
  - Task dependencies: T16.1
  - Commit group: email-profile-identity
  - Scope: Admin create/edit name/email/password and name/email listings; both login screens use email/password. Display names in account/group/access surfaces with email for disambiguation. Keep write-only/reset and coherent revision reads/frozen editors, accessible feedback/focus and responsive shadcn composition. Update strict decoders/generated contracts and behavioral tests; close BUG-005 only with actual evidence.
  - Verification: Frontend tests/strictTS/lint/format/build; actual compiled Mac/central browser create/edit/login/duplicate/stale/password flows at desktop/narrow widths, keyboard and no relevant console errors. No display-name/username sign-in.
  - Handoff: Browser screenshots/evidence, field semantics and contract/security regression proof.
- [ ] T16.3 Rebuild and verify the email-identity MVP artifacts
  - Task dependencies: T16.2
  - Commit group: email-identity-release
  - Scope: Update bootstrap/operator/Mac/agent/API/compatibility/release runbooks and bug record. Rebuild image/arm64 embedded archive/checksum. Run full relevant Rust/frontend/realPG checks and current package oneSTDIO/nativeKeychain/fourauth/deployment/restart/freshrestore/keyrecovery journey. Clean exact owned fixtures; no production data deletion, publication or signing.
  - Verification: All AC001–013 current evidence, archive/source/hash identity, full relevant checks and disposable actual deployment-to-Mac-to-agent/restore proof. Explicit prerelease freshdatabase/no legacycompatibility documented.
  - Handoff: Final artifacts, exact checksums/platform limits and cumulative acceptance evidence.
- [ ] T16.4 User Manual Verification
  - Verification: Verify BUG-005/AC013 plus original security/policy/lease/credential/agent journeys under inherited Autonomous authority; record actual evidence without human approval claims.

- Phase completion commit: pending

## Phase 17: Track-level User Manual Verification

### Outcome

Verify the full integrated revised MVP before cumulative review and explicit final completion approval.

- [ ] T17.1 User Manual Verification

- Phase completion commit: pending

## Approval

Pending human approval of this specification/plan/learning/state/dependency-context proposal in one combined receipt-backed planning commit. Implementation approval mode is selected by the implement workflow; this planning operation does not claim task execution or verification.

<!-- Regular phases declare dependencies explicitly. Regular tasks declare same-phase dependencies explicitly. -->
<!-- Phase User Manual Verification implicitly depends on every sibling task. -->
<!-- Track-level User Manual Verification implicitly depends on every preceding phase. -->
<!-- For a completed task, append a commit marker containing the full hexadecimal SHA. -->

<!-- Commit groups stay within one phase; preserve task evidence and reject cycles after collapsing groups. -->
