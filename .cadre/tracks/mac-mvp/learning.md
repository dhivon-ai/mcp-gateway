# Incremental Learning: macOS enterprise MCP Gateway MVP

Never overwrite an earlier phase. Each phase begins by reading the learning of every declared dependency phase. A root phase reads the marked Pattern Seed section below. Parallel sibling phases do not assume or consume each other's learning.

<!-- cadre:pattern-seed:start -->
## Pattern Seed

<!-- cadre:memory:start -->
{"schemaVersion":1,"specRevision":1,"planRevision":3,"patterns":[]}
<!-- cadre:memory:end -->

- Spec revision: 1
- Plan revision: 3

| Pattern | Relevance to this track | Constraints to carry forward |
| --- | --- | --- |

No existing pattern applies: `.cadre/patterns/index.md` is an empty approved catalog and contains no pattern records. No active/archived delivery tracks exist. The empty memory array records that inspection, not missing research. Carry the approved product/guidelines/styleguides and this track's specification into execution context; do not invent reusable learning before implementation.

- Review-cycle 1 reassessment: the approved pattern catalog remains empty. Append-only remediation P10–P13 preserves prior phase history and the unchanged specification. Carry the full P3/P5 identity/credential invariants and P6/P7 browser contracts into the new phases; findings are confirmed defects within those requirements. No dependency track or reusable pattern was introduced.

- Review-cycle 2 reassessment: same empty approved catalog and unchanged specification. Preserve all P1–P13 history. P14 carries forward backend/frontend contract validation and P11 revision/command fixes; accept server-valid unbounded admin collections without weakening explicit bounded contracts. No dependency/pattern introduced.

<!-- cadre:pattern-seed:end -->

## Phase 1: Workspace and policy contracts

### Inputs read

Approved workflow/product/guidelines/tech-stack/styleguides, full mac-mvp spec/plan, empty Pattern Seed, and current workspace/SDK source. No pre-existing product code or dependency track was present.

### Task observations

- T1.1: `926e9a41d6d5d01dec79dcef869e91709d054136` established Rust binaries/core and the strict React/TS/Vite/shadcn workspace; help, healthz, clean shutdown, and unauthenticated empty-stdout rejection passed.
- T1.2 and T1.3: `2be53f911eb3f3d89db4278541d3eff36736ff08` added shared wire contracts, public TS generation, default-deny/additive policy, bounded identity/session-aware cache leases, and eight core tests. Fresh generated TS matched committed bindings and compiled.
- T1.4: `187754250b4ff432715796ae512f4121fb2b24db` added real rmcp STDIO/Streamable HTTP fixtures, pagination/echo/failure tests, a controlled PKCE/rotating-refresh OAuth fixture, a healthy PostgreSQL 18.6 arm64 fixture, CI entry points, and a decoder/CSRF/same-origin frontend API boundary with four tests.
- Phase checks passed: all workspace Rust tests (11 behavioral tests), Clippy with warnings denied, rustfmt, generated-contract comparison, frontend tests (4), lint/typecheck/build and format checks. Hosted CI is configured but has not been executed.

### Decisions and corrections

- Rust 1.95.0, Node 24.19.0 LTS, pnpm 11.4.0; TS 6.0.3 matches the ESLint parser peer range. Use the supplied Node 24 runtime locally rather than the host's Node 26.
- Shared contracts in `crates/gateway-core` use camelCase fields and explicit tagged variants. Tagged unit variants accepted unwanted fields despite Serde deny_unknown_fields; empty struct variants now reject them, with a regression test. Only codegen's unsupported-attribute diagnostic is suppressed; Serde validation remains enabled/tested.
- Cache validity is positive, <=3,600 seconds, bounded by session expiry, and exclusive at expiresAt. Reading/credential refresh cannot renew it. Endpoint native monotonic/sleep/restart enforcement remains Phase 3 work.
- Credential DTOs deliberately have no Debug implementation and are excluded from frontend exports. Secret writes expose inputs only; ordinary read APIs must use metadata.
- rmcp 3.5.1 uses ServerConfig, ContentBlock and HTTP config builders; reqwest 0.13 needs explicit query/form features. Fixtures advertise only the tested 2025-11-25 profile.
- PostgreSQL Compose project `cadre-gateway-tests`, loopback port 18432, database/user gateway_test and documented fixture-only password are owned test infrastructure. Docker/OrbStack is running. The image is pinned by version and digest.
- All build output is in the runtime-owned cache; fixture dependencies are centrally prepared. Actual admin/local product UIs and persisted central auth/vault/runtime features remain later nodes.

### Reusable pattern candidates

Typed policy predicates shared by discovery/dispatch, explicit unit-variant boundary tests, and one-operation lease validation are candidates only; no approved reusable pattern is claimed.

### Phase completion

- Commit: 187754250b4ff432715796ae512f4121fb2b24db
- Human verification: not performed or claimed. Actual automated evidence was recorded under persisted policy-v2 Autonomous authorization at T1.5.

## Phase 2: Central identity, catalog, enablement, and encrypted secrets

### Inputs read

Approved project instructions, full mac-mvp specification/plan, declared P1 learning, current core/server/frontend contracts, SQLx/rmcp/RustCrypto source, and Postgres best-practices guidance. No repository AGENTS instructions apply.

### Task observations

- T2.1/T2.2: `37911cbc3912e2baca39159c5406235d41be9842` adds explicit bootstrap, Argon2id, revocable opaque 12-hour sessions, roles, account/password revocation, group membership and browser/native auth boundaries.
- T2.3/T2.4: `8c407380eb3bc0c2fcac6b1310067b5812fea154` adds catalog/grants, deployment revisions, server-owned user opt-ins, complete bounded snapshots, last-grant removal/regrant behavior, and non-authoritative tool inventory. Concurrent writes and revoke/enable races cannot silently overwrite state.
- T2.5/T2.6: `ce051f249b8110858813339ced107dcb86fb1dec` adds reusable AEAD, external private key-ring files, write-only user-secret CRUD, owner-constrained binding/resolution, encrypted installation-scoped OAuth records, version/refresh leases, and operator key rotation. It also closes identity/group revision checks and bounds catalog definitions to fit endpoint caches.
- Real PostgreSQL 18.6 tests passed: seven comprehensive identity/configuration/vault/operator tests, each using a unique owned database. Live operator smoke verified private key generation, bootstrap, native HTTP login/config/health and SIGINT shutdown. Tests inspect ciphertext and redacted logs; missing/wrong keys and owner/version substitution fail closed.
- Workspace Rust tests, Clippy with warnings denied, formatting/build and generated contract comparison passed; frontend lint/typecheck/four tests/format/build passed. Hosted CI has not run. Both actual product websites and Mac gateway execution remain later phases.

### Decisions and corrections

