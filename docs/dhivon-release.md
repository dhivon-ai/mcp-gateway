# Dhivon release verification

## Verified Mac archive — 2026-11-03

The locally built Apple Silicon archive `dhivon-mcp-gateway-0.1.0-macos-arm64.tar.gz` from commit `41d8c65` has SHA-256 `e2388fdf4af9371195a2307cdf4e654bb90fd604f62101c739606cdd4900ddc4`. This build uses one Keychain master key for encrypted local state.

In the packaged first-fetch test, a delayed success could not override a newer refusal. The affected tool stayed unavailable after an offline restart until the server authorized a fresh request. The packaged console also recovered from simultaneous marker and index damage through independent sign-in. All 72 CLI tests and the PostgreSQL-backed OAuth regression passed. See [endpoint storage verification](testing.md#single-key-endpoint-storage).

## Initial renamed artifacts and environment — 2026-11-02

The [original MVP report](release.md) preserves the earlier artifact identities.

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

| Item                         | Notice-only intermediate identity                                         |
| ---------------------------- | ------------------------------------------------------------------------- |
| Archive SHA-256              | `95b9aa68baaecbd19ca97c9a3d9b18d56e09d4e6063a29d90771877f5078b2d6`        |
| Extracted executable SHA-256 | `b789e155ce8251e9cfc5b13fe8abeb23010782a3abf64b1b83413b4aba960b22`        |
| Service image ID             | `sha256:7c6b562735589c7dbed0552c26a7d3f3fda20e6eb5fa773589c7364ae7ae0b07` |

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

## HTTPS/native OAuth remediation build — 2026-11-02

The owner selected a Docker HTTPS simulation for review BUG-006. The service image
above terminates TLS through nginx 1.30.6, pinned to
`sha256:0e52bd054126cdc040b1276c0af347aeb02a41b9cfe305d6094767ac30e97949`.
A second TLS listener exposes the controlled MCP/OAuth fixture. Both publish only
loopback ports. Certificate and hostname verification remain enabled.

| Item                         | HTTPS/OAuth build identity                                                          |
| ---------------------------- | ------------------------------------------------------------------------- |
| Mac archive SHA-256          | `e66ae212267e01f7af952db662c6da33edc284e8529e07bc40cb142134092799`        |
| Extracted executable SHA-256 | `f36d5e229cf3fd2bf97ea780340b0304a605f6b848044755754ecee64c83d5d7`        |
| Service image ID, unchanged  | `sha256:7c6b562735589c7dbed0552c26a7d3f3fda20e6eb5fa773589c7364ae7ae0b07` |
| Simulated control plane      | `https://localhost:57518`                                                 |
| Simulated MCP/issuer         | `https://localhost:57519`                                                 |
| Actual native callback       | `http://127.0.0.1:57521/oauth/callback`                                   |

The extracted Mac client signed in and synchronized over HTTPS. The authorization
request used the actual management port, S256 PKCE, state, configured public client
and exact resource. Forged state and issuer callbacks were rejected without
consuming the live flow. Safari followed the HTTPS provider redirect and displayed
**OAuth connected** at the native callback, with no certificate warning bypass.
Consent left the MCP disabled. Management restart preserved the credential;
explicit enablement allowed a real Node STDIO agent's exact echo call.

The expiry test exposed a real resolver defect: a provider rejection of the old
access token could block its republishing before the expired-token refresh branch
ran. The fix validates the online expired record and invokes the existing
version/lease-checked refresh before attempting to publish that old token. It
retains rejection of the old binding, generation/policy checks, offline denial,
protected publication and the prohibition on replaying tool calls.

The final test observed the old token's scoped rejection, central credential
version 1 advancing to version 2, a successful subsequent tool call, disconnect
and denial of the retained alias. Discovery is polled while reconciliation runs;
side-effecting calls are not replayed. All 83 workspace Rust tests pass (12 external
or parent-driven cases remain ignored in that command), CLI all-target Clippy and
format pass, and the final extracted all-auth gateway suite passes in 47.13s.
Script strict TypeScript and formatting pass. The archive notice check still
passes after the production CLI fix. The earlier explicit PostgreSQL/frontend
results remain historical evidence for unchanged server/UI behavior.

The generating scripts confirmed empty native identity/credential/cleanup lists
and removed the exact generated Compose containers, network and volume. The
operator approved temporary SSL trust restricted to localhost for the one-day CA
with SHA-256 `9D69099345C36B4FCBADB35E47A55895D87181BC92CFDC0BE4744B10AAEC5B81`.
After operator authorization of cleanup, trust removal exited successfully, the
certificate was absent from the login keychain, and the leaf failed verification.
No temporary certificate trust remains from this test.

In-app Chromium and Chrome rejected the constrained test CA. Safari's native
macOS trust handling completed the browser step. This is controlled simulation
evidence only: public DNS/hosting certificates, real-user provider consent and
named external-provider registration compatibility remain unverified. The
localhost ports above were temporary and are now closed after fixture cleanup.
Use the [reproducible HTTPS rehearsal](https-oauth-rehearsal.md) for a new run.

## Rehearsal cleanup remediation — 2026-11-02

Review cycle 2, BUG-008, changes the verification harness and recovery runbook.
The Mac executable, archive and service image retain the identities recorded for that build above;
Rust application, embedded UI and license source bytes are unchanged. Exact archive
and image notice comparisons pass again. The earlier HTTPS/native OAuth acceptance
remains evidence for those unchanged artifacts; this cleanup cycle installs no
new certificate trust and makes no new external-provider compatibility claim.

The pinned Node 24.19.0 process-level suite passes all 11 tests, including its
parent test, in 149.68 seconds. It uses the actual Docker service/PostgreSQL and
extracted Mac CLI with real isolated native credential ownership. Its ten scenarios
cover SIGINT/SIGTERM after resource startup and native login, successful completion,
ordinary workflow failure, failed diagnostics, incomplete stack cleanup, incomplete
native logout, and the original HTTPS entry point interrupted during PostgreSQL
startup. Repeated signals and an interrupt-ignoring child exercise bounded teardown.

The original entry point now exits 143 after SIGTERM and writes truthful cleanup
evidence. After successful cleanup, independent assertions find no owned child,
container, network, volume or key-generation container; native identity, credential
accounts and pending cleanup are empty. An unrelated process survives. Diagnostic
failure still permits native and Docker cleanup while recording a failed run.

Injected stack and logout failures retain the creating binary and exact ownership
record. Native failure also retains central resources for recovery. The recovery
command rejects a still-active supervisor and changed Compose inputs. After input
reconciliation, actual logout/teardown and a second recovery invocation pass;
previous failed attempts and the failed verification outcome remain recorded.
Private per-run records and consolidated `cleanup-test-results.json` are retained
under execution `2026-11-02T09-16-20-231Z` in the owned build cache.

Strict script TypeScript, formatting, static-site type/build and generated guide
link checks cover the changed harness/runbook. The focused probes use loopback
HTTP for native cleanup and stop the HTTPS entry point before TLS/native sign-in.
The full HTTPS/browser/OAuth journey was not repeated in this cleanup cycle.
Manual certificate-trust ownership and the earlier public-hosting/provider limits
remain explicit in the [cleanup and recovery instructions](https-oauth-rehearsal.md#interruption-cleanup-and-recovery).

## Recovery ownership follow-ups — 2026-11-02

Review cycle 3, BUG-009 and BUG-010, changes only rehearsal supervision, regression
fixtures and recovery documentation. The Mac executable/archive, service image,
product Rust/UI and license bytes retain the identities above. Prior controlled
HTTPS/native OAuth acceptance and public-provider limits remain in effect. These
cleanup checks install no certificate trust and do not repeat the browser journey.

The combined pinned Node 24.19.0 suite passes 25/25 tests in 163.60 seconds. It
retains the original actual Docker/PostgreSQL/extracted-Mac scenarios and adds
process regressions for identity lookup failure, post-spawn persistence failure,
SIGTERM during registration and a live interrupt-ignoring diagnostic child whose
registration fails during cleanup. Each known child is confirmed exited before
native logout, including children created by failed cleanup commands.

An ambiguous-intent negative fixture correctly records `recovery_required` and
skips native logout/stack removal. That controlled fixture never spawns its
synthetic intent or creates a credential/container; its unresolved journal remains
as failure evidence. Missing ownership no longer counts as confirmed absence.

Simultaneous stale-lock contenders produce one owner and one rejection. Forced
exit with an empty lock or a held lock permits retry on the same inode. Repeated
release by an old owner cannot affect a live successor. Full recovery rejects a
concurrent owner, then reads the latest journal after handover. Malformed lock bytes
are harmless to acquisition, and missing Python refuses authority while leaving
retry possible after the prerequisite is restored. A permanent kernel-held lock
replaces stale-PID-file deletion. Python 3.15.2's standard-library `fcntl` bridges
the descriptor operation on the validated Mac; no Python dependency is added to
the distributed client or service.

Private results, source/artifact fingerprints and logs are retained under owned
execution `2026-11-02T09-53-25-214Z`: `recovery-tests-final.log`,
`cleanup-test-results.json` and `ownership-test-results.json`. Actual Docker/native
resource outcomes are checked independently; unrelated processes survive. Strict
script TypeScript/format, site type/build/link checks and exact archive/image/site
notice comparisons cover the changed source and documentation. See the updated
[recovery runbook](https-oauth-rehearsal.md#interruption-cleanup-and-recovery).

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
