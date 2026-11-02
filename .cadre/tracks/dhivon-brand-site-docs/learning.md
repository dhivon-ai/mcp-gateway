# Incremental Learning: Dhivon MCP Gateway rebrand, product website and documentation

Read this seed at each root phase. Later phases read the learning from every declared dependency phase. Append verified observations and handoffs during execution.

<!-- cadre:pattern-seed:start -->
## Pattern Seed

<!-- cadre:memory:start -->
{"schemaVersion":1,"specRevision":1,"planRevision":4,"patterns":[{"path":"patterns/authorization-leases.md","sha256":"a949846db0786cbcc3b82b93a073f6a0427175fc8c02033d7fcc771db11360c8"},{"path":"patterns/browser-contract-consistency.md","sha256":"85449fd4523c3f17cb692d2cfaea42497ef85870884f2795d963fe4a17d6931d"},{"path":"patterns/credential-publication.md","sha256":"28346d9e885a27b8d0334138788cb40eaee2f1b1f395b670d33cf1bd74dfb7b3"},{"path":"patterns/isolated-release-verification.md","sha256":"3453134357e4efd47faaca9080982f997ea6485ff7b9297504f11339e85dbd5a"},{"path":"patterns/owned-mcp-transports.md","sha256":"d407bb97d2b46c96f9b109e6c18af6fcb769a2d4ed52051c4d4d6f8f808083a3"}]}
<!-- cadre:memory:end -->

- Spec revision: 1
- Plan revision: 4

| Pattern | Relevance to this track | Constraints to carry forward |
| --- | --- | --- |
| `patterns/authorization-leases.md` | Security and troubleshooting guides explain revocation and offline use. | Only a complete durable sync renews the lease. Enforce the earliest deadline, pinned generation and per-call policy. Expiry denies use and offline mutations are rejected. |
| `patterns/browser-contract-consistency.md` | Both UI shells and forms change. | Keep coherent revisions, explicit conflict reload, lossless argument/environment editing, backend-aligned decoding, stable account identities and keyboard/secret controls. |
| `patterns/credential-publication.md` | Fresh native namespaces and remote-auth guides touch ownership and cleanup. | Preserve protected heads, scoped Keychain accounts, bounded metadata, preflight and retirement journals. Preserve complete OAuth CAS recovery, token lifetimes, origin pinning and metadata-only UI reads. |
| `patterns/isolated-release-verification.md` | CLI names, packaged assets and docs all change the release inputs. | Test the extracted binary and actual image in generated private scopes. Retain the creating CLI through verified logout. Exercise restart, fresh restore, independent keys and redaction; label limits. |
| `patterns/owned-mcp-transports.md` | The public diagram, docs and agent rehearsal describe the actual gateway. | Document the tested tools-only profile, bounds and alias mapping. Preserve per-dispatch policy, protocol-only stdout, no automatic replay and ownership-safe child cleanup. |
<!-- cadre:pattern-seed:end -->

## Phase learning

Awaiting execution.

## Phase 1 — Product identity and approved assets

Renamed Cargo packages/directories, binaries, native namespaces, cookies, build/deploy environment keys and developer tooling in 757d8b169cedc6578742c8a99e10fea45b24e1d1. The approved brand kit and TTF embedding support arrived in bb057261035dadea4a830e5ef66e9a092bae6ce0, integrated by 3ca90793f24df65b376e2b3930f7c4babf248351.

Locked workspace check/build, embedded frontend production build, generated contracts, frozen pnpm install, formatting, 44 frontend tests, fresh Compose configuration/start and CLI version/help passed. Native cache and identity suites passed. Full native gateway passed on a stable-binary retry after an initial run overlapped a rebuild. PostgreSQL tests needed their safety guard renamed with the isolated fixture database. Extended OAuth verification is still running.