- Browser cookies are HttpOnly/SameSite=Strict and Secure on HTTPS; mutation CSRF is required. Native login returns a private endpoint token; credential routes reject browser cookies. Exact public Host/Origin checks and no-store responses protect HTTP boundaries. Only token hashes live in sessions.
- Revisions are deployment-wide and may conflict on unrelated changes. All admin identity/group/catalog/grant writes and user settings are checked transactionally. User settings conflicts include the current complete snapshot; no blind retry. Secret/OAuth values additionally use record-version CAS.
- A snapshot lease is at most 3,600 seconds and bounded by absolute session expiry; unchanged complete authenticated refresh can renew it. Endpoint code must validate and durably install before doing so; credential resolution cannot renew it.
- Group/last-grant/catalog/account changes reconcile opt-ins under the authority lock. Changing transport/auth deletes old bindings/OAuth records to prevent credential forwarding to a new destination. Regranting requires a new user enable action.
- External key-ring JSON supports retained old keys and one active key. Format1 AES-256-GCM AAD includes kind, owner, ID, version and key ID; nonce comes from fallible OS randomness. Browser secret reads are metadata only; native DTOs have no Debug/frontend exports. Rotation preserves values/versions and requires old backup keys to be retained separately.
- OAuth identity is `(user, MCP, installation)`, with a 60-second exclusive refresh lease, version CAS and atomic encrypted replacement. Callback/discovery/PKCE/upstream refresh behavior is still Phase5 endpoint work; server storage tests do not claim real-provider compatibility.
- SQLx0.9 permits dynamic SQL only through audited wrappers; only fixture database identifiers use fixed-prefix/UUID generated SQL. All production data uses bound parameters. No hashing or external network runs under revision locks.
- Two databases left by a deliberately failing test revision were removed by exact owned names. All later test runs cleaned their databases and temporary key files normally; the owned base Compose fixture remains available.

### Reusable pattern candidates

Ownership-bound AEAD with stable-ID/version CAS, browser/native response separation, and transactional opt-in tombstones are candidates only, not approved patterns.

### Phase completion

- Commit: ce051f249b8110858813339ced107dcb86fb1dec
- Human verification: not performed or claimed; phase evidence is authorized by persisted policy-v2 Autonomous mode.

## Phase 3: Mac CLI identity, encrypted cache, and online configuration sync

### Inputs read

Approved project instructions/full spec/plan, declared P2 learning, current native/core/server contracts and tests, safe platform APIs and Apple kernel/Libc sources. No applicable AGENTS instructions. Main reviewed every group before committing and integrating; workers edited product files only.

### Task observations

- T3.1/T3.2: `723ea7a600bf6797a91bc6d258bf58462ca3d71c` adds scoped native sessions/Keychain, private state, ui/mcp/status/logout structure, protected loopback cookies/Host/Origin/CSRF, identity isolation and known-session rejection.
- T3.3/T3.4: `cd2d02ab4881ccbcb9a0981af9e083aaf504bbbc` adds complete atomic encrypted snapshots, immutable Keychain keys/protected checkpoints, shared-process latest-cache reads, sleep-aware native deadlines and success-only startup/60-second/10-second sync.
- T3.5/T3.6: `3c0de2000a830cf788851658164d366b2585cb10` adds online generation-pinned settings/vault APIs, redacted public local DTOs, explicit bounded installation/receipts/retry-enable and safe owned process groups.
- Final CLI/core quality run passed 50 tests with one intentional subprocess-helper marker; the actual Mac cache parent separately invokes that helper. CLI all-target Clippy with warnings denied, formatting/build/generated-contract equality/diff checks passed. Actual macOS27.0.1 arm64 Keychain/cache/process-restart and CLI SIGTERM/Ctrl-C with TERM-ignoring owned descendants passed.
- Deterministic tests cover exclusive 3,600-second expiry, restart/rollback/boot/sleep uncertainty, failed/partial/stale publication and sync, offline no-queue writes, conflicts, stale-principal races, installed-disabled recovery, explicit rerun and post-success cancellation/expiry/revocation/300-second deadline with zero automatic enable attempts. Actual machine sleep/reboot, older macOS, Linux endpoint, browser local UI and agent aggregation are not claimed.

### Decisions and corrections

- Cache format MCPCACH1 protects full identity/installation/generation/snapshot/receipt/time anchors with AES-GCM; maximum file size is 2 MiB plus framing. Keychain immutable AES keys and fresh checkpoint accounts avoid observed cross-process item-update failure. Protected publication/retirement barriers fail closed after partial persistence; old ciphertext/pointers cannot renew authorization.
- Darwin CLOCK_MONOTONIC_RAW maps to continuous sleep-aware time; kern.bootsessionuuid and protected last-seen watermarks make same-boot restart conservative. Exact equality expires. Charge network/persistence time to the lease and recheck after final Keychain publication. Credential resolution cannot renew it.
- Every new dispatch must read SnapshotCache.latest, including same-Mac writes. P4 must independently cancel running work/close upstreams on deadline or learned revocation; a status read is not dispatch authority. Startup/every60-second complete refresh has a10-second request deadline; bounded failure retries/reconnect do not extend the old lease.
- Same-server selection preserves valid identity/cache; selecting another origin clears the old scope. Older UI processes report server_mismatch and cannot poll/write/logout the newer selection. Browser actions carry the displayed generation, validated against both loaded session and snapshot; native requests keep that original principal through login races.
- Settings are acknowledged only after complete durable shared publication; conflicts carry sanitized current metadata for explicit reconsideration. Vault values remain write-only; confirmed writes distinguish failed follow-up synchronization. Offline mutations/install/OAuth connection are never queued. Browser logout atomically clears only its expected generation and cancels only its jobs.
- Installation uses structured commands, explicit environment/cwd, private output, a300-second native limit, owner-only cross-process lock and scoped bounded atomic receipts. Successful local work records installed-disabled before optional enablement; cancellation and post-cleanup authorization/deadline checks retain the original lease ceiling until enable dispatch. Explicit enable retry never reruns an installer. Requests already sent may have applied centrally; recovery uses fresh confirmed state.
- Safe rustix waitid NOWAIT retains the leader PID through group TERM/KILL/reap and Drop cleanup under one maximum five-second deadline. nix waitid is unavailable on Mac. Darwin zombie-only groups return EPERM; safe libproc verifies exact reserved-group exited members before accepting that case. Live/query-error cases fail. Deliberately escaped/inaccessible descendants require external supervision; abrupt endpoint SIGKILL cannot run graceful handlers.
- Native fixture roots are existing private directories with separate real-Keychain services; creating-CLI logout avoids different-binary access-control failures and journals survive failed cleanup. Three orphaned entries from the failed approach were removed only after exact creator ACL/namespace/interval proof and metadata-only absence verification; production selection was preserved.
- Main preserved exact prior P1/P2/P6 learning with a named path-scoped stash during canonical integration, restored its SHA unchanged, then removed only that owned stash. No product/learning was discarded.

