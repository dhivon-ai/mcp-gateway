# Product

## Project context

- Classification: `greenfield`
- Evidence: On 2026-10-14, /Users/vishal/Projects/cadre-mcp-gateway was empty: no manifests, source, tests, repository guidance, Cadre state, or Git history. git rev-parse found no enclosing repository. Setup requires Git initialization at this exact root.

## Name and summary

MCP Gateway (repository: cadre-mcp-gateway) is an enterprise-managed gateway for coding agents. Administrators publish an MCP catalog and control who can access each server and which tools it exposes. Users authenticate and opt in through a React + TypeScript + shadcn/ui interface served locally by a Rust CLI. Coding agents connect to one local STDIO MCP server, which aggregates the user's permitted, enabled local and remote MCP servers. Deliver self-hosting and macOS first; support a hosted SaaS offering and SAML in later work.

## Users and problems

- Enterprise administrators need a central admin website to manage users, user groups, MCP definitions, assignments, and tool policies.
- Developers need a simple way to log in, view their assigned MCP catalog, run a defined installation command, and enable or disable an MCP on their Mac.
- Coding agents need a single STDIO entry point that exposes the developer's effective tools without individual upstream MCP configuration.

## Outcomes and success measures

- A new deployment grants no MCP access by default. Adding an MCP to the catalog alone does not expose it to users or agents.
- An administrator can assign an MCP to an individual user, a group, or all users within the enterprise.
- An authenticated user sees the assigned catalog, explicitly installs when an installation command exists, and enables an MCP. A failed installation is reported and does not enable it.
- A coding agent configured with only the gateway can discover and invoke permitted tools from both a local STDIO server and a remote server.
- Both tool discovery and invocation enforce the MCP assignment, the user's enablement, and the administrator's tool policy. A direct invocation of a hidden or disallowed tool is rejected.
- macOS CLI login, local browser UI, installation, upstream process lifecycle, and STDIO integration are verified through an end-to-end journey against a self-hosted deployment.

## Scope boundaries

Initial delivery:
- Self-hosted central Rust service and React + TypeScript + shadcn/ui admin website, initially serving one enterprise per deployment.
- Rust CLI for macOS that starts a loopback web server for login and MCP management; the CLI also offers a separate agent-facing STDIO gateway mode.
- Username/password authentication and identity-bound configuration loading. Admin and ordinary user permissions are enforced at API boundaries.
- Administrator-defined local STDIO MCP commands, remote MCP endpoints, and optional installation commands executed on the user's Mac after an explicit action.
- Assignments to users, groups, or all enterprise users; user enable/disable controls; MCP-level tool allowlist or denylist policies.
- Proposed policy baseline for this setup: assignments are additive grants across user, group, and all-users targets. Absence of a grant denies access. User enablement cannot widen an admin grant. An allowlist exposes only selected tools, including denying newly discovered tools until selected; a denylist excludes selected tools and permits other discovered tools for an otherwise eligible MCP.
- Proposed initial remote transport: Streamable HTTP. The gateway's agent-facing transport remains STDIO. Compatibility requirements will be specified before implementation.
- Keep control-plane configuration separate from endpoint execution and local credentials so later SaaS and SAML work can extend the design.

Later scope:
- Hosted multi-enterprise SaaS, its tenant isolation, provisioning, billing, and operational design.
- SAML federation and related enterprise identity-provider configuration.
- Windows and Linux endpoint support, further authentication methods, and legacy remote MCP transport adapters.

Not committed by setup:
- A complete proxy for every MCP capability beyond tool discovery and invocation; resources, prompts, subscriptions, and other surfaces need an explicit feature specification.
- Enterprise-wide performance targets, compliance certifications, high availability, or arbitrary endpoint software sandboxing.

## Domain language

- Control plane: the central service and admin website that own identity, catalog definitions, and administrator policy.
- Local gateway: the Rust process used by the coding agent as its one STDIO MCP server and by users for local management.
- Upstream MCP: a local child process or remote MCP endpoint aggregated by the gateway.
- Assignment: an administrator's access grant to a user, group, or all enterprise users; it does not automatically install or enable the MCP.
- Enablement: the user's local opt-in to an assigned MCP.
- Tool policy: an administrator-selected allowlist or denylist for an upstream MCP's tool identifiers.
- Effective tools: discovered upstream tools that satisfy assignment, local enablement, and tool policy.
- Installation command: an optional administrator-defined command that runs on the user's Mac; it is distinct from the upstream's runtime command.

## Open questions

Resolve these in the relevant feature track before implementing dependent behavior:
- Minimum supported macOS version, Apple Silicon and Intel release coverage, distribution channel, signing, and notarization.
- Policy refresh/revocation timing, offline behavior, cached configuration validity, and handling of already-running calls when access is revoked.
- Remote MCP authentication and secret ownership: per-user credentials, administrator-managed credentials, and which auth schemes must ship initially.
- Exact CLI commands, server discovery/configuration, login session lifetime, logout/re-login behavior, and headless use.
- Tool-name collision mapping, upstream schema changes, supported MCP versions and coding agents, and treatment of non-tool capabilities.
- Administrator bootstrap, account recovery, audit retention, and initial expected deployment size.
