# Engineering Guidelines

## Architecture

- Use a Rust Cargo workspace with explicit boundaries for domain/policy logic, central service, endpoint CLI/gateway, and MCP transport adapters. Keep policy evaluation independent of HTTP and SDK handlers.
- Use React + TypeScript + shadcn/ui for both the central admin site and the locally served user interface. Share useful UI primitives and API contracts while preserving their different permissions and execution contexts.
- Separate the local management server/login lifecycle from STDIO MCP execution. Gateway stdout contains protocol traffic only; diagnostics go to stderr. Agent startup must not require an interactive browser prompt in the STDIO stream.
- The central service owns identity, catalog, grants, tool policy and per-user enablement/bindings; the endpoint owns installation, process management and native credential readiness. Never substitute browser filtering for server or gateway authorization.
- Apply the same effective-tool evaluator to discovery and dispatch. Resolve upstream names deterministically and route calls to the correct original tool identifier; unknown or denied names are rejected.
- Use typed command and transport definitions. Prefer executable plus argument arrays over shell interpolation; explicitly model any supported shell-command execution.
- Target macOS first. Isolate platform paths, credential storage, browser opening, and child-process behavior behind small interfaces for future ports.
- Keep later SaaS and SAML boundaries visible without implementing speculative tenant infrastructure or identity flows in the initial self-hosted release.
- Rust conventions: rustfmt and Clippy, typed errors at public boundaries, Result-based failure handling, no routine panic/unwrap on external input, and preserve the workspace prohibition on unsafe code.
- Frontend conventions: strict TypeScript for application and configuration source; no new application JavaScript. Use shadcn component composition and semantic theme tokens, accessible labels/focus, and explicit loading, empty, success, and error states. Keep state local and avoid derived-state Effects or speculative memoization.

## Testing strategy

- Test policy behavior with a matrix of no assignments, user/group/all-users grants, disabled MCPs, allowlists, denylists, newly discovered tools, and attempted direct invocation.
- Test central authentication and authorization boundaries, cross-user configuration isolation, logout/session invalidation, and persisted configuration migrations.
- Use fixture MCP servers to exercise STDIO and Streamable HTTP discovery/dispatch, duplicate tool names, malformed upstream responses, timeouts, cancellation, and child-process exit/cleanup.
- Exercise macOS installation success/failure, re-enable behavior, executable/argument handling, local UI protection, and separation of gateway stdout from logs.
- Test React flows through user-visible interactions and accessibility; include login, assigned catalog visibility, installation feedback, and enable/disable controls.
- Run an end-to-end self-hosted/macOS journey with an agent connected only to the gateway before claiming the initial release is ready.
- Reuse passing evidence only while relevant source, inputs and verification policy are unchanged. The four archived tracks record the delivered baseline; current context refresh does not claim a new application test run.
- Keep fixture namespaces private and generated. Retain the immutable creating CLI until its Keychain cleanup is confirmed; remove only exact owned databases/processes/Compose stacks and retain recovery journals after failure.

## Security and privacy

- Default deny MCP access. Authenticate principals and enforce administrator/user separation centrally; validate all externally supplied definitions and configuration on the endpoint.
- User enablement never bypasses assignment or tool restrictions. Recheck current protected authorization at dispatch and completion. Complete-only durable refresh renews a lease of at most 3,600 seconds; equality expires, known session rejection invalidates immediately, and uncertain clocks/cache state require online recovery.
- Use salted Argon2id password hashing with reviewed parameters, login throttling, and expiring/revocable sessions. Do not persist user passwords locally.
- Store one random master in macOS Keychain. Publish synced encrypted records and index before switching the protected account identifier and key together. Pin the credential selection and refusal stamp across network I/O, including a first fetch. Use private user-scoped storage; bind stable identity/installation/generation/revision/expiry and protect rollback/publication checkpoints. Ordinary central configuration stays plaintext; server user-secret/OAuth values use owner/version-bound AEAD with external keys.
- Protect loopback HTTP against untrusted origins, DNS rebinding, and CSRF. Bind local management to loopback; use short-lived local authorization and keep sessions/secrets out of URLs and frontend storage.
- Use verified TLS for central/remote production connections. Model upstream credential scope explicitly and minimize inherited environment variables and secrets passed to child processes.
- Installation runs with the user's privileges after their explicit action. Catalog browsing, login, and agent startup must not silently execute installation commands. Validate command definitions and report failures without enabling the MCP.
- Redact credentials and sensitive inputs from logs and UI. Record identity, MCP/tool identifiers, policy decision, and outcome for relevant management and gateway events; payload logging and retention require an explicit specification.
- Treat upstream names, schemas, descriptions, and results as untrusted input; avoid unsafe HTML rendering and uncontrolled output sizes.