### Reusable pattern candidates

Success-only identity-bound leases, immutable protected cache checkpoints, generation-pinned online actions, installed-disabled recovery and unreaped-leader process ownership are candidates only, not approved reusable patterns.

### Phase completion

- Product head: 3c0de2000a830cf788851658164d366b2585cb10; canonical integration: e23a8e25675117892d381cb4fdc5334875330e13. Cadre removed the task and phase worktrees/branches after verified integration.
- Human verification: not performed or claimed; actual automated T3.7 evidence is authorized by persisted policy-v2 Autonomous.

## Phase 4: STDIO gateway and upstream tool aggregation

### Inputs read

Full approved project/spec/plan and declared P3 learning, actual core/native/server contracts and tests, pinned rmcp/Tokio transport and cancellation source and primary MCP protocol documentation. No applicable AGENTS instructions. Main executed both cohesive groups and reviewed complete code/diffs; no workers were needed for this single ready phase.

### Task observations

- T4.1/T4.2: `38a858e23fde380bf37a7658367715e96cd333e2` adds bounded official-SDK STDIO/Streamable HTTP adapters, no-auth native resolver seam, stable namespaced routing, paginated schema/result preservation, shared current-policy gates and bounded original-name inventory.
- T4.3/T4.4: `fa504880b4d89cd26df3661c6ddab9bf5c981a2d` delivers the actual `mcp` STDIO command, original-generation refresh/retirement, tool-list notifications and downstream paging, native call deadlines/cancellation and16-call admission, safe nonblocking Mac protocol pipes and owned abort-on-drop discovery.
- Relevant CLI/core/fixtures full suite passed67 tests with one intentional cache-child marker; its parent exercises the actual Mac helper. Affected standard-client runtime and actual CLI tests passed again after final task-ownership correction. Final formatting/all-target Clippy-Dwarnings/build/generated-contract equality/diff checks passed. No hosted CI or other endpoint platform test is claimed.
- Actual macOS27.0.1 arm64 creating-CLI isolated Keychain login plus one standard rmcp2025 client invoked local protocol and real SDK HTTP tools. Confirmed same-Mac disable, valid-cache cold offline restart, immediate reconnect refresh, protocol-only stdout, SIGTERM/SIGINT, idle input, blocked output and interrupted startup with TERM-ignoring descendants passed; exact owned PIDs were absent and creating-CLI logout cleared journals/credentials.
- Controlled fixtures prove duplicate upstream names/stable aliases, schema/list changes and typed structured/tool-error results; direct-call/empty-allowlist witnesses; malformed/looping/duplicate catalogs isolated from healthy tools; task-required omission; frame/JSON/fragmented-SSE bounds; fixed logging exclusion, exact destinations, no redirect credential forwarding and no HTTP404 call replay. Standard-client tests prove caller cancellation, native60-second deadline,16-admission/no-queue, policy3600 equality expiry, tool-list changes and cursor invalidation.

### Decisions and corrections

- rmcp3.5.1 supports the tested2025-11-25 profile. Only tools/list, ordinary tools/call and tool-list changes are exposed; clients advertise no optional callbacks. The SDK drops execution.taskSupport: raw wire filtering omits task-required tools and rejects agent task params before ordinary execution. Unknown extensions are not promised to survive typed SDK decoding.
- `gw_<32hex MCP-ID hash>_<64hex original-name hash>` aliases are stable100-byte ASCII; ambiguous registry entries are rejected. Policies use original names. Every new list/call consumes latest committed cache and matching original generation/runtime/secret definition; choosing a route is not authority for a later dispatch/completion.
- Bounds: 2MiB whole RPC/JSON/retained SSE,256KiB tool,2048 tools/2MiB per MCP,32 discovery pages/distinct opaque1024-byte cursors,32 upstreams/4 concurrent starts,2048 aggregate tools/16MiB and1MiB downstream pages. Smaller SDK SSE limits are honored. Empty opaque cursors are allowed; duplicates/loops reject.
- Custom exact-URI HTTP client rejects redirects/untrusted headers and uses only native upstream credentials. Explicit reinit_on_expired_session=false prevents SDK default POST replay. Network connection/control headers bound10seconds; tool response headers may take up to60seconds under the independent native call bound. Central sessions never go upstream.
- Same-generation startup refresh then run_scheduled avoids a second startup fetch. Reconciliation is separate from250ms current-policy retirement and retains existing healthy routes while discovery runs. First listing waits up to30seconds; remaining bounded waves may announce later via list changes. Native50ms in-flight fences and awake/native60-second bounds reject results after learned denial/deadline. Cancellation is best effort and cannot undo completed side effects; no call retry.
- Owned process leaders stay reserved while the SDK uses detached pipes. Default Tokio stdin has an uncancellable blocking thread that made signal exit hang; safe Unix-pipe adapters own duplicated checked nonblocking descriptors without product unsafe code. Open-input/blocked-output exit passed. Default JoinHandle drop would detach discovery; abort-on-drop owns every coordinator job and pending-startup cleanup passed. Agent protocol uses pipes, not terminal login/file playback.
- Application-only INFO/WARN/ERROR telemetry excludes SDK/network/debug payload events independent of RUST_LOG. Tool events contain fixed outcomes, correlation/duration and validated identifiers; no arguments/results/credential values. Inventory contains only MCP ID/original names, never renews policy or grants access.
- One new harness initially used the wrong settings path; corrected. Its exact fixture Keychain entries were cleared through the creating CLI, and one orphan was removed only after owned PID/group/command witness and absence verification. The signal failure's second fixture likewise cleared before removal. Failure guards now request graceful CLI cleanup and retain uncleared journals; no production selection changed.
- Main preserved root learning through one exact path-scoped named stash during fast-forward integration, restored identical SHA110815f8, then removed only that owned stash. P1/P2/P3/P6 sections remain exact.
- P5 must add per-dispatch native credential-version/readiness checks (OAuth version is not in SnapshotSecretRef), scoped 401 handling, and token expiry bounded by sleep-aware elapsed time as well as absolute expiry. Configuration refresh cannot extend token lifetime. P8 should check installed-disabled progress wording against another client's concurrent central enablement; receipts never establish authoritative enablement.

### Reusable pattern candidates

Bounded raw protocol seams around a typed SDK, no-replay session handling, stable strict route registries, current-policy completion fences, nonblocking owned native pipes and abort-on-drop discovery are candidates only, not approved patterns.

### Phase completion

- Product/canonical head: fa504880b4d89cd26df3661c6ddab9bf5c981a2d, fast-forward integrated by Cadre; phase worktree/branch removed.
- Human verification: not performed or claimed. Actual automated T4.5 evidence authorized by persisted policy-v2 Autonomous. Bearer/OAuth remain P5; local React UI remains P7; full release/acceptance and named commercial-agent/older-Mac/provider compatibility are not claimed here.

