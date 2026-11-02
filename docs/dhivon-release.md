# Dhivon release verification

Version 0.1.0 was built and exercised locally on 2026-11-02 as Dhivon MCP Gateway,
a product of Dhivon AI. The initial renamed artifacts are retained below;
the notice-bearing remediation build supersedes them for current distribution.
The [original MVP report](release.md) preserves the earlier artifact identities.

## Artifacts and environment

| Item             | Verified identity                                                         |
| ---------------- | ------------------------------------------------------------------------- |
| Mac archive      | `dhivon-mcp-gateway-0.1.0-macos-arm64.tar.gz`                             |
| Archive SHA-256  | `2467f67ededf51f41e14e4c542d392995ea3be29965bfe37c61538259a41f275`        |
| Service image    | `dhivon-mcp-gateway-server:0.1.0`                                         |
| Image ID         | `sha256:31d889b0612b8e049d454d7c658e379ec5d1504a56f31662587256df21570e5b` |
| Host             | Apple Silicon arm64, macOS 27.0.1 (26A434)                                |
| Toolchains       | Rust 1.95.0, Node 24.19.0, pnpm 11.4.0                                    |
| Service platform | Linux arm64, PostgreSQL 18.6, Docker 29.8.2, Compose 5.5.0                |
| MCP profile      | rmcp 3.5.1, MCP 2025-11-25 tools, local STDIO and Streamable HTTP         |
| Browser          | In-app Chromium at desktop and 390px widths                               |

The packaging script produces the archive and checksum under the owned build
directory's `releases/` folder. Both binaries embed the current management assets.
The final archive includes the current compatibility guide. Its executable is
byte-identical to the extracted binary used by the packaged checks.
The archive is unsigned and locally built. Signing, notarization and public
distribution remain [roadmap candidates](roadmap.md).

## Notice-bearing remediation build — 2026-11-02

Review cycle 1, BUG-007, adds the owner-selected Apache-2.0 source license and
Dhivon AI copyright/ownership, without changing third-party licenses.

| Item | Current identity |
| --- | --- |
| Archive SHA-256 | `95b9aa68baaecbd19ca97c9a3d9b18d56e09d4e6063a29d90771877f5078b2d6` |
| Extracted executable SHA-256 | `b789e155ce8251e9cfc5b13fe8abeb23010782a3abf64b1b83413b4aba960b22` |
| Service image ID | `sha256:7c6b562735589c7dbed0552c26a7d3f3fda20e6eb5fa773589c7364ae7ae0b07` |

The archive and runtime image carry project LICENSE/NOTICE and the unchanged
full Manrope OFL at the [documented locations](licensing.md). The static website
publishes readable text notices and a licensing page linked from every footer.
Checks compare all three emitted notice files against exact source bytes. The
previous archive is rejected by the new check, demonstrating the missing-notice
case. The CI website and deployment entry points run their respective checks.

The final archive checksum and extracted executable equality pass. The extracted
CLI all-authentication/owned-process suite passes in 34.28 seconds. A fresh run
of the notice-bearing image and extracted Mac completes login, install, explicit
enablement, a Node STDIO tool call and disable denial, plus restart, actual dump
and fresh restore, independent wrong/matching vault keys and redacted logs.
Native credentials, generated stacks and volumes are cleaned by their fixtures.
The public licensing page and Apache text link were exercised in in-app Chromium;
1280px and 390px layouts have no page-level horizontal overflow or relevant
console warnings/errors. The guide's table scrolls within its own container.

Rust application behavior and embedded UI source are unchanged by this phase.
The initial full workspace/database/frontend evidence below remains applicable;
the focused packaging, artifact, site, type/format and current packaged/deployment
checks above verify the changed delivery paths. Builds use pinned Rust 1.95.0,
Node 24.19.0 and pnpm 11.4.0. An initial local Node 26 invocation was repeated
with the pinned Node toolchain before final evidence was recorded.

## Verified user journey and recovery

A fresh isolated Compose deployment applied schema initialization, generated
external vault keys and bootstrapped an administrator. The rehearsal exercised
roles, default-deny assignments, per-user enablement, encrypted vault values and
embedded administrator assets.