Keep exact original dependency pins. Dated historical evidence retains the identities it actually tested. Emerald against white is 2.90:1 and is an accent. Use navy actions and darker semantic success text. Both asset packers support TTF. Shared assets live in assets/brand and need copying into the Docker web build stage when consumed.

An initial native gateway failure retained its private fixture journal. Reconcile using the creating binary before removing anything. Final release rehearsal must use an immutable extracted binary and record new artifact identity. Current project metadata branding requires the planned post-track Cadre refresh.

## Phase 2 — Administrator and developer interfaces

Shared/admin branding is committed in 03a0ae79e333cae95b3260ca8e3419df797d254b. Local identity and removal of unused Geist are committed in 2d2b662d1885b44b389687c545ebddb32a49b08a. Manrope, navy controls/sidebar and actual approved light/dark SVGs are bundled locally.

In-app Chromium exercised compiled admin login/catalog/editor, keyboard Escape/focus return and desktop/390px layouts. Native local login, status, installation feedback, server-confirmed enable and disable passed, with no relevant console errors or document overflow. Vite SVG data URIs initially failed CSP. External SVG imports fixed image delivery without weakening CSP. Final local sidebar typography correction is in source and will be checked again with packaged P5 assets.

Frontend 44 tests, lint, type/build pass. Native Rust suites passed; rustdoc subsequently observed deleted old web chunks while frontend output was rebuilt. A stable final-source rustdoc retry passes. Extended central/native OAuth passes sequentially in 30.93 seconds. Final checks must keep asset inputs and binaries stable for their entire run.

## Phase 3 — Product and operator documentation

T3.1 delivered README and role index, source-to-first-call quickstart, local/remote auth recipes and runnable Node fixture client in 3a927a3643fa8b435d8c197230ccef2361d5309c. T3.2 added architecture, security, symptom-specific troubleshooting, contributor and undated roadmap guides plus corrected current deployment/compatibility/identity/endpoint-management references in 3c5d4f3b65fffa45674c054ee1a6c0ee7905cbbd.

Canonical top-level Markdown is the site publication input. Guide links/anchors, shell/JSON/Node syntax, CLI options and runtime configuration were checked. The Node client called the SDK fixture using a protocol-only wrapper. Root's rebranded container and extracted Mac rehearsal then passed deployment/bootstrap, explicit install/enable, one agent STDIO call and disable denial, restart, fresh restore, independent wrong/matching keys and log redaction. P5 records final identities and any changed packaging inputs.

Historical release/acceptance/design/bug reports keep their original tested names. Roadmap labels the current delivery, has no approved follow-on track and gives candidates concrete outcomes/evidence without dates. Add a current release-report link only after that file exists.

## Phase 4 — Public product and documentation site

The static apps/site workspace renders canonical Markdown through pinned Marked 18.1.2 and copies the shared official brand kit. Commit 4a3ccc03f1334237204fc8ba23b328bf414241bc implements the public-site group. Outputs are separate from embedded management assets, with scoped build/preview commands.

Three Image Gen section concepts informed the white/navy editorial layout. Implementation uses original SVGs and dark links, omitting invented tool categories. IAB desktop/390px verified the rendered homepage, quickstart navigation, roadmap direct links/refresh, local Manrope, asset loading and no relevant console errors or document overflow. Whole-site crawl generated 31 pages and found a stale identity anchor, fixed in canonical source. Static typecheck, frozen install, build and formatting pass. P5 repeats link crawl after final report publication.

The public site can be served independently with the control plane and native management stopped. It explains the need, ownership and execution locations and concrete echo grant→enable→call→disable flow. Hosting and physical checkout administration follow the approved later handoff.

## Phase 5 — Release rehearsal and Definition of Done

Release verification and current evidence are committed in fe930285c9c18bafbab36a72635b600aba47c7b5. The audit also corrected stale development fallback branding and added a packaged-CLI override to the existing local browser fixture. CI now includes public-site checks.