## Phase 5: Bearer selection and OAuth remote connections

### Inputs read

Full approved project/spec/plan and declared P4 learning, actual native/core/server contracts and tests, pinned rmcp3.5.1/oauth2.5.0.0 source, and primary MCP2025-11-25 authorization specification. No applicable AGENTS instructions. Main executed/reviewed both cohesive groups; no worker was needed for the single ready phase.

### Task observations

- T5.1/T5.2: `d4fb795a60fbb6f93a64575d88861f93ba8c0643` adds scope/version-bound native bearer resolution, immutable Keychain heads, receipt/read-watermark distinction, per-list/dispatch readiness, persistent scoped remote rejection and offline native credential fences.
- T5.3/T5.4/T5.5: `64a57dbd489202da80507c361152decd9bb1cae1` adds protected local consent/callback/cancel/status/disconnect routes, preregistered SDK public-client discovery/PKCE/resource/issuer validation, encrypted central CAS/refresh coordination, complete Keychain pending-save recovery, native disconnect fences, readiness TS contracts and provider runbook.
- Relevant CLI/core/fixtures full suite passed76 tests plus one intentional cache-child marker (its parent exercises the actual Mac helper). Final affected native credentials (6) and four-MCP CLI (42.20s) passed. Eight real PostgreSQL tests plus external-key permissions passed; final affected OAuth journey with actual isolated Mac Keychain passed53.73s. Workspace formatting/Clippy-Dwarnings/build/contracts/diff and frontend10tests/lint/typecheck/format/build passed. No hostedCI is claimed.
- Actual macOS27.0.1 arm64 CLI plus standard rmcp2025 client used one STDIO entry to call local STDIO and none/bearer/OAuth remote MCPs, then same-Mac disable, cold cached offline restart, reconnect policy, protocol-only stdout and owned process/signal cleanup. Local files contained no access/refresh token; creating-CLI logout cleared its accounts.
- SDK fixtures prove advertised/well-known protected-resource discovery, PKCE/client/resource/callback binding, duplicate/wrong/missing required issuer/state,300s equality expiry, one-use callback/refresh rotation, unsupported metadata and token-redirect zero-destination witness. Real control-plane/native fixtures prove central encryption, consent without automatic enablement, explicit enable/protected call, concurrent refresh single winner, failed-save/unknown-commit/restarted-controller recovery without another provider exchange, pre-exchange/in-flight cancellation, disconnect before a record exists, pending-save removal and same-generation version1 reconnect.

### Decisions and corrections

- Native token records are Keychain-only, bound to deployment/user/installation/login generation/MCP/runtime/secret version and protected snapshot receipt. Reading rollback watermarks never changes policy receipt or token time anchor. Any new configuration revision requires credential revalidation; stable credential signatures permit connection reuse after confirmation. New key IDs may restart their version; late old-binding rejection cannot poison them.
- Partial publication stores the complete new head before deleting the old one and retains protected recovery anchors. An identical OAuth credential or failed-publication recovery cannot renew its sleep-aware lifetime. Native rejection sentinels are immutable/scoped; a remote401 cannot restore the same credential or silently switch auth mode.
- OAuth uses preregistered public clients only. Exact protected resource, validated issuer and published token endpoint, verified TLS, no proxies/redirects/ambient auth,10s operations/aggregate discovery,1MiB metadata/token responses and64KiB outbound bodies. Legacy endpoint synthesis/client secrets/non-expiring token profiles are rejected. A fixed callback port is an operator registration choice; STDIO never opens consent.
- Consent retains cancellation while its exchange is in progress; claim guards prevent replay. At most16 flows/one per MCP/generation, with exclusive native300s expiry. Cancel/logout/reconnect/disconnect cancel matching pending work. Provider issuance or an already-sent central request cannot be undone; confirmed state must be reloaded. No partial result replaces a working central credential.
- Refresh uses complete online authorization and a central60s exclusive lease/version CAS. Complete issued responses are journaled in Keychain before save; recovery confirms the current central version or retries the same complete CAS, preserving original time anchors and never repeating provider/tool calls. OAuth consent's private credential read allows assigned active OAuth before enablement, never generic bearer secrets or agent execution.
- Disconnect retires the native head and pending saves before its central request, protects the old receipt with an immutable scoped marker, and advances central revision even when no token record exists. Otherwise a late initial consent could resurrect it or an old native version could reject a new version1 record. Compact full-hash/receipt marker labels respect the existing100-byte kind bound; obsolete markers are pruned only after a current protected publication.
- A frozen test wall clock conflicted with advancing server timestamps; the portable fixture uses live wall/elapsed anchors. The original wrong echo argument was corrected to the fixture's message field. Four native authenticated MCPs exceeded a5s raw test listing deadline;35s accommodates the product's actual30s initial-list bound without changing product limits.
- Failed fixture databases were removed only by exact owned UUID/name/user/record witnesses. One failed CLI fixture retained its journal; creating binary logout cleared accounts, exact PIDs76138/76139 were absent, then the exact directory was removed. OAuth fixture cleanup now retains its journal on Keychain failure. No production scope was touched.
- Main preserved root learning through the exact named path-scoped P5 stash9e608efd, restored identical SHA9c699122, then dropped only that owned stash. P1/P2/P3/P4/P6 sections remain exact. P7 consumes current generated local DTOs, central-vs-native auth/enablement distinctions, protected API boundaries and online-only behavior. P8 still owns the installed-disabled progress wording/concurrent-client regression identified in P4.

### Reusable pattern candidates

Receipt-bound immutable credential heads, preserved native token anchors, complete-response-only recovery under central CAS, scoped disconnect/cancellation fences and exact-destination SDK OAuth I/O are candidates only, not approved reusable patterns.

### Phase completion

- Product/canonical head:64a57dbd489202da80507c361152decd9bb1cae1, fast-forward integrated by Cadre; phase worktree/branch removed.
- Human verification: not performed or claimed. Actual automated T5.6 evidence is authorized by persisted policy-v2 Autonomous. Local React UI remains P7; full release acceptance remains P8/P9. External-provider, named commercial-agent and older-Mac compatibility are not claimed.

## Phase 6: Administrator React website

### Inputs read

Approved project instructions and complete spec/plan; declared P2 learning; actual public API/router/contracts; frontend builder, React and shadcn composition/testing guidance; three native ImageGen references and current product sources. No applicable repository AGENTS instructions.

### Task observations

