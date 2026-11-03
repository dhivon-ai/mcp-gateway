# Specification: A simpler product website and two-command setup

- Track: `consumer-website`
- Type: `feature`
- Revision: 1
- Status: proposed

## Goal and scope

Help a first-time visitor understand the value of Dhivon MCP Gateway, see one concrete example, and choose the next step. A developer joining a team and an operator starting a deployment have different jobs. Give each a short path. Make the site inviting through clear benefits, readable design and a relevant product example. The goal is adoption; pricing, checkout, analytics, hosted signup and new distribution platforms are outside scope.

## Functional Requirements

- FR-001: Lead with a concrete benefit: connect your coding agent once and use tools your team allows. Show one example (such as granting access, enabling a tool, making a call and disabling it), with a single prominent Get started action. Technical detail belongs in linked guides.
- FR-002: On Get started, offer “Connect to my team” and “Set up a gateway.” Explain in plain language who provides the server URL and account, when to open the Mac console, and what a successful first connection looks like. Developers joining a team need no server setup. Make compatibility, security, troubleshooting and admin reference discoverable as secondary reading.
- FR-003: Explicitly select what the public build publishes, including copied assets. Keep internal tickets, design/review notes, test transcripts, historic release evidence and Cadre identifiers out of public routes, navigation, metadata and rendered guide text. Preserve source history. Keep necessary public release facts, checksums, compatibility and notices in maintained user-facing guides. Preserve useful public links or provide a relevant redirect; deliberately retired internal URLs return 404.
- FR-004: From the repo root, after the stated Docker Compose, host/DNS, HTTPS and private first-admin inputs are ready, one command, `docker compose up -d --build --wait`, builds and starts the service and persistent PostgreSQL. Describe exactly which deployment inputs users supply and how they provide them privately. Automate fresh DB credentials, vault initialization, migrations and first-admin provisioning. Expose the URL to visit and the expected health result. Include a short HTTPS setup path and identify external prerequisites before promising the command works.
- FR-005: From a fresh checkout on a supported Apple Silicon Mac with the documented build toolchain, one command, `bash scripts/package-macos.sh`, installs locked dependencies, embeds the local UI, builds the native CLI and prints the archive path and checksum location. Provide a safe output default while retaining explicit owned output overrides. Users receiving a trusted built archive can skip building. Show the verified first-run command and agent connection steps.

## Non-Functional Requirements

- NFR-001: Follow approved brand, typography, contrast and license rules. The public experience is static, responsive and usable with only a keyboard. Avoid unsupported claims about signing, platforms, public downloads, hosted service, customer endorsements or commercial results.
- NFR-002: Database remains private. Maintain verified HTTPS, first-admin protections, nonroot service, independent private vault material and durable data. Never expose secrets in instructions, logs, command arguments or tracked files. Interrupted setup and repeated Compose starts must preserve existing accounts, keys and data; refuse inconsistent state with actionable recovery guidance.
- NFR-003: Keep user guides canonical across the website and repository. Keep backup, restore, proxy integration and advanced build options accessible without making them steps for every reader.

## Acceptance Criteria

- AC-001: At desktop and 390 px, from the homepage a reviewer can describe the benefit, find Get started, and reach either audience guide in at most two link activations. One clear primary action, visible focus, no page-level horizontal overflow; core setup is understandable without reading architecture or release history.
- AC-002: A production build/crawl covers public pages, metadata, links, anchors, assets, notices and retired routes. An internal fixture added under docs is excluded by default. The former BUG-005 URL returns 404; public checksums and supported setup guides remain reachable.
- AC-003: An operator following only the short guide on a clean supported host gets a healthy private database, signs in as the first admin via the configured HTTPS origin, and repeats the command without duplicating the account or replacing keys. Verify interrupted startup recovery, restart persistence and a credential read after fresh restore with the retained key. Record the infrastructure prerequisites separately.
- AC-004: On the supported Mac, run the one build command without manually exporting a build directory or prebuilding the UI. Verify checksum, notices, extracted binary version and embedded console. With the tested stack and an assigned tool, demonstrate sign-in, opt-in, first agent call and refusal after disable.
- AC-005: Validate both user paths against actual commands, compare emitted notice files to source, and record desktop/mobile screenshots and results for manual review.

## Dependencies

No pending track dependencies. Existing site, compose stack, packaging script and the deployed ticket exclusion at bf2f816 are the baseline.

## Additional Information

Public site and deployment/build ergonomics only. Keep the protected admin and local applications, authorization behavior and verified platform matrix in place. Build from the checkout; registry publishing, signing, paid plans and DNS/account provisioning are separate decisions.

## Dependent-track impact

No active downstream tracks. Preserve the existing advanced deployment, build-cache overrides and isolated verification entry points.

## Approval

Pending approval of this combined specification and plan.