## Reliability and observability

- Bound upstream startup, install execution, connection attempts, tool execution, and shutdown. Define timeout/cancellation behavior in track acceptance criteria.
- Isolate upstream failures so one unavailable MCP has an actionable status without corrupting the agent-facing protocol or hiding other available MCPs.
- Clean up owned child processes and connections on disable, logout, gateway exit, and interrupted startup as specified. Do not kill unrelated system processes.
- Do not automatically retry calls that may have side effects. Record correlation IDs, durations, and failure categories without sensitive payloads.
- Version persisted policy/configuration and API contracts. Central enablement is revision-checked and server-confirmed; every offline mutation is rejected without a queue. Pin the original generation across I/O and atomically publish only complete current data.
- Central migrations, backup/restore, health checks, and local diagnostic commands are delivery concerns with concrete verification before release.

## Documentation and compatibility

- Document self-hosted deployment, macOS CLI setup, login, coding-agent STDIO configuration, catalog/assignment administration, tool-policy semantics, and troubleshooting.
- Document tested macOS releases/architectures, supported coding agents/MCP versions, and remote transport/auth requirements.
- Keep public Serde/ts-rs Rust and generated TypeScript contracts consistent with scripts/check-contracts.sh. Runtime-decode responses with actual backend UTF-8/collection bounds; coherent revision-bracketed admin reads and frozen editor revisions prevent stale security data from inheriting newer authority.
- Verify compatibility and feature flags against the pinned official Rust MCP SDK rather than assuming every protocol capability is proxied.
- Preserve protocol negotiation and document unsupported features. Expand compatibility only through explicit acceptance criteria.
- Follow the bundled Cadre workflow for approved tracks, verification, review, receipts, and commits.

## Concrete identity, browser and credential invariants

- Email is the sole canonical login; Name is display-only and stable user IDs own
  membership/grants/credentials. Normalize/validate centrally, constrain unique
  emails and recheck original credentials under the final login lock after password
  verification. Profile/password/account changes revoke affected sessions.
- Preserve the explicit prerelease identity contract: fresh DB for migration 0004,
  safe refusal of populated older accounts, no silent deletion or legacy alias.
  A future live-data migration requires a separately scoped compatibility design.
- Edit structured runtime/install argument arrays and environment maps losslessly
  as JSON; policy-only saves must not reinterpret empty strings or literal newlines.
- Validate bounded metadata identically on read/write; preflight allocation before
  persistence/Keychain side effects. Publication failure cannot corrupt unrelated
  identity/credentials/logout. Preserve immutable heads and cleanup barriers.
- Managed OAuth refresh uses central lease/version CAS and complete-response-only
  recovery; never replay a provider exchange or renew native token lifetime while
  recovering. Keep auth values out of browser reads, logs and ordinary files.

## Repository-specific commands

Use pinned Node 24.19.0/pnpm 11.4.0 and Rust 1.95.0; Cargo/pnpm lockfiles are authoritative.
During Cadre execution use its returned owned DHIVON_MCP_GATEWAY_BUILD_CACHE/CARGO_TARGET_DIR;
compile embedded assets with DHIVON_MCP_GATEWAY_WEB_ASSETS pointing at the current web output.
See docs/testing.md for isolated fixtures and exact environment setup.

- Rust: cargo fmt --all -- --check; cargo clippy --locked --workspace --all-targets
  -- -D warnings; cargo test --locked --workspace; cargo build --locked --workspace.
- Frontend: pnpm install --frozen-lockfile; pnpm format:check; pnpm lint;
  pnpm typecheck; pnpm test; pnpm build. Protected web applications use strict TypeScript.
- Contracts: bash scripts/check-contracts.sh. Never hand-edit generated TS.
- Database: explicitly run cargo test --locked -p dhivon-mcp-gateway-server -- --ignored with
  the isolated TEST_DATABASE_URL. Normal cargo test skips these external cases;
  the CLI ignored native_cache_child marker is invoked by its actual Mac parent.