- T6.1: `59a60a018461b0506f183305aa9a9db8e459962c` records the coordinated catalog, tool-policy and group-membership references and production design/API/accessibility contract.
- T6.2/T6.3: `ee84d90ead2431af8cee1483bccb48e52a3b9fe9` delivers central React/strict-TypeScript/shadcn login, catalog/connection/tool policy, additive assignments, users/groups/membership and explicit destructive confirmation. It embeds compiled assets in Rust and repairs stale browser-cookie re-login without weakening native session rejection.
- Ten frontend behavioral tests, lint/typecheck/format/build, standalone strict fixture typecheck, server fmt/Clippy-Dwarnings/build and six explicitly executed PostgreSQL identity/configuration/vault tests passed. An ordinary external-key permissions test also passed. Embedded index/script/security headers and unknown/traversal rejection passed.
- Actual in-app-browser journeys used production Rust-served assets on macOS27.0.1 arm64: login/logout/re-login, observed/manual policy save, group membership, structured commands, assignments, account creation, ordinary-role denial, real409 explicit reload, required-input feedback, long names and empty search. Desktop1586×992, laptop1200×900 and narrow390×844 checked; keyboard focus trap/Escape/return and responsive regions passed, with no relevant console error/overlay.
- All three reference/latest screenshot pairs were inspected with view_image. docs/admin-ui.md holds the fidelity ledger, copy diff, usage/build/fixture instructions and limits. Four final screenshots are user-facing files outside the repository; disposable capture directories, server/key directories and unique fixture databases were cleaned.

### Decisions and corrections

- Frozen editor revisions and full read-set reload resolve concurrency; never auto-retry stale writes. Saved follows server confirmation; failed follow-up reads prevent another write until reload.
- Runtime-decoded public DTOs, in-memory CSRF, HttpOnly cookies, ten-second fetch deadlines and cleared password inputs preserve browser/native separation. Administrator screens never resolve user credential values or execute commands centrally.
- True white canvas, cool gray rail, forest controls and Geist/Lucide typography continue the references. A faithful code-native three-node mark, shadcn InputGroup addon, real administrator membership and server row ordering are documented intentional details.
- Fixed initial control-size/column drift. Catalog minimum width1100 keeps action buttons inside columns at laptop width; labelled keyboard-focusable horizontal regions preserve narrow access without page overflow. Sheets retain footer actions and restore focus to their opening control.
- Rust build-time bounded embedding rejects symlinks/unknown file types; no runtime filesystem lookup. Missing assets produce actionable503. Same-origin CSP permits Radix inline positioning styles but excludes inline/external scripts.
- Cadre integration permits owned journal/state changes but rejects uncommitted learning. A named path-scoped Git stash preserved exact P1/P2 learning during the fast-forward; restoration matched its SHA before phase cleanup. No product or learning was discarded.

### Reusable pattern candidates

Frozen-revision editor recovery, confirmed-write/read-failure separation, runtime-decoded UI contracts and bounded compile-time asset embedding are candidates only, not approved patterns.

### Phase completion

- Commit: ee84d90ead2431af8cee1483bccb48e52a3b9fe9, fast-forward integrated into canonical main; phase worktree removed through Cadre.
- Human verification: not performed or claimed. Phase checks authorized by persisted policy-v2 Autonomous at T6.4. Safari/Firefox, older macOS, hosted CI, endpoint installation and actual remote OAuth remain separate/unvalidated scopes at this phase.

## Phase 7: Local user React website

### Inputs read

Full approved project/spec/plan, declared P3/P5 learning and current native/core/server contracts, actual shared admin primitives, frontend builder/React/shadcn/imagegen/testing guidance. No applicable AGENTS instructions. Main executed the single ready phase without workers.

### Task observations

- T7.1: `6a0d0e951ba5f59f7b8db685bf4dbe7ac0ddc558` records five inspected generated concept references, visual/copy/component conventions and protected generation/online-only flow contracts.
- T7.2/T7.3: `828be9a37bd0fc518011e88774e2e82153a5b58f` delivers the actual embedded local React/TypeScript/shadcn app: login/catalog/install/confirmed settings, write-only central secret CRUD/owned bearer selection, OAuth consent/reconnect/disconnect/cancel, redacted native diagnostics and offline/expiry status.
- Final frontend20 tests/lint/typecheck/format/build pass. Endpoint/core full65 tests pass plus one intentional subprocess-helper marker (parent invokes actual Mac helper); four-auth-mode actual CLI journey37.68s and management suite38.50s pass. Real PostgreSQL/SDK/MacKeychain OAuth regression56.19s passes with pending status before/during exchange. Workspace Clippy-Dwarnings/fmt/build, generated-contract equality and diff checks pass. Hosted CI is not claimed.
- Actual macOS27.0.1 arm64/IAB compiled assets verified at1584x993/1200x900/390x844: login, explicit install then enable, owned bearer selection, secret create/rename/replace, bound-delete rejection then disable/delete, real OAuth callback/explicit enable/disconnect, central outage with unchanged validation time/read-only/countdown and reconnect, Alice/Bob isolation/logout, empty search and keyboard trap/Escape return. Native-size concept/latest-render view_image comparison and copy/fidelity ledger recorded in docs/local-ui.md. Final source rebuild smoke passed.

### Decisions and corrections

- Local native HTML mode selects a separate app; central administrator UI behavior is preserved. Existing SVG mark/shadcn primitives and semantic palette remain shared; local geometry and portaled sheets are explicitly scoped. No raster screenshot is shipped as UI.
- Browser state holds CSRF only in memory; no native token, cache key, config or secret value in browser storage/read APIs. Public native status adds only username. Every read/logout/action pins original generation; identity changes unmount/abort prior account data and late context responses cannot restore it.
- Central enablement is explicit primary state; native installation/auth/availability are separate diagnostics. Catalog diagnostics inspect files/Keychain only and never launch commands. Empty allowlists are blocked. No optimistic enabled success or offline mutation queue.
- Settings preserve editor-opening revision and conflicts require explicit reload. Secret inputs are write-only/cleared after each attempt; stable owned IDs survive rename/replacement. Active bindings prevent deletion. Generic conflict feedback includes active-binding/version review rather than claiming every conflict changed a revision.
- Installation confirms the administrator's command, reports private outcomes, permits explicit optional enable-after-success and separate reinstall. Progress receipts never establish authoritative enablement. P8 still owns the native installed-disabled wording/concurrent-client regression identified in P4/P5.
- OAuth provider navigation is explicit; connected/native-ready/server-enabled states are separate. A real browser exposed status polling rotating a protected receipt during callback persistence. Pending-flow feedback is now ephemeral and does not refresh/recover during its callback; complete issued-token journaling uses the current scope receipt and at most three local-publication retries, never another provider exchange. Real regression includes pending polls and cancellation while provider exchange is in progress.
- Browser multistep native operations have a90s deadline; each native control-plane operation retains10s. Online browser reconnection requests complete native refresh; native startup/60s loop remains authoritative. One-hour local cookie renewal requires page reload independently of policy lease. Inline styles support Radix; same-origin-only scripts, Host/Origin/cookie/CSRF protection remain.
- First concepts added window chrome/inconsistent background nav; refined primary catalog and explicitly bound secondary form references to it. Corrected catalog typography/row rhythm and secret header positioning/sheet width. JSDOM scrollIntoView/fractional-timestamp fixture issues were corrected without weakening product checks. Responsive viewport initially targeted the OAuth popup; closed it and verified the actual390px management document width.
- Three owned browser fixtures cleaned their unique databases/private directories/Keychain journals through an immutable copy of the creating CLI. Exact paused central PID84043 was resumed before cleanup. QA tabs closed and viewport reset; production selection was untouched. Main preserved root learning through exact path-scoped stashb233b3b4, restored SHA0328b71c unchanged, then dropped only that owned stash.

