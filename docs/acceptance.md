# MVP acceptance evidence

The acceptance suite combines real PostgreSQL API tests, actual Apple Silicon
Keychain/process/CLI checks, official rmcp clients and controlled remote/OAuth
servers. Browser evidence uses the compiled Rust-served websites. Fixtures use
synthetic credentials and do not contact production providers.

| Criterion | Executable evidence |
| --- | --- |
| AC-001 identity/default deny | Server `identity` and `configuration` integration suites: bootstrap, two users/groups, role denial, no grants or enablement by default. Admin browser CRUD in [admin UI evidence](admin-ui.md). |
| AC-002 additive grants/tool policy | `additive_grants_last_removal_and_policy_edit`, core policy matrix, fixtures `aggregation`: user/group/all grants, last-grant clearing/regrant, manual/observed names, empty allowlist/new tools and zero-upstream direct denial witnesses. |
| AC-003 identity isolation | CLI `identity`, actual Mac `cache` and `gateway` suites, browser Alice/logout/Bob: generation/deployment isolation, native Keychain, known rejection, local Host/Origin/cookie/CSRF. |
| AC-004 confirmed settings/install | CLI `management`: literal structured commands, failed install/server commit, revision conflicts, explicit enable retry without reinstall, concurrent-client enablement, no offline queue. Local browser install/enable/bearer journey. |
| AC-005 one-hour ceiling | Core `policy`, CLI `cache`, standard-client `aggregation`: 3599/3600/3601, failure/partial responses, restart, ciphertext replay, sleep-aware native clock, rollback, in-flight cancellation and recovery. Actual Keychain subprocess restart also runs. |
| AC-006 refresh/current state | CLI `cache` startup/60s scheduling/10s deadline/reconnect tests, actual four-MCP `gateway` same-Mac disable/cold offline/reconnect journey. Full responses alone renew freshness. |
| AC-007 local encryption | CLI `cache`: actual native Keychain and encrypted file inspection, tamper, missing key, copied identity, protected checkpoint/receipt replay, boot/clock uncertainty. Core AEAD tests. |
| AC-008 central secret/bearer | Server `vault`, native `upstream_credentials`, authenticated fixtures and actual CLI: write-only CRUD, ciphertext/owner/version checks, reference deletion, owned stable IDs, replacement/401/exact-destination headers and offline fences. Browser create/rename/replace/bound-delete/disable/delete. |
| AC-009 OAuth | Fixtures `oauth_gateway` plus server `remote_oauth`: discovery, PKCE/state/issuer/resource, central ciphertext, protected call, rotating refresh/concurrency, cancellation, pending polling, complete-response recovery and unrelated token redirect witness. Actual Mac Keychain and browser consent/reload/explicit enable/disconnect. |
| AC-010 one agent entry | Actual CLI `gateway` uses one official rmcp client to call local STDIO and remote none/bearer/OAuth. Native descriptor regressions exercise Node-style Unix stream pairs, idle read cancellation, output backpressure and rejection of non-protocol descriptors. Fixtures `aggregation`, CLI `protocol`: pagination/schema changes/collisions, bounded invalid input, cancellation/deadline/admission, policy revocation, process exit/descendants and protocol-only stdout. |
| AC-011 compiled websites | [Admin evidence](admin-ui.md) and [local evidence](local-ui.md): actual browser principal journeys, real 409/reload, ordinary-role denial, offline/read-only countdown, keyboard focus/Escape/return, desktop/laptop/390px layouts. Frontend behavioral suite includes exact expiry and late old-identity responses. |
| AC-012 release/restore | [Release evidence](release.md): clean container migration/health/restart, pg_dump/fresh pg_restore, session invalidation, wrong/matching external keys, arm64 checksum/extracted embedded UI, actual container-to-Mac-to-Node-STDIO install/enable/call/disable journey and creating-binary Keychain cleanup. |

Run the documented commands in [testing](testing.md). Database tests are explicitly
ignored by ordinary `cargo test`; execute them with the isolated fixture DSN.
The one `native_cache_child` ignored marker is a helper invoked by its real Mac
parent test, not missing coverage. Do not run all ignored CLI tests independently.

Native validation uses macOS 27.0.1 (26A434), Apple Silicon, Rust 1.95.0 and the
in-app Chromium browser. The supported MCP profile is tools-only 2025-11-25 using
rmcp 3.5.1. No named commercial agent, external OAuth provider, Safari/Firefox,
older Mac, actual machine suspend/reboot, hosted CI or signed distribution is
claimed. Controlled continuous-clock/boot simulations prove fail-closed behavior;
the actual Mac helper proves same-boot process restart. Cancellation cannot undo
an upstream side effect and privileged OS compromise is outside the trust model.
