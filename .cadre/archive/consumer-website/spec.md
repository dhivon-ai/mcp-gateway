# Specification: Simpler product guides and two-command setup

- Track: `consumer-website`
- Type: `feature`
- Revision: 2
- Status: planned

## Goal and scope

Simplify the public pages reached through the existing homepage, especially Documentation, Get started, deployment, Mac setup and agent configuration. Help someone joining a team or setting up a gateway find the relevant instructions, follow them and recognize success. The existing homepage copy, layout, navigation, styling and destinations are outside the editing scope.

## Functional Requirements

- FR-001: Organize detail pages around reader tasks. Open each setup guide with what it helps the reader do and what they need. Put the shortest working steps and expected result first, with explanations of unfamiliar terms at first use. Place optional configuration, troubleshooting and technical background in clearly labeled secondary sections or reference pages. Keep the first tool call as a concrete worked example.
- FR-002: On Get started, offer “Connect to my team” and “Set up a gateway.” Explain in plain language who provides the server URL and account, when to open the Mac console, and what a successful first connection looks like. Developers joining a team need no server setup. Make compatibility, security, troubleshooting and admin reference discoverable as secondary reading.
- FR-003: Explicitly select what the public build publishes, including copied assets. Keep internal tickets, design/review notes, test transcripts, historic release evidence and Cadre identifiers out of public routes, navigation, metadata and rendered guide text. Preserve source history. Keep necessary public release facts, checksums, compatibility and notices in maintained user-facing guides. Preserve useful public links or provide a relevant redirect; deliberately retired internal URLs return 404.
- FR-004: From the repo root, after the stated Docker Compose, host/DNS, HTTPS and private first-admin inputs are ready, one command, `docker compose up -d --build --wait`, builds and starts the service and persistent PostgreSQL. Describe exactly which deployment inputs users supply and how they provide them privately. Automate fresh DB credentials, vault initialization, migrations and first-admin provisioning. Expose the URL to visit and the expected health result. Include a short HTTPS setup path and identify external prerequisites before promising the command works.
- FR-005: From a fresh checkout on a supported Apple Silicon Mac with the documented build toolchain, one command, `bash scripts/package-macos.sh`, installs locked dependencies, embeds the local UI, builds the native CLI and prints the archive path and checksum location. Provide a safe output default while retaining explicit owned output overrides. Users receiving a trusted built archive can skip building. Show the verified first-run command and agent connection steps.

## Non-Functional Requirements

- NFR-001: Follow approved brand, typography, contrast and license rules. The public experience is static, responsive and usable with only a keyboard. Avoid unsupported claims about signing, platforms, public downloads, hosted service, customer endorsements or commercial results.
- NFR-002: Database remains private. Maintain verified HTTPS, first-admin protections, nonroot service, independent private vault material and durable data. Never expose secrets in instructions, logs, command arguments or tracked files. Interrupted setup and repeated Compose starts must preserve existing accounts, keys and data; refuse inconsistent state with actionable recovery guidance.
- NFR-003: Keep user guides canonical across the website and repository. Keep backup, restore, proxy integration and advanced build options accessible without making them steps for every reader.

## Acceptance Criteria

- AC-001: Starting from the existing Get started or Documentation link, a reader can find either audience guide within two further link activations, identify the prerequisites, follow the short instructions and recognize success without reading architecture or release history. Verify at desktop and 390 px, with visible keyboard focus and no page-level horizontal overflow. Compare the homepage with the approved baseline to confirm its visible content, layout, navigation and link destinations are preserved.
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

Proposed in revision 2; see revisions/revision-2.md.