### Reusable pattern candidates

Generation-keyed UI lifetimes, confirmed-state controls, write-only secret forms, ephemeral consent feedback and current-receipt-only local token publication are candidates only, not approved patterns.

### Phase completion

- Product/canonical head:828be9a37bd0fc518011e88774e2e82153a5b58f, fast-forward integrated by Cadre; phase worktree/branch removed.
- Human verification: not performed or claimed. Actual automated/browser T7.4 evidence authorized by persisted policy-v2 Autonomous. Release integration/packaging remains P8/P9; olderMac/Safari/Firefox/externalprovider/named commercial-agent/hostedCI compatibility is not claimed.

## Phase 8: Complete MVP integration, deployment and Mac packaging

### Inputs read

Full approved project/spec/plan and declared P4/P5/P6/P7 learning; actual runtime/native/server/test and deployment source; primary Docker/Caddy and pinned Rust/Rustix source/issue evidence. No applicable AGENTS instructions. Main executed both cohesive groups without workers.

### Task observations

- T8.1:`bb74928cfc0e96ee682b998d75da5a7195b691a9` records cumulative acceptance mapping and fixes stale installer wording. A concurrent-client regression proves confirmed enablement remains primary while cancellation/revocation receipts describe only that attempt.
- T8.2/T8.3:`4469f1aaba87f2f1d447900da28afc2a78bfe860` delivers nonroot/read-only container/Compose/health/external-key/TLS guidance, arm64 embedded archive/checksum, deployment/restore/Mac/agent/provider/compatibility runbooks and executable rehearsal/DoD.
- Final workspace81 tests pass (one Mac child marker invoked by its parent, eight external DB cases separately executed); all8 real PostgreSQL tests and20 frontend tests pass. WorkspaceClippy-Dwarnings/fmt/build/contracts, strictTS/lint/format/production build/shell syntax/doc links pass. Actual final extracted-archive official rmcp four-auth-mode/signal/offline suite34.50s passes; native binary is byte-identical to the successful combined container/Node/Keychain rehearsal.
- Actual Linuxarm64 Docker29.8.2/Compose5.5.0/PG18.6 fresh migration/bootstrap/health/embedded UI/default-deny/roles/vault/restart passes. Real pg_dump/fresh pg_restore, copied-session invalidation/old-session401, wrong-key503/matching-key recovery and redacted logs pass. Actual macOS27.0.1 arm64 extracted CLI login/explicit install/enable→Node-spawned one STDIO exact-tool call→confirmed disable denial and creating-binary Keychain cleanup pass. P6/P7 compiled-browser/fidelity/keyboard/responsive evidence remains applicable to unchanged UI.

### Decisions and corrections

- Native pipe-only adapters failed actual Node child_process on Mac, whose stdio uses Unix socket pairs. Safe duplicated descriptors now select FIFO adapters or connected AF_UNIX STREAM Tokio adapters; files/terminals/datagrams/TCP reject. Native regressions prove byte preservation, idle-read cancellation, backpressure cancellation, EOF and invalid-type denial; existing actual pipe/signal/descendant suite still passes.
- Pinned Rust strip=true produced mis-aligned LINKEDIT proc-macro dylibs on macOS27, confirmed by dlopen and upstream rust-lang/rust157750. strip=false keeps release builds usable without changing the approved toolchain. Final Mach-O target27.0.1/system dependencies/checksum inspected; signing/notarization/publication remain deferred.
- Final archive SHA256:`afbda4e96c29cda7d3102d08bfb36d116cb725522ff7841ae2543dfe7fdf7bfe`. Local service imageID:`sha256:82303d778e91c982cbac86f0aaf9576e8b9e2fc83ec6709b570f919648cd2bd1`. Outputs remain in the execution-owned cache; no prod credentials or existing agent configs touched.
- Compose waits for database health, binds service loopback/private PG, mounts owner-only external keys, preserves exact public Host/Origin and uses SIGINT for central graceful shutdown. Restore uses a fresh project/database and clears copied sessions; old endpoint revisions require logout/login rather than bypassing protected checks. Old vault keys remain independently retained for backup recovery.
- Rehearsal corrected Node fetch Host rewriting/keepalive, group memberIds, expectedGeneration/202 acceptance and stable MCP-ID lookup without weakening product checks. Six failed generated fixtures removed only after exact project/volume absence and empty native identity/accounts/pending-cleanup metadata. Success removed both owned stacks/volumes/native journals. Old execution-owned Cargo targets cleaned for disk space; four temporary first-pass QA captures removed, final screenshots retained.
- Main preserved root learning with exact path-scoped stash10123c99; restored identical SHA4ac5d7ba then removed only that stash. P1–P7 learning remains byte-identical. No actual public certificate, external provider, named commercial agent, Safari/Firefox/olderMac, actual sleep/reboot or hostedCI claim.

### Reusable pattern candidates

Typed nonblocking STDIO descriptor selection, receipt-only install outcomes, immutable creating-binary release rehearsals and independent-key fresh restore proofs are candidates only, not approved reusable patterns.

### Phase completion

- Product/canonical head:4469f1aaba87f2f1d447900da28afc2a78bfe860, fast-forward integrated through Cadre; phase worktree/branch removed.
- Human verification: not performed or claimed. Actual automated/native/browser evidence authorized by persisted policy-v2 Autonomous at T8.4. P9 and autonomous review/final explicit completion approval remain.

## Phase 9: Track-level User Manual Verification

### Inputs read

Complete current approved spec/plan/project instructions, declared P1–P8 learning and current handoffs; canonical product/source/release/acceptance and Git/worktree evidence. Full context retrieval completed with unchanged sections retained by token/hash. No applicable AGENTS instructions.

### Task observations

- T9.1 automated verification completed against fully integrated canonical4469f1aaba87f2f1d447900da28afc2a78bfe860 under persisted policy-v2 Autonomous. Sole main worktree, Cadre-only dirt, committed/canonical bytes, archive/runbook checksum agreement, generated-fixture absence and rustfmt checked.
- Unchanged final product verification reused:81 workspace Rust tests,8 explicit real PostgreSQL tests,20 frontend tests, Clippy-Dwarnings/fmt/strictTS/lint/format/build/contracts; actual Mac Keychain/native time/cache/process/pipe/socket/4auth journeys; final extracted archive official client34.50s; combined container→packaged Mac→Node one-entry MCP install/enable/call/disable proof; migration/restart/dump/fresh restore/copied-session401/wrong-key503/matching-key recovery/redacted logs; compiled browser/fidelity/keyboard/responsive P6/P7 evidence.
- docs/acceptance.md maps all AC001–012; docs/release.md records final artifact/DoD and exact supported/untested surfaces. No missing required human-only or external prerequisite was identified for the approved fixture-backed MVP.

