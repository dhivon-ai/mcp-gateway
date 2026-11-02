# Specification: Dhivon MCP Gateway rebrand, product website and documentation

- Track: `dhivon-brand-site-docs`
- Type: `feature`
- Revision: 1
- Status: proposed

## Goal and scope

Introduce Dhivon MCP Gateway as a product of Dhivon AI. A prospective user can decide whether it solves their team's MCP access problem. An operator can set it up from the repository, and a developer can connect an agent and see how access is granted and withdrawn. The public site lives in this repository. Both existing management interfaces adopt the Dhivon identity.

## Functional Requirements

- FR-001: Apply a clean-break rename across product-owned identifiers. The project slug and Mac executable become `dhivon-mcp-gateway`, the service executable/image becomes `dhivon-mcp-gateway-server`, Cargo packages/directories use `dhivon-mcp-gateway-{cli,core,server,fixtures}`, and the fixture binary uses `dhivon-mcp-gateway-fixture`. Use `@dhivon/mcp-gateway-web` for the management frontend, `DHIVON_MCP_GATEWAY_*` for product-defined environment variables and `com.dhivon.mcp-gateway` for native service and Application Support namespaces. Align manifests, imports, lockfiles, help/version, protocol implementation names, product-specific cookie/CSRF identifiers, deploy defaults, scripts, archives, CI, fixtures, agent examples and docs. Audit actual usages and record any exceptions by owner. Standard names such as `DATABASE_URL`, MCP method names, third-party packages and Cadre workflow/tool identifiers retain their defined meaning.
- FR-002: Support new installations in fresh namespaces. Provide no legacy executable aliases, compatibility reads, rename migrations, redirects from an old product site or cleanup of old databases, files or Keychain entries. Normal fresh schema initialization and same-version backup/restore are required. Historical receipts, hashes and test reports retain the identity of the artifact actually tested.
- FR-003: Vendor the approved assets from `/Users/vishal/Projects/dhivon/brand` with Manrope's OFL. Use the Soft Gate mark, lowercase outlined lockup, navy #102C4C, emerald #00AD78 and white. Follow USAGE.txt for mark clearance, minimum size, compact mark and dark variants. Use emerald as an accent and semantic colors for control states. Fresh builds must work with the gateway repository alone.
- FR-004: Restyle the central admin and local Mac experiences through the shared theme, typography, navigation, forms, alerts, tables, dialogs and responsive states. Cover sign-in, identity/group management, assignment/tool policy, local install/enable, bearer/OAuth and status/logout. Keep roles clear, labels accurate and all existing behavior reachable. Serve bundled brand assets correctly from both Rust binaries.
- FR-005: Deliver an independently buildable and previewable public site in this repo. Lead with the concrete problem of configuring each upstream in each agent and managing which users and tools can use it. Show one worked example from admin assignment through explicit developer opt-in, agent discovery/call and denial after access changes. Explain which decisions belong to the control plane and local gateway with a labeled diagram and text equivalent. Cover audience, use cases, current capabilities and limits. Provide working paths to the quickstart, full guides and roadmap. Public pages must work without credentials, a running control plane or a local management service.
- FR-006: Publish a navigable documentation set with one maintained source per guide and working links from the README and site. Include a prerequisite checklist, fresh self-hosted deployment/bootstrap, admin setup/policy, Mac install/login, agent configuration and first tool call. Include local STDIO and remote Streamable HTTP configuration, none/bearer/supported OAuth, security and trust boundaries, lease/offline behavior, operations/backup/restore, troubleshooting by symptom, compatibility and contributing/testing. Provide runnable commands or clearly marked substitutions, expected outcomes and links to the next step. Distinguish local unsigned artifacts and controlled fixtures from publicly available downloads or certified integrations.
- FR-007: Publish an undated roadmap. Separate capabilities verified today, next work with an approved basis, and ideas under consideration. Explain what each would enable and how readers will know it landed. Use the project record to distinguish candidates such as signed distribution, wider client/provider validation, additional endpoint platforms, SAML and SaaS. Include an honest empty planned section when there is no approved follow-on. No speculative release dates or commitments.

## Non-Functional Requirements

- NFR-001: Keep central/local access boundaries, default deny, explicit user enablement, revision checks, credential isolation, lease expiry and protocol-only STDIO intact. Keep secrets out of examples, browser storage, logs and public content. The static site cannot depend on authenticated routes or include private runtime data.
- NFR-002: At desktop and 390px widths all three experiences support keyboard navigation, visible focus, readable labels, semantic headings, legible contrast and no page-level horizontal overflow. Check normal text at 4.5:1 and large text/control boundaries at 3:1 as applicable. Keep status understandable without color. Bundle fonts/icons locally for the protected UIs.
- NFR-003: Use the pinned toolchains, existing shadcn primitives and strict TypeScript where applicable. Production assets, documentation links, public deep links and code samples must work from a clean checkout. Record exact evidence and untested limits for renamed builds.

## Acceptance Criteria

- AC-001: A tracked-source audit accounts for former product identifiers. Both renamed executables report the new identity. Frozen pnpm/Cargo builds, imports, CLI/agent snippets, env handling and packaging agree. No legacy lookup, alias or data migration is needed or provided. New isolated resources can be created and removed without touching pre-existing state.
- AC-002: In a clean build the brand files load locally on the public site, in the container admin UI and in the extracted Mac CLI UI. Verify mark variants, font, document titles, icons, loading/error/empty states, keyboard flows, contrast and desktop/mobile evidence for each surface.
- AC-003: Starting at the public home page, a reader can identify who operates the service, why one gateway helps, who chooses which tools are usable and where calls run. They can follow a concrete example and reach setup, compatibility and roadmap pages directly. Direct links, refresh, internal anchors and documented preview/build commands pass with the services stopped.
- AC-004: From the published guide, use a fresh self-hosted deployment and isolated Mac to bootstrap, grant an MCP, sign in, explicitly enable it, connect one agent STDIO entry, discover/call a fixture and observe denial after disable. Verify local and supported remote/auth examples against their fixtures. Check troubleshooting against at least an unassigned MCP, an expired policy lease and unavailable credentials.
- AC-005: The published docs expose every FR-006 topic with substantive instructions. Verify deploy health, restart, fresh restore with independent keys and the contributor commands. Date release evidence and link it to the exercised archive/checksum/image, browser, platform and protocol. The roadmap clearly separates validated capabilities, approved next work and exploratory items.
- AC-006: Run formatter, linter, type, Rust/TS tests, explicit isolated PostgreSQL tests, generated-contract checks, production site and embedded asset builds, browser checks and the renamed release/deployment rehearsal. Record outcomes and any blockers before acceptance.

## Dependencies

No prerequisite track is open. The reviewed macOS MVP is already part of the main branch and is the implementation baseline. Apply its archived lessons through the five approved project patterns. The supplied Dhivon brand directory is a design input.

## Additional Information

Use repository-owned static site output, with no hosting vendor or public domain assumed. There is no configured Git remote. The canonical repository slug for future distribution is `dhivon-mcp-gateway`. A physical checkout move belongs after Cadre operation reconciliation because active operations use the absolute project root. Use Cadre refresh for current project-level branding metadata, preserving immutable history. Additional platforms, hosted service delivery and signing are roadmap subjects. The scope includes documenting the current product and verifying the rebranded artifacts.

## Dependent-track impact

There are no active dependent tracks. Subsequent work should use the new identifiers and cite the current compatibility evidence.

## Approval

Pending combined specification and plan approval.