Passing evidence includes 83 Rust workspace tests, 11 explicitly run PostgreSQL cases, 44 frontend tests, formatting/lint/type/Clippy/build and generated-contract checks. The public build contains 32 pages with zero internal link/anchor failures. Compiled browser checks cover all three experiences, desktop/390px, keyboard and role-specific behavior. Packaged local sign-in, missing bearer credential gating, mark/font delivery and mobile status navigation pass.

The final archive SHA-256 is 2467f67ededf51f41e14e4c542d392995ea3be29965bfe37c61538259a41f275. Its executable is byte-identical to the extracted tested binary d7a870f6f64a70272afe092d78472e44febc13714341eeeb1fd43804ab68ff68. The service image ID is sha256:31d889b0612b8e049d454d7c658e379ec5d1504a56f31662587256df21570e5b. The extracted all-auth agent suite passes in 32.03s. The final image/binary rehearsal passes container bootstrap, actual Keychain login, explicit install/enable, Node STDIO call, disable denial, restart, pg_dump/fresh pg_restore, independent wrong/matching keys and redacted logs.

All generated rehearsal stacks/volumes and native credentials are cleaned through their owning fixtures. The early retained failure journal was logged out, confirmed credential/publication/cleanup lists empty and removed. Local-only static preview remains on port 4173. The runtime cache retains deliverable archive/checksum, site and test evidence.

Current product source is complete. Track mode now requires explicit human track-level verification before execution_finish and ordinary review. Post-track project metadata refresh, physical checkout naming and hosting follow the approved handoff. Preserve historical Cadre and release identities.

## Review cycle 1 — Consolidated findings and deferred remediation

The completed implementation range and subsequent owner-requested OAuth/license
documentation were reviewed through d16c28a1ea2c63d56e1f1cca939ff7b0ef3faab1.
BUG-006 retains the hosted HTTPS control-plane/native-loopback verification
follow-up. BUG-007 records the confirmed omission of full copyright/license
notices from distributed artifacts. The report includes reproduction and
acceptance criteria and distinguishes controlled-fixture evidence from a real
provider/deployment.

The owner selected Apache-2.0 with Dhivon AI copyright. Source LICENSE/NOTICE and
Cargo/npm metadata were added before the instruction to defer implementation,
then reverted at the owner's explicit request along with the three supplemental
OAuth/report commits. Git confirms all non-Cadre tracked content again matches
the completed execution HEAD. The license decision and OAuth requirements remain
in the consolidated findings and pending plan. No remediation execution or
deployment has started. Completed tasks and verification barriers are preserved.

Pattern applicability was reassessed for spec1/plan2. Isolated release verification
governs notice-bearing archive/container identity and clean fixture ownership.
Credential publication, native leases and owned transports govern real-provider
OAuth validation and protected loopback/native behavior. Browser contract
consistency remains relevant to any visible ownership/notice surfaces.
All original pattern content hashes remain current.

## Phase 7 — Project licensing and distribution notices

The owner-approved Apache-2.0/Dhivon AI source decision and delivery notices are
implemented in 327d978. Original third-party OFL bytes are unchanged. Actual
archive, runtime image and static output pass source-byte comparisons; the older
archive fails the new check. Packaging, deployment rehearsal and website CI now
verify emitted notices. The site exposes a readable ownership guide and full text.

Pinned Node24.19.0 checks replace an initial system Node26 invocation. Site/script
TypeScript, frozen install, format and shell syntax pass. All four Cargo packages
report Apache-2.0/Dhivon AI. The 33-page site crawl finds no broken links/anchors.
IAB licensing navigation/text delivery and 1280px/390px no-overflow/console checks
pass. The extracted all-auth agent suite passes34.28s; final archive executable
matches the tested immutable binary. Actual notice-bearing container/Mac/Node
call/disable, restart, fresh restore, independent keys and redacted-log cleanup
rehearsal passes. Exact current and historical identities are in docs/dhivon-release.md.