### Decisions and corrections

- Fast-forward integration changes no product bytes; reuse passing evidence while source/test inputs/policy remain unchanged. Cadre bookkeeping does not invalidate it. Final implementation HEAD remains4469f1a, never replace it with a bookkeeping commit.
- Human clean-completion approval remains an explicit later review boundary. Track verification is automated, not human approval. Signing/publication/SAML/SaaS/extra platforms/providers/agents/hostedCI/actual machine sleep-reboot remain deferred or unvalidated as documented.

### Reusable pattern candidates

Canonical source/artifact identity checks with reused unchanged verification are candidates only, not approved patterns.

### Phase completion

- Product/canonical head:4469f1aaba87f2f1d447900da28afc2a78bfe860. All implementation nodes complete; phase/workers removed.
- Human verification: not performed or claimed. Actual automated evidence authorized by persisted policy-v2 Autonomous; implementation bookkeeping then autonomous review/remediation and one explicit final completion decision remain.

## Phase 10: Bounded endpoint credential metadata

### Inputs read

Complete current approved project/spec/plan and preserved P1–P9 learning, BUG-001, native metadata/credential/cache/session code and tests. Original pattern catalog remains empty. Main executes directly; no worker/worktree required for this focused change.

### Task observations

- T10.1: `798b0fb` centralizes metadata schema/ownership/pointer/count validation for reads and writes, preflights new credential head/pending capacity before journaling or Keychain effects, and exposes actionable bounded-capacity recovery.
- Full CLI tests:61 passed plus one intentional Mac subprocess helper marker invoked by its parent. Eight native credential tests include128/129 sequential MCPs with one assigned at a time, preserved identity/cache/session/existing-credential revalidation and complete logout cleanup. Actual Mac Keychain/cache restart, four-auth-mode CLI41.49s, management39.03s and identity/protocol suites pass. All-target Clippy-Dwarnings/fmt/diff pass.

### Decisions and corrections

- Keep bounded128 head identities per login generation, including retired MCP identities; do not weaken reader bounds or protected rollback/publication/token-anchor fences. Existing head updates remain available at capacity; logout and online sign-in clear retired generation state. Ordinary metadata never gains token values.
- Shared writer validation rejects malformed publication before replacing valid durable bytes. Raw attack replay tests must bypass the normal writer; the existing fixture now asserts write rejection then tampers directly, preserving the negative security test.
- Initial move left three multiline metadata identifiers; compiler caught these and they were corrected before verification. Reused original execution-owned Cargo/assets to avoid duplicating20GiB build dependencies; current execution owns test logs. No production credentials touched.

### Phase completion

- Product/canonical commit:798b0fb; actual automated phase verification is authorized by persisted policy-v2 Autonomous. No human verification or approval claimed.

## Phase 11: Consistent administrator state and lossless catalog editing

### Inputs read

Full approved current spec/plan/project guidance, P9 evidence and BUG-002/003/004, relevant central transaction/identity/catalog contracts and browser/native UI/callers/tests. React/shadcn/testing skills and current official Field/Textarea documentation were applied. Main executes directly with no workers/worktrees.

### Task observations

- T11.1/T11.2 deliver shared Rust-compatible UTF-8 name/environment bounds, lossless JSON command editing and consistent revision-bracketed admin reads for both initial and post-save loading. Long title overflow found in actual browser was fixed with shared visual truncation/full accessible name.
- Final34 frontend tests/strictTS/lint/format/production/embedded builds pass. Boundary data includes valid200-byte ASCII/Unicode names and129-byte environment keys, invalid overlimits, literal empty/multiline runtime/install arrays/maps and explicit edits. Deterministic initial group/account races and post-confirmation race/network failures deny mixed read sets and never replay a mutation.
- Actual macOS27.0.1 arm64 compiled Rust websites/IAB at1584x993 and390x844: admin login, long catalog/editor, policy-only save with backend byte-exact command witness, invalidJSON feedback/valid explicit save, second-admin member removal/stale409/explicit reload retaining removal, Escape/focus and no page overflow; native login/long catalog/manage/confirmed enable-disable/refresh/logout pass. No relevant console errors/overlay; screenshots retained outside repository.

### Decisions and corrections

- Both identity arrays must belong to the global revision used by editors. Configuration reads bracket parallel identity/group reads; mismatch fails explicitly, without automatic write replay. Server lock/repeatable-read behavior and every relevant bump were checked. Preserve frozen revisions and saved-but-followup-read-failed semantics.
- JSON arrays and string maps keep literal values rather than newline joining/splitting. Browser limits count UTF-8 bytes; no existing server data contract tightened. Truncated headings retain full accessible names and visible close controls.
- Browser plugin unavailable; package Playwright exists but its browser binary is absent. Existing CUA in-app browser was used without dependency installation. Exact viewport CDP screenshots corrected scaled capture artifacts. One interrupted fixture was recovered only by exact PID absence/creatingCLI logout/empty journals/owned DB witnesses. Final fixtures cleaned their Keychain/native state, child PIDs, unique databases/files; QA tabs closed and viewport reset. Production selection untouched.

### Phase completion

- Current browser group product/canonical commit recorded in execution journal. Human verification not performed or claimed; actual automated/browser evidence is authorized by original persisted policy-v2 Autonomous authority. P12/P13 and cumulative clean review remain.

## Phase 12: Rebuilt and verified MVP artifacts

### Inputs read

Complete approved current scope/plan/project guidance, declared P10/P11 fixes/learning and P1–P9 cumulative acceptance evidence; current package/deploy/restore/client test and runbook source. Main executed direct canonical verification without workers/worktrees. No production DSN, provider, account, signing identity or agent configuration was accepted.

### Task observations

- T12.1 rebuilds the reviewed service image and arm64 embedded archive, updates acceptance/release/design/Mac/compatibility runbooks and records current artifact identities. All original AC001–012 and four regression witnesses are covered.
- Final workspace83 Rust tests pass;9 intentional ignored markers comprise the Mac child invoked by its parent and8 DB tests separately executed. All8 realPG tests pass, including actual SDK/MacKeychain OAuth71.49s. Frontend34 tests, workspaceClippy-Dwarnings/fmt/build, generatedcontracts, strictTS/lint/format and production builds pass.
- Actual final extracted archive officialrmcp client suite38.48s passes local+none/bearer/OAuth remote, sameMac updates, cold valid-cache offline restart/reconnect, protocol-only stdout and signals/owned descendant cleanup. Fresh Linuxarm64 Compose→packagedMac compiledUI login/explicit install/enable→Node childprocess Unixsocketpair oneSTDIO exacttool call→confirmeddisable rejection/creatingbinaryKeychain cleanup passes.
- Fresh migrations/health/assets/defaultdeny/roles/vault, restart durability, pg_dump/freshpg_restore, copied-session401, independentwrong-key503/matchingkeyrecover and log redaction pass. Both owned stacks/volumes/private native directories removed. Sole canonical main, no stashes/worktrees and no active generated fixtures verified.

