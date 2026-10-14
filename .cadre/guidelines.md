# Engineering Guidelines

## Architecture

- Use a Rust Cargo workspace with explicit boundaries for domain/policy logic, central service, endpoint CLI/gateway, and MCP transport adapters. Keep policy evaluation independent of HTTP and SDK handlers.
- Use React + TypeScript + shadcn/ui for both the central admin site and the locally served user interface. Share useful UI primitives and API contracts while preserving their different permissions and execution contexts.
- Separate the local management server/login lifecycle from STDIO MCP execution. Gateway stdout contains protocol traffic only; diagnostics go to stderr. Agent startup must not require an interactive browser prompt in the STDIO stream.
- The central service owns catalog and access policy; the endpoint owns installation, process management, and local enablement. Never substitute browser filtering for server or gateway authorization.
- Apply the same effective-tool evaluator to discovery and dispatch. Resolve upstream names deterministically and route calls to the correct original tool identifier; unknown or denied names are rejected.
- Use typed command and transport definitions. Prefer executable plus argument arrays over shell interpolation; explicitly model any supported shell-command execution.
- Target macOS first. Isolate platform paths, credential storage, browser opening, and child-process behavior behind small interfaces for future ports.
- Keep later SaaS and SAML boundaries visible without implementing speculative tenant infrastructure or identity flows in the initial self-hosted release.
- Rust conventions: rustfmt and Clippy, typed errors at public boundaries, Result-based failure handling, no routine panic/unwrap on external input, and no unsafe code without a documented need and review.
- Frontend conventions: strict TypeScript for application and configuration source; no new application JavaScript. Use shadcn component composition and semantic theme tokens, accessible labels/focus, and explicit loading, empty, success, and error states. Keep state local and avoid derived-state Effects or speculative memoization.

## Testing strategy

- Test policy behavior with a matrix of no assignments, user/group/all-users grants, disabled MCPs, allowlists, denylists, newly discovered tools, and attempted direct invocation.
- Test central authentication and authorization boundaries, cross-user configuration isolation, logout/session invalidation, and persisted configuration migrations.
- Use fixture MCP servers to exercise STDIO and Streamable HTTP discovery/dispatch, duplicate tool names, malformed upstream responses, timeouts, cancellation, and child-process exit/cleanup.
- Exercise macOS installation success/failure, re-enable behavior, executable/argument handling, local UI protection, and separation of gateway stdout from logs.
- Test React flows through user-visible interactions and accessibility; include login, assigned catalog visibility, installation feedback, and enable/disable controls.
- Run an end-to-end self-hosted/macOS journey with an agent connected only to the gateway before claiming the initial release is ready.
- Verification belongs to the relevant approved track. Cadre setup creates documentation only; do not claim application tests ran before source exists.

## Security and privacy

- Default deny MCP access. Authenticate principals and enforce administrator/user separation centrally; validate all externally supplied definitions and configuration on the endpoint.
- User enablement never bypasses assignment or tool restrictions. Recheck authorization at dispatch; define cache validity and revocation guarantees explicitly before introducing offline execution.
- Use salted Argon2id password hashing with reviewed parameters, login throttling, and expiring/revocable sessions. Do not persist user passwords locally.
- Store endpoint credentials in macOS Keychain; store non-secret local configuration under a user-scoped application directory with restrictive permissions and identity/deployment separation. Specify credential lifecycle in the auth track.
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
- Version persisted policy/configuration and API contracts; persist local enablement safely and keep each principal's state separate.
- Central migrations, backup/restore, health checks, and local diagnostic commands are delivery concerns with concrete verification before release.

## Documentation and compatibility

- Document self-hosted deployment, macOS CLI setup, login, coding-agent STDIO configuration, catalog/assignment administration, tool-policy semantics, and troubleshooting.
- Document tested macOS releases/architectures, supported coding agents/MCP versions, and remote transport/auth requirements.
- Keep API schemas and Rust/TypeScript contracts consistent through schema generation or contract checks; choose the mechanism in the foundation track.
- Verify compatibility and feature flags against the pinned official Rust MCP SDK rather than assuming every protocol capability is proxied.
- Preserve protocol negotiation and document unsupported features. Expand compatibility only through explicit acceptance criteria.
- Follow the bundled Cadre workflow for approved tracks, verification, review, receipts, and commits.

## Repository-specific commands

No application manifests or executable commands exist yet. Establish these commands during the foundation track, then update the documentation with verified commands:
- Rust: cargo fmt --all -- --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace; cargo build --workspace.
- Frontend (proposed pnpm workspace): pnpm install --frozen-lockfile; pnpm lint; pnpm typecheck; pnpm test; pnpm build, after their scripts exist.
- shadcn CLI: use the project's package runner and inspect current component docs/configuration before additions.
- Integration and macOS packaging verification: track-specific commands once fixtures and packaging exist.