For P8 the owner selected an isolated Docker HTTPS simulation instead of a public
deployment, then explicitly approved temporary user-domain SSL trust restricted
to localhost for the generated one-day CA, with trust/certificate cleanup. Keep
certificate validation enabled. Label controlled-provider evidence accurately.

## Phase 8 — Docker HTTPS and actual native OAuth

The owner selected a Docker HTTPS simulation instead of public hosting. Product
3d6eecb delivers a reproducible private Compose/nginx TLS rehearsal and an HTTPS
advertised-origin option confined to the loopback-only fixture. Production TLS
and hostname verification remain unchanged. The extracted Mac signed in/synced
over HTTPS; Safari followed the HTTPS authorization redirect to the actual native
127.0.0.1 listener and displayed OAuth connected without a certificate warning.
Chromium rejected the constrained temporary CA; no warning was bypassed.

The initial expiry test found a real resolver defect: publishing the centrally
returned expired token hit its scoped rejection marker before reaching refresh.
The fix validates that record and uses existing version/lease-checked refresh
before old-token publication. Actual final evidence includes an old-token
rejection, credential version1→2, permitted tool call, disconnect and subsequent
denial, with no automatic replay. Consent alone never enables; management restart
preserves credentials. State/issuer negatives preserve the live flow.

The initial nginx startup needed a health dependency. An expired browser window
was cleaned and replaced with a fresh Safari flow. Discovery polling accommodates
scheduled reconciliation without replaying tool calls. All83 Rust tests, CLI
all-target Clippy/fmt, strict script TS/format, extracted all-auth suite47.13s,
current archive/image/site notice checks and 34-page links/anchors pass. Final
archive/executable and unchanged service-image identities are in docs/dhivon-release.md.

Creating binaries confirmed empty native credentials/cleanup lists. Exact private
Compose resources were removed. The owner authorized temporary localhost-only
SSL trust and the OS cleanup dialog. Trust removal exited0, the certificate is
absent and its leaf now fails validation. Generated private certificate keys were
removed. Public DNS/hosting certificates, real-user consent and named external
providers remain outside this controlled evidence. Final P9 requires explicit
human verification under Track mode; no final approval has been recorded.

## Phase 9 — Final track-level user verification

The owner explicitly replied “approve” to the final verification proposal for
product3d6eecb, including licensing/distribution notices, the Docker HTTPS/native
OAuth flow and refresh fix, passing checks and cleanup. The presented limits
retain unverified public hosting and named external-provider compatibility.
Git confirms no product drift since that verified commit; 2c272be is Cadre-only
checkpoint bookkeeping. This approval completes the Track-mode implementation
verification gate; clean review/completion remains a separate lifecycle decision.

## Review cycle 2 — Interrupted rehearsal cleanup

Reviewed 69ba295..2c272be and completion-only changes through 9f5bf22. Prior
licensing requirements pass actual artifact byte checks; the owner-selected
Docker HTTPS/native OAuth scenario and refresh fix retain their evidence and
public-provider limits. Eight focused credential/rejection tests pass; no new
runtime defect was confirmed.

A signal probe of the actual HTTPS script exited -15 after SIGTERM, retained a
PostgreSQL container and volume, and wrote no result record. The probe stopped
before native login and installed no trust. Exact generated resources were then
removed and their absence verified. BUG-008 proposes supervised idempotent
cleanup and durable ownership/recovery before effects. No fix is implemented.

Patterns were reassessed for spec1/plan3. Isolated release verification requires
scoped ownership, creating-binary logout and retained failure recovery. Credential
publication and authorization leases constrain late cleanup and native identity
isolation. Owned transports govern child shutdown; browser consistency remains
applicable to retained product flows. All five pattern hashes remain unchanged
and current. Preserve completed P1–P9 and their historical evidence.

## Phase 10 — Interruption-safe rehearsal ownership and recovery