### Decisions and corrections

- Final archiveSHA256:1f35a72c4438e283fe039e65c0b338af858a0fa8b8e87896a2119691030a7349; local serviceimageID:sha256:2bee0c4125c011e7a619c7aa29144a8f9c3c4b02360b84010bd1a9e433c0ba21. Native binary and packaged README/COMPATIBILITY exactly match current source/build. Mach-Oarm64 min27.0.1/system dependencies inspected. Service and Mac build have identical production asset filenames.
- Current execution owns archive/assets/logs and extracted rehearsal output; original owned Cargo target is reused for dependencies. Compiled P11 browser/source bytes are unchanged, so its real login/commands/long-data/409-reload/confirmedsettings/keyboard/desktop/mobile/console evidence is reused. No additional code change followed those checks.
- docs/release.md and acceptance map now name83Rust/8PG/34frontend and all four review fixes. Original OS/provider/agent/CI/signing/SAML/SaaS limits remain explicit. No new failure or external/human-only verification requirement occurred.

### Phase completion

- Product/canonical head:b3ea6ab. Actual automated P12 evidence is authorized by original policy-v2 Autonomous; no human verification or approval claimed. P13 canonical verification, implementation bookkeeping and cumulative review remain.

## Phase 13: Track-level User Manual Verification

### Inputs read

Complete approved scope/plan/project guidance, P1–P12 learning and current execution handoffs; canonical product/artifact/log identity and initial review findings. Full scoped context is ready with no stale memory.

### Task observations

- T13.1 verifies integrated b3ea6ab52ce58249946831d67cba30106cbd8f90. Reused unchanged83 workspaceRust/8 realPG/34 frontend checks, actual compiled-browser evidence, final archive official client38.48s and disposable deployment-to-Mac-to-Node/restart/freshrestore/keyrecovery proofs. Source/artifact bytes and checksums match; no generated fixtures/worktrees/stashes remain.
- Separate review follow-up is confirmed: generic browser array10000 cap rejects valid10001 assignments/users/groups even though these backend collections have no such row cap. Actual source decoder probe accepts10000 and rejects10001 for all three. This recurs under stable finding catalog-decoder-contract-bounds; prior string-bound fixes remain verified. No clean completion is claimed; append remediation after execution bookkeeping and cumulative review.

### Decisions and phase completion

- Record actual available verification under original persisted policy-v2 Autonomous, separately from final human completion approval. Preserve completed phase identities/history; never reopen P10/P11 to hide the newly found contract gap. Current product head remains b3ea6ab52ce58249946831d67cba30106cbd8f90.

## Phase 14: Complete administrator collection contract

### Inputs read

Full current approved scope/plan/project guidance and P13 learning, BUG-002 recurrence, actual frontend/core/server/API/test callers. All inherited handoffs were verified identical to previously consumed source; scoped context completed with no stale memory. Direct main; no workers/worktrees.

### Task observations

- T14.1 c96f9c4 removes the arbitrary generic browser10k cap for server-unbounded users/groups/assignments and makes actual10k-member bound explicit. Every element still decodes.41frontendtests pass, including10000/10001/12000assignments, two coherent10001-row initial/follow-up readsets, malformedlateelements and explicitgroup/MCP/command/tool bounds. RealPG/API test returns10002users/10001groups/10001assignments in2.05s, cleans its exact DB. AlltargetClippy-Dwarnings/strictTS/lint/format/build/contracts/fmt/diff pass.
- T14.2 52c288b rebuilds Mac archiveSHA95d8e6283e1e77609154ddd771c8f3d5d4f46ef4eda4d2c45b6293178e5f4ce4 and imageIDsha256:3aa9d38940e78888f28b600fc15e27041a4634906a1bec8be4273b704a63c075. Actual finalarchive officialrmcp4auth/sameMac/offline/reconnect/protocol/signal/descendants38.60s passes. FreshCompose→MacUI/install/enable→NodeSTDIOexactcall→disabledenial plus creatingbinaryKeychaincleanup, restart/dump/freshrestore/copiedsession401/wrongkey503/matchingkeyrecover/redactedlogs all pass. Package/checkfile/executable/currentdocs/runbook identities, minOS27.0.1 and exactfixture/stack/volumeabsence verified.

### Decisions and corrections

- Preserve actual bounded fields and complete runtime validation; no tightened backend data, new pagination or enterprise-scale performance claim. Only browser array contract changed, Rust runtime and previous security/native/DB testinputs/layouts unchanged. Reuse83workspace/8priorPG and compiledbrowser layout evidence with source identity; addedPGcase makes9total cases. Current41frontend plus finalpackaged/deployment proofs run afresh.
- Current owned cache holds assets/archive/logs; original ownedCargo target reused. Initial contract-check invocation lacked owned build variable; rerun passed. Single-task complete_group requires task scope; initial phase-scope call rejected without mutation, correctedT14.1 completed atomically. No production scope touched.

### Phase completion

- Canonical product head52c288b; actual automated P14barrier authorized by original persisted policy-v2 Autonomous. Human approval not performed or claimed; canonicalP15 then cumulative review and finalexplicitapproval remain.

## Phase 15: Track-level User Manual Verification

### Inputs and observations

Complete current approved scope/plan, P1–P14 learning and current handoffs consumed; contextReady/no stale memory. T15.1 verifies fully integrated52c288b70ad0d8839aea8b3f12952ecd3e0cb26d. Current41frontend/addedrealPG collection test, strictTS/lint/format/build/contracts/alltargetClippy/fmt pass. Source-unchanged83workspace/8priorPG/security/native/compiledbrowserlayout evidence reused. Finalextracted officialclient38.60s and freshcontainer/MacUI/NodeSTDIO/restart/dump/freshrestore/sessioninvalidation/wrong-matchingkey/redactedlogs/creatingbinaryKeychaincleanup allpass. Package/executable/docs/checkfile/runbookidentities confirmed, no productdirt/generatedfixtures/worktrees/stashes; solecanonicalmain.

### Phase completion

All implementation nodes complete under original persisted policy-v2 Autonomous. No human approval or clean-review result claimed; cumulative review now decides findings against originalbaseline through52c288b. Bookkeeping preserves producthead and all phase history. Final explicit human clean-completion approval remains.