The extracted Mac CLI logged in through the private fixture control-plane hop,
explicitly installed and enabled an assigned MCP, and served an actual Node
STDIO agent connection. A permitted echo call succeeded. After confirmed
disablement, calling the retained alias was denied. Native Keychain logout
confirmed empty owned credential and cleanup metadata.

Restart preserved configuration. A real PostgreSQL dump restored into a fresh
second deployment using separately retained keys. Copied sessions were
invalidated. An independent wrong key denied credential access, and the matching
key restored access. Captured logs excluded the synthetic password, session and
bearer values. The generated rehearsal stacks and volumes were removed.

The internal fixture hop uses loopback HTTP with the exact public Host.
Production deployment requires the documented verified HTTPS origin. Controlled
OAuth and SDK fixtures define the interoperability evidence. See
[compatibility](compatibility.md) for the supported scope.

## Interface and documentation checks

The administrator and Mac interfaces use the approved Soft Gate SVGs, navy and
emerald palette and locally bundled Manrope. Compiled browser checks exercised
administrator sign-in, catalog editor, keyboard Escape/focus return, local login,
catalog/status navigation, installation feedback and confirmed enable/disable.
Desktop and 390px layouts have no document-level horizontal overflow. Tables
scroll within their own containers. Browser console checks found no relevant
application errors.

A browser check found that inlined SVGs were rejected by the existing content
policy. External bundled SVGs resolved the issue. Emerald on white is used as a
brand accent. Navy and darker semantic colors carry readable text and controls.

The public site builds independently and renders the canonical Markdown guides.
It explains policy ownership and execution locations, walks through
grant → enable → call → disable, and links role-specific setup, compatibility and
roadmap content. Browser navigation, direct page refresh, mobile layout, local
fonts and the generated link/anchor crawl were checked.

## Quality checks

The complete Rust workspace suite passes 83 tests, including native policy,
identity, cache, process, transport and credential behavior. Eleven explicitly
run isolated PostgreSQL cases pass. The frontend suite passes 44 tests.

Locked builds, all-target Clippy with warnings denied, Rust formatting, strict
TypeScript, frontend lint/format, generated-contract comparison and static-site
build/type/format checks pass. The extracted CLI's all-authentication agent
suite also exercises current policy, remote none/bearer/OAuth, offline restart,
protocol-only stdout, cancellation and owned shutdown.

Native checks use stable binaries and assets for each run. An early verification
run overlapped rebuilds and was repeated with stable inputs. Its retained native
fixture was logged out, confirmed empty and removed before final acceptance.

## Identifier audit and acceptance coverage

Product identifiers use Dhivon across Cargo packages/directories, executables,
pnpm workspace names, environment variables, cookies, native state/Keychain,
images, deployment examples and scripts. Fresh installations use the new
namespaces directly.

The audit retains standard third-party environment and protocol names, generic
gateway domain terms such as tool aliases and event categories, and Cadre's
workflow identifiers. Historical receipts, archived tracks, design references,
bug reports and the original release/acceptance records retain their exact
historical identities.

| Acceptance                 | Evidence                                                                              |
| -------------------------- | ------------------------------------------------------------------------------------- |
| AC-001, complete rename    | Locked builds, CLI help/version, fresh namespaces, deployment config and source audit |
| AC-002, branded interfaces | Compiled desktop/mobile and packaged local asset checks                               |
| AC-003, public explanation | Independent static build, architecture/example content and browser guide navigation   |
| AC-004, first tool call    | Container → extracted Mac → Node STDIO success and confirmed disable denial           |
| AC-005, docs and roadmap   | Canonical guide set, restart/fresh restore/key recovery and explicit roadmap status   |
| AC-006, verification       | Rust, database, frontend, contract, browser, archive and container checks above       |

## Follow-up administration

The current delivery keeps the repository at its active Cadre project root.
After track reconciliation, use the approved Cadre refresh workflow to update
current project-level identity and engineering context to the delivered baseline.
The distribution slug is `dhivon-mcp-gateway`. Hosting and a physical checkout
move can then use that identity while preserving historical receipts.

Future signing, broader providers/agents, additional platforms, SAML, hosted
delivery and scale work require their own scoped verification. The roadmap gives
candidate outcomes and evidence criteria without release dates.