Product commit 03764affe7f76d019a24e34c020ed68d40eb14cf implements BUG-008.
The rehearsal persists private ownership, creating-CLI/config hashes and child
PID/start/command identities before effects. Cancellable asynchronous commands
replace blocking Docker calls. SIGINT/SIGTERM and ordinary failures converge on
one bounded, idempotent cleanup; malformed agent output also enters that path.
Diagnostic failures cannot skip independent teardown. Owned writers stop before
creating-binary logout; incomplete native cleanup preserves central resources.
Recovery validates scope/hashes and refuses active supervisors/concurrent recovery.
Failed attempts and failed verification outcomes survive successful cleanup retry.
Certificate trust remains explicitly owned by the operator, outside these scripts.

The pinned Node24.19.0 process suite passes11/11 in149.68s, covering ten scenarios:
both signals after Docker startup and real Mac login, normal completion, ordinary
failure, diagnostic failure, incomplete stack/native cleanup with repeated recovery,
and the original HTTPS entry point interrupted during PostgreSQL startup. Repeated
signals, an interrupt-ignoring child, active-supervisor/changed-input refusal,
independent exact child/container/network/volume/credential checks and preservation
of an unrelated process pass. All28 generated scopes across exploratory and final
runs are clean. Current execution-owned cache retains private per-run manifests,
cleanup-test-results.json, cleanup-tests-verified.log, artifact identities and link
check evidence. The runbook and docs/dhivon-release.md publish the redacted outcomes.

Script/site strict TypeScript, formatting, static build,34-page link/anchor crawl,
archive/image/site notice-byte checks and Git diff checks pass. Actual CLI SHA
f36d5e229cf3fd2bf97ea780340b0304a605f6b848044755754ecee64c83d5d7,
archive SHA e66ae212267e01f7af952db662c6da33edc284e8529e07bc40cb142134092799
and service image sha256:7c6b562735589c7dbed0552c26a7d3f3fda20e6eb5fa773589c7364ae7ae0b07
are unchanged. Product Rust/UI/license inputs match the prior verified baseline.
The full HTTPS/browser/OAuth acceptance is reused for those unchanged artifacts;
cleanup probes make no new TLS/public-provider claim and install no new trust.

P10 verification is authorized by persisted Track mode, not a human final approval.
Next: explicit P11 Track-level User Manual Verification of product03764af and these
outcomes, then execution_finish and ordinary Cadre review. Preserve P1–P9 history.

## Phase 11 — Final track-level user verification

The owner explicitly replied “approve” to the final verification proposal for
product03764af: BUG-008 cleanup/recovery, passing11-test and artifact/notice/type/
format/site-link evidence, and transition to Ready for Review. Canonical5caf554
contains only Cadre checkpoint bookkeeping after that product commit; Git confirms
no product drift and the working tree was clean at approval recording. Reuse the
verified evidence and retained original HTTPS/OAuth/provider limits. This approval
satisfies the Track-mode final implementation gate; ordinary review still owns
findings and clean completion. Finish and reconcile implementation bookkeeping.


## Review cycle 3 — Ownership registration and recovery exclusion

Reviewed completed execution2026-11-02T09-16-20-231Z, range04656bd..5caf554,
and Cadre-only completion193f4ba. All seven changed product files and affected
callers/tests/runbooks were read. The existing process suite passes11/11 again in
149.99s. Strict script TypeScript/format and actual archive/image/site notice-byte
checks pass. Unchanged Rust/UI/licensing inputs retain prior artifact and
HTTPS/native OAuth evidence and public-provider limits. No new trust is created.

BUG-008's original signal leak is prevented by the tested normal paths. New
BUG-009 shows that failed post-spawn identity capture leaves no registered handle
or owner: cleanup reports clean while the generated child is alive. The reviewer
stopped only that exact child after recording the outcome. BUG-010 shows two
stale-lock contenders both acquiring cleanup ownership; a separate interrupted
acquisition leaves a zero-byte lock that blocks retry with a parse error. These
are verification-harness defects, not confirmed production OAuth regressions.