- Browser: exercise compiled central/local assets with controlled services,
  desktop/390px layout, keyboard/focus/feedback and console checks.
- Release: build deploy/Dockerfile; bash scripts/package-macos.sh on Apple Silicon;
  test the extracted DHIVON_MCP_GATEWAY_TEST_CLI with the CLI gateway suite; run
  node scripts/verify-deployment.ts with DHIVON_MCP_GATEWAY_TEST_CLI/DHIVON_MCP_GATEWAY_FIXTURE_BIN for
  the full container/Mac/Node/Keychain journey and fresh restore/key recovery.
- shadcn: inspect components.json and current component docs before additions.
  Existing semantic primitives, strict TypeScript and accessibility remain required.

## Durable learning and limits

Consult .cadre/patterns/index.md by semantic applicability before planning new
work; read relevant pattern bodies and preserve their provenance/limits. Archived
mac-mvp and dhivon-brand-site-docs resolve delivered scope without rewriting their
historical records.
Actual supported platforms/providers/protocol/browsers are in docs/compatibility.md;
dated release/test evidence is in docs/checksums.md, docs/consumer-setup-verification.md and docs/dhivon-release.md, with original MVP
evidence retained in docs/release.md and docs/acceptance.md. Do not infer
unrun hosted CI, signing, real providers/agents or extra platform compatibility.

## Delivered Dhivon product conventions

- Dhivon MCP Gateway is a product of Dhivon AI. Keep current CLI/package/image,
  `DHIVON_MCP_GATEWAY_*` and native namespace identifiers aligned with manifests.
  Preserve standard external names and historical receipts/reports; fresh-install
  scope grants no authority to migrate/delete unrelated older state.
- `apps/site` renders canonical repository Markdown independently of protected
  React admin/local assets. Maintain one source per guide and verify deep links,
  anchors and copied assets after final evidence edits. Explain policy ownership,
  opt-in and execution location with the documented first-call/denial journey.
- Use vendored approved Soft Gate assets and local Manrope from assets/brand.
  Preserve full original font notices and the existing CSP; check emitted SVG URLs
  in compiled artifacts. Read the brand usage/contrast guidance before changes.
- Retain Apache-2.0 project licensing, Copyright 2026 Dhivon AI and third-party
  ownership. Verify actual archive/image/site LICENSE/NOTICE/OFL bytes with
  scripts/check-notices.ts; source metadata alone is insufficient delivery proof.
- Distinguish verified HTTPS service origins from native loopback callbacks.
  Supported provider registrations/ports need evidence. Keep TLS checks enabled,
  state/issuer/PKCE and explicit enablement intact; temporary trust requires its
  separately authorized operator cleanup. Controlled simulation does not certify
  public hosting, named providers or all browser behavior.
- After validated online OAuth resolution, refresh expired records through the
  existing lease/version CAS before publishing a rejected old binding. Never clear
  rejection fences, refresh expired credentials offline or replay a tool call.
- Verification harnesses retain handles before fallible post-spawn registration,
  fail closed on unknown ownership, and hold exclusive recovery before journal
  reads through final evidence writes. Drain cleanup-created children too. Stop
  writers before creating-binary logout; verify native cleanup before stack removal.
  Keep failed histories and exact recovery evidence; never reclaim locks through
  stale-PID unlink/recreate. The private recovery harness uses Python3/fcntl. The separate operator setup helper also requires Python 3.8 or later.
- Public site checks: pnpm --filter @dhivon/mcp-gateway-site typecheck;
  pnpm exec prettier --check apps/site; pnpm site:build; node apps/site/check.ts; run the emitted notice
  checker and rendered link/anchor/browser checks described in release evidence.
  Root pnpm build/typecheck/test target the protected web workspace, not the site.
- Run the 25-test rehearsal lifecycle/ownership suites with pinned Node 24, Docker,
  Python3 and the extracted CLI only in runtime-owned generated scopes, following
  docs/https-oauth-rehearsal.md. Reuse unaffected product evidence by exact inputs;
  report script checks separately from full browser/OAuth acceptance.

- Keep setup prompts and the service health probe consistent with the server’s
  origin parser. Test with different database and operator UIDs, an interrupted
  setup, and a failed export that must preserve the last complete backup.
- Verify the static publication allowlist, missing routes and current stylesheet
  on the production hostname. Keep public guides and assets static; the scoped
  Pages Function serves a non-cacheable 404 for unpublished documentation.