Controlled probes, results and logs are retained beneath the completed execution's
owned cache in review3-llrmvais. All13 generated review scopes are clean; the exact
probe child and recovery processes were reconciled, with no production targets or
certificate changes. Canonical source/state remains unchanged pending approval.
The proposal preserves all completed P1–P11 and appends P12 and final P13 only.

Reassessed the five seed patterns for spec1/plan4. Isolated release verification
requires truthful cleanup and retained immutable creators; owned transports require
ownership to survive registration errors and confirmed exact shutdown. Credential
publication requires stopped writers and serialized cleanup with no broad Keychain
removal. Authorization leases and browser contract consistency remain constraints
on unchanged product behavior. All approved pattern bytes/hashes remain current;
no dependency or specification scope changed. Implement only after this review's
exact bug/plan/learning/dependency/state proposal is approved and reconciled.

## Phase 12 — Fail-closed child ownership and exclusive recovery

Product9aba60c2341b3154f70a27ad04422bb812feef4a resolves BUG-009/BUG-010.
The actual ChildProcess handle/PID is retained before fallible identity capture or
persistence. Explicit intent/spawned/exited/not_spawned states distinguish proven
absence from uncertainty. Unknown ownership remains recovery_required and denies
native logout/stack removal. Cleanup also drains newly created cleanup-command
handles before logout and after teardown; the diagnostic-registration regression
proves a live interrupt-ignoring child is stopped, not merely omitted from results.

A permanent cleanup.lock inode carries an OS flock held by Node's descriptor.
A bounded Python3 stdlib helper acquires on the shared open file description;
its exit retains Node's lock. Closing the descriptor/process exit releases it,
without stale-owner unlink/recreate or owner-byte parsing. Empty/partial bytes
cannot strand recovery, and an old release cannot remove a successor's lock.
Recover acquires before reading the journal and holds ownership through all final
evidence writes. Python3 is required only for verification/recovery scripts on
the validated Mac/local filesystem; product/runtime dependencies are unchanged.

Combined pinned Node24.19.0 process verification passes25/25 in163.60s. Original
actual Docker/PostgreSQL/extracted-Mac scenarios pass; new cases cover identity/
persistence failure, signal during registration, failed diagnostic child capture,
ambiguous intent, simultaneous stale contenders, empty/held-lock forced exits,
successor-safe release, full-recovery journal freshness, missing Python and partial
lock bytes. A final focused14/14 pass10.87s after aligning the private probe env
variable to the product prefix. Strict script/site TS, scoped formatting, static
build,34-page link/anchor crawl, Git checks and actual archive/image/site project/
Manrope notice-byte checks pass. Final source hashes and both verification input
sets are retained in the current execution-owned recovery-artifact-identities.json.

All34 normal generated scopes are clean. Four deliberately ambiguous no-effect
intent fixtures retain failed journals as expected; none spawned that synthetic
intent, created native credentials or started Docker resources. Exact resource
checks confirm no generated containers/networks/volumes or native credentials
remain. Product CLI f36d5e229cf3fd2bf97ea780340b0304a605f6b848044755754ecee64c83d5d7,
archive e66ae212267e01f7af952db662c6da33edc284e8529e07bc40cb142134092799
and service image sha256:7c6b562735589c7dbed0552c26a7d3f3fda20e6eb5fa773589c7364ae7ae0b07
match prior verified artifacts. No Rust/UI/license or product OAuth-policy changes,
new OS trust or new public-provider claims. Prior full HTTPS/browser acceptance
remains evidence for unchanged product inputs; it was not repeated in this phase.

P12 phase verification uses persisted Track mode. Next is explicit P13 final human
verification of product9aba60c and these results, then execution_finish and ordinary
review. Preserve every completed P1–P11 phase and prior approval/history unchanged.
